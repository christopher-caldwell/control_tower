use super::{
    dto::*,
    runtime::{MovementPermit, WorkspaceRuntime, checkpoint_for_state, stage_identity},
    workspace::{SETUP_GUIDANCE, WorkspaceContext},
};
use axum::http::StatusCode;
use control_tower_application::{
    Direction, ExecutionProgress, MoveStatus, MoveToError, MoveToInput, TransitionFailure,
    WorkbenchState,
};
use std::{collections::HashMap, sync::Arc, time::Instant};

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
    pub(super) operation_id: String,
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
        operation_id,
        mut permit,
    } = task;
    let mut role_started = HashMap::<(u32, &'static str), Instant>::new();
    let outcome = workspace.workbench.move_to_observed(
        MoveToInput {
            workspace_root: &workspace.root,
            direction,
            target_stage,
            expected_checkpoint: Some(&expected),
        },
        &mut |progress| match progress {
            ExecutionProgress::Admitted { .. } => runtime.admitted(
                &project_name,
                &workspace,
                &operation_id,
                direction,
                target_stage,
            ),
            progress => runtime.observe(&project_name, &workspace, progress, &mut role_started),
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
            runtime.rejected(&project_name, &workspace);
            permit.complete();
            return WorkerResult {
                error: Some((status, code, format!("{error}. {SETUP_GUIDANCE}"))),
            };
        }
    };
    let choices = outcome
        .verification_choices()
        .map(|choices| RecoveryChoicesView {
            retry: movement_choice_view(choices.retry),
            reverse: choices.reverse.map(movement_choice_view),
        });
    let mut observation = runtime
        .current_observation()
        .expect("admission created the current movement");
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
    runtime.finish(&project_name, &workspace, observation);
    permit.complete();
    WorkerResult { error: None }
}
