use super::dto::*;
use control_tower_application::{Direction, Stage, StatusError, WorkbenchStatus};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub(super) const MAX_DEFINITION_BYTES: usize = 128 * 1024;

pub(super) const SETUP_GUIDANCE: &str = "Prepare this workspace explicitly with control-tower-db bootstrap-local, migrate-local, and verify-local; reads never initialize or repair storage.";

#[derive(Clone)]
pub(super) struct ProjectContext {
    pub(super) name: String,
    pub(super) workspaces: Vec<WorkspaceContext>,
    pub(super) discovery_error: Option<String>,
}

#[derive(Clone)]
pub(super) struct WorkspaceContext {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) root: PathBuf,
    pub(super) unavailable_reason: Option<String>,
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
    let workspaces_dir = root.join("workspaces");
    let mut workspaces = Vec::new();
    let mut discovery_error = None;
    match fs::symlink_metadata(&workspaces_dir) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => discovery_error = Some(format!("Cannot inspect workspaces/: {error}")),
        Ok(_) => match workspaces_dir.canonicalize() {
            Ok(canonical) if canonical.starts_with(&root) && canonical.is_dir() => {
                match fs::read_dir(&canonical) {
                    Ok(entries) => {
                        let mut seen = BTreeSet::new();
                        for entry in entries {
                            let entry = match entry {
                                Ok(entry) => entry,
                                Err(error) => {
                                    discovery_error =
                                        Some(format!("Cannot read a workspace entry: {error}"));
                                    continue;
                                }
                            };
                            let path = entry.path();
                            if !path.is_dir() {
                                continue;
                            }
                            let id = entry.file_name().to_string_lossy().into_owned();
                            if !valid_workspace_id(&id) {
                                continue;
                            }
                            let name = id.replace(['-', '_'], " ");
                            match path.canonicalize() {
                                Ok(workspace_root) if workspace_root.starts_with(&canonical) => {
                                    if !seen.insert(workspace_root.clone()) {
                                        continue;
                                    }
                                    let unavailable_reason = workspace_tree_issue(&workspace_root);
                                    workspaces.push(WorkspaceContext {
                                        id,
                                        name,
                                        root: workspace_root,
                                        unavailable_reason,
                                    });
                                }
                                Ok(_) => workspaces.push(WorkspaceContext {
                                    id,
                                    name,
                                    root: path,
                                    unavailable_reason: Some(
                                        "Workspace path resolves outside the discovered workspaces/ directory."
                                            .to_owned(),
                                    ),
                                }),
                                Err(error) => workspaces.push(WorkspaceContext {
                                    id,
                                    name,
                                    root: path,
                                    unavailable_reason: Some(format!(
                                        "Workspace path cannot be resolved safely: {error}"
                                    )),
                                }),
                            }
                        }
                    }
                    Err(error) => {
                        discovery_error = Some(format!("Cannot list workspaces/: {error}"))
                    }
                }
            }
            Ok(_) => {
                discovery_error = Some(
                    "workspaces/ must resolve to a directory contained in the launch Project."
                        .to_owned(),
                )
            }
            Err(error) => discovery_error = Some(format!("Cannot resolve workspaces/: {error}")),
        },
    }
    workspaces.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(ProjectContext {
        name,
        workspaces,
        discovery_error,
    })
}

pub(super) fn valid_workspace_id(id: &str) -> bool {
    !id.is_empty()
        && id != "."
        && id != ".."
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

pub(super) fn workspace_tree_issue(root: &Path) -> Option<String> {
    let stages_root = root.join("stages");
    match stages_root.canonicalize() {
        Ok(canonical) if canonical.starts_with(root) && canonical.is_dir() => {}
        Ok(_) => return Some("stages/ resolves outside this Workspace.".to_owned()),
        Err(error) => return Some(format!("Cannot access stages/: {error}")),
    }
    let database = root.join(".control_tower/state.sqlite3");
    if database.exists() {
        match database.canonicalize() {
            Ok(canonical) if canonical.starts_with(root) => {}
            Ok(_) => return Some("Checkpoint storage resolves outside this Workspace.".to_owned()),
            Err(error) => return Some(format!("Cannot resolve checkpoint storage: {error}")),
        }
    }
    None
}

pub(super) fn project_snapshot(project: &ProjectContext) -> ProjectView {
    let workspaces = project
        .workspaces
        .iter()
        .map(|workspace| match read_status(workspace) {
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
                issue: Some(issue),
                stage_count: None,
                accepted_stage: None,
                pending_transition: None,
            },
        })
        .collect();
    ProjectView {
        name: project.name.clone(),
        workspace_root: "workspaces/".to_owned(),
        workspaces,
        discovery_error: project.discovery_error.clone(),
    }
}

