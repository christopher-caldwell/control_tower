use super::{
    dto::*,
    runtime::WorkspaceRuntime,
    workspace::{
        SETUP_GUIDANCE, WorkspaceContext, ensure_confined, stage_identity, workspace_tree_issue,
    },
};
use axum::http::StatusCode;
use control_tower_application::{
    Direction, ExecutionProgress, MoveStatus, MoveToError, MoveToInput, MovementChoice, Stage,
    TransitionFailure, WorkbenchState,
};
use std::{collections::HashMap, sync::Arc, time::Instant};

pub(super) struct WorkerResult {
    pub(super) observation: MovementObservation,
    pub(super) error: Option<(StatusCode, &'static str, String)>,
}

pub(super) fn execute_movement(
    workspace: &WorkspaceContext,
    direction: Direction,
    target_stage: u32,
    runtime: Arc<WorkspaceRuntime>,
    server_instance_id: &str,
    operation_id: &str,
) -> WorkerResult {
    let unavailable = |message: String, code: &'static str, status: StatusCode| {
        let mut observation =
            runtime
                .current_observation()
                .unwrap_or_else(|| MovementObservation {
                    workspace_id: workspace.id.clone(),
                    operation_id: operation_id.to_owned(),
                    server_instance_id: server_instance_id.to_owned(),
                    revision: 0,
                    direction: direction.as_str(),
                    target_stage,
                    state: "unavailable",
                    active_role: None,
                    role_results: Vec::new(),
                    omitted_role_results: 0,
                    outputs_evicted: 0,
                    confirmed_checkpoint: None,
                    attempted_checkpoint: None,
                    failure: None,
                    verification_choices: None,
                });
        observation.state = "unavailable";
        observation.active_role = None;
        observation.failure = Some(FailureView {
            kind: code,
            message: message.clone(),
            stage: None,
            role: None,
        });
        WorkerResult {
            observation,
            error: Some((status, code, message)),
        }
    };

    if let Some(issue) = workspace.unavailable_reason.as_ref() {
        return unavailable(
            issue.clone(),
            "workspace_unavailable",
            StatusCode::SERVICE_UNAVAILABLE,
        );
    }
    if let Some(issue) = workspace_tree_issue(&workspace.root) {
        return unavailable(
            issue,
            "workspace_unavailable",
            StatusCode::SERVICE_UNAVAILABLE,
        );
    }
    let workbench = match crate::deps::workbench(&workspace.root) {
        Ok(workbench) => workbench,
        Err(error) => {
            return unavailable(
                format!("{error}. {SETUP_GUIDANCE}"),
                "workspace_unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
            );
        }
    };
    let status = match workbench.status(&workspace.root) {
        Ok(status) => status,
        Err(error) => {
            return unavailable(
                format!("{error}. {SETUP_GUIDANCE}"),
                "workspace_unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
            );
        }
    };
    for stage in &status.stages {
        if let Err(issue) = ensure_confined(&workspace.root, &stage.directory) {
            return unavailable(
                issue,
                "workspace_unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
            );
        }
        for executable in [&stage.up, &stage.down, &stage.verify_up, &stage.verify_down]
            .into_iter()
            .flatten()
        {
            if let Err(issue) = ensure_confined(&workspace.root, executable) {
                return unavailable(
                    issue,
                    "workspace_unavailable",
                    StatusCode::SERVICE_UNAVAILABLE,
                );
            }
        }
    }

    let mut role_started = HashMap::<(u32, &'static str), Instant>::new();
    let workspace_id = workspace.id.clone();
    let mut observe = |progress: ExecutionProgress<'_>| match progress {
        ExecutionProgress::Starting { stage, role } => {
            role_started.insert((stage.number, role.as_str()), Instant::now());
            runtime.record_role_started(
                &workspace_id,
                server_instance_id,
                operation_id,
                stage,
                role,
            );
        }
        ExecutionProgress::Finished { stage, execution } => {
            let elapsed_ms = role_started
                .remove(&(stage.number, execution.role.as_str()))
                .map(|started| started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
            runtime.record_role_finished(
                &workspace_id,
                server_instance_id,
                operation_id,
                stage,
                execution,
                elapsed_ms,
            );
        }
    };
    let outcome = match workbench.move_to_observed(
        MoveToInput {
            workspace_root: &workspace.root,
            direction,
            target_stage,
        },
        &mut observe,
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            let (status, code) = match &error {
                MoveToError::InvalidTarget(_) => (StatusCode::BAD_REQUEST, "invalid_target"),
                MoveToError::StageDiscovery(_)
                | MoveToError::Persistence(_)
                | MoveToError::InvalidState(_) => {
                    (StatusCode::SERVICE_UNAVAILABLE, "workspace_unavailable")
                }
            };
            return unavailable(error.to_string(), code, status);
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
        .unwrap_or_else(|| MovementObservation {
            workspace_id: workspace.id.clone(),
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
            attempted_checkpoint: None,
            failure: None,
            verification_choices: None,
        });
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
            observation.attempted_checkpoint = attempted_checkpoint(failure, &outcome.stages);
            observation.failure = Some(failure_view(failure, &outcome.stages));
        }
    }
    WorkerResult {
        observation,
        error: None,
    }
}

pub(super) fn movement_choice_view(choice: MovementChoice) -> RecoveryChoiceView {
    RecoveryChoiceView {
        direction: choice.direction.as_str(),
        target_stage: choice.target_stage,
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
    }
}

pub(super) fn attempted_checkpoint(
    failure: &TransitionFailure,
    stages: &[Stage],
) -> Option<CheckpointView> {
    match failure {
        TransitionFailure::StateCouldNotBeSaved { attempted, .. } => {
            Some(checkpoint_for_state(attempted, stages))
        }
        _ => None,
    }
}

pub(super) fn failure_view(failure: &TransitionFailure, stages: &[Stage]) -> FailureView {
    let (kind, message, stage_number, role) = match failure {
        TransitionFailure::MissingExecutable { stage_number, role } => (
            "missing_executable",
            failure.to_string(),
            Some(*stage_number),
            Some(role.as_str()),
        ),
        TransitionFailure::ExecutableFailed {
            stage_number, role, ..
        } => (
            "process_failed",
            failure.to_string(),
            Some(*stage_number),
            Some(role.as_str()),
        ),
        TransitionFailure::ExecutableCouldNotStart {
            stage_number, role, ..
        } => (
            "launch_failed",
            failure.to_string(),
            Some(*stage_number),
            Some(role.as_str()),
        ),
        TransitionFailure::DirectionDoesNotReachTarget { .. } => {
            ("unreachable_target", failure.to_string(), None, None)
        }
        TransitionFailure::StateCouldNotBeSaved { .. } => {
            ("checkpoint_save_failed", failure.to_string(), None, None)
        }
    };
    FailureView {
        kind,
        message,
        stage: stage_number
            .and_then(|number| stages.iter().find(|stage| stage.number == number))
            .map(stage_identity),
        role,
    }
}
