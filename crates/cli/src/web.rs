use std::collections::{BTreeSet, HashMap, VecDeque};
use std::fs;
use std::io::Read;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{Path as RoutePath, State};
use axum::http::header::{
    AUTHORIZATION, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_SECURITY_POLICY, CONTENT_TYPE,
    COOKIE, HOST, ORIGIN, SET_COOKIE, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderName, HeaderValue, Request, Response, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::{Router, serve};
use control_tower_application::{
    Direction, ExecutableRole, ExecutionProgress, MoveStatus, MoveToError, MoveToInput,
    MovementChoice, Stage, StatusError, TransitionFailure, WorkbenchState, WorkbenchStatus,
};
use futures_util::stream;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener as TokioTcpListener;
use tokio::signal;
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(RustEmbed)]
#[folder = "../../ui/dist/"]
struct FrontendAssets;

const MAX_DEFINITION_BYTES: usize = 128 * 1024;
const MAX_OUTPUT_STREAM_BYTES: usize = 1024 * 1024;
const MAX_WORKSPACE_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_ROLE_RESULTS: usize = 128;
const EVENT_BUFFER: usize = 256;
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
    observations: Arc<ObservationStore>,
}

struct ObservationStore {
    server_instance_id: String,
    workspaces: Mutex<HashMap<String, Arc<WorkspaceRuntime>>>,
}

struct WorkspaceRuntime {
    record: Mutex<WorkspaceRecord>,
    events: broadcast::Sender<RuntimeEvent>,
    next_revision: AtomicU64,
}

struct WorkspaceRecord {
    busy: bool,
    observation: Option<MovementObservation>,
    outputs: HashMap<String, CapturedOutput>,
    output_order: VecDeque<String>,
    output_bytes: usize,
}

#[derive(Clone, Serialize)]
struct RuntimeEvent {
    workspace_id: String,
    server_instance_id: String,
    revision: u64,
    operation_id: Option<String>,
    kind: &'static str,
    direction: Option<&'static str>,
    target_stage: Option<u32>,
    stage: Option<StageIdentity>,
    role: Option<&'static str>,
}

#[derive(Clone, Serialize)]
struct RuntimeSnapshot {
    workspace_id: String,
    server_instance_id: String,
    revision: u64,
    movement_busy: bool,
    observation: Option<MovementObservation>,
}

#[derive(Clone, Serialize)]
struct MovementObservation {
    workspace_id: String,
    operation_id: String,
    server_instance_id: String,
    revision: u64,
    direction: &'static str,
    target_stage: u32,
    state: &'static str,
    active_role: Option<RoleIdentity>,
    role_results: Vec<RoleObservation>,
    omitted_role_results: usize,
    outputs_evicted: usize,
    confirmed_checkpoint: Option<CheckpointView>,
    attempted_checkpoint: Option<CheckpointView>,
    failure: Option<FailureView>,
    verification_choices: Option<RecoveryChoicesView>,
}

#[derive(Clone, Serialize)]
struct RoleIdentity {
    stage: StageIdentity,
    role: &'static str,
}

#[derive(Clone, Serialize)]
struct RoleObservation {
    stage: StageIdentity,
    role: &'static str,
    state: &'static str,
    exit_code: Option<i32>,
    message: Option<String>,
    elapsed_ms: Option<u64>,
    output_id: Option<String>,
    output_state: &'static str,
    stdout_bytes: u64,
    stderr_bytes: u64,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

struct CapturedOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_bytes: u64,
    stderr_bytes: u64,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

#[derive(Clone, Serialize)]
struct FailureView {
    kind: &'static str,
    message: String,
    stage: Option<StageIdentity>,
    role: Option<&'static str>,
}

#[derive(Clone, Serialize)]
struct RecoveryChoicesView {
    retry: RecoveryChoiceView,
    reverse: Option<RecoveryChoiceView>,
}

#[derive(Clone, Serialize)]
struct RecoveryChoiceView {
    direction: &'static str,
    target_stage: u32,
}

#[derive(Deserialize)]
struct MovementRequest {
    direction: String,
    target_stage: u32,
}

#[derive(Serialize)]
struct MovementResponse {
    observation: MovementObservation,
}

#[derive(Clone, Serialize)]
struct ApiErrorDetail {
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
struct MovementErrorEnvelope {
    error: ApiErrorDetail,
    operation_id: Option<String>,
}

struct WorkerResult {
    observation: MovementObservation,
    error: Option<(StatusCode, &'static str, String)>,
}

struct OperationGuard {
    runtime: Arc<WorkspaceRuntime>,
    workspace_id: String,
    server_instance_id: String,
    operation_id: String,
    finalized: bool,
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
    server_instance_id: String,
    observation_revision: u64,
    movement_busy: bool,
    observation: Option<MovementObservation>,
}

#[derive(Clone, Serialize)]
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
        observations: Arc::new(ObservationStore::new()),
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
        .route("/api/workspaces/{id}/movements", post(start_movement))
        .route("/api/workspaces/{id}/events", get(workspace_events))
        .route(
            "/api/workspaces/{id}/outputs/{output_id}/{stream_name}",
            get(read_output),
        )
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
        Ok(Ok(mut view)) => {
            let runtime = context.observations.workspace_state(&id);
            let snapshot = runtime.snapshot(&id, &context.observations.server_instance_id);
            view.server_instance_id = snapshot.server_instance_id;
            view.observation_revision = snapshot.revision;
            view.movement_busy = snapshot.movement_busy;
            view.observation = snapshot.observation;
            api_json(view)
        }
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

async fn start_movement(
    State(context): State<Arc<ServerContext>>,
    RoutePath(id): RoutePath<String>,
    request: Result<Json<MovementRequest>, axum::extract::rejection::JsonRejection>,
) -> Response<Body> {
    let request = match request {
        Ok(Json(request)) => request,
        Err(error) => {
            return movement_error(
                StatusCode::BAD_REQUEST,
                "malformed_request",
                format!("Movement request is not valid JSON: {error}"),
                None,
            );
        }
    };
    let direction = match request.direction.as_str() {
        "up" => Direction::Up,
        "down" => Direction::Down,
        _ => {
            return movement_error(
                StatusCode::BAD_REQUEST,
                "invalid_direction",
                "Movement direction must be `up` or `down`.",
                None,
            );
        }
    };
    if direction == Direction::Up && request.target_stage == 0 {
        return movement_error(
            StatusCode::BAD_REQUEST,
            "invalid_target",
            "An upward movement must name a numbered Stage.",
            None,
        );
    }
    let Some(workspace) = find_workspace(&context.project, &id) else {
        return movement_error(
            StatusCode::NOT_FOUND,
            "unknown_workspace",
            "That Workspace was not discovered when Control Tower started.",
            None,
        );
    };
    if let Some(issue) = workspace.unavailable_reason.as_deref() {
        return movement_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "workspace_unavailable",
            issue,
            None,
        );
    }

