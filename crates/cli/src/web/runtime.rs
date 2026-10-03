use super::{dto::*, workspace::stage_identity};
use axum::response::sse::Event as SseEvent;
use control_tower_application::{Direction, ExecutableRole, Stage};
use serde::Serialize;
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::sync::broadcast;
use uuid::Uuid;

pub(super) const MAX_OUTPUT_STREAM_BYTES: usize = 1024 * 1024;

pub(super) const MAX_WORKSPACE_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

pub(super) const MAX_ROLE_RESULTS: usize = 128;

pub(super) const EVENT_BUFFER: usize = 256;

pub(super) struct ObservationStore {
    pub(super) server_instance_id: String,
    pub(super) workspaces: Mutex<HashMap<String, Arc<WorkspaceRuntime>>>,
}

pub(super) struct WorkspaceRuntime {
    pub(super) record: Mutex<WorkspaceRecord>,
    pub(super) events: broadcast::Sender<RuntimeEvent>,
    pub(super) next_revision: AtomicU64,
}

pub(super) struct WorkspaceRecord {
    pub(super) busy: bool,
    pub(super) observation: Option<MovementObservation>,
    pub(super) last_known_view: Option<WorkspaceView>,
    pub(super) outputs: HashMap<String, CapturedOutput>,
    pub(super) output_order: VecDeque<String>,
    pub(super) output_bytes: usize,
}

pub(super) struct CapturedOutput {
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
    pub(super) stdout_bytes: u64,
    pub(super) stderr_bytes: u64,
    pub(super) stdout_truncated: bool,
    pub(super) stderr_truncated: bool,
}

pub(super) struct OperationGuard {
    pub(super) runtime: Arc<WorkspaceRuntime>,
    pub(super) workspace_id: String,
    pub(super) server_instance_id: String,
    pub(super) operation_id: String,
    pub(super) finalized: bool,
}

impl ObservationStore {
    pub(super) fn new() -> Self {
        Self {
            server_instance_id: Uuid::new_v4().to_string(),
            workspaces: Mutex::new(HashMap::new()),
        }
    }

    pub(super) fn workspace_state(&self, id: &str) -> Arc<WorkspaceRuntime> {
        let mut workspaces = lock_unpoison(&self.workspaces);
        workspaces
            .entry(id.to_owned())
            .or_insert_with(WorkspaceRuntime::new)
            .clone()
    }
}

impl WorkspaceRuntime {
    pub(super) fn new() -> Arc<Self> {
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        Arc::new(Self {
            record: Mutex::new(WorkspaceRecord {
                busy: false,
                observation: None,
                last_known_view: None,
                outputs: HashMap::new(),
                output_order: VecDeque::new(),
                output_bytes: 0,
            }),
            events,
            next_revision: AtomicU64::new(0),
        })
    }

    pub(super) fn snapshot(&self, workspace_id: &str, server_instance_id: &str) -> RuntimeSnapshot {
        let record = lock_unpoison(&self.record);
        let observation = record.observation.clone();
        // Select checkpoint and choices from the same source under the same lock.
        let (checkpoint, movement_choices) = observation
            .as_ref()
            .and_then(|current| {
                current
                    .confirmed_checkpoint
                    .clone()
                    .map(|checkpoint| (Some(checkpoint), current.movement_choices.clone()))
            })
            .or_else(|| {
                record.last_known_view.as_ref().map(|view| {
                    (
                        Some(view.checkpoint.clone()),
                        Some(view.movement_choices.clone()),
                    )
                })
            })
            .unwrap_or((None, None));
        RuntimeSnapshot {
            workspace_id: workspace_id.to_owned(),
            server_instance_id: server_instance_id.to_owned(),
            revision: self.next_revision.load(Ordering::Relaxed),
            movement_busy: record.busy,
            checkpoint,
            movement_choices,
            observation,
        }
    }

    pub(super) fn remember_view(&self, view: WorkspaceView) {
        lock_unpoison(&self.record).last_known_view = Some(view);
    }

    pub(super) fn last_known_view(&self) -> Option<WorkspaceView> {
        lock_unpoison(&self.record).last_known_view.clone()
    }

    pub(super) fn begin_operation(
        self: &Arc<Self>,
        workspace_id: &str,
        server_instance_id: &str,
        operation_id: &str,
        direction: Direction,
        target_stage: u32,
    ) -> Option<MovementObservation> {
        let mut record = lock_unpoison(&self.record);
        if record.busy {
            return None;
        }
        record.busy = true;
        record.outputs.clear();
        record.output_order.clear();
        record.output_bytes = 0;
        let mut observation = MovementObservation {
            workspace_id: workspace_id.to_owned(),
            operation_id: operation_id.to_owned(),
            server_instance_id: server_instance_id.to_owned(),
            revision: 0,
            direction: direction.as_str(),
            target_stage,
            state: "running",
            active_role: None,
            role_results: Vec::new(),
            omitted_role_results: 0,
            outputs_evicted: 0,
            confirmed_checkpoint: None,
            movement_choices: None,
            attempted_checkpoint: None,
            failure: None,
            verification_choices: None,
        };
        self.publish_locked(
            &mut record,
            workspace_id,
            server_instance_id,
            "movement.started",
            Some(operation_id.to_owned()),
            &mut observation,
        );
        record.observation = Some(observation.clone());
        Some(observation)
    }

