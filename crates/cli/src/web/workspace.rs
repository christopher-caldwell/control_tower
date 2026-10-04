use super::dto::*;
use control_tower_application::{Stage, Workbench, WorkbenchStatus};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub(super) struct WorkspaceContext {
    pub(super) name: String,
    pub(super) workflows: Vec<WorkflowContext>,
}

#[derive(Clone)]
pub(super) struct WorkflowContext {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) root: PathBuf,
}

pub(super) fn discover_workspace(workspace_root: &Path) -> Result<WorkspaceContext, String> {
    if !workspace_root.is_dir() {
        return Err(format!(
            "launch Workspace {} is not a directory",
            workspace_root.display()
        ));
    }
    let name = workspace_root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Workspace".to_owned());
    let inventory_path = workspace_root.join("workflows");
    let mut entries = match fs::read_dir(&inventory_path) {
        Ok(entries) => entries
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Cannot list workflows/: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(format!("Cannot inspect workflows/: {error}")),
    };
    entries.sort_by_key(|entry| entry.file_name());
    let workflows = entries
        .into_iter()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            let id = entry.file_name().to_string_lossy().into_owned();
            WorkflowContext {
                name: id.replace(['-', '_'], " "),
                id,
                root: entry.path(),
            }
        })
        .collect();
    Ok(WorkspaceContext { name, workflows })
}

pub(super) fn workspace_snapshot(workspace: &WorkspaceContext) -> WorkspaceView {
    WorkspaceView {
        name: workspace.name.clone(),
        workflows: workspace
            .workflows
            .iter()
            .map(|workflow| WorkflowIdentity {
                id: workflow.id.clone(),
                name: workflow.name.clone(),
            })
            .collect(),
    }
}

pub(super) fn workflow_snapshot(
    workspace_name: &str,
    workflow: &WorkflowContext,
    workbench: &Workbench,
    observation: Option<MovementObservation>,
    movement_busy: bool,
) -> WorkflowView {
    match workbench.status(&workflow.root) {
        Ok(status) => {
            available_workflow_view(workspace_name, workflow, status, observation, movement_busy)
        }
        Err(error) => unavailable_workflow_view(
            workspace_name,
            workflow,
            error.to_string(),
            observation,
            movement_busy,
        ),
    }
}

pub(super) fn unavailable_workflow_view(
    workspace_name: &str,
    workflow: &WorkflowContext,
    issue: String,
    observation: Option<MovementObservation>,
    movement_busy: bool,
) -> WorkflowView {
    WorkflowView {
        workspace_name: workspace_name.to_owned(),
        workflow: workflow_identity(workflow),
        current_status: "unavailable",
        status_issue: Some(issue),
        checkpoint: None,
        movement_choices: Vec::new(),
        stages: Vec::new(),
        movement_busy,
        observation,
    }
}

fn available_workflow_view(
    workspace_name: &str,
    workflow: &WorkflowContext,
    status: WorkbenchStatus,
    observation: Option<MovementObservation>,
    movement_busy: bool,
) -> WorkflowView {
    let accepted = accepted_stage(&status);
    let pending = pending_view(&status);
    let stages = status
        .stages
        .iter()
        .enumerate()
        .map(|(index, stage)| StageView {
            number: stage.number,
            name: stage.name.clone(),
            state: if status
                .state
                .pending
                .is_some_and(|pending| pending.stage_index == index)
            {
                "pending"
            } else if index < status.state.completed_stage_count {
                "accepted"
            } else {
                "future"
            },
            is_accepted_checkpoint: index < status.state.completed_stage_count
                && index + 1 == status.state.completed_stage_count,
            definitions: definition_summaries(stage, &workflow.root),
        })
        .collect();
    WorkflowView {
        workspace_name: workspace_name.to_owned(),
        workflow: workflow_identity(workflow),
        current_status: "available",
        status_issue: None,
        checkpoint: Some(CheckpointView {
            accepted_stage: accepted,
            pending_transition: pending,
            state: CheckpointState::from(&status.state),
        }),
        movement_choices: status
            .movement_choices()
            .into_iter()
            .map(movement_choice_view)
            .collect(),
        stages,
        movement_busy,
        observation,
    }
}

fn workflow_identity(workflow: &WorkflowContext) -> WorkflowIdentity {
    WorkflowIdentity {
        id: workflow.id.clone(),
        name: workflow.name.clone(),
    }
}

pub(super) fn accepted_stage(status: &WorkbenchStatus) -> Option<StageIdentity> {
    status
        .state
        .completed_stage_count
        .checked_sub(1)
        .and_then(|index| status.stages.get(index))
        .map(stage_identity)
}

pub(super) fn pending_view(status: &WorkbenchStatus) -> Option<PendingView> {
    status.state.pending.and_then(|pending| {
        status
            .stages
            .get(pending.stage_index)
            .map(|stage| PendingView {
                direction: pending.direction.as_str(),
                stage: stage_identity(stage),
            })
    })
}

pub(super) fn stage_identity(stage: &Stage) -> StageIdentity {
    StageIdentity {
        number: stage.number,
        name: stage.name.clone(),
    }
}

pub(super) fn definition_summaries(stage: &Stage, workflow_root: &Path) -> Vec<DefinitionSummary> {
    role_paths(stage)
        .into_iter()
        .filter_map(|(role, path)| {
            path.map(|path| DefinitionSummary {
                role,
                path: relative_path(workflow_root, path),
            })
        })
        .collect()
}

pub(super) fn role_paths(stage: &Stage) -> [(&'static str, Option<&Path>); 4] {
    [
        ("up", stage.up.as_deref()),
        ("down", stage.down.as_deref()),
        ("verify-up", stage.verify_up.as_deref()),
        ("verify-down", stage.verify_down.as_deref()),
    ]
}

pub(super) fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(super) fn find_workflow(workspace: &WorkspaceContext, id: &str) -> Option<WorkflowContext> {
    workspace
        .workflows
        .iter()
        .find(|workflow| workflow.id == id)
        .cloned()
}
