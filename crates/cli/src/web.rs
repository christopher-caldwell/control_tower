use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path as RoutePath, State};
use axum::http::header::{
    AUTHORIZATION, CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, COOKIE, HOST, ORIGIN,
    SET_COOKIE, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderName, HeaderValue, Request, Response, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::{Router, serve};
use control_tower_application::{Direction, Stage, StatusError, WorkbenchStatus};
use rust_embed::RustEmbed;
use serde::Serialize;
use tokio::net::TcpListener as TokioTcpListener;
use tokio::signal;
use uuid::Uuid;

#[derive(RustEmbed)]
#[folder = "../../ui/dist/"]
struct FrontendAssets;

const MAX_DEFINITION_BYTES: usize = 128 * 1024;
const SETUP_GUIDANCE: &str = "Prepare this workspace explicitly with control-tower-db bootstrap-local, migrate-local, and verify-local; reads never initialize or repair storage.";

#[derive(Clone)]
struct ProjectContext {
    name: String,
    workspaces: Vec<WorkspaceContext>,
    discovery_error: Option<String>,
}

#[derive(Clone)]
struct WorkspaceContext {
    id: String,
    name: String,
    root: PathBuf,
    unavailable_reason: Option<String>,
}

#[derive(Clone)]
struct ServerContext {
    project: ProjectContext,
    expected_host: String,
    expected_origin: String,
    bootstrap_token: String,
    session_token: String,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ApiError,
}

#[derive(Serialize)]
struct ApiError {
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
struct ProjectView {
    name: String,
    workspace_root: String,
    workspaces: Vec<WorkspaceSummary>,
    discovery_error: Option<String>,
}

#[derive(Serialize)]
struct WorkspaceSummary {
    id: String,
    name: String,
    available: bool,
    issue: Option<String>,
    stage_count: Option<usize>,
    accepted_stage: Option<StageIdentity>,
    pending_transition: Option<PendingView>,
}

#[derive(Clone, Serialize)]
struct StageIdentity {
    number: u32,
    name: String,
}

#[derive(Serialize)]
struct WorkspaceIdentity {
    id: String,
    name: String,
}

#[derive(Clone, Serialize)]
struct PendingView {
    direction: &'static str,
    stage: StageIdentity,
}

#[derive(Serialize)]
struct WorkspaceView {
    project_name: String,
    workspace: WorkspaceIdentity,
    checkpoint: CheckpointView,
    selected_stage_number: Option<u32>,
    stages: Vec<StageView>,
}

#[derive(Serialize)]
struct CheckpointView {
    accepted_stage: Option<StageIdentity>,
    pending_transition: Option<PendingView>,
    workflow_started: bool,
}

#[derive(Serialize)]
struct StageView {
    number: u32,
    name: String,
    state: &'static str,
    is_accepted_checkpoint: bool,
    definitions: Vec<DefinitionSummary>,
}

#[derive(Serialize)]
struct DefinitionSummary {
    role: &'static str,
    path: String,
}

#[derive(Serialize)]
struct StageDefinitionView {
    stage: StageIdentity,
    definitions: Vec<DefinitionView>,
}

#[derive(Serialize)]
struct DefinitionView {
    role: &'static str,
    path: String,
    contents: Option<String>,
    truncated: bool,
    issue: Option<String>,
}

pub(super) fn run_from_current_directory() -> Result<(), String> {
    #[cfg(not(target_os = "macos"))]
    {
        Err("the browser workbench is supported on macOS only".to_owned())
    }

    #[cfg(target_os = "macos")]
    run_on_macos(
        &std::env::current_dir()
            .map_err(|error| format!("cannot determine the launch Project directory: {error}"))?,
    )
}

#[cfg(target_os = "macos")]
fn run_on_macos(project_root: &Path) -> Result<(), String> {
    let project = discover_project(project_root)?;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| format!("cannot bind the loopback UI listener: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("cannot prepare the UI listener: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("cannot inspect the UI listener: {error}"))?;
    let host = format!("127.0.0.1:{}", address.port());
    let origin = format!("http://{host}");
    let bootstrap_token = Uuid::new_v4().to_string();
    let session_token = Uuid::new_v4().to_string();
    let url = format!("{origin}/#session={bootstrap_token}");
    let context = Arc::new(ServerContext {
        project,
        expected_host: host,
        expected_origin: origin,
        bootstrap_token,
        session_token,
    });

    println!("Control Tower UI: {url}");
    match Command::new("open").arg(&url).status() {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!("Could not open the system browser (exit status {status}).");
            eprintln!("Open the URL above manually; the local server is still available.");
        }
        Err(error) => {
            eprintln!("Could not open the system browser: {error}");
            eprintln!("Open the URL above manually; the local server is still available.");
        }
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("cannot start the UI runtime: {error}"))?;
    runtime.block_on(async move {
        let listener = TokioTcpListener::from_std(listener)
            .map_err(|error| format!("cannot start the UI listener: {error}"))?;
        serve(listener, router(context))
            .with_graceful_shutdown(async {
                let _ = signal::ctrl_c().await;
            })
            .await
            .map_err(|error| format!("the UI server stopped unexpectedly: {error}"))
    })
}

fn router(context: Arc<ServerContext>) -> Router {
    Router::new()
        .route("/api/session", post(create_session))
        .route("/api/project", get(project_view))
        .route("/api/workspaces/{id}", get(workspace_view))
        .route(
            "/api/workspaces/{id}/stages/{stage_number}",
            get(stage_definition),
        )
        .route("/", get(index))
        .route("/assets/{*path}", get(asset))
        .fallback(not_found)
        .layer(middleware::from_fn_with_state(context.clone(), security))
        .with_state(context)
}

async fn security(
    State(context): State<Arc<ServerContext>>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let headers = request.headers();
    let host_matches = headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|host| host == context.expected_host);
    if !host_matches {
        return api_error(
            StatusCode::MISDIRECTED_REQUEST,
            "invalid_host",
            "This server accepts requests only for its advertised loopback host.",
        );
    }

    if request.uri().path().starts_with("/api/") {
        let origin = headers.get(ORIGIN).and_then(|value| value.to_str().ok());
        let origin_is_valid = origin.is_none_or(|value| value == context.expected_origin);
        let origin_required = request.method() == axum::http::Method::POST;
        if !origin_is_valid || (origin_required && origin.is_none()) {
            return api_error(
                StatusCode::FORBIDDEN,
                "invalid_origin",
                "API requests must come from this UI origin.",
            );
        }

        if request.uri().path() != "/api/session"
            && !session_cookie_matches(headers.get(COOKIE), &context.session_token)
        {
            return api_error(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Open this UI from the URL printed by the Control Tower terminal.",
            );
        }
    }

    next.run(request).await
}

