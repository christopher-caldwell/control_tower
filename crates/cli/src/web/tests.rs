use super::*;
use super::{http::*, runtime::*, workspace::*};
use axum::{
    Router,
    body::Body,
    http::{Request, Response, StatusCode},
};
use control_tower_database::operations;
use http_body_util::BodyExt;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tower::ServiceExt;

static NEXT: AtomicU64 = AtomicU64::new(1);
struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ct-web-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
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

fn prepared_workflow(workspace: &Path, id: &str, script: &str) -> PathBuf {
    fs::create_dir_all(workspace.join("workflows")).unwrap();
    fs::write(
        workspace.join("control-tower.toml"),
        "[workspace]\nlabel = \"Demo Workspace\"\n",
    )
    .unwrap();
    let root = workspace.join("workflows").join(id);
    let stage = root.join("stages/010-seed");
    fs::create_dir_all(&stage).unwrap();
    fs::write(stage.join("up"), script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(stage.join("up"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let db = root.join(".control_tower/state.sqlite3");
    operations::bootstrap(&db).unwrap();
    operations::migrate(&db).unwrap();
    root
}

fn configured_workspace(workspace: &Path) -> WorkspaceContext {
    fs::create_dir_all(workspace.join("workflows")).unwrap();
    fs::write(
        workspace.join("control-tower.toml"),
        "[workspace]\nlabel = \"Demo Workspace\"\n",
    )
    .unwrap();
    discover_workspace(workspace).unwrap()
}

#[test]
fn ui_workspace_discovery_keeps_the_workspace_contract_without_parsing_dotenv() {
    let temp = TempDir::new();
    let workspace = temp.path().join("demo");
    fs::create_dir_all(&workspace).unwrap();
    fs::write(
        workspace.join("control-tower.toml"),
        "[workspace]\nlabel = \"Configured Workspace\"\n",
    )
    .unwrap();

    assert!(
        discover_workspace(&workspace).is_err(),
        "workflows/ is required"
    );

    fs::create_dir(workspace.join("workflows")).unwrap();
    fs::write(workspace.join(".env"), "INVALID LINE\n").unwrap();
    let context = discover_workspace(&workspace).unwrap();
    assert_eq!(context.name, "Configured Workspace");
    assert!(context.workflows.is_empty());
}

fn context(workspace: WorkspaceContext) -> Arc<ServerContext> {
    Arc::new(ServerContext {
        workspace,
        observations: Arc::new(ObservationStore::new()),
    })
}

fn request(method: &str, uri: &str, body: Option<String>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    builder
        .body(body.map_or_else(Body::empty, Body::from))
        .unwrap()
}

async fn json(response: Response<Body>) -> (StatusCode, serde_json::Value) {
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

async fn sse_snapshot(response: Response<Body>) -> serde_json::Value {
    assert_eq!(response.status(), StatusCode::OK);
    let frame = tokio::time::timeout(Duration::from_secs(2), response.into_body().frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .into_data()
        .unwrap();
    let event = String::from_utf8(frame.to_vec()).unwrap();
    let data = event
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .unwrap();
    serde_json::from_str(data).unwrap()
}

async fn view(app: &Router, id: &str) -> serde_json::Value {
    sse_snapshot(
        app.clone()
            .oneshot(request("GET", &format!("/api/workflows/{id}/events"), None))
            .await
            .unwrap(),
    )
    .await
}

async fn move_to(
    app: &Router,
    id: &str,
    direction: &str,
    target: u32,
    checkpoint: &serde_json::Value,
) -> Response<Body> {
    app.clone()
        .oneshot(request(
            "POST",
            &format!("/api/workflows/{id}/movements"),
            Some(
                serde_json::json!({
                    "direction": direction,
                    "target_stage": target,
                    "expected_checkpoint": checkpoint,
                })
                .to_string(),
            ),
        ))
        .await
        .unwrap()
}

#[tokio::test]
async fn inventory_lists_workflow_directories_without_preparing_or_validating_siblings() {
    let temp = TempDir::new();
    let workspace = temp.path().join("demo");
    fs::create_dir_all(workspace.join("workflows/unprepared")).unwrap();
    let healthy = prepared_workflow(
        &workspace,
        "healthy",
        "#!/bin/sh\necho unexpected > \"$CONTROL_TOWER_WORKFLOW/ran\"\n",
    );
    let app = router(context(configured_workspace(&workspace)));

    let (status, inventory) = json(
        app.clone()
            .oneshot(request("GET", "/api/workspace", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(inventory["name"], "Demo Workspace");
    let ids = inventory["workflows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|workflow| workflow["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["healthy", "unprepared"]);
    assert!(!healthy.join("ran").exists());
    assert!(
        !workspace
            .join("workflows/unprepared/.control_tower/state.sqlite3")
            .exists()
    );

    let unprepared = view(&app, "unprepared").await;
    assert_eq!(unprepared["current_status"], "unavailable");
    assert!(
        unprepared["status_issue"]
            .as_str()
            .is_some_and(|issue| !issue.is_empty())
    );
    let usable = view(&app, "healthy").await;
    assert_eq!(usable["current_status"], "available");
}

#[tokio::test]
async fn ui_reloads_workspace_dotenv_before_each_movement_and_rejects_invalid_files() {
    let temp = TempDir::new();
    let workspace = temp.path().join("demo");
    let key = format!("CT_UI_VALUE_{}", std::process::id());
    fs::create_dir_all(&workspace).unwrap();
    fs::write(workspace.join(".env"), format!("{key}=startup\n")).unwrap();
    let observed = workspace.join("observed-values");
    let first_script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"${{{key}-unset}}\" >> '{}'\n",
        observed.display()
    );
    let root = prepared_workflow(&workspace, "fixture", &first_script);
    let second = root.join("stages/020-second");
    fs::create_dir_all(&second).unwrap();
    let second_script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"${{{key}-unset}}\" >> '{}'\n",
        observed.display()
    );
    fs::write(second.join("up"), &second_script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(second.join("up"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let app = router(context(configured_workspace(&workspace)));

    fs::write(workspace.join(".env"), format!("{key}=changed\n")).unwrap();
    let before_first = view(&app, "fixture").await;
    let response = move_to(
        &app,
        "fixture",
        "up",
        10,
        &before_first["checkpoint"]["state"],
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(fs::read_to_string(&observed).unwrap().trim(), "changed");

    fs::remove_file(workspace.join(".env")).unwrap();
    let before_second = view(&app, "fixture").await;
    let response = move_to(
        &app,
        "fixture",
        "up",
        20,
        &before_second["checkpoint"]["state"],
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        fs::read_to_string(&observed).unwrap().lines().last(),
        Some("unset")
    );

    let before_invalid = view(&app, "fixture").await;
    fs::write(
        workspace.join(".env"),
        format!("{key}=\"synthetic-secret\n"),
    )
    .unwrap();
    let response = move_to(
        &app,
        "fixture",
        "down",
        0,
        &before_invalid["checkpoint"]["state"],
    )
    .await;
    let (status, error) = json(response).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!error.to_string().contains("synthetic-secret"));
    assert_eq!(
        view(&app, "fixture").await["checkpoint"]["state"],
        before_invalid["checkpoint"]["state"]
    );

    fs::remove_file(workspace.join(".env")).unwrap();
    fs::write(workspace.join(".env"), format!("{key}=hidden\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(workspace.join(".env"), fs::Permissions::from_mode(0o000)).unwrap();
        let response = move_to(
            &app,
            "fixture",
            "down",
            0,
            &before_invalid["checkpoint"]["state"],
        )
        .await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        fs::set_permissions(workspace.join(".env"), fs::Permissions::from_mode(0o600)).unwrap();
    }
    fs::remove_file(workspace.join(".env")).unwrap();
    fs::create_dir(workspace.join(".env")).unwrap();
    let response = move_to(
        &app,
        "fixture",
        "down",
        0,
        &before_invalid["checkpoint"]["state"],
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        view(&app, "fixture").await["checkpoint"]["state"],
        before_invalid["checkpoint"]["state"]
    );
}

#[tokio::test]
async fn selected_workflow_can_move_and_reconnect_with_completed_text_output_and_full_definitions()
{
    let temp = TempDir::new();
    let workspace = temp.path().join("demo");
    fs::create_dir_all(&workspace).unwrap();
    let root = prepared_workflow(
        &workspace,
        "fixture",
        "#!/bin/sh\nprintf 'mutation stdout\\n'\nprintf 'mutation stderr\\n' >&2\n",
    );
    let stage = root.join("stages/010-seed");
    fs::write(stage.join("down"), "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(stage.join("down"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let long_definition = "stage definition contents\n".repeat(8192);
    fs::write(stage.join("verify-down"), &long_definition).unwrap();
    let app = router(context(configured_workspace(&workspace)));
    let initial = view(&app, "fixture").await;
    assert_eq!(initial["current_status"], "available");

    assert_eq!(
        move_to(&app, "fixture", "up", 10, &initial["checkpoint"]["state"])
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    let latest = view(&app, "fixture").await;
    assert_eq!(latest["observation"]["state"], "complete");
    assert_eq!(
        latest["observation"]["role_results"][0]["stdout"],
        "mutation stdout\n"
    );
    assert_eq!(
        latest["observation"]["role_results"][0]["stderr"],
        "mutation stderr\n"
    );

    let (status, definition) = json(
        app.clone()
            .oneshot(request("GET", "/api/workflows/fixture/stages/10", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        definition["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["role"] == "verify-down")
            .unwrap()["contents"],
        long_definition
    );
    assert_eq!(
        app.oneshot(request(
            "GET",
            "/api/workflows/fixture/outputs/anything/0/stdout",
            None
        ))
        .await
        .unwrap()
        .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn stale_checkpoint_is_rejected_before_another_role_or_checkpoint_write() {
    let temp = TempDir::new();
    let workspace = temp.path().join("demo");
    fs::create_dir_all(&workspace).unwrap();
    let root = prepared_workflow(
        &workspace,
        "fixture",
        "#!/bin/sh\necho run >> \"$CONTROL_TOWER_WORKFLOW/runs\"\n",
    );
    let app = router(context(configured_workspace(&workspace)));
    let initial = view(&app, "fixture").await;
    assert_eq!(
        move_to(&app, "fixture", "up", 10, &initial["checkpoint"]["state"])
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        fs::read_to_string(root.join("runs"))
            .unwrap()
            .lines()
            .count(),
        1
    );

    let (status, error) =
        json(move_to(&app, "fixture", "down", 0, &initial["checkpoint"]["state"]).await).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "stale_checkpoint");
    assert_eq!(
        fs::read_to_string(root.join("runs"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert_eq!(
        view(&app, "fixture").await["checkpoint"]["accepted_stage"]["number"],
        10
    );
}

#[tokio::test]
async fn a_dropped_request_keeps_workflow_admission_until_the_worker_finishes() {
    let temp = TempDir::new();
    let workspace = temp.path().join("demo");
    fs::create_dir_all(&workspace).unwrap();
    let root = prepared_workflow(
        &workspace,
        "fixture",
        "#!/bin/sh\ntouch \"$CONTROL_TOWER_WORKFLOW/started\"\nwhile [ ! -e \"$CONTROL_TOWER_WORKFLOW/release\" ]; do sleep 0.02; done\necho complete\n",
    );
    let app = router(context(configured_workspace(&workspace)));
    let initial = view(&app, "fixture").await;
    let expected = initial["checkpoint"]["state"].clone();
    let body =
        serde_json::json!({"direction":"up", "target_stage":10, "expected_checkpoint":expected})
            .to_string();
    let first_app = app.clone();
    let first_body = body.clone();
    let first = tokio::spawn(async move {
        first_app
            .oneshot(request(
                "POST",
                "/api/workflows/fixture/movements",
                Some(first_body),
            ))
            .await
            .unwrap()
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !root.join("started").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    first.abort();
    let _ = first.await;

    let (status, error) = json(
        app.clone()
            .oneshot(request(
                "POST",
                "/api/workflows/fixture/movements",
                Some(body.clone()),
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "workflow_busy");
    fs::write(root.join("release"), "").unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if view(&app, "fixture").await["observation"]["state"] == "complete" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(view(&app, "fixture").await["movement_busy"], false);
}

#[tokio::test]
async fn sse_shows_mutation_output_while_the_later_verifier_is_running() {
    let temp = TempDir::new();
    let workspace = temp.path().join("demo");
    fs::create_dir_all(&workspace).unwrap();
    let root = prepared_workflow(
        &workspace,
        "fixture",
        "#!/bin/sh\nprintf 'mutation complete\\n'\n",
    );
    fs::write(root.join("stages/010-seed/verify-up"), "#!/bin/sh\ntouch \"$CONTROL_TOWER_WORKFLOW/verifier-started\"\nwhile [ ! -e \"$CONTROL_TOWER_WORKFLOW/release\" ]; do sleep 0.02; done\nprintf 'verified\\n'\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            root.join("stages/010-seed/verify-up"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    let app = router(context(configured_workspace(&workspace)));
    let mut events = app
        .clone()
        .oneshot(request("GET", "/api/workflows/fixture/events", None))
        .await
        .unwrap()
        .into_body();
    let initial_frame = tokio::time::timeout(Duration::from_secs(2), events.frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .into_data()
        .unwrap();
    let initial_event = String::from_utf8(initial_frame.to_vec()).unwrap();
    let initial_data = initial_event
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .unwrap();
    let initial: serde_json::Value = serde_json::from_str(initial_data).unwrap();
    let body = serde_json::json!({"direction":"up", "target_stage":10, "expected_checkpoint":initial["checkpoint"]["state"]}).to_string();
    let movement_app = app.clone();
    let movement = tokio::spawn(async move {
        movement_app
            .oneshot(request(
                "POST",
                "/api/workflows/fixture/movements",
                Some(body),
            ))
            .await
            .unwrap()
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !root.join("verifier-started").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    let frame = tokio::time::timeout(Duration::from_secs(2), events.frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .into_data()
        .unwrap();
    let event = String::from_utf8(frame.to_vec()).unwrap();
    let data = event
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .unwrap();
    let running: serde_json::Value = serde_json::from_str(data).unwrap();
    assert_eq!(running["movement_busy"], true);
    assert_eq!(running["observation"]["active_role"]["role"], "verify-up");
    assert_eq!(
        running["observation"]["role_results"][0]["stdout"],
        "mutation complete\n"
    );
    assert_eq!(
        running["observation"]["role_results"][1]["state"],
        "in_progress"
    );

    fs::write(root.join("release"), "").unwrap();
    assert_eq!(movement.await.unwrap().status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn missing_workflow_inventory_is_rejected_as_an_invalid_workspace() {
    let temp = TempDir::new();
    let workspace = temp.path().join("demo");
    fs::create_dir_all(&workspace).unwrap();
    fs::write(
        workspace.join("control-tower.toml"),
        "[workspace]\nlabel = \"Demo\"\n",
    )
    .unwrap();
    assert!(
        discover_workspace(&workspace)
            .err()
            .unwrap()
            .contains("requires workflows/")
    );
}
