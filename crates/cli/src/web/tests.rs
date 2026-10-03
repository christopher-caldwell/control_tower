use super::*;
use super::{http::*, runtime::*, workspace::*};
use axum::{
    Router,
    body::Body,
    http::{Request, Response, StatusCode},
};
use control_tower_application::{WorkbenchQueries, WorkbenchState, WorkbenchWrites};
use control_tower_database::{
    operations,
    workbench::{SqliteWorkbenchQueries, SqliteWorkbenchWrites},
};
use http_body_util::BodyExt;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::sync::watch;
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
fn prepared_workspace(project: &Path, id: &str, script: &str) -> PathBuf {
    let root = project.join("workspaces").join(id);
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
fn context(project: ProjectContext) -> Arc<ServerContext> {
    let (shutdown, _) = watch::channel(false);
    Arc::new(ServerContext {
        project,
        expected_host: "127.0.0.1:43001".into(),
        expected_origin: "http://127.0.0.1:43001".into(),
        bootstrap_token: "startup-secret".into(),
        session_token: "browser-session".into(),
        observations: Arc::new(ObservationStore::new()),
        shutdown,
    })
}
fn request(method: &str, uri: &str, body: Option<String>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("host", "127.0.0.1:43001")
        .header("origin", "http://127.0.0.1:43001")
        .header("cookie", "ct_session=browser-session");
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
async fn view(app: &Router, id: &str) -> serde_json::Value {
    let response = app
        .clone()
        .clone()
        .oneshot(request("GET", &format!("/api/workspaces/{id}"), None))
        .await
        .unwrap();
    let (status, payload) = json(response).await;
    assert_eq!(status, StatusCode::OK);
    payload
}
async fn move_to(
    app: &Router,
    id: &str,
    direction: &str,
    target: u32,
    checkpoint: &serde_json::Value,
) -> Response<Body> {
    app.clone().oneshot(request("POST", &format!("/api/workspaces/{id}/movements"), Some(serde_json::json!({ "direction": direction, "target_stage": target, "expected_checkpoint": checkpoint }).to_string()))).await.unwrap()
}

#[test]
fn startup_requires_prepared_workspaces_and_never_repairs_storage() {
    let temp = TempDir::new();
    let project = temp.path().join("empty");
    fs::create_dir_all(&project).unwrap();
    let empty_error = discover_project(&project).err().unwrap();
    assert!(empty_error.contains("No prepared workspaces"));
    assert!(empty_error.contains("workspaces"));
    assert!(!empty_error.contains("this workspace"));
    prepared_workspace(&project, "unprepared", "#!/bin/sh\nexit 0\n");
    let db = project.join("workspaces/unprepared/.control_tower/state.sqlite3");
    fs::remove_file(&db).unwrap();
    let error = discover_project(&project).err().unwrap();
    assert!(error.contains("unprepared"));
    assert!(error.contains("control-tower-db bootstrap-local"));
    assert!(!db.exists());
}

#[test]
fn startup_aggregates_workspace_failures_by_category_without_repair_or_execution() {
    let temp = TempDir::new();
    let project = temp.path().join("mixed");
    fs::create_dir_all(&project).unwrap();

    let healthy = prepared_workspace(&project, "healthy", "#!/bin/sh\nexit 0\n");
    let missing_storage = prepared_workspace(&project, "missing-storage", "#!/bin/sh\nexit 0\n");
    let missing_db = missing_storage.join(".control_tower/state.sqlite3");
    fs::remove_file(&missing_db).unwrap();

    let bootstrap_only = prepared_workspace(&project, "bootstrap-only", "#!/bin/sh\nexit 0\n");
    let bootstrap_db = bootstrap_only.join(".control_tower/state.sqlite3");
    fs::remove_file(&bootstrap_db).unwrap();
    operations::bootstrap(&bootstrap_db).unwrap();

    let invalid_stages = prepared_workspace(&project, "invalid-stages", "#!/bin/sh\nexit 0\n");
    fs::rename(
        invalid_stages.join("stages/010-seed"),
        invalid_stages.join("stages/not-a-stage"),
    )
    .unwrap();

    let invalid_checkpoint =
        prepared_workspace(&project, "invalid-checkpoint", "#!/bin/sh\nexit 0\n");
    let invalid_checkpoint_db = invalid_checkpoint.join(".control_tower/state.sqlite3");
    SqliteWorkbenchWrites::open(&invalid_checkpoint_db)
        .unwrap()
        .record_checkpoint(&WorkbenchState {
            completed_stage_count: 2,
            uuid: None,
            pending: None,
        })
        .unwrap();

    for workspace in [
        &healthy,
        &missing_storage,
        &bootstrap_only,
        &invalid_stages,
        &invalid_checkpoint,
    ] {
        let stage = fs::read_dir(workspace.join("stages"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        fs::write(
            stage.join("up"),
            format!("#!/bin/sh\ntouch {}/startup-ran\n", workspace.display()),
        )
        .unwrap();
    }

    let error = discover_project(&project).err().unwrap();
    assert!(error.contains("UI startup rejected before binding or browser opening"));
    let entry = |id: &str| {
        error
            .split("\n- ")
            .find(|entry| entry.contains(&format!("Workspace `{id}`")))
            .unwrap_or_else(|| panic!("missing diagnostic for {id}: {error}"))
            .to_owned()
    };

    let missing = entry("missing-storage");
    assert!(missing.contains("[storage]"));
    assert!(missing.contains("control-tower-db bootstrap-local"));
    assert!(missing.contains("control-tower-db migrate-local"));
    assert!(missing.contains("control-tower-db verify-local"));

    let bootstrap = entry("bootstrap-only");
    assert!(bootstrap.contains("[storage]"));
    assert!(bootstrap.contains("control-tower-db migrate-local"));

    let stages = entry("invalid-stages");
    assert!(stages.contains("[stages]"));
    assert!(stages.contains("must begin with a number"));
    assert!(stages.contains("numbered directories"));
    assert!(stages.contains("matching mutation"));
    assert!(!stages.contains("control-tower-db"));

    let checkpoint = entry("invalid-checkpoint");
    assert!(checkpoint.contains("[checkpoint]"));
    assert!(checkpoint.contains("invalid stored workbench state"));
    assert!(checkpoint.contains("does not repair checkpoint state"));
    assert!(!checkpoint.contains("control-tower-db"));

    assert!(!missing_db.exists());
    assert!(operations::verify(&bootstrap_db).is_err());
    assert_eq!(
        SqliteWorkbenchQueries::open(&invalid_checkpoint_db)
            .unwrap()
            .read_checkpoint()
            .unwrap()
            .unwrap()
            .completed_stage_count,
        2
    );
    for workspace in [
        &healthy,
        &missing_storage,
        &bootstrap_only,
        &invalid_stages,
        &invalid_checkpoint,
    ] {
        assert!(!workspace.join("startup-ran").exists());
    }
}

#[test]
fn startup_rejects_empty_stages_with_the_discovery_cause() {
    let temp = TempDir::new();
    let project = temp.path().join("empty-stages");
    fs::create_dir_all(&project).unwrap();
    let workspace = prepared_workspace(&project, "no-stages", "#!/bin/sh\nexit 0\n");
    fs::remove_dir_all(workspace.join("stages")).unwrap();
    fs::create_dir_all(workspace.join("stages")).unwrap();

    let error = discover_project(&project).err().unwrap();
    assert!(error.contains("Workspace `no-stages` [stages]"));
    assert!(
        error.contains("no numbered stage directories were found under `stages/`"),
        "{error}"
    );
    assert!(!error.contains("control-tower-db"));
}

#[cfg(unix)]
#[test]
fn startup_aggregates_entry_boundary_failures_without_storage_guidance() {
    let temp = TempDir::new();
    let project = temp.path().join("project");
    let outside_project = temp.path().join("outside-project");
    fs::create_dir_all(&project).unwrap();
    let outside = prepared_workspace(&outside_project, "outside", "#!/bin/sh\nexit 0\n");
    prepared_workspace(&project, "healthy", "#!/bin/sh\nexit 0\n");
    std::os::unix::fs::symlink(&outside, project.join("workspaces/outside-alias")).unwrap();

    let error = discover_project(&project).err().unwrap();
    assert!(error.contains("Workspace entry `outside-alias` [entry path]"));
    assert!(error.contains("resolves outside workspaces/"));
    assert!(!error.contains("control-tower-db"));
}
#[test]
fn startup_rejects_empty_inventory_invalid_stages_and_invalid_checkpoints() {
    let temp = TempDir::new();
    let absent_inventory = temp.path().join("absent");
    fs::create_dir_all(&absent_inventory).unwrap();
    let absent_error = discover_project(&absent_inventory).err().unwrap();
    assert!(absent_error.contains("No prepared workspaces"));
    assert!(absent_error.contains(absent_inventory.join("workspaces").to_str().unwrap()));
    assert!(!absent_error.contains("this workspace"));

    let empty_inventory = temp.path().join("empty-inventory");
    fs::create_dir_all(empty_inventory.join("workspaces")).unwrap();
    fs::write(empty_inventory.join("workspaces/notes.txt"), "ignored").unwrap();
    let empty_error = discover_project(&empty_inventory).err().unwrap();
    assert!(empty_error.contains("No prepared workspaces"));
    assert!(empty_error.contains(empty_inventory.join("workspaces").to_str().unwrap()));
    assert!(!empty_error.contains("this workspace"));

    let invalid_stages = temp.path().join("invalid-stages");
    fs::create_dir_all(&invalid_stages).unwrap();
    let root = prepared_workspace(&invalid_stages, "broken-stages", "#!/bin/sh\nexit 0\n");
    fs::rename(
        root.join("stages/010-seed"),
        root.join("stages/not-a-stage"),
    )
    .unwrap();
    let error = discover_project(&invalid_stages).err().unwrap();
    assert!(error.contains("broken-stages"));
    assert!(error.contains("must begin with a number"));

    let invalid_checkpoint = temp.path().join("invalid-checkpoint");
    fs::create_dir_all(&invalid_checkpoint).unwrap();
    let root = prepared_workspace(
        &invalid_checkpoint,
        "broken-checkpoint",
        "#!/bin/sh\nexit 0\n",
    );
    SqliteWorkbenchWrites::open(&root.join(".control_tower/state.sqlite3"))
        .unwrap()
        .record_checkpoint(&WorkbenchState {
            completed_stage_count: 2,
            uuid: None,
            pending: None,
        })
        .unwrap();
    let error = discover_project(&invalid_checkpoint).err().unwrap();
    assert!(error.contains("broken-checkpoint"));
    assert!(error.contains("invalid stored workbench state"));
}
#[cfg(unix)]
#[test]
fn startup_deduplicates_symlink_aliases_before_assigning_workspace_state() {
    let temp = TempDir::new();
    let project = temp.path().join("demo");
    fs::create_dir_all(&project).unwrap();
    let root = prepared_workspace(&project, "real", "#!/bin/sh\nexit 0\n");
    std::os::unix::fs::symlink(&root, project.join("workspaces/alias")).unwrap();
    assert_eq!(discover_project(&project).unwrap().workspaces.len(), 1);
}

#[cfg(unix)]
#[tokio::test]
async fn startup_ignores_plain_inventory_files_and_checks_role_execute_permission_at_launch() {
    let temp = TempDir::new();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let workspace = prepared_workspace(&project, "fixture", "#!/bin/sh\nexit 0\n");
    fs::write(project.join("workspaces/notes.txt"), "ignored").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(
        workspace.join("stages/010-seed/up"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();

    let discovered = discover_project(&project).unwrap();
    assert_eq!(discovered.workspaces.len(), 1);
    let app = router(context(discovered));
    let initial = view(&app, "fixture").await;
    assert_eq!(
        move_to(&app, "fixture", "up", 10, &initial["checkpoint"]["state"])
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        view(&app, "fixture").await["observation"]["role_results"][0]["state"],
        "launch_failed"
    );
}
#[tokio::test]
async fn stale_checkpoint_is_rejected_before_any_second_effect() {
    let temp = TempDir::new();
    let project_root = temp.path().join("demo");
    fs::create_dir_all(&project_root).unwrap();
    let root = prepared_workspace(
        &project_root,
        "fixture",
        "#!/bin/sh\necho run >> \"$CONTROL_TOWER_WORKSPACE/runs\"\n",
    );
    let app = router(context(discover_project(&project_root).unwrap()));
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
async fn stale_uuid_is_compared_as_part_of_checkpoint_state() {
    let temp = TempDir::new();
    let project = temp.path().join("demo");
    fs::create_dir_all(&project).unwrap();
    let root = prepared_workspace(&project, "fixture", "#!/bin/sh\nexit 0\n");
    SqliteWorkbenchWrites::open(&root.join(".control_tower/state.sqlite3"))
        .unwrap()
        .record_checkpoint(&WorkbenchState {
            completed_stage_count: 0,
            uuid: Some("different-run".into()),
            pending: None,
        })
        .unwrap();
    let app = router(context(discover_project(&project).unwrap()));
    let expected = serde_json::json!({"completed_stage_count":0,"uuid":null,"pending":null});
    let (status, error) = json(move_to(&app, "fixture", "up", 10, &expected).await).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "stale_checkpoint");
}
#[tokio::test]
async fn encoded_unicode_workspace_ids_resolve_only_the_discovered_workspace() {
    let temp = TempDir::new();
    let project = temp.path().join("Project with spaces");
    fs::create_dir_all(&project).unwrap();
    prepared_workspace(&project, "café workspace", "#!/bin/sh\nexit 0\n");
    let app = router(context(discover_project(&project).unwrap()));
    let response = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/workspaces/caf%C3%A9%20workspace",
            None,
        ))
        .await
        .unwrap();
    let (status, view) = json(response).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["workspace"]["id"], "café workspace");
    let arbitrary = app
        .oneshot(request("GET", "/api/workspaces/%2e%2e%2foutside", None))
        .await
        .unwrap();
    assert_eq!(arbitrary.status(), StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn reads_and_reconnects_stay_live_and_disconnect_does_not_release_admission() {
    let temp = TempDir::new();
    let project = temp.path().join("demo");
    fs::create_dir_all(&project).unwrap();
    let root = prepared_workspace(
        &project,
        "fixture",
        "#!/bin/sh\ntouch \"$CONTROL_TOWER_WORKSPACE/started\"\nsleep 0.5\n",
    );
    let app = router(context(discover_project(&project).unwrap()));
    let initial = view(&app, "fixture").await;
    let body = serde_json::json!({ "direction": "up", "target_stage": 10, "expected_checkpoint": initial["checkpoint"]["state"] }).to_string();
    let first_app = app.clone();
    let first = tokio::spawn(async move {
        first_app
            .oneshot(request(
                "POST",
                "/api/workspaces/fixture/movements",
                Some(body),
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
    let read = app
        .clone()
        .oneshot(request("GET", "/api/workspaces/fixture", None))
        .await
        .unwrap();
    assert_eq!(read.status(), StatusCode::OK);
    let definition = app
        .clone()
        .oneshot(request("GET", "/api/workspaces/fixture/stages/10", None))
        .await
        .unwrap();
    assert_eq!(definition.status(), StatusCode::OK);
    let stream = app
        .clone()
        .oneshot(request("GET", "/api/workspaces/fixture/events", None))
        .await
        .unwrap();
    let frame = tokio::time::timeout(Duration::from_secs(1), stream.into_body().frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .into_data()
        .unwrap();
    assert!(
        String::from_utf8(frame.to_vec())
            .unwrap()
            .contains("event: snapshot")
    );
    first.abort();
    let _ = first.await;
    let second_body = serde_json::json!({ "direction":"up", "target_stage":10, "expected_checkpoint":initial["checkpoint"]["state"] }).to_string();
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/workspaces/fixture/movements",
            Some(second_body),
        ))
        .await
        .unwrap();
    let (status, error) = json(response).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "workspace_busy");
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(
        view(&app, "fixture").await["observation"]["state"],
        "complete"
    );
}
#[tokio::test]
async fn snapshots_are_complete_and_output_is_lossless_until_next_admitted_move() {
    let temp = TempDir::new();
    let project = temp.path().join("demo");
    fs::create_dir_all(&project).unwrap();
    let large = 1024 * 1024 + 23;
    prepared_workspace(
        &project,
        "fixture",
        &format!("#!/bin/sh\npython3 -c 'import sys; sys.stdout.buffer.write(b\"z\" * {large})'\n"),
    );
    let app = router(context(discover_project(&project).unwrap()));
    let initial = view(&app, "fixture").await;
    assert_eq!(
        move_to(&app, "fixture", "up", 10, &initial["checkpoint"]["state"])
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    let latest = view(&app, "fixture").await;
    let observation = &latest["observation"];
    assert_eq!(observation["state"], "complete");
    assert_eq!(observation["role_results"].as_array().unwrap().len(), 1);
    assert_eq!(observation["role_results"][0]["stdout_bytes"], large);
    assert_eq!(observation["role_results"][0]["output_available"], true);
    let path = format!(
        "/api/workspaces/fixture/outputs/{}/{}/stdout",
        observation["operation_id"].as_str().unwrap(),
        0
    );
    let output = app
        .clone()
        .oneshot(request("GET", &path, None))
        .await
        .unwrap();
    assert_eq!(output.status(), StatusCode::OK);
    assert_eq!(
        output.into_body().collect().await.unwrap().to_bytes().len(),
        large as usize
    );
    assert_eq!(
        move_to(&app, "fixture", "down", 0, &latest["checkpoint"]["state"])
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.oneshot(request("GET", &path, None))
            .await
            .unwrap()
            .status(),
        StatusCode::GONE
    );
}
#[tokio::test]
async fn definition_reading_uses_workbench_and_keeps_plain_text_preview_limit() {
    let temp = TempDir::new();
    let project = temp.path().join("demo");
    fs::create_dir_all(&project).unwrap();
    let root = prepared_workspace(&project, "fixture", "#!/bin/sh\nexit 0\n");
    fs::write(
        root.join("stages/010-seed/up"),
        "x".repeat(MAX_DEFINITION_BYTES + 3),
    )
    .unwrap();
    let app = router(context(discover_project(&project).unwrap()));
    let (status, result) = json(
        app.clone()
            .oneshot(request("GET", "/api/workspaces/fixture/stages/10", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        result["definitions"][0]["contents"].as_str().unwrap().len(),
        MAX_DEFINITION_BYTES
    );
    assert_eq!(result["definitions"][0]["truncated"], true);
    assert_eq!(
        app.clone()
            .oneshot(request("GET", "/api/workspaces/fixture/stages/999", None))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );

    fs::rename(
        root.join("stages/010-seed"),
        root.join("stages/not-a-stage"),
    )
    .unwrap();
    let (status, unavailable) = json(
        app.oneshot(request("GET", "/api/workspaces/fixture/stages/10", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(unavailable["error"]["code"], "stage_unavailable");
}
#[tokio::test]
async fn snapshot_stream_sends_complete_current_state_after_reconnect() {
    let temp = TempDir::new();
    let project = temp.path().join("demo");
    fs::create_dir_all(&project).unwrap();
    prepared_workspace(&project, "fixture", "#!/bin/sh\nprintf 'mutation';\n");
    let app = router(context(discover_project(&project).unwrap()));
    let initial = view(&app, "fixture").await;
    assert_eq!(
        move_to(&app, "fixture", "up", 10, &initial["checkpoint"]["state"])
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    let response = app
        .oneshot(request("GET", "/api/workspaces/fixture/events", None))
        .await
        .unwrap();
    let mut body = response.into_body();
    let frame = tokio::time::timeout(Duration::from_secs(2), body.frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .into_data()
        .unwrap();
    let text = String::from_utf8(frame.to_vec()).unwrap();
    assert!(text.contains("event: snapshot"));
    assert!(text.contains("\"state\":\"complete\""));
    assert!(text.contains("\"role_results\":["));
}
#[tokio::test]
async fn security_guards_remain_for_private_reads_and_movement() {
    let temp = TempDir::new();
    let project = temp.path().join("demo");
    fs::create_dir_all(&project).unwrap();
    prepared_workspace(
        &project,
        "fixture",
        "#!/bin/sh\ntouch \"$CONTROL_TOWER_WORKSPACE/ran\"\n",
    );
    let app = router(context(discover_project(&project).unwrap()));
    let unauth = Request::builder()
        .method("GET")
        .uri("/api/project")
        .header("host", "127.0.0.1:43001")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(unauth).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    let current = view(&app, "fixture").await;
    assert_eq!(
        move_to(&app, "fixture", "up", 10, &current["checkpoint"]["state"])
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert!(project.join("workspaces/fixture/ran").exists());
}