    let operation_id = Uuid::new_v4().to_string();
    let runtime = context.observations.workspace_state(&id);
    let Some(_started_observation) = runtime.begin_operation(
        &id,
        &context.observations.server_instance_id,
        &operation_id,
        direction,
        request.target_stage,
    ) else {
        return movement_error(
            StatusCode::CONFLICT,
            "workspace_busy",
            "A movement is already executing for this Workspace.",
            None,
        );
    };
    let runtime_for_worker = runtime.clone();
    let server_instance_id = context.observations.server_instance_id.clone();
    let operation_id_for_worker = operation_id.clone();
    let handle = tokio::task::spawn_blocking(move || {
        let mut guard = OperationGuard::new(
            runtime_for_worker.clone(),
            id.clone(),
            server_instance_id.clone(),
            operation_id_for_worker.clone(),
        );
        let mut worker_result = execute_movement(
            &workspace,
            direction,
            request.target_stage,
            runtime_for_worker,
            &server_instance_id,
            &operation_id_for_worker,
        );
        worker_result.observation = guard.finish(worker_result.observation, "movement.finished");
        worker_result
    });

    match handle.await {
        Ok(result) if result.error.is_none() => api_json(MovementResponse {
            observation: result.observation,
        }),
        Ok(result) => {
            let (status, code, message) = result.error.expect("checked above");
            movement_error(status, code, message, Some(result.observation.operation_id))
        }
        Err(error) => movement_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "movement_task_failed",
            format!("The movement worker stopped unexpectedly: {error}"),
            Some(operation_id),
        ),
    }
}

async fn workspace_events(
    State(context): State<Arc<ServerContext>>,
    RoutePath(id): RoutePath<String>,
) -> Response<Body> {
    if find_workspace(&context.project, &id).is_none() {
        return api_error(
            StatusCode::NOT_FOUND,
            "unknown_workspace",
            "That Workspace was not discovered when Control Tower started.",
        );
    }
    let runtime = context.observations.workspace_state(&id);
    // Subscribe before taking the first snapshot. An operation that begins in
    // the gap is either represented in the snapshot or queued for this stream.
    let receiver = runtime.events.subscribe();
    let snapshot = runtime.snapshot(&id, &context.observations.server_instance_id);
    let stream_id = id.clone();
    let server_instance_id = context.observations.server_instance_id.clone();
    let event_stream = stream::unfold(
        (
            receiver,
            Some(snapshot),
            runtime,
            stream_id,
            server_instance_id,
        ),
        |(mut receiver, initial, runtime, workspace_id, server_instance_id)| async move {
            if let Some(snapshot) = initial {
                let event = sse_json_event("snapshot", &snapshot);
                return Some((
                    Ok::<_, std::convert::Infallible>(event),
                    (receiver, None, runtime, workspace_id, server_instance_id),
                ));
            }
            loop {
                match receiver.recv().await {
                    Ok(event) if event.workspace_id == workspace_id => {
                        let kind = event.kind;
                        let next = sse_json_event(kind, &event);
                        return Some((
                            Ok(next),
                            (receiver, None, runtime, workspace_id, server_instance_id),
                        ));
                    }
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let snapshot = runtime.snapshot(&workspace_id, &server_instance_id);
                        let event = sse_json_event("resync", &snapshot);
                        return Some((
                            Ok(event),
                            (receiver, None, runtime, workspace_id, server_instance_id),
                        ));
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        },
    );
    let mut response = Sse::new(event_stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    add_browser_headers(response)
}

async fn read_output(
    State(context): State<Arc<ServerContext>>,
    RoutePath((id, output_id, stream_name)): RoutePath<(String, String, String)>,
) -> Response<Body> {
    let Some(_workspace) = find_workspace(&context.project, &id) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "unknown_workspace",
            "That Workspace was not discovered when Control Tower started.",
        );
    };
    let stream_name = match stream_name.as_str() {
        "stdout" => "stdout",
        "stderr" => "stderr",
        _ => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "invalid_output_stream",
                "Output stream must be `stdout` or `stderr`.",
            );
        }
    };
    let runtime = context.observations.workspace_state(&id);
    let record = lock_unpoison(&runtime.record);
    let Some(output) = record.outputs.get(&output_id) else {
        return api_error(
            StatusCode::GONE,
            "output_unavailable",
            "This output is no longer retained by the current in-process observation.",
        );
    };
    let (bytes, original_length, truncated) = if stream_name == "stdout" {
        (
            output.stdout.clone(),
            output.stdout_bytes,
            output.stdout_truncated,
        )
    } else {
        (
            output.stderr.clone(),
            output.stderr_bytes,
            output.stderr_truncated,
        )
    };
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{stream_name}.bin\""))
    {
        response.headers_mut().insert(CONTENT_DISPOSITION, value);
    }
    if let Ok(value) = HeaderValue::from_str(&original_length.to_string()) {
        response.headers_mut().insert(
            HeaderName::from_static("x-control-tower-original-bytes"),
            value,
        );
    }
    response.headers_mut().insert(
        HeaderName::from_static("x-control-tower-truncated"),
        HeaderValue::from_static(if truncated { "true" } else { "false" }),
    );
    add_browser_headers(response)
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