async fn create_session(
    State(context): State<Arc<ServerContext>>,
    request: Request<Body>,
) -> Response<Body> {
    let expected = format!("Bearer {}", context.bootstrap_token);
    let supplied = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if !constant_time_eq(supplied.as_bytes(), expected.as_bytes()) {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "invalid_startup_token",
            "The startup token is missing or invalid. Relaunch the UI from the terminal.",
        );
    }

    let mut response = StatusCode::NO_CONTENT.into_response();
    let cookie = format!(
        "ct_session={}; Path=/; HttpOnly; SameSite=Strict",
        context.session_token
    );
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(SET_COOKIE, value);
    }
    add_browser_headers(response)
}

async fn project_view(State(context): State<Arc<ServerContext>>) -> Response<Body> {
    let project = context.project.clone();
    match tokio::task::spawn_blocking(move || project_snapshot(&project)).await {
        Ok(view) => api_json(view),
        Err(error) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "project_read_failed",
            format!("Could not read Project status: {error}"),
        ),
    }
}

async fn workspace_view(
    State(context): State<Arc<ServerContext>>,
    RoutePath(id): RoutePath<String>,
) -> Response<Body> {
    let Some(workspace) = find_workspace(&context.project, &id) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "unknown_workspace",
            "That Workspace was not discovered when Control Tower started.",
        );
    };
    let project_name = context.project.name.clone();
    match tokio::task::spawn_blocking(move || workspace_snapshot(&project_name, &workspace)).await {
        Ok(Ok(view)) => api_json(view),
        Ok(Err(message)) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "workspace_unavailable",
            message,
        ),
        Err(error) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "workspace_read_failed",
            format!("Could not read Workspace status: {error}"),
        ),
    }
}

