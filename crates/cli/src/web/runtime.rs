use super::{
    dto::*,
    workspace::{WorkflowContext, unavailable_workflow_view, workflow_snapshot},
};
use axum::response::sse::Event as SseEvent;
use control_tower_application::{Direction, ExecutionProgress, Stage, Workbench};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::sync::watch;

pub(super) struct ObservationStore {
    workflows: Mutex<HashMap<String, Arc<WorkflowRuntime>>>,
}
pub(super) struct WorkflowRuntime {
    record: Mutex<WorkflowRecord>,
    snapshots: watch::Sender<WorkflowView>,
}
pub(super) struct WorkflowRecord {
    pub(super) busy: bool,
    pub(super) observation: Option<MovementObservation>,
}
pub(super) struct MovementPermit {
    runtime: Arc<WorkflowRuntime>,
    completed: bool,
}
pub(super) struct RoleObservationInput<'a> {
    pub(super) workspace_name: &'a str,
    pub(super) workflow: &'a WorkflowContext,
    pub(super) workbench: &'a Workbench,
    pub(super) direction: Direction,
    pub(super) target_stage: u32,
    pub(super) event: ExecutionProgress<'a>,
    pub(super) attempt_started: &'a mut bool,
}

impl ObservationStore {
    pub(super) fn new() -> Self {
        Self {
            workflows: Mutex::new(HashMap::new()),
        }
    }

    pub(super) fn workflow_state(
        &self,
        workspace_name: &str,
        workflow: &WorkflowContext,
    ) -> Arc<WorkflowRuntime> {
        let mut workflows = lock(&self.workflows);
        workflows
            .entry(workflow.id.clone())
            .or_insert_with(|| WorkflowRuntime::new(workspace_name, workflow))
            .clone()
    }
}

impl WorkflowRuntime {
    fn new(workspace_name: &str, workflow: &WorkflowContext) -> Arc<Self> {
        let snapshot = unavailable_workflow_view(
            workspace_name,
            workflow,
            "Current status has not been read yet.".to_owned(),
            None,
            false,
        );
        let (snapshots, _) = watch::channel(snapshot);
        Arc::new(Self {
            record: Mutex::new(WorkflowRecord {
                busy: false,
                observation: None,
            }),
            snapshots,
        })
    }

    pub(super) fn subscribe(&self) -> watch::Receiver<WorkflowView> {
        self.snapshots.subscribe()
    }

    pub(super) fn current_snapshot(
        &self,
        workspace_name: &str,
        workflow: &WorkflowContext,
        workbench: Result<&Workbench, String>,
    ) -> WorkflowView {
        let record = lock(&self.record);
        self.publish(workspace_name, workflow, workbench, &record)
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

    pub(super) fn rejected(
        &self,
        workspace_name: &str,
        workflow: &WorkflowContext,
        workbench: Result<&Workbench, String>,
    ) {
        let mut record = lock(&self.record);
        record.busy = false;
        self.publish(workspace_name, workflow, workbench, &record);
    }

    pub(super) fn observe(&self, input: RoleObservationInput<'_>) {
        let RoleObservationInput {
            workspace_name,
            workflow,
            workbench,
            direction,
            target_stage,
            event,
            attempt_started,
        } = input;
        let mut record = lock(&self.record);
        if !*attempt_started {
            record.observation = Some(new_observation(direction, target_stage));
            *attempt_started = true;
        }
        if let Some(observation) = record.observation.as_mut() {
            match event {
                ExecutionProgress::Starting { stage, role } => {
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
                        stdout: None,
                        stderr: None,
                    });
                }
                ExecutionProgress::Finished { stage, execution } => {
                    let result = observation.role_results.iter_mut().rev().find(|result| {
                        result.stage.number == stage.number
                            && result.role == execution.role.as_str()
                            && result.state == "in_progress"
                    });
                    if let Some(result) = result {
                        match &execution.result {
                            Ok(output) => {
                                result.state = if output.success {
                                    "succeeded"
                                } else {
                                    "failed"
                                };
                                result.exit_code = output.exit_code;
                                result.stdout =
                                    Some(String::from_utf8_lossy(&output.stdout).into_owned());
                                result.stderr =
                                    Some(String::from_utf8_lossy(&output.stderr).into_owned());
                                if !output.success {
                                    result.message = Some(output.exit_code.map_or_else(
                                        || {
                                            "Process ended without a successful exit status."
                                                .to_owned()
                                        },
                                        |code| format!("Process exited with status {code}."),
                                    ));
                                }
                            }
                            Err(error) => {
                                result.state = "launch_failed";
                                result.message = Some(error.to_string());
                            }
                        }
                    }
                    observation.active_role = None;
                }
            }
        }
        self.publish(workspace_name, workflow, Ok(workbench), &record);
    }

    pub(super) fn finish(
        &self,
        workspace_name: &str,
        workflow: &WorkflowContext,
        workbench: &Workbench,
        observation: MovementObservation,
    ) {
        let mut record = lock(&self.record);
        record.observation = Some(observation);
        record.busy = false;
        self.publish(workspace_name, workflow, Ok(workbench), &record);
    }

    pub(super) fn current_observation(&self) -> Option<MovementObservation> {
        lock(&self.record).observation.clone()
    }

    fn publish(
        &self,
        workspace_name: &str,
        workflow: &WorkflowContext,
        workbench: Result<&Workbench, String>,
        record: &WorkflowRecord,
    ) -> WorkflowView {
        let snapshot = match workbench {
            Ok(workbench) => workflow_snapshot(
                workspace_name,
                workflow,
                workbench,
                record.observation.clone(),
                record.busy,
            ),
            Err(issue) => unavailable_workflow_view(
                workspace_name,
                workflow,
                issue,
                record.observation.clone(),
                record.busy,
            ),
        };
        self.snapshots.send_replace(snapshot.clone());
        snapshot
    }
}

pub(super) fn new_observation(direction: Direction, target_stage: u32) -> MovementObservation {
    MovementObservation {
        direction: direction.as_str(),
        target_stage,
        state: "running",
        active_role: None,
        role_results: Vec::new(),
        failure: None,
        verification_choices: None,
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
        let mut snapshot = self.runtime.snapshots.borrow().clone();
        snapshot.movement_busy = false;
        self.runtime.snapshots.send_replace(snapshot);
    }
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
