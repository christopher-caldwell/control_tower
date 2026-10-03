use super::{
    dto::*,
    workspace::{WorkspaceContext, workspace_snapshot},
};
use axum::response::sse::Event as SseEvent;
use control_tower_application::{Direction, ExecutionProgress, Stage, WorkbenchState};
use serde::Serialize;
use std::{
    collections::HashMap,
    process,
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};
use tokio::sync::watch;

pub(super) struct ObservationStore {
    workspaces: Mutex<HashMap<String, Arc<WorkspaceRuntime>>>,
}
pub(super) struct WorkspaceRuntime {
    record: Mutex<WorkspaceRecord>,
    snapshots: watch::Sender<WorkspaceView>,
}
pub(super) struct WorkspaceRecord {
    pub(super) busy: bool,
    pub(super) observation: Option<MovementObservation>,
    pub(super) outputs: Vec<Option<CapturedOutput>>,
}
pub(super) struct CapturedOutput {
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}
pub(super) struct MovementPermit {
    runtime: Arc<WorkspaceRuntime>,
    completed: bool,
}

impl ObservationStore {
    pub(super) fn new() -> Self {
        Self {
            workspaces: Mutex::new(HashMap::new()),
        }
    }
    pub(super) fn workspace_state(
        &self,
        project_name: &str,
        workspace: &WorkspaceContext,
    ) -> Arc<WorkspaceRuntime> {
        let mut workspaces = lock(&self.workspaces);
        workspaces
            .entry(workspace.id.clone())
            .or_insert_with(|| WorkspaceRuntime::new(project_name, workspace))
            .clone()
    }
}
impl WorkspaceRuntime {
    fn new(project_name: &str, workspace: &WorkspaceContext) -> Arc<Self> {
        let snapshot = WorkspaceView {
            project_name: project_name.to_owned(),
            workspace: WorkspaceIdentity {
                id: workspace.id.clone(),
                name: workspace.name.clone(),
            },
            current_status: "unavailable",
            status_issue: Some("Current status has not been read yet.".to_owned()),
            checkpoint: None,
            movement_choices: Vec::new(),
            selected_stage_number: None,
            stages: Vec::new(),
            movement_busy: false,
            observation: None,
        };
        let (snapshots, _) = watch::channel(snapshot);
        Arc::new(Self {
            record: Mutex::new(WorkspaceRecord {
                busy: false,
                observation: None,
                outputs: Vec::new(),
            }),
            snapshots,
        })
    }
    pub(super) fn subscribe(&self) -> watch::Receiver<WorkspaceView> {
        self.snapshots.subscribe()
    }
    pub(super) fn current_snapshot(
        &self,
        project_name: &str,
        workspace: &WorkspaceContext,
    ) -> WorkspaceView {
        let record = lock(&self.record);
        self.publish(project_name, workspace, &record);
        self.snapshots.borrow().clone()
    }
    pub(super) fn try_admit(self: &Arc<Self>) -> Option<MovementPermit> {
        let mut record = lock(&self.record);
        if record.busy {
            return None;
        }
        record.busy = true;
        Some(MovementPermit {
            runtime: self.clone(),
            completed: false,
        })
    }
    pub(super) fn rejected(&self, project_name: &str, workspace: &WorkspaceContext) {
        let mut record = lock(&self.record);
        record.busy = false;
        self.publish(project_name, workspace, &record);
    }
    pub(super) fn admitted(
        &self,
        project_name: &str,
        workspace: &WorkspaceContext,
        operation_id: &str,
        direction: Direction,
        target_stage: u32,
    ) {
        let mut record = lock(&self.record);
        record.outputs.clear();
        record.observation = Some(MovementObservation {
            operation_id: operation_id.to_owned(),
            direction: direction.as_str(),
            target_stage,
            state: "running",
            active_role: None,
            role_results: Vec::new(),
            confirmed_checkpoint: None,
            attempted_checkpoint: None,
            failure: None,
            verification_choices: None,
        });
        self.publish(project_name, workspace, &record);
    }
    pub(super) fn observe(
        &self,
        project_name: &str,
        workspace: &WorkspaceContext,
        event: ExecutionProgress<'_>,
        started: &mut HashMap<(u32, &'static str), Instant>,
    ) {
        let mut record = lock(&self.record);
        let mut captured_output = None;
        {
            let Some(observation) = record.observation.as_mut() else {
                return;
            };
            match event {
                ExecutionProgress::Admitted { .. } => return,
                ExecutionProgress::Starting { stage, role } => {
                    started.insert((stage.number, role.as_str()), Instant::now());
                    observation.active_role = Some(RoleIdentity {
                        stage: stage_identity(stage),
                        role: role.as_str(),
                    });
                    observation.role_results.push(RoleObservation {
                        stage: stage_identity(stage),
                        role: role.as_str(),
                        state: "in_progress",
                        exit_code: None,
                        message: None,
                        elapsed_ms: None,
                        output_available: false,
                        stdout_bytes: 0,
                        stderr_bytes: 0,
                    });
                    record.outputs.push(None);
                }
                ExecutionProgress::Finished { stage, execution } => {
                    let index = observation.role_results.len().saturating_sub(1);
                    let elapsed = started
                        .remove(&(stage.number, execution.role.as_str()))
                        .map(|instant| {
                            instant.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                        });
                    let result = &mut observation.role_results[index];
                    result.elapsed_ms = elapsed;
                    match &execution.result {
                        Ok(output) => {
                            result.state = if output.success {
                                "succeeded"
                            } else {
                                "failed"
                            };
                            result.exit_code = output.exit_code;
                            result.output_available = true;
                            result.stdout_bytes = output.stdout.len() as u64;
                            result.stderr_bytes = output.stderr.len() as u64;
                            if !output.success {
                                result.message = Some(output.exit_code.map_or_else(
                                    || "Process ended without a successful exit status.".to_owned(),
                                    |code| format!("Process exited with status {code}."),
                                ));
                            }
                            captured_output = Some((
                                index,
                                CapturedOutput {
                                    stdout: output.stdout.clone(),
                                    stderr: output.stderr.clone(),
                                },
                            ));
                        }
                        Err(error) => {
                            result.state = "launch_failed";
                            result.message = Some(error.to_string());
                        }
                    }
                    observation.active_role = None;
                }
            }
        }
        if let Some((index, output)) = captured_output {
            record.outputs[index] = Some(output);
        }
        self.publish(project_name, workspace, &record);
    }
    pub(super) fn finish(
        &self,
        project_name: &str,
        workspace: &WorkspaceContext,
        observation: MovementObservation,
    ) {
        let mut record = lock(&self.record);
        record.observation = Some(observation);
        record.busy = false;
        self.publish(project_name, workspace, &record);
    }
    pub(super) fn read_output(
        &self,
        operation_id: &str,
        result_index: usize,
        stream: &str,
    ) -> Option<Vec<u8>> {
        let record = lock(&self.record);
        if record
            .observation
            .as_ref()
            .map(|observation| observation.operation_id.as_str())
            != Some(operation_id)
        {
            return None;
        }
        let output = record.outputs.get(result_index)?.as_ref()?;
        Some(if stream == "stdout" {
            output.stdout.clone()
        } else {
            output.stderr.clone()
        })
    }
    pub(super) fn current_observation(&self) -> Option<MovementObservation> {
        lock(&self.record).observation.clone()
    }
    fn publish(&self, project_name: &str, workspace: &WorkspaceContext, record: &WorkspaceRecord) {
        let snapshot = workspace_snapshot(
            project_name,
            workspace,
            record.observation.clone(),
            record.busy,
        );
        self.snapshots.send_replace(snapshot);
    }
}
impl MovementPermit {
    pub(super) fn complete(&mut self) {
        self.completed = true;
    }
}
impl Drop for MovementPermit {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        let mut record = lock(&self.runtime.record);
        record.busy = false;
    }
}
pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(_) => {
            eprintln!("Control Tower internal state is corrupted; restart the UI.");
            process::exit(1);
        }
    }
}
pub(super) fn sse_json_event<T: Serialize>(name: &str, value: &T) -> SseEvent {
    SseEvent::default()
        .event(name)
        .data(serde_json::to_string(value).expect("snapshot serialization must succeed"))
}
pub(super) fn stage_identity(stage: &Stage) -> StageIdentity {
    StageIdentity {
        number: stage.number,
        name: stage.name.clone(),
    }
}
pub(super) fn checkpoint_for_state(state: &WorkbenchState, stages: &[Stage]) -> CheckpointView {
    let accepted_stage = state
        .completed_stage_count
        .checked_sub(1)
        .and_then(|index| stages.get(index))
        .map(stage_identity);
    let pending_transition = state.pending.and_then(|pending| {
        stages.get(pending.stage_index).map(|stage| PendingView {
            direction: pending.direction.as_str(),
            stage: stage_identity(stage),
        })
    });
    CheckpointView {
        accepted_stage,
        pending_transition,
        workflow_started: state.uuid.is_some(),
        state: CheckpointState::from(state),
    }
}