pub(super) fn workspace_snapshot(
    project_name: &str,
    workspace: &WorkspaceContext,
) -> Result<WorkspaceView, String> {
    let status = read_status(workspace)?;
    let accepted = accepted_stage(&status);
    let pending = pending_view(&status);
    let selected_stage_number = pending
        .as_ref()
        .map(|pending| pending.stage.number)
        .or_else(|| accepted.as_ref().map(|stage| stage.number))
        .or_else(|| status.stages.first().map(|stage| stage.number));
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
    Ok(WorkspaceView {
        project_name: project_name.to_owned(),
        workspace: WorkspaceIdentity {
            id: workspace.id.clone(),
            name: workspace.name.clone(),
        },
        checkpoint: CheckpointView {
            accepted_stage: accepted,
            pending_transition: pending,
            workflow_started: status.state.uuid.is_some(),
        },
        selected_stage_number,
        stages,
        server_instance_id: String::new(),
        observation_revision: 0,
        movement_busy: false,
        observation: None,
        storage_issue: None,
    })
}

pub(super) fn read_status(workspace: &WorkspaceContext) -> Result<WorkbenchStatus, String> {
    if let Some(issue) = &workspace.unavailable_reason {
        return Err(issue.clone());
    }
    workspace_tree_issue(&workspace.root).map_or(Ok(()), Err)?;
    let service = crate::deps::workbench(&workspace.root)
        .map_err(|error| format!("{error}. {SETUP_GUIDANCE}"))?;
    let status = match service.status(&workspace.root) {
        Ok(status) => status,
        Err(StatusError::Persistence(error)) => {
            return Err(format!("{error}. {SETUP_GUIDANCE}"));
        }
        Err(error) => return Err(error.to_string()),
    };
    for stage in &status.stages {
        ensure_confined(&workspace.root, &stage.directory)?;
        for executable in [&stage.up, &stage.down, &stage.verify_up, &stage.verify_down]
            .into_iter()
            .flatten()
        {
            ensure_confined(&workspace.root, executable)?;
        }
    }
    Ok(status)
}

pub(super) fn ensure_confined(root: &Path, path: &Path) -> Result<(), String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Cannot resolve stage path safely: {error}"))?;
    if canonical.starts_with(root) {
        Ok(())
    } else {
        Err("A stage or executable path resolves outside this Workspace.".to_owned())
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
                direction: match pending.direction {
                    Direction::Up => "up",
                    Direction::Down => "down",
                },
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

pub(super) fn definition_view(stage: &Stage, workspace_root: &Path) -> StageDefinitionView {
    let definitions = role_paths(stage)
        .into_iter()
        .filter_map(|(role, path)| {
            path.map(|path| {
                let relative = relative_path(workspace_root, path);
                let (contents, truncated, issue) = match fs::File::open(path) {
                    Ok(file) => {
                        let mut bytes = Vec::new();
                        match file
                            .take((MAX_DEFINITION_BYTES + 1) as u64)
                            .read_to_end(&mut bytes)
                        {
                            Ok(_) => {
                                let truncated = bytes.len() > MAX_DEFINITION_BYTES;
                                bytes.truncate(MAX_DEFINITION_BYTES);
                                (
                                    Some(String::from_utf8_lossy(&bytes).into_owned()),
                                    truncated,
                                    None,
                                )
                            }
                            Err(error) => (
                                None,
                                false,
                                Some(format!("Cannot read definition: {error}")),
                            ),
                        }
                    }
                    Err(error) => (
                        None,
                        false,
                        Some(format!("Cannot read definition: {error}")),
                    ),
                };
                DefinitionView {
                    role,
                    path: relative,
                    contents,
                    truncated,
                    issue,
                }
            })
        })
        .collect();
    StageDefinitionView {
        stage: stage_identity(stage),
        definitions,
    }
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
    if !valid_workspace_id(id) {
        return None;
    }
    project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == id)
        .cloned()
}