fn movement_error(
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
    operation_id: Option<String>,
) -> Response<Body> {
    let mut response = (
        status,
        Json(MovementErrorEnvelope {
            error: ApiErrorDetail {
                code,
                message: message.into(),
            },
            operation_id,
        }),
    )
        .into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    add_browser_headers(response)
}

impl ObservationStore {
    fn new() -> Self {
        Self {
            server_instance_id: Uuid::new_v4().to_string(),
            workspaces: Mutex::new(HashMap::new()),
        }
    }

    fn workspace_state(&self, id: &str) -> Arc<WorkspaceRuntime> {
        let mut workspaces = lock_unpoison(&self.workspaces);
        workspaces
            .entry(id.to_owned())
            .or_insert_with(WorkspaceRuntime::new)
            .clone()
    }
}

impl WorkspaceRuntime {
    fn new() -> Arc<Self> {
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        Arc::new(Self {
            record: Mutex::new(WorkspaceRecord {
                busy: false,
                observation: None,
                outputs: HashMap::new(),
                output_order: VecDeque::new(),
                output_bytes: 0,
            }),
            events,
            next_revision: AtomicU64::new(0),
        })
    }

    fn snapshot(&self, workspace_id: &str, server_instance_id: &str) -> RuntimeSnapshot {
        let record = lock_unpoison(&self.record);
        RuntimeSnapshot {
            workspace_id: workspace_id.to_owned(),
            server_instance_id: server_instance_id.to_owned(),
            revision: self.next_revision.load(Ordering::Relaxed),
            movement_busy: record.busy,
            observation: record.observation.clone(),
        }
    }

    fn begin_operation(
        self: &Arc<Self>,
        workspace_id: &str,
        server_instance_id: &str,
        operation_id: &str,
        direction: Direction,
        target_stage: u32,
    ) -> Option<MovementObservation> {
        let mut record = lock_unpoison(&self.record);
        if record.busy {
            return None;
        }
        record.busy = true;
        record.outputs.clear();
        record.output_order.clear();
        record.output_bytes = 0;
        let mut observation = MovementObservation {
            workspace_id: workspace_id.to_owned(),
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
        };
        self.publish_locked(
            &mut record,
            workspace_id,
            server_instance_id,
            "movement.started",
            Some(operation_id.to_owned()),
            &mut observation,
        );
        record.observation = Some(observation.clone());
        Some(observation)
    }

    fn record_role_started(
        &self,
        workspace_id: &str,
        server_instance_id: &str,
        operation_id: &str,
        stage: &Stage,
        role: ExecutableRole,
    ) {
        let mut record = lock_unpoison(&self.record);
        let Some(mut observation) = record.observation.take() else {
            return;
        };
        if observation.operation_id != operation_id {
            record.observation = Some(observation);
            return;
        }
        let stage = stage_identity(stage);
        let role_name = role.as_str();
        observation.active_role = Some(RoleIdentity {
            stage: stage.clone(),
            role: role_name,
        });
        observation.role_results.push(RoleObservation {
            stage,
            role: role_name,
            state: "in_progress",
            exit_code: None,
            message: None,
            elapsed_ms: None,
            output_id: None,
            output_state: "not_returned",
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdout_truncated: false,
            stderr_truncated: false,
        });
        trim_role_results(&mut record, &mut observation);
        self.publish_locked(
            &mut record,
            workspace_id,
            server_instance_id,
            "role.started",
            Some(operation_id.to_owned()),
            &mut observation,
        );
        record.observation = Some(observation);
    }

    fn record_role_finished(
        &self,
        workspace_id: &str,
        server_instance_id: &str,
        operation_id: &str,
        stage: &Stage,
        execution: &control_tower_application::ExecutionEvent,
        elapsed_ms: Option<u64>,
    ) {
        let role_name = execution.role.as_str();
        let output_data = execution.result.as_ref().ok().map(capture_output);
        let output_id = output_data.as_ref().map(|_| Uuid::new_v4().to_string());
        let mut record = lock_unpoison(&self.record);
        let Some(mut observation) = record.observation.take() else {
            return;
        };
        if observation.operation_id != operation_id {
            record.observation = Some(observation);
            return;
        }
        if let (Some(output_id), Some(output)) = (&output_id, output_data) {
            insert_output(&mut record, &mut observation, output_id.clone(), output);
        }
        let identity = stage_identity(stage);
        let Some(role_observation) = observation
            .role_results
            .iter_mut()
            .rev()
            .find(|result| result.stage.number == identity.number && result.role == role_name)
        else {
            record.observation = Some(observation);
            return;
        };
        role_observation.elapsed_ms = elapsed_ms;
        role_observation.output_id = output_id;
        match &execution.result {
            Ok(output) => {
                role_observation.state = if output.success {
                    "succeeded"
                } else {
                    "failed"
                };
                role_observation.exit_code = output.exit_code;
                role_observation.output_state = "available";
                role_observation.stdout_bytes = output.stdout.len() as u64;
                role_observation.stderr_bytes = output.stderr.len() as u64;
                role_observation.stdout_truncated = output.stdout.len() > MAX_OUTPUT_STREAM_BYTES;
                role_observation.stderr_truncated = output.stderr.len() > MAX_OUTPUT_STREAM_BYTES;
                if !output.success {
                    role_observation.message = Some(match output.exit_code {
                        Some(code) => format!("Process exited with status {code}."),
                        None => "Process ended without a successful exit status.".to_owned(),
                    });
                }
            }
            Err(error) => {
                role_observation.state = "launch_failed";
                role_observation.output_state = "unavailable";
                role_observation.message = Some(error.to_string());
            }
        }
        observation.active_role = None;
        self.publish_locked(
            &mut record,
            workspace_id,
            server_instance_id,
            "role.finished",
            Some(operation_id.to_owned()),
            &mut observation,
        );
        record.observation = Some(observation);
    }

