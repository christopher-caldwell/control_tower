use super::{
    dto::*,
    runtime::{
        MovementPermit, RoleObservationInput, WorkspaceRuntime, checkpoint_for_state,
        new_observation, stage_identity,
    },
    workspace::WorkspaceContext,
};
use axum::http::StatusCode;
use control_tower_application::{
    Direction, MoveStatus, MoveToError, MoveToInput, TransitionFailure, WorkbenchState,
};
use std::sync::Arc;

pub(super) struct WorkerResult {
    pub(super) error: Option<(StatusCode, &'static str, String)>,
}
pub(super) struct MovementTask {
    pub(super) project_name: String,
    pub(super) workspace: WorkspaceContext,
    pub(super) direction: Direction,
    pub(super) target_stage: u32,
    pub(super) expected: WorkbenchState,
    pub(super) runtime: Arc<WorkspaceRuntime>,
    pub(super) permit: MovementPermit,
}

pub(super) fn execute_movement(task: MovementTask) -> WorkerResult {
    let MovementTask {
        project_name,
        workspace,
        direction,
        target_stage,
        expected,
        runtime,
        mut permit,
    } = task;
    let workbench = match crate::deps::workbench(&workspace.root) {
        Ok(workbench) => workbench,
        Err(error) => {
            runtime.rejected(&project_name, &workspace, Err(error.to_string()));
            permit.complete();
            return WorkerResult {
                error: Some((
                    StatusCode::SERVICE_UNAVAILABLE,
                    "workspace_unavailable",
                    error.to_string(),
                )),
            };
        }
    };
    let mut attempt_started = false;
    let outcome = workbench.move_to_observed(
        MoveToInput {
            workspace_root: &workspace.root,
            direction,
            target_stage,
            expected_checkpoint: Some(&expected),
        },
        &mut |progress| {
            runtime.observe(RoleObservationInput {
                project_name: &project_name,
                workspace: &workspace,
                workbench: &workbench,
                direction,
                target_stage,
                event: progress,
                attempt_started: &mut attempt_started,
            })
        },
    );
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => {
            let (status, code) = match &error {
                MoveToError::InvalidTarget(_) => (StatusCode::BAD_REQUEST, "invalid_target"),
                MoveToError::StaleCheckpoint => (StatusCode::CONFLICT, "stale_checkpoint"),
                MoveToError::StageDiscovery(_)
                | MoveToError::Persistence(_)
                | MoveToError::InvalidState(_) => {
                    (StatusCode::SERVICE_UNAVAILABLE, "workspace_unavailable")
                }
            };
            runtime.rejected(&project_name, &workspace, Ok(&workbench));
            permit.complete();
            return WorkerResult {
                error: Some((status, code, error.to_string())),
            };
        }
    };
    let choices = outcome
        .verification_choices()
        .map(|choices| RecoveryChoicesView {
            retry: movement_choice_view(choices.retry),
            reverse: choices.reverse.map(movement_choice_view),
        });
    let mut observation = if attempt_started {
        runtime
            .current_observation()
            .unwrap_or_else(|| new_observation(direction, target_stage))
    } else {
        new_observation(direction, target_stage)
    };
    observation.verification_choices = choices;
    match &outcome.status {
        MoveStatus::Complete(state) => {
            observation.state = "complete";
            observation.confirmed_checkpoint = Some(checkpoint_for_state(state, &outcome.stages));
            observation.attempted_checkpoint = None;
            observation.failure = None;
        }
        MoveStatus::Stopped { state, failure } => {
            observation.state = "stopped";
            observation.confirmed_checkpoint = Some(checkpoint_for_state(state, &outcome.stages));
            if let TransitionFailure::StateCouldNotBeSaved { attempted, .. } = failure {
                observation.attempted_checkpoint =
                    Some(checkpoint_for_state(attempted, &outcome.stages));
            }
            let (kind, stage_number, role) = match failure {
                TransitionFailure::MissingExecutable { stage_number, role } => (
                    "missing_executable",
                    Some(*stage_number),
                    Some(role.as_str()),
                ),
                TransitionFailure::ExecutableFailed {
                    stage_number, role, ..
                } => ("process_failed", Some(*stage_number), Some(role.as_str())),
                TransitionFailure::ExecutableCouldNotStart {
                    stage_number, role, ..
                } => ("launch_failed", Some(*stage_number), Some(role.as_str())),
                TransitionFailure::DirectionDoesNotReachTarget { .. } => {
                    ("unreachable_target", None, None)
                }
                TransitionFailure::StateCouldNotBeSaved { .. } => {
                    ("checkpoint_save_failed", None, None)
                }
            };
            observation.failure = Some(FailureView {
                kind,
                message: failure.to_string(),
                stage: stage_number
                    .and_then(|number| outcome.stages.iter().find(|stage| stage.number == number))
                    .map(stage_identity),
                role,
            });
        }
    }
    runtime.finish(&project_name, &workspace, &workbench, observation);
    permit.complete();
    WorkerResult { error: None }
}