    pub(super) fn record_role_started(
        &self,
        workspace_id: &str,
        server_instance_id: &str,
        operation_id: &str,
        stage: &Stage,
        role: ExecutableRole,
    ) {
        let mut record = lock_unpoison(&self.record);
        let Some(mut observation) = record.observation.take() else {
            return;
        };
        if observation.operation_id != operation_id {
            record.observation = Some(observation);
            return;
        }
        let stage = stage_identity(stage);
        let role_name = role.as_str();
        observation.active_role = Some(RoleIdentity {
            stage: stage.clone(),
            role: role_name,
        });
        observation.role_results.push(RoleObservation {
            stage,
            role: role_name,
            state: "in_progress",
            exit_code: None,
            message: None,
            elapsed_ms: None,
            output_id: None,
            output_state: "not_returned",
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_truncated: false,
            stderr_truncated: false,
        });
        trim_role_results(&mut record, &mut observation);
        self.publish_locked(
            &mut record,
            workspace_id,
            server_instance_id,
            "role.started",
            Some(operation_id.to_owned()),
            &mut observation,
        );
        record.observation = Some(observation);
    }

    pub(super) fn record_role_finished(
        &self,
        workspace_id: &str,
        server_instance_id: &str,
        operation_id: &str,
        stage: &Stage,
        execution: &control_tower_application::ExecutionEvent,
        elapsed_ms: Option<u64>,
    ) {
        let role_name = execution.role.as_str();
        let output_data = execution.result.as_ref().ok().map(capture_output);
        let output_id = output_data.as_ref().map(|_| Uuid::new_v4().to_string());
        let mut record = lock_unpoison(&self.record);
        let Some(mut observation) = record.observation.take() else {
            return;
        };
        if observation.operation_id != operation_id {
            record.observation = Some(observation);
            return;
        }
        if let (Some(output_id), Some(output)) = (&output_id, output_data) {
            insert_output(&mut record, &mut observation, output_id.clone(), output);
        }
        let identity = stage_identity(stage);
        let Some(role_observation) = observation
            .role_results
            .iter_mut()
            .rev()
            .find(|result| result.stage.number == identity.number && result.role == role_name)
        else {
            record.observation = Some(observation);
            return;
        };
        role_observation.elapsed_ms = elapsed_ms;
        role_observation.output_id = output_id;
        match &execution.result {
            Ok(output) => {
                role_observation.state = if output.success {
                    "succeeded"
                } else {
                    "failed"
                };
                role_observation.exit_code = output.exit_code;
                role_observation.output_state = "available";
                role_observation.stdout_bytes = output.stdout.len() as u64;
                role_observation.stderr_bytes = output.stderr.len() as u64;
                role_observation.stdout_truncated = output.stdout.len() > MAX_OUTPUT_STREAM_BYTES;
                role_observation.stderr_truncated = output.stderr.len() > MAX_OUTPUT_STREAM_BYTES;
                if !output.success {
                    role_observation.message = Some(match output.exit_code {
                        Some(code) => format!("Process exited with status {code}."),
                        None => "Process ended without a successful exit status.".to_owned(),
                    });
                }
            }
            Err(error) => {
                role_observation.state = "launch_failed";
                role_observation.output_state = "unavailable";
                role_observation.message = Some(error.to_string());
            }
        }
        observation.active_role = None;
        self.publish_locked(
            &mut record,
            workspace_id,
            server_instance_id,
            "role.finished",
            Some(operation_id.to_owned()),
            &mut observation,
        );
        record.observation = Some(observation);
    }

    pub(super) fn current_observation(&self) -> Option<MovementObservation> {
        lock_unpoison(&self.record).observation.clone()
    }

    pub(super) fn publish_locked(
        &self,
        record: &mut WorkspaceRecord,
        workspace_id: &str,
        server_instance_id: &str,
        kind: &'static str,
        operation_id: Option<String>,
        observation: &mut MovementObservation,
    ) {
        let revision = self.next_revision.fetch_add(1, Ordering::Relaxed) + 1;
        observation.revision = revision;
        let (stage, role) = match kind {
            "role.started" => observation
                .active_role
                .as_ref()
                .map(|identity| (Some(identity.stage.clone()), Some(identity.role)))
                .unwrap_or((None, None)),
            "role.finished" => observation
                .role_results
                .last()
                .map(|result| (Some(result.stage.clone()), Some(result.role)))
                .unwrap_or((None, None)),
            _ => (None, None),
        };
        let _ = self.events.send(RuntimeEvent {
            workspace_id: workspace_id.to_owned(),
            server_instance_id: server_instance_id.to_owned(),
            revision,
            operation_id,
            kind,
            direction: Some(observation.direction),
            target_stage: Some(observation.target_stage),
            stage,
            role,
        });
        record.observation = Some(observation.clone());
    }
}