    fn current_observation(&self) -> Option<MovementObservation> {
        lock_unpoison(&self.record).observation.clone()
    }

    fn publish_locked(
        &self,
        record: &mut WorkspaceRecord,
        workspace_id: &str,
        server_instance_id: &str,
        kind: &'static str,
        operation_id: Option<String>,
        observation: &mut MovementObservation,
    ) {
        let revision = self.next_revision.fetch_add(1, Ordering::Relaxed) + 1;
        observation.revision = revision;
        let (stage, role) = match kind {
            "role.started" => observation
                .active_role
                .as_ref()
                .map(|identity| (Some(identity.stage.clone()), Some(identity.role)))
                .unwrap_or((None, None)),
            "role.finished" => observation
                .role_results
                .last()
                .map(|result| (Some(result.stage.clone()), Some(result.role)))
                .unwrap_or((None, None)),
            _ => (None, None),
        };
        let _ = self.events.send(RuntimeEvent {
            workspace_id: workspace_id.to_owned(),
            server_instance_id: server_instance_id.to_owned(),
            revision,
            operation_id,
            kind,
            direction: Some(observation.direction),
            target_stage: Some(observation.target_stage),
            stage,
            role,
        });
        record.observation = Some(observation.clone());
    }
}

impl OperationGuard {
    fn new(
        runtime: Arc<WorkspaceRuntime>,
        workspace_id: String,
        server_instance_id: String,
        operation_id: String,
    ) -> Self {
        Self {
            runtime,
            workspace_id,
            server_instance_id,
            operation_id,
            finalized: false,
        }
    }

    fn finish(
        &mut self,
        mut observation: MovementObservation,
        kind: &'static str,
    ) -> MovementObservation {
        let mut record = lock_unpoison(&self.runtime.record);
        record.busy = false;
        self.runtime.publish_locked(
            &mut record,
            &self.workspace_id,
            &self.server_instance_id,
            kind,
            Some(self.operation_id.clone()),
            &mut observation,
        );
        self.finalized = true;
        observation
    }
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        if self.finalized {
            return;
        }
        let mut record = lock_unpoison(&self.runtime.record);
        record.busy = false;
        let mut observation = record
            .observation
            .take()
            .unwrap_or_else(|| MovementObservation {
                workspace_id: self.workspace_id.clone(),
                operation_id: self.operation_id.clone(),
                server_instance_id: self.server_instance_id.clone(),
                revision: 0,
                direction: "up",
                target_stage: 0,
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
            kind: "worker_interrupted",
            message: "The movement worker stopped before reporting a terminal result. Inspect author-owned effects and the durable checkpoint.".to_owned(),
            stage: None,
            role: None,
        });
        self.runtime.publish_locked(
            &mut record,
            &self.workspace_id,
            &self.server_instance_id,
            "movement.finished",
            Some(self.operation_id.clone()),
            &mut observation,
        );
    }
}

fn lock_unpoison<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn sse_json_event<T: Serialize>(name: &str, value: &T) -> SseEvent {
    let data = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_owned());
    SseEvent::default().event(name).data(data)
}

fn capture_output(output: &control_tower_application::ProcessOutput) -> CapturedOutput {
    let stdout_bytes = output.stdout.len() as u64;
    let stderr_bytes = output.stderr.len() as u64;
    let stdout_truncated = output.stdout.len() > MAX_OUTPUT_STREAM_BYTES;
    let stderr_truncated = output.stderr.len() > MAX_OUTPUT_STREAM_BYTES;
    CapturedOutput {
        stdout: output
            .stdout
            .iter()
            .copied()
            .take(MAX_OUTPUT_STREAM_BYTES)
            .collect(),
        stderr: output
            .stderr
            .iter()
            .copied()
            .take(MAX_OUTPUT_STREAM_BYTES)
            .collect(),
        stdout_bytes,
        stderr_bytes,
        stdout_truncated,
        stderr_truncated,
    }
}

fn insert_output(
    record: &mut WorkspaceRecord,
    observation: &mut MovementObservation,
    output_id: String,
    output: CapturedOutput,
) {
    let output_size = output.stdout.len() + output.stderr.len();
    while record.output_bytes + output_size > MAX_WORKSPACE_OUTPUT_BYTES {
        let Some(oldest) = record.output_order.pop_front() else {
            break;
        };
        if let Some(evicted) = record.outputs.remove(&oldest) {
            record.output_bytes = record
                .output_bytes
                .saturating_sub(evicted.stdout.len() + evicted.stderr.len());
            if let Some(role) = observation
                .role_results
                .iter_mut()
                .find(|role| role.output_id.as_deref() == Some(oldest.as_str()))
            {
                role.output_state = "evicted";
                observation.outputs_evicted += 1;
            }
        }
    }
    record.output_bytes += output_size;
    record.output_order.push_back(output_id.clone());
    record.outputs.insert(output_id, output);
}

fn trim_role_results(record: &mut WorkspaceRecord, observation: &mut MovementObservation) {
    while observation.role_results.len() > MAX_ROLE_RESULTS {
        let removed = observation.role_results.remove(0);
        observation.omitted_role_results += 1;
        if let Some(output_id) = removed.output_id {
            if let Some(output) = record.outputs.remove(&output_id) {
                record.output_bytes = record
                    .output_bytes
                    .saturating_sub(output.stdout.len() + output.stderr.len());
                record.output_order.retain(|id| id != &output_id);
                observation.outputs_evicted += 1;
            }
        }
    }
}

