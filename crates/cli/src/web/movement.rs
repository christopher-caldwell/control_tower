use super::{
    dto::*,
    runtime::{
        MovementPermit, RoleObservationInput, WorkflowRuntime, new_observation, stage_identity,
    },
    workspace::WorkflowContext,
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
    pub(super) workspace_name: String,
    pub(super) env_defaults: std::collections::HashMap<String, String>,
    pub(super) workflow: WorkflowContext,
    pub(super) direction: Direction,
    pub(super) target_stage: u32,
    pub(super) expected: WorkbenchState,
    pub(super) runtime: Arc<WorkflowRuntime>,
    pub(super) permit: MovementPermit,
}

pub(super) fn execute_movement(task: MovementTask) -> WorkerResult {
    let MovementTask {
        workspace_name,
        env_defaults,
        workflow,
        direction,
        target_stage,
        expected,
        runtime,
        mut permit,
    } = task;
    let workbench = match crate::deps::workbench(&workflow.root, env_defaults) {
        Ok(workbench) => workbench,
        Err(error) => {
            runtime.rejected(&workspace_name, &workflow, Err(error.to_string()));
            permit.complete();
            return WorkerResult {
                error: Some((
                    StatusCode::SERVICE_UNAVAILABLE,
                    "workflow_unavailable",
                    error.to_string(),
                )),
            };
        }
    };
    let mut attempt_started = false;
    let outcome = workbench.move_to_observed(
        MoveToInput {
            workflow_root: &workflow.root,
            direction,
            target_stage,
            expected_checkpoint: Some(&expected),
        },
        &mut |progress| {
            runtime.observe(RoleObservationInput {
                workspace_name: &workspace_name,
                workflow: &workflow,
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
                    (StatusCode::SERVICE_UNAVAILABLE, "workflow_unavailable")
                }
            };
            runtime.rejected(&workspace_name, &workflow, Ok(&workbench));
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
        MoveStatus::Complete(_) => {
            observation.state = "complete";
            observation.failure = None;
        }
        MoveStatus::Stopped { failure, .. } => {
            observation.state = "stopped";
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
    runtime.finish(&workspace_name, &workflow, &workbench, observation);
    permit.complete();
    WorkerResult { error: None }
}
