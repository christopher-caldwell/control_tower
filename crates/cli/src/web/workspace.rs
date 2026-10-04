use super::dto::*;
use control_tower_application::{Stage, Workbench, WorkbenchStatus};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub(super) struct ProjectContext {
    pub(super) name: String,
    pub(super) workspaces: Vec<WorkspaceContext>,
}

#[derive(Clone)]
pub(super) struct WorkspaceContext {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) root: PathBuf,
}

pub(super) fn discover_project(project_root: &Path) -> Result<ProjectContext, String> {
    if !project_root.is_dir() {
        return Err(format!(
            "launch Project {} is not a directory",
            project_root.display()
        ));
    }
    let name = project_root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Project".to_owned());
    let inventory_path = project_root.join("workspaces");
    let mut entries = match fs::read_dir(&inventory_path) {
        Ok(entries) => entries
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Cannot list workspaces/: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(format!("Cannot inspect workspaces/: {error}")),
    };
    entries.sort_by_key(|entry| entry.file_name());
    let workspaces = entries
        .into_iter()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            let id = entry.file_name().to_string_lossy().into_owned();
            WorkspaceContext {
                name: id.replace(['-', '_'], " "),
                id,
                root: entry.path(),
            }
        })
        .collect();
    Ok(ProjectContext { name, workspaces })
}

pub(super) fn project_snapshot(project: &ProjectContext) -> ProjectView {
    ProjectView {
        name: project.name.clone(),
        workspaces: project
            .workspaces
            .iter()
            .map(|workspace| WorkspaceIdentity {
                id: workspace.id.clone(),
                name: workspace.name.clone(),
            })
            .collect(),
    }
}

pub(super) fn workspace_snapshot(
    project_name: &str,
    workspace: &WorkspaceContext,
    workbench: &Workbench,
    observation: Option<MovementObservation>,
    movement_busy: bool,
) -> WorkspaceView {
    match workbench.status(&workspace.root) {
        Ok(status) => {
            available_workspace_view(project_name, workspace, status, observation, movement_busy)
        }
        Err(error) => unavailable_workspace_view(
            project_name,
            workspace,
            error.to_string(),
            observation,
            movement_busy,
        ),
    }
}

pub(super) fn unavailable_workspace_view(
    project_name: &str,
    workspace: &WorkspaceContext,
    issue: String,
    observation: Option<MovementObservation>,
    movement_busy: bool,
) -> WorkspaceView {
    WorkspaceView {
        project_name: project_name.to_owned(),
        workspace: workspace_identity(workspace),
        current_status: "unavailable",
        status_issue: Some(issue),
        checkpoint: None,
        movement_choices: Vec::new(),
        stages: Vec::new(),
        movement_busy,
        observation,
    }
}

fn available_workspace_view(
    project_name: &str,
    workspace: &WorkspaceContext,
    status: WorkbenchStatus,
    observation: Option<MovementObservation>,
    movement_busy: bool,
) -> WorkspaceView {
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
            definitions: definition_summaries(stage, &workspace.root),
        })
        .collect();
    WorkspaceView {
        project_name: project_name.to_owned(),
        workspace: workspace_identity(workspace),
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

fn workspace_identity(workspace: &WorkspaceContext) -> WorkspaceIdentity {
    WorkspaceIdentity {
        id: workspace.id.clone(),
        name: workspace.name.clone(),
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

pub(super) fn definition_summaries(stage: &Stage, workspace_root: &Path) -> Vec<DefinitionSummary> {
    role_paths(stage)
        .into_iter()
        .filter_map(|(role, path)| {
            path.map(|path| DefinitionSummary {
                role,
                path: relative_path(workspace_root, path),
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

pub(super) fn find_workspace(project: &ProjectContext, id: &str) -> Option<WorkspaceContext> {
    project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == id)
        .cloned()
}
