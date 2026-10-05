use super::{
    ServerContext,
    dto::*,
    movement::{MovementTask, execute_movement},
    runtime::sse_json_event,
    workspace::{find_workflow, relative_path, stage_identity, workspace_snapshot},
};
use axum::{
    Router,
    body::Body,
    extract::{Path as RoutePath, State},
    http::{
        HeaderName, HeaderValue, Request, Response, StatusCode,
        header::{CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS},
    },
    response::{
        IntoResponse, Json,
        sse::{KeepAlive, Sse},
    },
    routing::{get, post},
};
use control_tower_application::StageDefinitionsInput;
use futures_util::stream;
use rust_embed::RustEmbed;
use serde::Serialize;
use std::{path::Path, sync::Arc, time::Duration};

#[derive(RustEmbed)]
#[folder = "../../ui/dist/"]
pub(super) struct FrontendAssets;

pub(super) fn router(context: Arc<ServerContext>) -> Router {
    Router::new()
        .route("/api/workspace", get(workspace_view))
        .route("/api/workflows/{id}/movements", post(start_movement))
        .route("/api/workflows/{id}/events", get(workflow_events))
        .route(
            "/api/workflows/{id}/stages/{stage_number}",
            get(stage_definition),
        )
        .route("/", get(index))
        .route("/assets/{*path}", get(asset))
        .fallback(not_found)
        .with_state(context)
}

pub(super) async fn workspace_view(State(context): State<Arc<ServerContext>>) -> Response<Body> {
    api_json(workspace_snapshot(&context.workspace))
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
        "up" => control_tower_application::Direction::Up,
        "down" => control_tower_application::Direction::Down,
        _ => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "invalid_direction",
                "Movement direction must be up or down.",
            );
        }
    };
    let Some(workflow) = find_workflow(&context.workspace, &id) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "unknown_workflow",
            "That Workflow was not found under workflows/ at startup.",
        );
    };
    let runtime = context
        .observations
        .workflow_state(&context.workspace.name, &workflow);
    let Some(permit) = runtime.try_admit() else {
        return api_error(
            StatusCode::CONFLICT,
            "workflow_busy",
            "A movement is already executing for this Workflow.",
        );
    };
    let task = MovementTask {
        workspace_name: context.workspace.name.clone(),
        env_defaults: context.workspace.env_defaults.clone(),
        workflow,
        direction,
        target_stage: request.target_stage,
        expected: request.expected_checkpoint.into(),
        runtime,
        permit,
    };
    match tokio::task::spawn_blocking(move || execute_movement(task)).await {
        Ok(result) => match result.error {
            Some((status, code, message)) => api_error(status, code, message),
            None => add_browser_headers(StatusCode::NO_CONTENT.into_response()),
        },
        Err(error) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "worker_failed",
            error.to_string(),
        ),
    }
}

pub(super) async fn workflow_events(
    State(context): State<Arc<ServerContext>>,
    RoutePath(id): RoutePath<String>,
) -> Response<Body> {
    let Some(workflow) = find_workflow(&context.workspace, &id) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "unknown_workflow",
            "That Workflow was not found under workflows/ at startup.",
        );
    };
    let runtime = context
        .observations
        .workflow_state(&context.workspace.name, &workflow);
    let mut snapshots = runtime.subscribe();
    let workspace_name = context.workspace.name.clone();
    let env_defaults = context.workspace.env_defaults.clone();
    let runtime_for_snapshot = runtime.clone();
    let workflow_for_snapshot = workflow.clone();
    match tokio::task::spawn_blocking(move || {
        let workbench = crate::deps::workbench(&workflow_for_snapshot.root, env_defaults);
        runtime_for_snapshot.current_snapshot(
            &workspace_name,
            &workflow_for_snapshot,
            workbench.as_ref().map_err(ToString::to_string),
        )
    })
    .await
    {
        Ok(_) => {}
        Err(error) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "worker_failed",
                error.to_string(),
            );
        }
    }
    let initial = snapshots.borrow_and_update().clone();
    let stream = stream::unfold(
        (snapshots, Some(initial)),
        |(mut snapshots, initial)| async move {
            if let Some(snapshot) = initial {
                return Some((
                    Ok::<_, std::convert::Infallible>(sse_json_event("snapshot", &snapshot)),
                    (snapshots, None),
                ));
            }
            if snapshots.changed().await.is_err() {
                return None;
            }
            let snapshot = snapshots.borrow_and_update().clone();
            Some((Ok(sse_json_event("snapshot", &snapshot)), (snapshots, None)))
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
    let Some(workflow) = find_workflow(&context.workspace, &id) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "unknown_workflow",
            "That Workflow was not found under workflows/ at startup.",
        );
    };
    let root = workflow.root.clone();
    let env_defaults = context.workspace.env_defaults.clone();
    let result = tokio::task::spawn_blocking(move || {
        let workbench = crate::deps::workbench(&workflow.root, env_defaults)
            .map_err(|error| error.to_string())?;
        workbench
            .stage_definitions(StageDefinitionsInput {
                workflow_root: &workflow.root,
                stage_number,
            })
            .map_err(|error| error.to_string())
    })
    .await;
    match result {
        Ok(Ok(result)) => {
            let definitions = result
                .definitions
                .into_iter()
                .map(|definition| match definition.contents {
                    Ok(contents) => DefinitionView {
                        role: definition.role.as_str(),
                        path: relative_path(&root, &definition.path),
                        contents: Some(String::from_utf8_lossy(&contents).into_owned()),
                        issue: None,
                    },
                    Err(error) => DefinitionView {
                        role: definition.role.as_str(),
                        path: relative_path(&root, &definition.path),
                        contents: None,
                        issue: Some(error.to_string()),
                    },
                })
                .collect();
            api_json(StageDefinitionView {
                stage: stage_identity(&result.stage),
                definitions,
            })
        }
        Ok(Err(error)) => api_error(StatusCode::SERVICE_UNAVAILABLE, "stage_unavailable", error),
        Err(error) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "worker_failed",
            error.to_string(),
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
    response.headers_mut().insert(CONTENT_SECURITY_POLICY, HeaderValue::from_static(
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
    ));
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
