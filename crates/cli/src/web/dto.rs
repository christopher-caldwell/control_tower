use control_tower_application::MovementChoice;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
pub(super) struct RuntimeEvent {
    pub(super) workspace_id: String,
    pub(super) server_instance_id: String,
    pub(super) revision: u64,
    pub(super) operation_id: Option<String>,
    pub(super) kind: &'static str,
    pub(super) direction: Option<&'static str>,
    pub(super) target_stage: Option<u32>,
    pub(super) stage: Option<StageIdentity>,
    pub(super) role: Option<&'static str>,
}

#[derive(Clone, Serialize)]
pub(super) struct RuntimeSnapshot {
    pub(super) workspace_id: String,
    pub(super) server_instance_id: String,
    pub(super) revision: u64,
    pub(super) movement_busy: bool,
    pub(super) checkpoint: Option<CheckpointView>,
    pub(super) movement_choices: Option<Vec<MovementChoiceView>>,
    pub(super) observation: Option<MovementObservation>,
}

#[derive(Clone, Serialize)]
pub(super) struct MovementObservation {
    pub(super) workspace_id: String,
    pub(super) operation_id: String,
    pub(super) server_instance_id: String,
    pub(super) revision: u64,
    pub(super) direction: &'static str,
    pub(super) target_stage: u32,
    pub(super) state: &'static str,
    pub(super) active_role: Option<RoleIdentity>,
    pub(super) role_results: Vec<RoleObservation>,
    pub(super) omitted_role_results: usize,
    pub(super) outputs_evicted: usize,
    pub(super) confirmed_checkpoint: Option<CheckpointView>,
    pub(super) movement_choices: Option<Vec<MovementChoiceView>>,
    pub(super) attempted_checkpoint: Option<CheckpointView>,
    pub(super) failure: Option<FailureView>,
    pub(super) verification_choices: Option<RecoveryChoicesView>,
}

#[derive(Clone, Serialize)]
pub(super) struct RoleIdentity {
    pub(super) stage: StageIdentity,
    pub(super) role: &'static str,
}

#[derive(Clone, Serialize)]
pub(super) struct RoleObservation {
    pub(super) stage: StageIdentity,
    pub(super) role: &'static str,
    pub(super) state: &'static str,
    pub(super) exit_code: Option<i32>,
    pub(super) message: Option<String>,
    pub(super) elapsed_ms: Option<u64>,
    pub(super) output_id: Option<String>,
    pub(super) output_state: &'static str,
    pub(super) stdout_bytes: u64,
    pub(super) stderr_bytes: u64,
    pub(super) stdout_truncated: bool,
    pub(super) stderr_truncated: bool,
}

#[derive(Clone, Serialize)]
pub(super) struct FailureView {
    pub(super) kind: &'static str,
    pub(super) message: String,
    pub(super) stage: Option<StageIdentity>,
    pub(super) role: Option<&'static str>,
}

#[derive(Clone, Serialize)]
pub(super) struct RecoveryChoicesView {
    pub(super) retry: MovementChoiceView,
    pub(super) reverse: Option<MovementChoiceView>,
}

#[derive(Clone, Serialize)]
pub(super) struct MovementChoiceView {
    pub(super) direction: &'static str,
    pub(super) target_stage: u32,
}

#[derive(Deserialize)]
pub(super) struct MovementRequest {
    pub(super) direction: String,
    pub(super) target_stage: u32,
}

#[derive(Serialize)]
pub(super) struct MovementResponse {
    pub(super) observation: MovementObservation,
}

#[derive(Clone, Serialize)]
pub(super) struct ApiErrorDetail {
    pub(super) code: &'static str,
    pub(super) message: String,
}

#[derive(Serialize)]
pub(super) struct MovementErrorEnvelope {
    pub(super) error: ApiErrorDetail,
    pub(super) operation_id: Option<String>,
}

#[derive(Serialize)]
pub(super) struct ErrorEnvelope {
    pub(super) error: ApiError,
}

#[derive(Serialize)]
pub(super) struct ApiError {
    pub(super) code: &'static str,
    pub(super) message: String,
}

#[derive(Serialize)]
pub(super) struct ProjectView {
    pub(super) name: String,
    pub(super) workspace_root: String,
    pub(super) workspaces: Vec<WorkspaceSummary>,
    pub(super) discovery_error: Option<String>,
}

#[derive(Serialize)]
pub(super) struct WorkspaceSummary {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) available: bool,
    pub(super) issue: Option<String>,
    pub(super) stage_count: Option<usize>,
    pub(super) accepted_stage: Option<StageIdentity>,
    pub(super) pending_transition: Option<PendingView>,
}

#[derive(Clone, Serialize)]
pub(super) struct StageIdentity {
    pub(super) number: u32,
    pub(super) name: String,
}

#[derive(Clone, Serialize)]
pub(super) struct WorkspaceIdentity {
    pub(super) id: String,
    pub(super) name: String,
}

#[derive(Clone, Serialize)]
pub(super) struct PendingView {
    pub(super) direction: &'static str,
    pub(super) stage: StageIdentity,
}

#[derive(Clone, Serialize)]
pub(super) struct WorkspaceView {
    pub(super) project_name: String,
    pub(super) workspace: WorkspaceIdentity,
    pub(super) checkpoint: CheckpointView,
    pub(super) movement_choices: Vec<MovementChoiceView>,
    pub(super) selected_stage_number: Option<u32>,
    pub(super) stages: Vec<StageView>,
    pub(super) server_instance_id: String,
    pub(super) observation_revision: u64,
    pub(super) movement_busy: bool,
    pub(super) observation: Option<MovementObservation>,
    pub(super) storage_issue: Option<String>,
}

#[derive(Clone, Serialize)]
pub(super) struct CheckpointView {
    pub(super) accepted_stage: Option<StageIdentity>,
    pub(super) pending_transition: Option<PendingView>,
    pub(super) workflow_started: bool,
}

#[derive(Clone, Serialize)]
pub(super) struct StageView {
    pub(super) number: u32,
    pub(super) name: String,
    pub(super) state: &'static str,
    pub(super) is_accepted_checkpoint: bool,
    pub(super) definitions: Vec<DefinitionSummary>,
}

#[derive(Clone, Serialize)]
pub(super) struct DefinitionSummary {
    pub(super) role: &'static str,
    pub(super) path: String,
}

#[derive(Serialize)]
pub(super) struct StageDefinitionView {
    pub(super) stage: StageIdentity,
    pub(super) definitions: Vec<DefinitionView>,
}

#[derive(Serialize)]
pub(super) struct DefinitionView {
    pub(super) role: &'static str,
    pub(super) path: String,
    pub(super) contents: Option<String>,
    pub(super) truncated: bool,
    pub(super) issue: Option<String>,
}

pub(super) fn movement_choice_view(choice: MovementChoice) -> MovementChoiceView {
    MovementChoiceView {
        direction: choice.direction.as_str(),
        target_stage: choice.target_stage,
    }
}
