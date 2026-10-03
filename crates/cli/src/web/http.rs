use super::{
    ServerContext,
    dto::*,
    movement::execute_movement,
    runtime::{OperationGuard, WorkspaceRuntime, lock_unpoison, sse_json_event},
    workspace::{
        definition_view, find_workspace, project_snapshot, read_status, workspace_snapshot,
    },
};
use axum::{
    Router,
    body::Body,
    extract::{Path as RoutePath, State},
    http::{
        HeaderName, HeaderValue, Request, Response, StatusCode,
        header::{
            AUTHORIZATION, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_SECURITY_POLICY,
            CONTENT_TYPE, COOKIE, HOST, ORIGIN, SET_COOKIE, X_CONTENT_TYPE_OPTIONS,
        },
    },
    middleware::{self, Next},
    response::{
        IntoResponse, Json,
        sse::{KeepAlive, Sse},
    },
    routing::{get, post},
};
use control_tower_application::Direction;
use futures_util::stream;
use rust_embed::RustEmbed;
use serde::Serialize;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(RustEmbed)]
#[folder = "../../ui/dist/"]
pub(super) struct FrontendAssets;

pub(super) fn router(context: Arc<ServerContext>) -> Router {
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

pub(super) async fn security(
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

pub(super) async fn create_session(
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

pub(super) async fn project_view(State(context): State<Arc<ServerContext>>) -> Response<Body> {
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

pub(super) async fn workspace_view(
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
    let runtime = context.observations.workspace_state(&id);
    let server_instance_id = context.observations.server_instance_id.clone();
    match tokio::task::spawn_blocking(move || workspace_snapshot(&project_name, &workspace)).await {
        Ok(Ok(mut view)) => {
            view.storage_issue = None;
            runtime.remember_view(view.clone());
            let snapshot = runtime.snapshot(&id, &server_instance_id);
            apply_runtime_snapshot(&mut view, snapshot);
            runtime.remember_view(view.clone());
            api_json(view)
        }
        Ok(Err(message)) => cached_workspace_response(
            runtime,
            &id,
            &server_instance_id,
            message,
            StatusCode::SERVICE_UNAVAILABLE,
            "workspace_unavailable",
        ),
        Err(error) => cached_workspace_response(
            runtime,
            &id,
            &server_instance_id,
            format!("Could not read Workspace status: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
            "workspace_read_failed",
        ),
    }
}

pub(super) fn cached_workspace_response(
    runtime: Arc<WorkspaceRuntime>,
    workspace_id: &str,
    server_instance_id: &str,
    issue: String,
    status: StatusCode,
    code: &'static str,
) -> Response<Body> {
    let Some(mut view) = runtime.last_known_view() else {
        return api_error(status, code, issue);
    };
    view.storage_issue = Some(issue);
    let snapshot = runtime.snapshot(workspace_id, server_instance_id);
    apply_runtime_snapshot(&mut view, snapshot);
    api_json(view)
}

pub(super) fn apply_runtime_snapshot(view: &mut WorkspaceView, snapshot: RuntimeSnapshot) {
    view.server_instance_id = snapshot.server_instance_id;
    view.observation_revision = snapshot.revision;
    view.movement_busy = snapshot.movement_busy;
    let outcome_checkpoint = snapshot
        .observation
        .as_ref()
        .and_then(|observation| observation.confirmed_checkpoint.clone());
    view.observation = snapshot.observation;
    if let Some(checkpoint) = outcome_checkpoint.or(snapshot.checkpoint) {
        apply_checkpoint(view, checkpoint);
    }
}

pub(super) fn apply_checkpoint(view: &mut WorkspaceView, checkpoint: CheckpointView) {
    let accepted_index = checkpoint.accepted_stage.as_ref().and_then(|accepted| {
        view.stages
            .iter()
            .position(|stage| stage.number == accepted.number)
    });
    let pending_number = checkpoint
        .pending_transition
        .as_ref()
        .map(|pending| pending.stage.number);
    view.stages = view
        .stages
        .iter()
        .enumerate()
        .map(|(index, stage)| StageView {
            state: if pending_number == Some(stage.number) {
                "pending"
            } else if accepted_index.is_some_and(|accepted| index <= accepted) {
                "accepted"
            } else {
                "future"
            },
            is_accepted_checkpoint: accepted_index == Some(index),
            ..stage.clone()
        })
        .collect();
    view.checkpoint = checkpoint;
}

pub(super) async fn start_movement(
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

pub(super) async fn workspace_events(
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
    let shutdown = context.shutdown.subscribe();
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
            shutdown,
        ),
        |(mut receiver, initial, runtime, workspace_id, server_instance_id, mut shutdown)| async move {
            if *shutdown.borrow() {
                return None;
            }
            if let Some(snapshot) = initial {
                let event = sse_json_event("snapshot", &snapshot);
                return Some((
                    Ok::<_, std::convert::Infallible>(event),
                    (
                        receiver,
                        None,
                        runtime,
                        workspace_id,
                        server_instance_id,
                        shutdown,
                    ),
                ));
            }
            loop {
                let received = tokio::select! {
                    result = receiver.recv() => Some(result),
                    changed = shutdown.changed() => {
                        if changed.is_err() || *shutdown.borrow() {
                            return None;
                        }
                        continue;
                    }
                };
                match received.expect("the event or shutdown branch completed") {
                    Ok(event) if event.workspace_id == workspace_id => {
                        let kind = event.kind;
                        let next = sse_json_event(kind, &event);
                        return Some((
                            Ok(next),
                            (
                                receiver,
                                None,
                                runtime,
                                workspace_id,
                                server_instance_id,
                                shutdown,
                            ),
                        ));
                    }
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let snapshot = runtime.snapshot(&workspace_id, &server_instance_id);
                        let event = sse_json_event("resync", &snapshot);
                        return Some((
                            Ok(event),
                            (
                                receiver,
                                None,
                                runtime,
                                workspace_id,
                                server_instance_id,
                                shutdown,
                            ),
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

pub(super) async fn read_output(
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

pub(super) async fn stage_definition(
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

pub(super) async fn index() -> Response<Body> {
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

pub(super) async fn asset(RoutePath(path): RoutePath<String>) -> Response<Body> {
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

pub(super) async fn not_found(request: Request<Body>) -> Response<Body> {
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

pub(super) fn api_json<T: Serialize>(value: T) -> Response<Body> {
    let mut response = Json(value).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    add_browser_headers(response)
}

pub(super) fn add_browser_headers(mut response: Response<Body>) -> Response<Body> {
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

pub(super) fn api_error(
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
) -> Response<Body> {
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

pub(super) fn movement_error(
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

pub(super) fn session_cookie_matches(cookie: Option<&HeaderValue>, expected: &str) -> bool {
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

pub(super) fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        difference |= usize::from(
            left.get(index).copied().unwrap_or(0) ^ right.get(index).copied().unwrap_or(0),
        );
    }
    difference == 0
}