async fn stage_definition(
    State(context): State<Arc<ServerContext>>,
    RoutePath((id, stage_number)): RoutePath<(String, String)>,
) -> Response<Body> {
    let stage_number = match stage_number.parse::<u32>() {
        Ok(number) if number > 0 => number,
        _ => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "invalid_stage_number",
                "Stage numbers must be positive decimal identities.",
            );
        }
    };
    let Some(workspace) = find_workspace(&context.project, &id) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "unknown_workspace",
            "That Workspace was not discovered when Control Tower started.",
        );
    };
    match tokio::task::spawn_blocking(move || {
        let status = read_status(&workspace)?;
        let stage = status
            .stages
            .iter()
            .find(|stage| stage.number == stage_number)
            .ok_or_else(|| format!("Stage {stage_number} was not found in this Workspace."))?;
        Ok::<_, String>(definition_view(stage, &workspace.root))
    })
    .await
    {
        Ok(Ok(view)) => api_json(view),
        Ok(Err(message)) => {
            let status = if message.starts_with("Stage ") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };
            api_error(status, "stage_unavailable", message)
        }
        Err(error) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "stage_read_failed",
            format!("Could not read Stage definitions: {error}"),
        ),
    }
}

async fn index() -> Response<Body> {
    let Some(file) = FrontendAssets::get("index.html") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut response = Response::new(Body::from(file.data.into_owned()));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    add_browser_headers(response)
}

async fn asset(RoutePath(path): RoutePath<String>) -> Response<Body> {
    let embedded_path = format!("assets/{path}");
    let Some(file) = FrontendAssets::get(&embedded_path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let content_type = match Path::new(&path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    };
    let mut response = Response::new(Body::from(file.data.into_owned()));
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    add_browser_headers(response)
}

async fn not_found(request: Request<Body>) -> Response<Body> {
    if request.uri().path().starts_with("/api/") {
        api_error(
            StatusCode::NOT_FOUND,
            "unknown_route",
            "That API route is not part of this Control Tower host.",
        )
    } else {
        add_browser_headers(StatusCode::NOT_FOUND.into_response())
    }
}

fn api_json<T: Serialize>(value: T) -> Response<Body> {
    let mut response = Json(value).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    add_browser_headers(response)
}

fn add_browser_headers(mut response: Response<Body>) -> Response<Body> {
    response.headers_mut().insert(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
    );
    response
        .headers_mut()
        .insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    response.headers_mut().insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    response
}

fn api_error(status: StatusCode, code: &'static str, message: impl Into<String>) -> Response<Body> {
    let mut response = (
        status,
        Json(ErrorEnvelope {
            error: ApiError {
                code,
                message: message.into(),
            },
        }),
    )
        .into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    add_browser_headers(response)
}

fn session_cookie_matches(cookie: Option<&HeaderValue>, expected: &str) -> bool {
    let Some(cookie) = cookie.and_then(|value| value.to_str().ok()) else {
        return false;
    };
    cookie.split(';').any(|part| {
        let Some((name, value)) = part.trim().split_once('=') else {
            return false;
        };
        name == "ct_session" && constant_time_eq(value.as_bytes(), expected.as_bytes())
    })
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        difference |= usize::from(
            left.get(index).copied().unwrap_or(0) ^ right.get(index).copied().unwrap_or(0),
        );
    }
    difference == 0
}

