use super::dto::*;
use control_tower_application::{Stage, Workbench, WorkbenchStatus};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

pub(super) const MAX_DEFINITION_BYTES: usize = 128 * 1024;
pub(super) const SETUP_GUIDANCE: &str = "Prepare this workspace explicitly with control-tower-db bootstrap-local, migrate-local, and verify-local.";

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
    pub(super) workbench: Arc<Workbench>,
}

pub(super) fn discover_project(project_root: &Path) -> Result<ProjectContext, String> {
    let root = project_root.canonicalize().map_err(|error| {
        format!(
            "cannot open launch Project {}: {error}",
            project_root.display()
        )
    })?;
    if !root.is_dir() {
        return Err(format!(
            "launch Project {} is not a directory",
            root.display()
        ));
    }
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Project")
        .to_owned();
    let inventory = root.join("workspaces");
    let mut candidates = match fs::read_dir(&inventory) {
        Ok(entries) => entries
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Cannot list workspaces/: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!(
                "No prepared workspaces were found under {}. {SETUP_GUIDANCE}",
                inventory.display()
            ));
        }
        Err(error) => return Err(format!("Cannot inspect workspaces/: {error}")),
    };
    candidates.sort_by_key(|entry| entry.file_name());
    let inventory = inventory
        .canonicalize()
        .map_err(|error| format!("Cannot resolve workspaces/: {error}"))?;
    if !inventory.starts_with(&root) || !inventory.is_dir() {
        return Err("workspaces/ must be a directory inside the launch Project.".to_owned());
    }
    let mut seen = BTreeSet::new();
    let mut workspaces = Vec::new();
    for entry in candidates {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = entry.file_name().into_string().map_err(|_| {
            format!(
                "Workspace name in {} is not valid UTF-8.",
                inventory.display()
            )
        })?;
        let workspace_root = path
            .canonicalize()
            .map_err(|error| format!("Workspace `{id}` cannot be resolved: {error}"))?;
        if !workspace_root.starts_with(&inventory) {
            return Err(format!("Workspace `{id}` resolves outside workspaces/."));
        }
        if !seen.insert(workspace_root.clone()) {
            continue;
        }
        let workbench = Arc::new(
            crate::deps::workbench(&workspace_root)
                .map_err(|error| format!("Workspace `{id}`: {error}. {SETUP_GUIDANCE}"))?,
        );
        let status = workbench
            .status(&workspace_root)
            .map_err(|error| format!("Workspace `{id}` is not ready: {error}. {SETUP_GUIDANCE}"))?;
        if status.stages.is_empty() {
            return Err(format!(
                "Workspace `{id}` has no stages. Add at least one valid `stages/<number>-<name>/` directory with an executable `up` or `down` role, then prepare its database with {SETUP_GUIDANCE}"
            ));
        }
        workspaces.push(WorkspaceContext {
            id,
            name: entry.file_name().to_string_lossy().replace(['-', '_'], " "),
            root: workspace_root,
            workbench,
        });
    }
    if workspaces.is_empty() {
        return Err(format!(
            "No prepared workspaces were found under {}. {SETUP_GUIDANCE}",
            inventory.display()
        ));
    }
    Ok(ProjectContext { name, workspaces })
}

pub(super) fn project_snapshot(project: &ProjectContext) -> ProjectView {
    let workspaces = project
        .workspaces
        .iter()
        .map(
            |workspace| match workspace.workbench.status(&workspace.root) {
                Ok(status) => WorkspaceSummary {
                    id: workspace.id.clone(),
                    name: workspace.name.clone(),
                    available: true,
                    issue: None,
                    stage_count: Some(status.stages.len()),
                    accepted_stage: accepted_stage(&status),
                    pending_transition: pending_view(&status),
                },
                Err(issue) => WorkspaceSummary {
                    id: workspace.id.clone(),
                    name: workspace.name.clone(),
                    available: false,
                    issue: Some(format!("{issue}. {SETUP_GUIDANCE}")),
                    stage_count: None,
                    accepted_stage: None,
                    pending_transition: None,
                },
            },
        )
        .collect();
    ProjectView {
        name: project.name.clone(),
        workspace_root: "workspaces/".to_owned(),
        workspaces,
    }
}

pub(super) fn workspace_snapshot(
    project_name: &str,
    workspace: &WorkspaceContext,
    observation: Option<MovementObservation>,
    movement_busy: bool,
) -> WorkspaceView {
    match workspace.workbench.status(&workspace.root) {
        Ok(status) => {
            available_workspace_view(project_name, workspace, status, observation, movement_busy)
        }
        Err(error) => WorkspaceView {
            project_name: project_name.to_owned(),
            workspace: WorkspaceIdentity {
                id: workspace.id.clone(),
                name: workspace.name.clone(),
            },
            current_status: "unavailable",
            status_issue: Some(format!("{error}. {SETUP_GUIDANCE}")),
            checkpoint: None,
            movement_choices: Vec::new(),
            stages: Vec::new(),
            selected_stage_number: None,
            movement_busy,
            observation,
        },
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
    let selected_stage_number = pending
        .as_ref()
        .map(|p| p.stage.number)
        .or_else(|| accepted.as_ref().map(|s| s.number))
        .or_else(|| status.stages.first().map(|s| s.number));
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
        workspace: WorkspaceIdentity {
            id: workspace.id.clone(),
            name: workspace.name.clone(),
        },
        current_status: "available",
        status_issue: None,
        checkpoint: Some(CheckpointView {
            accepted_stage: accepted,
            pending_transition: pending,
            workflow_started: status.state.uuid.is_some(),
            state: CheckpointState::from(&status.state),
        }),
        movement_choices: status
            .movement_choices()
            .into_iter()
            .map(movement_choice_view)
            .collect(),
        stages,
        selected_stage_number,
        movement_busy,
        observation,
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