impl OperationGuard {
    pub(super) fn new(
        runtime: Arc<WorkspaceRuntime>,
        workspace_id: String,
        server_instance_id: String,
        operation_id: String,
    ) -> Self {
        Self {
            runtime,
            workspace_id,
            server_instance_id,
            operation_id,
            finalized: false,
        }
    }

    pub(super) fn finish(
        &mut self,
        mut observation: MovementObservation,
        kind: &'static str,
    ) -> MovementObservation {
        let mut record = lock_unpoison(&self.runtime.record);
        record.busy = false;
        self.runtime.publish_locked(
            &mut record,
            &self.workspace_id,
            &self.server_instance_id,
            kind,
            Some(self.operation_id.clone()),
            &mut observation,
        );
        self.finalized = true;
        observation
    }
}

pub(super) fn lock_unpoison<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn sse_json_event<T: Serialize>(name: &str, value: &T) -> SseEvent {
    let data = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_owned());
    SseEvent::default().event(name).data(data)
}

pub(super) fn capture_output(output: &control_tower_application::ProcessOutput) -> CapturedOutput {
    let stdout_bytes = output.stdout.len() as u64;
    let stderr_bytes = output.stderr.len() as u64;
    let stdout_truncated = output.stdout.len() > MAX_OUTPUT_STREAM_BYTES;
    let stderr_truncated = output.stderr.len() > MAX_OUTPUT_STREAM_BYTES;
    CapturedOutput {
        stdout: output
            .stdout
            .iter()
            .copied()
            .take(MAX_OUTPUT_STREAM_BYTES)
            .collect(),
        stderr: output
            .stderr
            .iter()
            .copied()
            .take(MAX_OUTPUT_STREAM_BYTES)
            .collect(),
        stdout_bytes,
        stderr_bytes,
        stdout_truncated,
        stderr_truncated,
    }
}

pub(super) fn insert_output(
    record: &mut WorkspaceRecord,
    observation: &mut MovementObservation,
    output_id: String,
    output: CapturedOutput,
) {
    let output_size = output.stdout.len() + output.stderr.len();
    while record.output_bytes + output_size > MAX_WORKSPACE_OUTPUT_BYTES {
        let Some(oldest) = record.output_order.pop_front() else {
            break;
        };
        if let Some(evicted) = record.outputs.remove(&oldest) {
            record.output_bytes = record
                .output_bytes
                .saturating_sub(evicted.stdout.len() + evicted.stderr.len());
            if let Some(role) = observation
                .role_results
                .iter_mut()
                .find(|role| role.output_id.as_deref() == Some(oldest.as_str()))
            {
                role.output_state = "evicted";
                observation.outputs_evicted += 1;
            }
        }
    }
    record.output_bytes += output_size;
    record.output_order.push_back(output_id.clone());
    record.outputs.insert(output_id, output);
}

pub(super) fn trim_role_results(
    record: &mut WorkspaceRecord,
    observation: &mut MovementObservation,
) {
    while observation.role_results.len() > MAX_ROLE_RESULTS {
        let removed = observation.role_results.remove(0);
        observation.omitted_role_results += 1;
        if let Some(output_id) = removed.output_id {
            if let Some(output) = record.outputs.remove(&output_id) {
                record.output_bytes = record
                    .output_bytes
                    .saturating_sub(output.stdout.len() + output.stderr.len());
                record.output_order.retain(|id| id != &output_id);
                observation.outputs_evicted += 1;
            }
        }
    }
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        if self.finalized {
            return;
        }
        let mut record = lock_unpoison(&self.runtime.record);
        record.busy = false;
        let mut observation = record
            .observation
            .take()
            .unwrap_or_else(|| MovementObservation {
                workspace_id: self.workspace_id.clone(),
                operation_id: self.operation_id.clone(),
                server_instance_id: self.server_instance_id.clone(),
                revision: 0,
                direction: "up",
                target_stage: 0,
                state: "unavailable",
                active_role: None,
                role_results: Vec::new(),
                omitted_role_results: 0,
                outputs_evicted: 0,
                confirmed_checkpoint: None,
                movement_choices: None,
                attempted_checkpoint: None,
                failure: None,
                verification_choices: None,
            });
        observation.state = "unavailable";
        observation.active_role = None;
        observation.failure = Some(FailureView {
            kind: "worker_interrupted",
            message: "The movement worker stopped before reporting a terminal result. Inspect author-owned effects and the durable checkpoint.".to_owned(),
            stage: None,
            role: None,
        });
        self.runtime.publish_locked(
            &mut record,
            &self.workspace_id,
            &self.server_instance_id,
            "movement.finished",
            Some(self.operation_id.clone()),
            &mut observation,
        );
    }
}