fn execute_movement(
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
    let workbench = match super::deps::workbench(&workspace.root) {
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

fn movement_choice_view(choice: MovementChoice) -> RecoveryChoiceView {
    RecoveryChoiceView {
        direction: choice.direction.as_str(),
        target_stage: choice.target_stage,
    }
}

fn checkpoint_for_state(state: &WorkbenchState, stages: &[Stage]) -> CheckpointView {
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

fn attempted_checkpoint(failure: &TransitionFailure, stages: &[Stage]) -> Option<CheckpointView> {
    match failure {
        TransitionFailure::StateCouldNotBeSaved { attempted, .. } => {
            Some(checkpoint_for_state(attempted, stages))
        }
        _ => None,
    }
}

fn failure_view(failure: &TransitionFailure, stages: &[Stage]) -> FailureView {
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
        server_instance_id: String::new(),
        observation_revision: 0,
        movement_busy: false,
        observation: None,
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
            observations: Arc::new(ObservationStore::new()),
        })
    }

    #[cfg(unix)]
    fn write_executable(path: &Path, source: &str) {
        use std::os::unix::fs::PermissionsExt;
        fs::write(path, source).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn api_request(method: &str, uri: &str, body: Option<String>) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header(HOST, "127.0.0.1:43001")
            .header(ORIGIN, "http://127.0.0.1:43001")
            .header(COOKIE, "ct_session=browser-session");
        let body = if let Some(body) = body {
            builder = builder.header(CONTENT_TYPE, "application/json");
            Body::from(body)
        } else {
            Body::empty()
        };
        builder.body(body).unwrap()
    }

    fn move_request(workspace_id: &str, direction: &str, target: u32) -> Request<Body> {
        api_request(
            "POST",
            &format!("/api/workspaces/{workspace_id}/movements"),
            Some(format!(
                "{{\"direction\":\"{direction}\",\"target_stage\":{target}}}"
            )),
        )
    }

    async fn response_json(response: Response<Body>) -> serde_json::Value {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).unwrap()
    }

    async fn wait_for_path(path: &Path) {
        tokio::time::timeout(Duration::from_secs(8), async {
            while !path.exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("the controlled child should reach its filesystem gate");
    }

    async fn next_sse_block(body: &mut Body) -> String {
        let mut bytes = Vec::new();
        let result = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let Some(Ok(frame)) = body.frame().await else {
                    break;
                };
                if let Ok(data) = frame.into_data() {
                    bytes.extend_from_slice(&data);
                    if bytes.windows(2).any(|window| window == b"\n\n")
                        || bytes.windows(4).any(|window| window == b"\r\n\r\n")
                    {
                        break;
                    }
                }
            }
        })
        .await;
        if result.is_err() {
            return String::new();
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    async fn wait_for_idle(app: &Router, workspace_id: &str) -> serde_json::Value {
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                let response = app
                    .clone()
                    .oneshot(api_request(
                        "GET",
                        &format!("/api/workspaces/{workspace_id}"),
                        None,
                    ))
                    .await
                    .unwrap();
                let view = response_json(response).await;
                if !view["movement_busy"].as_bool().unwrap_or(true) {
                    break view;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("movement bookkeeping should release workspace admission")
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

        for uri in [
            "/api/workspaces/fixture/events",
            "/api/workspaces/fixture/outputs/private-output/stdout",
        ] {
            let private_stream = router(context.clone())
                .oneshot(
                    Request::builder()
                        .uri(uri)
                        .header(HOST, "127.0.0.1:43001")
                        .header(ORIGIN, "http://127.0.0.1:43001")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(private_stream.status(), StatusCode::UNAUTHORIZED);
        }
        assert!(lock_unpoison(&context.observations.workspaces).is_empty());

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

    #[test]
    fn output_capture_keeps_a_bounded_lossless_prefix_and_original_lengths() {
        let stdout = (0..MAX_OUTPUT_STREAM_BYTES + 7)
            .map(|index| [0xff, 0, b'<'][index % 3])
            .collect::<Vec<_>>();
        let stderr = vec![0x1b; MAX_OUTPUT_STREAM_BYTES + 3];
        let captured = capture_output(&control_tower_application::ProcessOutput {
            success: true,
            exit_code: Some(0),
            stdout: stdout.clone(),
            stderr: stderr.clone(),
        });

        assert_eq!(captured.stdout.len(), MAX_OUTPUT_STREAM_BYTES);
        assert_eq!(captured.stderr.len(), MAX_OUTPUT_STREAM_BYTES);
        assert_eq!(captured.stdout, stdout[..MAX_OUTPUT_STREAM_BYTES]);
        assert_eq!(captured.stderr, stderr[..MAX_OUTPUT_STREAM_BYTES]);
        assert_eq!(captured.stdout_bytes, (MAX_OUTPUT_STREAM_BYTES + 7) as u64);
        assert_eq!(captured.stderr_bytes, (MAX_OUTPUT_STREAM_BYTES + 3) as u64);
        assert!(captured.stdout_truncated);
        assert!(captured.stderr_truncated);
    }

    #[tokio::test]
    async fn lagged_sse_subscriber_receives_a_resynchronization_snapshot() {
        let temp = TempDir::new();
        let project = temp.path().join("demo");
        fs::create_dir_all(&project).unwrap();
        let root = workspace(&project, "fixture", "010-seed", true);
        let server_context = context(discover_project(&project).unwrap());
        let events = router(server_context.clone())
            .oneshot(api_request("GET", "/api/workspaces/fixture/events", None))
            .await
            .unwrap();
        assert_eq!(events.headers()[CACHE_CONTROL], "no-store");
        let mut event_body = events.into_body();
        let initial_event = next_sse_block(&mut event_body).await;
        assert!(initial_event.contains("event: snapshot"));

        let runtime = server_context.observations.workspace_state("fixture");
        let server_instance_id = server_context.observations.server_instance_id.clone();
        runtime
            .begin_operation(
                "fixture",
                &server_instance_id,
                "operation-for-lag-test",
                Direction::Up,
                10,
            )
            .unwrap();
        let directory = root.join("stages/010-seed");
        let stage = Stage {
            number: 10,
            name: "seed".to_owned(),
            directory: directory.clone(),
            up: Some(directory.join("up")),
            down: None,
            verify_up: None,
            verify_down: None,
        };
        for _ in 0..=EVENT_BUFFER {
            runtime.record_role_started(
                "fixture",
                &server_instance_id,
                "operation-for-lag-test",
                &stage,
                ExecutableRole::Up,
            );
        }

        let resync = next_sse_block(&mut event_body).await;
        assert!(resync.contains("event: resync"), "lag response: {resync:?}");
        assert!(resync.contains("\"movement_busy\":true"));
        assert!(resync.contains("\"number\":10"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sse_keeps_role_output_available_while_next_role_runs_and_disconnect_does_not_release_admission()
     {
        let temp = TempDir::new();
        let project = temp.path().join("demo");
        fs::create_dir_all(&project).unwrap();
        let root = workspace(&project, "fixture", "010-seed", true);
        let second = root.join("stages/200-finish");
        fs::create_dir_all(&second).unwrap();
        write_executable(
            &root.join("stages/010-seed/up"),
            "#!/bin/sh\nprintf '10-up\\n' >> \"$CONTROL_TOWER_WORKSPACE/roles.log\"\nprintf '\\377\\000<script>safe</script>'\nprintf 'err\\001\\n' >&2\n",
        );
        write_executable(
            &second.join("up"),
            "#!/bin/sh\nprintf '200-up\\n' >> \"$CONTROL_TOWER_WORKSPACE/roles.log\"\ntouch \"$CONTROL_TOWER_WORKSPACE/second-started\"\nwhile [ ! -e \"$CONTROL_TOWER_WORKSPACE/release-second\" ]; do sleep 0.01; done\nprintf 'later output'\n",
        );
        let app = router(context(discover_project(&project).unwrap()));

        let events = app
            .clone()
            .oneshot(api_request("GET", "/api/workspaces/fixture/events", None))
            .await
            .unwrap();
        assert_eq!(events.status(), StatusCode::OK);
        let mut event_body = events.into_body();
        let initial_event = next_sse_block(&mut event_body).await;
        assert!(
            initial_event.contains("event: snapshot"),
            "initial SSE snapshot: {initial_event:?}"
        );

        let movement_app = app.clone();
        let post = tokio::spawn(async move {
            movement_app
                .oneshot(move_request("fixture", "up", 200))
                .await
                .unwrap()
        });
        wait_for_path(&root.join("second-started")).await;

        let mut saw_first_finish = false;
        let mut saw_second_start = false;
        let notifications_arrived = tokio::time::timeout(Duration::from_secs(8), async {
            while !saw_first_finish || !saw_second_start {
                let block = next_sse_block(&mut event_body).await;
                if block.is_empty() {
                    break;
                }
                if block.contains("event: role.finished") && block.contains("\"number\":10") {
                    saw_first_finish = true;
                }
                if block.contains("event: role.started") && block.contains("\"number\":200") {
                    saw_second_start = true;
                }
            }
        })
        .await
        .is_ok()
            && saw_first_finish
            && saw_second_start;
        if !notifications_arrived {
            fs::write(root.join("release-second"), "release after diagnostic").unwrap();
            let _ = post.await;
            panic!(
                "missing SSE role notifications: first_finished={saw_first_finish}, second_started={saw_second_start}"
            );
        }

        let view_response = app
            .clone()
            .oneshot(api_request("GET", "/api/workspaces/fixture", None))
            .await
            .unwrap();
        let view = response_json(view_response).await;
        assert_eq!(view["movement_busy"], true);
        assert_eq!(view["observation"]["active_role"]["stage"]["number"], 200);
        assert_eq!(view["observation"]["role_results"][0]["state"], "succeeded");
        assert_eq!(
            view["observation"]["role_results"][0]["output_state"],
            "available"
        );
        let output_id = view["observation"]["role_results"][0]["output_id"]
            .as_str()
            .unwrap();
        let stdout = app
            .clone()
            .oneshot(api_request(
                "GET",
                &format!("/api/workspaces/fixture/outputs/{output_id}/stdout"),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(stdout.status(), StatusCode::OK);
        assert_eq!(stdout.headers()["x-control-tower-truncated"], "false");
        assert_eq!(
            stdout
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .as_ref(),
            b"\xff\x00<script>safe</script>"
        );
        let stderr = app
            .clone()
            .oneshot(api_request(
                "GET",
                &format!("/api/workspaces/fixture/outputs/{output_id}/stderr"),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(
            stderr
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .as_ref(),
            b"err\x01\n"
        );

        let overlap = app
            .clone()
            .oneshot(move_request("fixture", "up", 200))
            .await
            .unwrap();
        assert_eq!(overlap.status(), StatusCode::CONFLICT);
        assert_eq!(
            response_json(overlap).await["error"]["code"],
            "workspace_busy"
        );

        post.abort();
        assert!(post.await.unwrap_err().is_cancelled());
        let after_disconnect = app
            .clone()
            .oneshot(move_request("fixture", "up", 200))
            .await
            .unwrap();
        assert_eq!(after_disconnect.status(), StatusCode::CONFLICT);

        fs::write(root.join("release-second"), "continue").unwrap();
        let final_view = wait_for_idle(&app, "fixture").await;
        assert_eq!(final_view["observation"]["state"], "complete");
        assert_eq!(final_view["checkpoint"]["accepted_stage"]["number"], 200);
        assert_eq!(
            fs::read_to_string(root.join("roles.log")).unwrap(),
            "10-up\n200-up\n"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn movement_outcomes_keep_confirmed_and_attempted_checkpoint_separate_on_final_save_failure()
     {
        let temp = TempDir::new();
        let project = temp.path().join("demo");
        fs::create_dir_all(&project).unwrap();
        let root = workspace(&project, "fixture", "010-seed", true);
        let second = root.join("stages/200-finish");
        fs::create_dir_all(&second).unwrap();
        write_executable(
            &root.join("stages/010-seed/up"),
            "#!/bin/sh\ntouch \"$CONTROL_TOWER_WORKSPACE/first-up\"\nprintf 'mutation output'\n",
        );
        write_executable(
            &root.join("stages/010-seed/verify-up"),
            "#!/bin/sh\ntouch \"$CONTROL_TOWER_WORKSPACE/first-verify\"\n",
        );
        write_executable(
            &second.join("up"),
            "#!/bin/sh\ntouch \"$CONTROL_TOWER_WORKSPACE/second-up\"\n",
        );
        let connection =
            rusqlite::Connection::open(root.join(".control_tower/state.sqlite3")).unwrap();
        connection
            .execute_batch("CREATE TRIGGER fail_final_acceptance BEFORE UPDATE ON workbench_state WHEN NEW.completed_stage_count = 1 BEGIN SELECT RAISE(ABORT, 'injected final checkpoint failure'); END;")
            .unwrap();
        drop(connection);
        let app = router(context(discover_project(&project).unwrap()));
        let response = app
            .oneshot(move_request("fixture", "up", 200))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let result = response_json(response).await["observation"].clone();
        assert_eq!(result["state"], "stopped");
        assert_eq!(result["failure"]["kind"], "checkpoint_save_failed");
        assert_eq!(
            result["confirmed_checkpoint"]["accepted_stage"],
            serde_json::Value::Null
        );
        assert_eq!(
            result["confirmed_checkpoint"]["pending_transition"]["direction"],
            "up"
        );
        assert_eq!(
            result["attempted_checkpoint"]["accepted_stage"]["number"],
            10
        );
        assert_eq!(result["role_results"][0]["state"], "succeeded");
        assert!(root.join("first-up").exists());
        assert!(root.join("first-verify").exists());
        assert!(
            !root.join("second-up").exists(),
            "later roles stop after a failed checkpoint write"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn pending_publication_failure_retains_mutation_output_and_never_runs_verifier() {
        let temp = TempDir::new();
        let project = temp.path().join("demo");
        fs::create_dir_all(&project).unwrap();
        let root = workspace(&project, "fixture", "010-seed", true);
        write_executable(
            &root.join("stages/010-seed/up"),
            "#!/bin/sh\ntouch \"$CONTROL_TOWER_WORKSPACE/mutation-ran\"\nprintf 'before pending save'\n",
        );
        write_executable(
            &root.join("stages/010-seed/verify-up"),
            "#!/bin/sh\ntouch \"$CONTROL_TOWER_WORKSPACE/verifier-ran\"\n",
        );
        let connection =
            rusqlite::Connection::open(root.join(".control_tower/state.sqlite3")).unwrap();
        connection
            .execute_batch("CREATE TRIGGER fail_pending_publication BEFORE UPDATE ON workbench_state WHEN NEW.pending_stage_index IS NOT NULL BEGIN SELECT RAISE(ABORT, 'injected pending checkpoint failure'); END;")
            .unwrap();
        drop(connection);
        let app = router(context(discover_project(&project).unwrap()));
        let response = app
            .oneshot(move_request("fixture", "up", 10))
            .await
            .unwrap();
        let result = response_json(response).await["observation"].clone();
        assert_eq!(result["state"], "stopped");
        assert_eq!(result["failure"]["kind"], "checkpoint_save_failed");
        assert_eq!(
            result["confirmed_checkpoint"]["pending_transition"],
            serde_json::Value::Null
        );
        assert_eq!(
            result["attempted_checkpoint"]["pending_transition"]["direction"],
            "up"
        );
        assert_eq!(result["role_results"][0]["output_state"], "available");
        assert!(root.join("mutation-ran").exists());
        assert!(!root.join("verifier-ran").exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn failed_verify_down_supports_check_only_retry_and_reapply_intents() {
        let temp = TempDir::new();
        let project = temp.path().join("demo");
        fs::create_dir_all(&project).unwrap();
        let root = workspace(&project, "fixture", "200-finish", true);
        let stage = root.join("stages/200-finish");
        write_executable(
            &stage.join("up"),
            "#!/bin/sh\nprintf 'up\\n' >> \"$CONTROL_TOWER_WORKSPACE/roles.log\"\n",
        );
        write_executable(
            &stage.join("down"),
            "#!/bin/sh\nprintf 'down\\n' >> \"$CONTROL_TOWER_WORKSPACE/roles.log\"\n",
        );
        write_executable(
            &stage.join("verify-down"),
            "#!/bin/sh\nprintf 'verify-down\\n' >> \"$CONTROL_TOWER_WORKSPACE/roles.log\"\nattempt=0\n[ ! -f \"$CONTROL_TOWER_WORKSPACE/verify-down-attempts\" ] || attempt=$(cat \"$CONTROL_TOWER_WORKSPACE/verify-down-attempts\")\nattempt=$((attempt + 1))\nprintf '%s' \"$attempt\" > \"$CONTROL_TOWER_WORKSPACE/verify-down-attempts\"\nif [ \"$attempt\" -eq 1 ] || [ \"$attempt\" -eq 3 ]; then exit 7; fi\n",
        );
        let app = router(context(discover_project(&project).unwrap()));
        let up = app
            .clone()
            .oneshot(move_request("fixture", "up", 200))
            .await
            .unwrap();
        assert_eq!(response_json(up).await["observation"]["state"], "complete");
        let down = app
            .clone()
            .oneshot(move_request("fixture", "down", 0))
            .await
            .unwrap();
        let stopped = response_json(down).await["observation"].clone();
        assert_eq!(stopped["state"], "stopped");
        assert_eq!(stopped["failure"]["role"], "verify-down");
        assert_eq!(
            stopped["verification_choices"]["retry"]["direction"],
            "down"
        );
        assert_eq!(stopped["verification_choices"]["retry"]["target_stage"], 0);
        assert_eq!(
            stopped["verification_choices"]["reverse"]["direction"],
            "up"
        );
        assert_eq!(
            stopped["verification_choices"]["reverse"]["target_stage"],
            200
        );

        let retry = app
            .clone()
            .oneshot(move_request("fixture", "down", 0))
            .await
            .unwrap();
        assert_eq!(
            response_json(retry).await["observation"]["state"],
            "complete"
        );
        assert_eq!(
            fs::read_to_string(root.join("roles.log")).unwrap(),
            "up\ndown\nverify-down\nverify-down\n",
            "verify-down retry must not replay the down mutation"
        );

        let up_again = app
            .clone()
            .oneshot(move_request("fixture", "up", 200))
            .await
            .unwrap();
        assert_eq!(
            response_json(up_again).await["observation"]["state"],
            "complete"
        );
        let down_again = app
            .clone()
            .oneshot(move_request("fixture", "down", 0))
            .await
            .unwrap();
        assert_eq!(
            response_json(down_again).await["observation"]["state"],
            "stopped"
        );

        let reapply = app
            .clone()
            .oneshot(move_request("fixture", "up", 200))
            .await
            .unwrap();
        assert_eq!(
            response_json(reapply).await["observation"]["state"],
            "complete"
        );
        assert_eq!(
            fs::read_to_string(root.join("roles.log")).unwrap(),
            "up\ndown\nverify-down\nverify-down\nup\ndown\nverify-down\nup\n"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn failed_verify_up_can_back_out_the_same_sparse_stage_without_trusting_selection() {
        let temp = TempDir::new();
        let project = temp.path().join("demo");
        fs::create_dir_all(&project).unwrap();
        let root = workspace(&project, "fixture", "200-finish", true);
        let stage = root.join("stages/200-finish");
        write_executable(
            &stage.join("up"),
            "#!/bin/sh\nprintf 'up\\n' >> \"$CONTROL_TOWER_WORKSPACE/roles.log\"\ntouch \"$CONTROL_TOWER_WORKSPACE/applied\"\n",
        );
        write_executable(
            &stage.join("verify-up"),
            "#!/bin/sh\nprintf 'verify-up\\n' >> \"$CONTROL_TOWER_WORKSPACE/roles.log\"\nif [ ! -e \"$CONTROL_TOWER_WORKSPACE/verify-up-failed\" ]; then touch \"$CONTROL_TOWER_WORKSPACE/verify-up-failed\"; exit 9; fi\n",
        );
        write_executable(
            &stage.join("down"),
            "#!/bin/sh\nprintf 'down\\n' >> \"$CONTROL_TOWER_WORKSPACE/roles.log\"\nrm -f \"$CONTROL_TOWER_WORKSPACE/applied\"\n",
        );
        let app = router(context(discover_project(&project).unwrap()));
        let up = app
            .clone()
            .oneshot(move_request("fixture", "up", 200))
            .await
            .unwrap();
        let stopped = response_json(up).await["observation"].clone();
        assert_eq!(stopped["state"], "stopped");
        assert_eq!(stopped["failure"]["role"], "verify-up");
        assert_eq!(stopped["verification_choices"]["retry"]["direction"], "up");
        assert_eq!(
            stopped["verification_choices"]["retry"]["target_stage"],
            200
        );
        assert_eq!(
            stopped["verification_choices"]["reverse"]["direction"],
            "down"
        );
        assert_eq!(
            stopped["verification_choices"]["reverse"]["target_stage"],
            0
        );

        let backout = app
            .clone()
            .oneshot(move_request("fixture", "down", 0))
            .await
            .unwrap();
        let completed = response_json(backout).await["observation"].clone();
        assert_eq!(completed["state"], "complete");
        assert_eq!(
            completed["confirmed_checkpoint"]["accepted_stage"],
            serde_json::Value::Null
        );
        assert!(!root.join("applied").exists());
        assert_eq!(
            fs::read_to_string(root.join("roles.log")).unwrap(),
            "up\nverify-up\ndown\n"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unauthorized_and_cross_origin_movements_cannot_run_authored_roles() {
        let temp = TempDir::new();
        let project = temp.path().join("demo");
        fs::create_dir_all(&project).unwrap();
        let root = workspace(&project, "fixture", "010-seed", true);
        write_executable(
            &root.join("stages/010-seed/up"),
            "#!/bin/sh\ntouch \"$CONTROL_TOWER_WORKSPACE/should-not-run\"\n",
        );
        let app = router(context(discover_project(&project).unwrap()));
        let unauthorized = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/fixture/movements")
                    .header(HOST, "127.0.0.1:43001")
                    .header(ORIGIN, "http://127.0.0.1:43001")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from("{\"direction\":\"up\",\"target_stage\":10}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
        let cross_origin = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/fixture/movements")
                    .header(HOST, "127.0.0.1:43001")
                    .header(ORIGIN, "https://attacker.example")
                    .header(COOKIE, "ct_session=browser-session")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from("{\"direction\":\"up\",\"target_stage\":10}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cross_origin.status(), StatusCode::FORBIDDEN);
        assert!(!root.join("should-not-run").exists());
    }
}
