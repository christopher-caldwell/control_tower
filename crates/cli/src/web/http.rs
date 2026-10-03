use super::{
    ServerContext,
    dto::*,
    movement::{MovementTask, execute_movement},
    runtime::sse_json_event,
    workspace::{
        MAX_DEFINITION_BYTES, find_workspace, project_snapshot, relative_path, stage_identity,
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
            "/api/workspaces/{id}/outputs/{operation_id}/{result_index}/{stream_name}",
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
    match tokio::task::spawn_blocking(move || project_snapshot(&context.project)).await {
        Ok(view) => api_json(view),
        Err(error) => fatal_worker_error(error),
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
    let runtime = context
        .observations
        .workspace_state(&context.project.name, &workspace);
    let project_name = context.project.name.clone();
    match tokio::task::spawn_blocking(move || runtime.current_snapshot(&project_name, &workspace))
        .await
    {
        Ok(view) => api_json(view),
        Err(error) => fatal_worker_error(error),
    }
}
pub(super) async fn start_movement(
    State(context): State<Arc<ServerContext>>,
    RoutePath(id): RoutePath<String>,
    request: Result<Json<MovementRequest>, axum::extract::rejection::JsonRejection>,
) -> Response<Body> {
    let request = match request {
        Ok(Json(request)) => request,
        Err(error) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "malformed_request",
                format!("Movement request is not valid JSON: {error}"),
            );
        }
    };
    let direction = match request.direction.as_str() {
        "up" => Direction::Up,
        "down" => Direction::Down,
        _ => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "invalid_direction",
                "Movement direction must be up or down.",
            );
        }
    };
    if direction == Direction::Up && request.target_stage == 0 {
        return api_error(
            StatusCode::BAD_REQUEST,
            "invalid_target",
            "An upward movement must name a numbered Stage.",
        );
    }
    let Some(workspace) = find_workspace(&context.project, &id) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "unknown_workspace",
            "That Workspace was not discovered when Control Tower started.",
        );
    };
    let runtime = context
        .observations
        .workspace_state(&context.project.name, &workspace);
    let Some(permit) = runtime.try_admit() else {
        return api_error(
            StatusCode::CONFLICT,
            "workspace_busy",
            "A movement is already executing for this Workspace.",
        );
    };
    let project_name = context.project.name.clone();
    let expected = request.expected_checkpoint.into();
    let target_stage = request.target_stage;
    let operation_id = Uuid::new_v4().to_string();
    let task = MovementTask {
        project_name,
        workspace,
        direction,
        target_stage,
        expected,
        runtime,
        operation_id,
        permit,
    };
    let worker = tokio::task::spawn_blocking(move || execute_movement(task));
    match worker.await {
        Ok(result) => match result.error {
            Some((status, code, message)) => api_error(status, code, message),
            None => add_browser_headers(StatusCode::NO_CONTENT.into_response()),
        },
        Err(error) => fatal_worker_error(error),
    }
}
pub(super) async fn workspace_events(
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
    let runtime = context
        .observations
        .workspace_state(&context.project.name, &workspace);
    let mut snapshots = runtime.subscribe();
    let project_name = context.project.name.clone();
    let runtime_for_snapshot = runtime.clone();
    let workspace_for_snapshot = workspace.clone();
    match tokio::task::spawn_blocking(move || {
        runtime_for_snapshot.current_snapshot(&project_name, &workspace_for_snapshot)
    })
    .await
    {
        Ok(_) => {}
        Err(error) => return fatal_worker_error(error),
    }
    let initial = snapshots.borrow_and_update().clone();
    let shutdown = context.shutdown.subscribe();
    let stream = stream::unfold(
        (snapshots, Some(initial), shutdown),
        |(mut snapshots, initial, mut shutdown)| async move {
            if *shutdown.borrow() {
                return None;
            }
            if let Some(snapshot) = initial {
                return Some((
                    Ok::<_, std::convert::Infallible>(sse_json_event("snapshot", &snapshot)),
                    (snapshots, None, shutdown),
                ));
            }
            loop {
                tokio::select! {
                    changed = snapshots.changed() => {
                        if changed.is_err() { return None; }
                        let snapshot = snapshots.borrow_and_update().clone();
                        return Some((Ok(sse_json_event("snapshot", &snapshot)), (snapshots, None, shutdown)));
                    }
                    changed = shutdown.changed() => {
                        if changed.is_err() || *shutdown.borrow() { return None; }
                    }
                }
            }
        },
    );
    let mut response = Sse::new(stream)
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
    RoutePath((id, operation_id, result_index, stream_name)): RoutePath<(
        String,
        String,
        String,
        String,
    )>,
) -> Response<Body> {
    let Some(workspace) = find_workspace(&context.project, &id) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "unknown_workspace",
            "That Workspace was not discovered when Control Tower started.",
        );
    };
    let stream_name = match stream_name.as_str() {
        "stdout" | "stderr" => stream_name,
        _ => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "invalid_output_stream",
                "Output stream must be stdout or stderr.",
            );
        }
    };
    let Ok(result_index) = result_index.parse::<usize>() else {
        return api_error(
            StatusCode::BAD_REQUEST,
            "invalid_output_index",
            "Output result index must be a nonnegative integer.",
        );
    };
    let runtime = context
        .observations
        .workspace_state(&context.project.name, &workspace);
    let Some(bytes) = runtime.read_output(&operation_id, result_index, &stream_name) else {
        return api_error(
            StatusCode::GONE,
            "output_unavailable",
            "This output is no longer retained by the current movement.",
        );
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
    let root = workspace.root.clone();
    match tokio::task::spawn_blocking(move || {
        workspace
            .workbench
            .stage_definitions(&workspace.root, stage_number, MAX_DEFINITION_BYTES)
    })
    .await
    {
        Ok(Ok(result)) => {
            let definitions = result
                .definitions
                .into_iter()
                .map(|definition| match definition.contents {
                    Ok(contents) => DefinitionView {
                        role: definition.role.as_str(),
                        path: relative_path(&root, &definition.path),
                        contents: Some(String::from_utf8_lossy(&contents.bytes).into_owned()),
                        truncated: contents.truncated,
                        issue: None,
                    },
                    Err(error) => DefinitionView {
                        role: definition.role.as_str(),
                        path: relative_path(&root, &definition.path),
                        contents: None,
                        truncated: false,
                        issue: Some(error.to_string()),
                    },
                })
                .collect();
            api_json(StageDefinitionView {
                stage: stage_identity(&result.stage),
                definitions,
            })
        }
        Ok(Err(error)) => {
            let status = if matches!(
                error,
                control_tower_application::StageDefinitionsError::UnknownStage(_)
            ) {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };
            api_error(status, "stage_unavailable", error.to_string())
        }
        Err(error) => fatal_worker_error(error),
    }
}
fn fatal_worker_error(error: tokio::task::JoinError) -> Response<Body> {
    eprintln!(
        "Control Tower worker stopped unexpectedly ({error}); exiting because execution state may be incomplete."
    );
    std::process::exit(1)
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