fn discover_project(project_root: &Path) -> Result<ProjectContext, String> {
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

fn valid_workspace_id(id: &str) -> bool {
    !id.is_empty()
        && id != "."
        && id != ".."
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn workspace_tree_issue(root: &Path) -> Option<String> {
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

fn project_snapshot(project: &ProjectContext) -> ProjectView {
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

fn workspace_snapshot(
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
    })
}

fn read_status(workspace: &WorkspaceContext) -> Result<WorkbenchStatus, String> {
    if let Some(issue) = &workspace.unavailable_reason {
        return Err(issue.clone());
    }
    workspace_tree_issue(&workspace.root).map_or(Ok(()), Err)?;
    let service = super::deps::workbench(&workspace.root)
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

fn ensure_confined(root: &Path, path: &Path) -> Result<(), String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Cannot resolve stage path safely: {error}"))?;
    if canonical.starts_with(root) {
        Ok(())
    } else {
        Err("A stage or executable path resolves outside this Workspace.".to_owned())
    }
}

fn accepted_stage(status: &WorkbenchStatus) -> Option<StageIdentity> {
    status
        .state
        .completed_stage_count
        .checked_sub(1)
        .and_then(|index| status.stages.get(index))
        .map(stage_identity)
}

fn pending_view(status: &WorkbenchStatus) -> Option<PendingView> {
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

fn stage_identity(stage: &Stage) -> StageIdentity {
    StageIdentity {
        number: stage.number,
        name: stage.name.clone(),
    }
}

fn definition_summaries(stage: &Stage, workspace_root: &Path) -> Vec<DefinitionSummary> {
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

fn definition_view(stage: &Stage, workspace_root: &Path) -> StageDefinitionView {
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

fn role_paths(stage: &Stage) -> [(&'static str, Option<&Path>); 4] {
    [
        ("up", stage.up.as_deref()),
        ("down", stage.down.as_deref()),
        ("verify-up", stage.verify_up.as_deref()),
        ("verify-down", stage.verify_down.as_deref()),
    ]
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn find_workspace(project: &ProjectContext, id: &str) -> Option<WorkspaceContext> {
    if !valid_workspace_id(id) {
        return None;
    }
    project
        .workspaces
        .iter()
        .find(|workspace| workspace.id == id)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::{AUTHORIZATION, COOKIE, HOST, ORIGIN};
    use control_tower_database::operations;
    use http_body_util::BodyExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tower::ServiceExt;

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "control-tower-web-{}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn workspace(project: &Path, id: &str, stage: &str, prepared: bool) -> PathBuf {
        let root = project.join("workspaces").join(id);
        let stage_dir = root.join("stages").join(stage);
        fs::create_dir_all(&stage_dir).unwrap();
        fs::write(stage_dir.join("up"), "#!/bin/sh\nprintf 'safe <tag>'\n").unwrap();
        if prepared {
            let db = root.join(".control_tower/state.sqlite3");
            operations::bootstrap(&db).unwrap();
            operations::migrate(&db).unwrap();
        }
        root
    }

    fn context(project: ProjectContext) -> Arc<ServerContext> {
        Arc::new(ServerContext {
            project,
            expected_host: "127.0.0.1:43001".to_owned(),
            expected_origin: "http://127.0.0.1:43001".to_owned(),
            bootstrap_token: "startup-secret".to_owned(),
            session_token: "browser-session".to_owned(),
        })
    }

    #[test]
    fn project_lists_independent_workspace_availability_and_real_stage_status() {
        let temp = TempDir::new();
        let project = temp.path().join("demo");
        fs::create_dir_all(&project).unwrap();
        let ready = workspace(&project, "ready", "200-later", true);
        let unprepared = workspace(&project, "unprepared", "001-first", false);
        fs::write(
            ready.join("stages/200-later/up"),
            "#!/bin/sh\ntouch $CONTROL_TOWER_WORKSPACE/should-not-run\n",
        )
        .unwrap();
        let discovered = discover_project(&project).unwrap();
        let view = project_snapshot(&discovered);
        assert_eq!(view.name, "demo");
        assert_eq!(view.workspaces.len(), 2);
        assert!(view.workspaces[0].available);
        assert_eq!(view.workspaces[0].stage_count, Some(1));
        assert!(view.workspaces[0].accepted_stage.is_none());
        assert!(!view.workspaces[1].available);
        assert!(
            view.workspaces[1]
                .issue
                .as_deref()
                .unwrap()
                .contains("Prepare this workspace explicitly")
        );
        assert!(!ready.join("should-not-run").exists());
        assert!(!unprepared.join(".control_tower").exists());
    }

    #[test]
    fn workspace_view_uses_sparse_numbers_and_unknown_historical_verification() {
        let temp = TempDir::new();
        let project = temp.path().join("demo");
        fs::create_dir_all(&project).unwrap();
        workspace(&project, "fixture", "010-seed", true);
        workspace(&project, "fixture", "200-finish", true);
        let discovered = discover_project(&project).unwrap();
        let view = workspace_snapshot("demo", &discovered.workspaces[0]).unwrap();
        assert_eq!(view.workspace.id, "fixture");
        assert_eq!(view.selected_stage_number, Some(10));
        assert_eq!(
            view.stages
                .iter()
                .map(|stage| stage.number)
                .collect::<Vec<_>>(),
            vec![10, 200]
        );
        assert!(view.stages.iter().all(|stage| stage.state == "future"));
        assert_eq!(view.stages[0].definitions[0].role, "up");
    }

    #[test]
    fn bad_workspace_layout_and_escaped_candidate_remain_local_to_each_item() {
        let temp = TempDir::new();
        let project = temp.path().join("demo");
        let outside = temp.path().join("outside");
        fs::create_dir_all(project.join("workspaces")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, project.join("workspaces/escaped")).unwrap();
        workspace(&project, "good", "001-first", true);
        workspace(&project, "malformed", "not-numbered", true);
        let view = project_snapshot(&discover_project(&project).unwrap());
        assert!(
            view.workspaces
                .iter()
                .any(|item| item.id == "good" && item.available)
        );
        assert!(view.workspaces.iter().any(|item| {
            item.id == "malformed"
                && !item.available
                && item
                    .issue
                    .as_deref()
                    .unwrap()
                    .contains("must begin with a number")
        }));
        #[cfg(unix)]
        assert!(view.workspaces.iter().any(|item| {
            item.id == "escaped"
                && !item.available
                && item
                    .issue
                    .as_deref()
                    .unwrap()
                    .contains("outside the discovered workspaces/")
        }));
    }

    #[test]
    fn definition_content_is_bounded_and_remains_plain_text() {
        let temp = TempDir::new();
        let workspace = temp.path().join("workspace");
        let stage_dir = workspace.join("stages/001-first");
        fs::create_dir_all(&stage_dir).unwrap();
        fs::write(
            stage_dir.join("up"),
            format!("<script>{}</script>", "x".repeat(MAX_DEFINITION_BYTES)),
        )
        .unwrap();
        let stage = Stage {
            number: 1,
            name: "first".to_owned(),
            directory: stage_dir.clone(),
            up: Some(stage_dir.join("up")),
            down: None,
            verify_up: None,
            verify_down: None,
        };
        let view = definition_view(&stage, &workspace);
        assert!(view.definitions[0].truncated);
        assert!(
            view.definitions[0]
                .contents
                .as_ref()
                .unwrap()
                .starts_with("<script>")
        );
    }

    #[test]
    fn startup_security_requires_origin_and_session_for_private_api() {
        assert!(valid_workspace_id("shell-python-node"));
        assert!(!valid_workspace_id("../secrets"));
        assert!(!valid_workspace_id("%2e%2e"));
        assert!(session_cookie_matches(
            Some(&HeaderValue::from_static(
                "other=x; ct_session=browser-session"
            )),
            "browser-session"
        ));
        assert!(!session_cookie_matches(
            Some(&HeaderValue::from_static("ct_session=wrong")),
            "browser-session"
        ));
    }

    #[tokio::test]
    async fn private_reads_require_expected_host_origin_and_session() {
        let temp = TempDir::new();
        let context = context(discover_project(temp.path()).unwrap());

        let missing_session = router(context.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/project")
                    .header(HOST, "127.0.0.1:43001")
                    .header(ORIGIN, "http://127.0.0.1:43001")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing_session.status(), StatusCode::UNAUTHORIZED);

        let wrong_origin = router(context.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/project")
                    .header(HOST, "127.0.0.1:43001")
                    .header(ORIGIN, "https://attacker.example")
                    .header(COOKIE, "ct_session=browser-session")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(wrong_origin.status(), StatusCode::FORBIDDEN);

        let wrong_host = router(context.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/project")
                    .header(HOST, "localhost:43001")
                    .header(ORIGIN, "http://127.0.0.1:43001")
                    .header(COOKIE, "ct_session=browser-session")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(wrong_host.status(), StatusCode::MISDIRECTED_REQUEST);

        let authorized = router(context)
            .oneshot(
                Request::builder()
                    .uri("/api/project")
                    .header(HOST, "127.0.0.1:43001")
                    .header(ORIGIN, "http://127.0.0.1:43001")
                    .header(COOKIE, "ct_session=browser-session")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(authorized.status(), StatusCode::OK);
        let body = authorized.into_body().collect().await.unwrap().to_bytes();
        let view: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(view["workspaces"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn startup_token_only_sets_an_httponly_same_site_cookie() {
        let temp = TempDir::new();
        let context = context(discover_project(temp.path()).unwrap());
        let response = router(context.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/session")
                    .header(HOST, "127.0.0.1:43001")
                    .header(ORIGIN, "http://127.0.0.1:43001")
                    .header(AUTHORIZATION, "Bearer startup-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let cookie = response
            .headers()
            .get(SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("browser-session"));
        assert!(!cookie.contains("startup-secret"));
    }

    #[tokio::test]
    async fn static_shell_has_browser_guards_and_uses_bundled_assets() {
        let temp = TempDir::new();
        let context = context(discover_project(temp.path()).unwrap());
        let response = router(context.clone())
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(HOST, "127.0.0.1:43001")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key(CONTENT_SECURITY_POLICY));
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("/assets/"));
        assert!(!html.contains("session="));

        let script = FrontendAssets::iter()
            .find(|path| path.ends_with(".js"))
            .unwrap()
            .to_string();
        let script_path = script.strip_prefix("assets/").unwrap();
        let response = router(context)
            .oneshot(
                Request::builder()
                    .uri(format!("/assets/{script_path}"))
                    .header(HOST, "127.0.0.1:43001")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key(CONTENT_SECURITY_POLICY));
    }
}
