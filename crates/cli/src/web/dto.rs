use control_tower_application::{Direction, MovementChoice, PendingTransition, WorkbenchState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
pub(super) struct MovementObservation {
    pub(super) direction: &'static str,
    pub(super) target_stage: u32,
    pub(super) state: &'static str,
    pub(super) active_role: Option<RoleIdentity>,
    pub(super) role_results: Vec<RoleObservation>,
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
    pub(super) stdout: Option<String>,
    pub(super) stderr: Option<String>,
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
    pub(super) expected_checkpoint: CheckpointState,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(super) struct CheckpointState {
    pub(super) completed_stage_count: usize,
    pub(super) uuid: Option<String>,
    pub(super) pending: Option<PendingCheckpoint>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(super) struct PendingCheckpoint {
    pub(super) stage_index: usize,
    pub(super) direction: StringDirection,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum StringDirection {
    Up,
    Down,
}
impl From<&WorkbenchState> for CheckpointState {
    fn from(state: &WorkbenchState) -> Self {
        Self {
            completed_stage_count: state.completed_stage_count,
            uuid: state.uuid.clone(),
            pending: state.pending.map(|p| PendingCheckpoint {
                stage_index: p.stage_index,
                direction: if p.direction == Direction::Up {
                    StringDirection::Up
                } else {
                    StringDirection::Down
                },
            }),
        }
    }
}
impl From<CheckpointState> for WorkbenchState {
    fn from(state: CheckpointState) -> Self {
        Self {
            completed_stage_count: state.completed_stage_count,
            uuid: state.uuid,
            pending: state.pending.map(|p| PendingTransition {
                stage_index: p.stage_index,
                direction: if p.direction == StringDirection::Up {
                    Direction::Up
                } else {
                    Direction::Down
                },
            }),
        }
    }
}
#[derive(Clone, Serialize)]
pub(super) struct ProjectView {
    pub(super) name: String,
    pub(super) workspaces: Vec<WorkspaceIdentity>,
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
pub(super) struct CheckpointView {
    pub(super) accepted_stage: Option<StageIdentity>,
    pub(super) pending_transition: Option<PendingView>,
    pub(super) state: CheckpointState,
}
#[derive(Clone, Serialize)]
pub(super) struct WorkspaceView {
    pub(super) project_name: String,
    pub(super) workspace: WorkspaceIdentity,
    pub(super) current_status: &'static str,
    pub(super) status_issue: Option<String>,
    pub(super) checkpoint: Option<CheckpointView>,
    pub(super) movement_choices: Vec<MovementChoiceView>,
    pub(super) stages: Vec<StageView>,
    pub(super) movement_busy: bool,
    pub(super) observation: Option<MovementObservation>,
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
    pub(super) issue: Option<String>,
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

pub(super) fn movement_choice_view(choice: MovementChoice) -> MovementChoiceView {
    MovementChoiceView {
        direction: choice.direction.as_str(),
        target_stage: choice.target_stage,
    }
}
