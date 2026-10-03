use super::*;
use super::{dto::*, http::*, runtime::*, workspace::*};
use axum::http::header::{
    AUTHORIZATION, CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, COOKIE, HOST, ORIGIN,
    SET_COOKIE,
};
use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Request, Response, StatusCode},
};
use control_tower_application::{Direction, ExecutableRole, Stage};
use control_tower_database::operations;
use http_body_util::BodyExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream as TokioTcpStream;
use tokio::sync::watch;
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
    context_at(project, "127.0.0.1:43001")
}

fn context_at(project: ProjectContext, host: &str) -> Arc<ServerContext> {
    let (shutdown, _) = watch::channel(false);
    Arc::new(ServerContext {
        project,
        expected_host: host.to_owned(),
        expected_origin: format!("http://{host}"),
        bootstrap_token: "startup-secret".to_owned(),
        session_token: "browser-session".to_owned(),
        observations: Arc::new(ObservationStore::new()),
        shutdown,
    })
}

struct NetworkResponse {
    reader: BufReader<TokioTcpStream>,
    status: u16,
    chunked: bool,
    remaining: Option<usize>,
    pending: Vec<u8>,
}

impl NetworkResponse {
    async fn open(
        host: &str,
        path: &str,
        method: &str,
        body: Option<&str>,
        authorized: bool,
        origin: Option<&str>,
        host_header: Option<&str>,
    ) -> Self {
        let stream = TokioTcpStream::connect(host).await.unwrap();
        let mut reader = BufReader::new(stream);
        let body = body.unwrap_or_default();
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: keep-alive\r\n",
            host_header.unwrap_or(host)
        );
        if let Some(origin) = origin {
            request.push_str(&format!("Origin: {origin}\r\n"));
        }
        if authorized {
            request.push_str("Cookie: ct_session=browser-session\r\n");
        }
        if !body.is_empty() || method == "POST" {
            request.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            ));
        }
        request.push_str("\r\n");
        request.push_str(body);
        reader
            .get_mut()
            .write_all(request.as_bytes())
            .await
            .unwrap();

        let mut response_headers = Vec::new();
        loop {
            let mut line = Vec::new();
            let count = reader.read_until(b'\n', &mut line).await.unwrap();
            assert_ne!(count, 0, "loopback server closed before response headers");
            let blank = line == b"\r\n" || line == b"\n";
            response_headers.extend_from_slice(&line);
            if blank {
                break;
            }
        }
        let headers = String::from_utf8_lossy(&response_headers);
        let mut lines = headers.lines();
        let status = lines
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|status| status.parse().ok())
            .expect("valid HTTP response status line");
        let chunked = headers.lines().any(|line| {
            line.split_once(':').is_some_and(|(name, value)| {
                name.eq_ignore_ascii_case("transfer-encoding")
                    && value.to_ascii_lowercase().contains("chunked")
            })
        });
        let remaining = headers.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        });
        Self {
            reader,
            status,
            chunked,
            remaining,
            pending: Vec::new(),
        }
    }

    async fn next_body_chunk(&mut self) -> Option<Vec<u8>> {
        if self.chunked {
            let mut line = Vec::new();
            if self.reader.read_until(b'\n', &mut line).await.ok()? == 0 {
                return None;
            }
            let size = String::from_utf8_lossy(&line)
                .split(';')
                .next()?
                .trim()
                .to_owned();
            let size = usize::from_str_radix(&size, 16).ok()?;
            if size == 0 {
                loop {
                    let mut trailer = Vec::new();
                    if self.reader.read_until(b'\n', &mut trailer).await.ok()? == 0
                        || trailer == b"\r\n"
                        || trailer == b"\n"
                    {
                        return None;
                    }
                }
            }
            let mut chunk = vec![0; size];
            self.reader.read_exact(&mut chunk).await.ok()?;
            let mut terminator = [0; 2];
            self.reader.read_exact(&mut terminator).await.ok()?;
            Some(chunk)
        } else if let Some(remaining) = self.remaining.as_mut() {
            if *remaining == 0 {
                return None;
            }
            let take = (*remaining).min(16 * 1024);
            let mut chunk = vec![0; take];
            self.reader.read_exact(&mut chunk).await.ok()?;
            *remaining -= take;
            Some(chunk)
        } else {
            let mut chunk = vec![0; 16 * 1024];
            let read = self.reader.read(&mut chunk).await.ok()?;
            if read == 0 {
                None
            } else {
                chunk.truncate(read);
                Some(chunk)
            }
        }
    }

    async fn body(&mut self) -> Vec<u8> {
        let mut body = Vec::new();
        while let Some(chunk) = self.next_body_chunk().await {
            body.extend_from_slice(&chunk);
        }
        body
    }

    async fn sse_event(&mut self) -> String {
        loop {
            if let Some(end) = sse_event_end(&self.pending) {
                let event = self.pending.drain(..end).collect::<Vec<_>>();
                return String::from_utf8_lossy(&event).into_owned();
            }
            let Some(chunk) = self.next_body_chunk().await else {
                return String::new();
            };
            self.pending.extend_from_slice(&chunk);
        }
    }
}

fn sse_event_end(bytes: &[u8]) -> Option<usize> {
    let lf = bytes
        .windows(2)
        .position(|pair| pair == b"\n\n")
        .map(|index| index + 2);
    let crlf = bytes
        .windows(4)
        .position(|quad| quad == b"\r\n\r\n")
        .map(|index| index + 4);
    match (lf, crlf) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(end), None) | (None, Some(end)) => Some(end),
        (None, None) => None,
    }
}

async fn network_request(
    host: &str,
    path: &str,
    method: &str,
    body: Option<&str>,
    authorized: bool,
    origin: Option<&str>,
    host_header: Option<&str>,
) -> (u16, Vec<u8>) {
    let mut response =
        NetworkResponse::open(host, path, method, body, authorized, origin, host_header).await;
    let status = response.status;
    (status, response.body().await)
}

async fn start_loopback_server(
    project: ProjectContext,
) -> (String, Arc<ServerContext>, tokio::task::JoinHandle<()>) {
    let listener = TokioTcpListener::bind("127.0.0.1:0").await.unwrap();
    let host = listener.local_addr().unwrap().to_string();
    let context = context_at(project, &host);
    let task_context = context.clone();
    let task = tokio::spawn(async move {
        serve(listener, router(task_context)).await.unwrap();
    });
    (host, context, task)
}

#[tokio::test(flavor = "current_thread")]
async fn graceful_shutdown_closes_idle_sse_clients_and_releases_listener() {
    let temp = TempDir::new();
    let project = temp.path().join("demo");
    fs::create_dir_all(&project).unwrap();
    workspace(&project, "fixture", "010-seed", true);
    let listener = TokioTcpListener::bind("127.0.0.1:0").await.unwrap();
    let host = listener.local_addr().unwrap().to_string();
    let context = context_at(discover_project(&project).unwrap(), &host);
    let shutdown = context.shutdown.clone();
    let (signal_shutdown, wait_for_shutdown) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        serve(listener, router(context))
            .with_graceful_shutdown(async move {
                let _ = wait_for_shutdown.await;
                shutdown.send_replace(true);
            })
            .await
            .unwrap();
    });

    let origin = format!("http://{host}");
    let mut events = NetworkResponse::open(
        &host,
        "/api/workspaces/fixture/events",
        "GET",
        None,
        true,
        Some(&origin),
        None,
    )
    .await;
    assert_eq!(events.status, StatusCode::OK.as_u16());
    assert!(events.sse_event().await.contains("event: snapshot"));

    signal_shutdown.send(()).unwrap();
    let closed = tokio::time::timeout(Duration::from_secs(2), events.next_body_chunk())
        .await
        .expect("idle SSE clients should close during graceful shutdown");
    assert!(closed.is_none());
    tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .expect("graceful server shutdown should finish")
        .unwrap();
    assert!(TokioTcpStream::connect(&host).await.is_err());
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

#[tokio::test]
async fn workspace_reads_overlay_retained_checkpoint_and_failure_when_storage_is_stale_or_unavailable()
 {
    let temp = TempDir::new();
    let project = temp.path().join("demo");
    fs::create_dir_all(&project).unwrap();
    let root = workspace(&project, "fixture", "010-seed", true);
    workspace(&project, "fixture", "200-finish", true);
    let server_context = context(discover_project(&project).unwrap());
    let runtime = server_context.observations.workspace_state("fixture");
    let stale_view = workspace_snapshot("demo", &server_context.project.workspaces[0]).unwrap();
    assert!(stale_view.checkpoint.accepted_stage.is_none());
    runtime.remember_view(stale_view.clone());

    let server_instance_id = server_context.observations.server_instance_id.clone();
    let mut observation = runtime
        .begin_operation(
            "fixture",
            &server_instance_id,
            "retained-save-failure",
            Direction::Up,
            200,
        )
        .unwrap();
    observation.state = "stopped";
    observation.confirmed_checkpoint = Some(CheckpointView {
        accepted_stage: Some(StageIdentity {
            number: 10,
            name: "seed".to_owned(),
        }),
        pending_transition: Some(PendingView {
            direction: "up",
            stage: StageIdentity {
                number: 200,
                name: "finish".to_owned(),
            },
        }),
        workflow_started: true,
    });
    observation.attempted_checkpoint = Some(CheckpointView {
        accepted_stage: Some(StageIdentity {
            number: 200,
            name: "finish".to_owned(),
        }),
        pending_transition: None,
        workflow_started: true,
    });
    observation.failure = Some(FailureView {
        kind: "checkpoint_save_failed",
        message: "the final checkpoint was not confirmed".to_owned(),
        stage: None,
        role: None,
    });
    let mut guard = OperationGuard::new(
        runtime.clone(),
        "fixture".to_owned(),
        server_instance_id,
        "retained-save-failure".to_owned(),
    );
    observation = guard.finish(observation, "movement.finished");
    assert_eq!(observation.state, "stopped");

    let app = router(server_context);
    let response = app
        .clone()
        .oneshot(api_request("GET", "/api/workspaces/fixture", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let fresh_storage_with_retained_outcome = response_json(response).await;
    assert_eq!(
        fresh_storage_with_retained_outcome["checkpoint"]["accepted_stage"]["number"], 10,
        "the runtime's newer confirmed checkpoint must overlay the stale status read"
    );
    assert_eq!(
        fresh_storage_with_retained_outcome["observation"]["attempted_checkpoint"]["accepted_stage"]
            ["number"],
        200
    );
    assert_eq!(
        fresh_storage_with_retained_outcome["observation"]["failure"]["kind"],
        "checkpoint_save_failed"
    );

    fs::rename(root.join("stages"), root.join("stages-unavailable")).unwrap();
    let response = app
        .oneshot(api_request("GET", "/api/workspaces/fixture", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cached = response_json(response).await;
    assert!(cached["storage_issue"].is_string());
    assert_eq!(cached["checkpoint"]["accepted_stage"]["number"], 10);
    assert_eq!(
        cached["observation"]["attempted_checkpoint"]["accepted_stage"]["number"],
        200
    );
    assert_eq!(
        cached["observation"]["failure"]["kind"],
        "checkpoint_save_failed"
    );
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
async fn loopback_http_sse_keeps_role_output_and_admission_after_request_disconnect() {
    let temp = TempDir::new();
    let project = temp.path().join("demo");
    fs::create_dir_all(&project).unwrap();
    let root = workspace(&project, "fixture", "010-seed", true);
    let security_root = workspace(&project, "security", "010-protected", true);
    let outside = temp.path().join("outside-workspace");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("private.txt"), "private outside bytes").unwrap();
    std::os::unix::fs::symlink(&outside, project.join("workspaces/escaped")).unwrap();
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
    write_executable(
        &security_root.join("stages/010-protected/up"),
        "#!/bin/sh\ntouch \"$CONTROL_TOWER_WORKSPACE/unauthorized-role-ran\"\n",
    );
    let (host, context, server) = start_loopback_server(discover_project(&project).unwrap()).await;
    let origin = format!("http://{host}");
    let mut events = NetworkResponse::open(
        &host,
        "/api/workspaces/fixture/events",
        "GET",
        None,
        true,
        Some(&origin),
        None,
    )
    .await;
    assert_eq!(events.status, 200);
    assert!(events.sse_event().await.contains("event: snapshot"));

    let post_host = host.clone();
    let post_origin = origin.clone();
    let post = tokio::spawn(async move {
        network_request(
            &post_host,
            "/api/workspaces/fixture/movements",
            "POST",
            Some("{\"direction\":\"up\",\"target_stage\":200}"),
            true,
            Some(&post_origin),
            None,
        )
        .await
    });
    wait_for_path(&root.join("second-started")).await;

    let mut saw_first_finish = false;
    let mut saw_second_start = false;
    tokio::time::timeout(Duration::from_secs(8), async {
        while !saw_first_finish || !saw_second_start {
            let event = events.sse_event().await;
            assert!(
                !event.is_empty(),
                "SSE connection ended before role observations"
            );
            saw_first_finish |=
                event.contains("event: role.finished") && event.contains("\"number\":10");
            saw_second_start |=
                event.contains("event: role.started") && event.contains("\"number\":200");
        }
    })
    .await
    .expect("role finish should arrive before the next role completes");

    let (status, body) = network_request(
        &host,
        "/api/workspaces/fixture",
        "GET",
        None,
        true,
        Some(&origin),
        None,
    )
    .await;
    assert_eq!(status, 200);
    let view: serde_json::Value = serde_json::from_slice(&body).unwrap();
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
    let output_path = format!("/api/workspaces/fixture/outputs/{output_id}/stdout");
    let (status, stdout) =
        network_request(&host, &output_path, "GET", None, true, Some(&origin), None).await;
    assert_eq!(status, 200);
    assert_eq!(stdout, b"\xff\0<script>safe</script>");
    let (status, stderr) = network_request(
        &host,
        &format!("/api/workspaces/fixture/outputs/{output_id}/stderr"),
        "GET",
        None,
        true,
        Some(&origin),
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(stderr, b"err\x01\n");

    for (path, session, request_origin, request_host, expected_status) in [
        (
            output_path.as_str(),
            false,
            Some(origin.as_str()),
            None,
            401,
        ),
        (
            output_path.as_str(),
            true,
            Some("http://attacker.invalid"),
            None,
            403,
        ),
        (
            output_path.as_str(),
            true,
            Some(origin.as_str()),
            Some("127.0.0.1:1"),
            421,
        ),
        (
            "/api/workspaces/unknown/outputs/private/stdout",
            true,
            Some(origin.as_str()),
            None,
            404,
        ),
        (
            "/api/workspaces/fixture/events",
            false,
            Some(origin.as_str()),
            None,
            401,
        ),
    ] {
        let (status, _) = network_request(
            &host,
            path,
            "GET",
            None,
            session,
            request_origin,
            request_host,
        )
        .await;
        assert_eq!(status, expected_status, "unexpected response for {path}");
    }
    let (status, _) = network_request(
        &host,
        "/api/workspaces/fixture/movements",
        "POST",
        Some("{\"direction\":\"up\",\"target_stage\":200}"),
        true,
        Some("http://attacker.invalid"),
        None,
    )
    .await;
    assert_eq!(status, 403);
    for (session, request_origin, request_host, expected_status) in [
        (true, Some("http://attacker.invalid"), None, 403),
        (true, None, None, 403),
        (false, Some(origin.as_str()), None, 401),
        (true, Some(origin.as_str()), Some("127.0.0.1:1"), 421),
    ] {
        let (status, _) = network_request(
            &host,
            "/api/workspaces/security/movements",
            "POST",
            Some("{\"direction\":\"up\",\"target_stage\":10}"),
            session,
            request_origin,
            request_host,
        )
        .await;
        assert_eq!(status, expected_status);
    }
    assert!(
        !security_root.join("unauthorized-role-ran").exists(),
        "untrusted HTTP requests must not execute an otherwise usable role"
    );
    let (status, _) = network_request(
        &host,
        "/api/workspaces/../fixture/outputs/private/stdout",
        "GET",
        None,
        true,
        Some(&origin),
        None,
    )
    .await;
    assert!(status == 400 || status == 404);
    let (status, body) = network_request(
        &host,
        &format!("/api/workspaces/escaped/outputs/{output_id}/stdout"),
        "GET",
        None,
        true,
        Some(&origin),
        None,
    )
    .await;
    assert_ne!(status, 200);
    assert!(!String::from_utf8_lossy(&body).contains("private outside bytes"));

    let (status, body) = network_request(
        &host,
        "/api/workspaces/fixture/movements",
        "POST",
        Some("{\"direction\":\"up\",\"target_stage\":200}"),
        true,
        Some(&origin),
        None,
    )
    .await;
    assert_eq!(status, 409);
    let conflict: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(conflict["error"]["code"], "workspace_busy");

    post.abort();
    assert!(post.await.unwrap_err().is_cancelled());
    let (status, body) = network_request(
        &host,
        "/api/workspaces/fixture/movements",
        "POST",
        Some("{\"direction\":\"up\",\"target_stage\":200}"),
        true,
        Some(&origin),
        None,
    )
    .await;
    assert_eq!(status, 409);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"]["code"],
        "workspace_busy"
    );

    drop(events);
    fs::write(root.join("release-second"), "continue").unwrap();
    let final_view = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let (status, body) = network_request(
                &host,
                "/api/workspaces/fixture",
                "GET",
                None,
                true,
                Some(&origin),
                None,
            )
            .await;
            assert_eq!(status, 200);
            let view: serde_json::Value = serde_json::from_slice(&body).unwrap();
            if !view["movement_busy"].as_bool().unwrap_or(true) {
                break view;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("movement should finish after the controlled child is released");
    assert_eq!(final_view["observation"]["state"], "complete");
    assert_eq!(final_view["checkpoint"]["accepted_stage"]["number"], 200);
    assert_eq!(
        fs::read_to_string(root.join("roles.log")).unwrap(),
        "10-up\n200-up\n"
    );

    let mut reconnected = NetworkResponse::open(
        &host,
        "/api/workspaces/fixture/events",
        "GET",
        None,
        true,
        Some(&origin),
        None,
    )
    .await;
    let snapshot = reconnected.sse_event().await;
    assert!(snapshot.contains("event: snapshot"));
    assert!(snapshot.contains("\"state\":\"complete\""));
    assert!(snapshot.contains("\"accepted_stage\":{\"number\":200"));
    server.abort();
    drop(context);
}

#[tokio::test(flavor = "current_thread")]
async fn loopback_sse_recovers_after_a_real_subscriber_lag() {
    let temp = TempDir::new();
    let project = temp.path().join("demo");
    fs::create_dir_all(&project).unwrap();
    workspace(&project, "fixture", "010-seed", true);
    let (host, context, server) = start_loopback_server(discover_project(&project).unwrap()).await;
    let origin = format!("http://{host}");
    let mut events = NetworkResponse::open(
        &host,
        "/api/workspaces/fixture/events",
        "GET",
        None,
        true,
        Some(&origin),
        None,
    )
    .await;
    assert!(events.sse_event().await.contains("event: snapshot"));
    let runtime = context.observations.workspace_state("fixture");
    for revision in 0..(EVENT_BUFFER + 8) {
        let _ = runtime.events.send(RuntimeEvent {
            workspace_id: "fixture".to_owned(),
            server_instance_id: context.observations.server_instance_id.clone(),
            revision: revision as u64 + 1,
            operation_id: Some("lag-operation".to_owned()),
            kind: "role.started",
            direction: Some("up"),
            target_stage: Some(10),
            stage: Some(StageIdentity {
                number: 10,
                name: "seed".to_owned(),
            }),
            role: Some("up"),
        });
    }
    let resync = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = events.sse_event().await;
            if event.contains("event: resync") {
                break event;
            }
            assert!(
                !event.is_empty(),
                "loopback SSE closed instead of resynchronizing"
            );
        }
    })
    .await
    .expect("a lagged TCP subscriber receives a fresh snapshot");
    assert!(resync.contains("\"workspace_id\":\"fixture\""));
    assert!(resync.contains("\"movement_busy\":false"));
    server.abort();
}

#[cfg(unix)]
#[tokio::test]
async fn loopback_http_retains_pending_and_final_checkpoint_save_failures() {
    let temp = TempDir::new();
    let project = temp.path().join("demo");
    fs::create_dir_all(&project).unwrap();
    let pending_root = workspace(&project, "pending-fault", "010-seed", true);
    write_executable(
        &pending_root.join("stages/010-seed/up"),
        "#!/bin/sh\nprintf 'mutation before pending failure'\ntouch \"$CONTROL_TOWER_WORKSPACE/up-ran\"\n",
    );
    write_executable(
        &pending_root.join("stages/010-seed/verify-up"),
        "#!/bin/sh\ntouch \"$CONTROL_TOWER_WORKSPACE/verifier-ran\"\n",
    );
    rusqlite::Connection::open(pending_root.join(".control_tower/state.sqlite3"))
        .unwrap()
        .execute_batch("CREATE TRIGGER fail_pending_publication BEFORE UPDATE ON workbench_state WHEN NEW.pending_stage_index IS NOT NULL BEGIN SELECT RAISE(ABORT, 'injected pending checkpoint failure'); END;")
        .unwrap();

    let final_root = workspace(&project, "final-fault", "010-seed", true);
    let later = final_root.join("stages/200-later");
    fs::create_dir_all(&later).unwrap();
    write_executable(
        &final_root.join("stages/010-seed/up"),
        "#!/bin/sh\nprintf 'first mutation'\nprintf 'first-up\\n' >> \"$CONTROL_TOWER_WORKSPACE/roles.log\"\n",
    );
    write_executable(
        &final_root.join("stages/010-seed/verify-up"),
        "#!/bin/sh\nprintf 'first-verify\\n' >> \"$CONTROL_TOWER_WORKSPACE/roles.log\"\n",
    );
    write_executable(
        &later.join("up"),
        "#!/bin/sh\nprintf 'later-up\\n' >> \"$CONTROL_TOWER_WORKSPACE/roles.log\"\n",
    );
    rusqlite::Connection::open(final_root.join(".control_tower/state.sqlite3"))
        .unwrap()
        .execute_batch("CREATE TRIGGER fail_final_acceptance BEFORE UPDATE ON workbench_state WHEN NEW.completed_stage_count = 1 BEGIN SELECT RAISE(ABORT, 'injected final checkpoint failure'); END;")
        .unwrap();

    let (host, _context, server) = start_loopback_server(discover_project(&project).unwrap()).await;
    let origin = format!("http://{host}");
    let mut pending_events = NetworkResponse::open(
        &host,
        "/api/workspaces/pending-fault/events",
        "GET",
        None,
        true,
        Some(&origin),
        None,
    )
    .await;
    assert!(pending_events.sse_event().await.contains("event: snapshot"));
    let (status, body) = network_request(
        &host,
        "/api/workspaces/pending-fault/movements",
        "POST",
        Some("{\"direction\":\"up\",\"target_stage\":10}"),
        true,
        Some(&origin),
        None,
    )
    .await;
    assert_eq!(status, 200);
    let pending: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let pending = &pending["observation"];
    assert_eq!(pending["state"], "stopped");
    assert_eq!(pending["failure"]["kind"], "checkpoint_save_failed");
    assert_eq!(
        pending["confirmed_checkpoint"]["pending_transition"],
        serde_json::Value::Null
    );
    assert_eq!(
        pending["attempted_checkpoint"]["pending_transition"]["direction"],
        "up"
    );
    assert_eq!(pending["role_results"][0]["state"], "succeeded");
    assert!(pending_root.join("up-ran").exists());
    assert!(!pending_root.join("verifier-ran").exists());
    let stored: (i64, Option<i64>) = rusqlite::Connection::open(
        pending_root.join(".control_tower/state.sqlite3"),
    )
    .unwrap()
    .query_row(
        "SELECT completed_stage_count, pending_stage_index FROM workbench_state WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .unwrap();
    assert_eq!(stored, (0, None));
    drop(pending_events);
    let mut pending_reconnect = NetworkResponse::open(
        &host,
        "/api/workspaces/pending-fault/events",
        "GET",
        None,
        true,
        Some(&origin),
        None,
    )
    .await;
    let pending_snapshot = pending_reconnect.sse_event().await;
    assert!(pending_snapshot.contains("\"state\":\"stopped\""));
    assert!(pending_snapshot.contains("\"pending_transition\":null"));

    let mut final_events = NetworkResponse::open(
        &host,
        "/api/workspaces/final-fault/events",
        "GET",
        None,
        true,
        Some(&origin),
        None,
    )
    .await;
    assert!(final_events.sse_event().await.contains("event: snapshot"));
    let (status, body) = network_request(
        &host,
        "/api/workspaces/final-fault/movements",
        "POST",
        Some("{\"direction\":\"up\",\"target_stage\":200}"),
        true,
        Some(&origin),
        None,
    )
    .await;
    assert_eq!(status, 200);
    let final_failure: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let final_failure = &final_failure["observation"];
    assert_eq!(final_failure["state"], "stopped");
    assert_eq!(final_failure["failure"]["kind"], "checkpoint_save_failed");
    assert_eq!(
        final_failure["confirmed_checkpoint"]["accepted_stage"],
        serde_json::Value::Null
    );
    assert_eq!(
        final_failure["confirmed_checkpoint"]["pending_transition"]["direction"],
        "up"
    );
    assert_eq!(
        final_failure["attempted_checkpoint"]["accepted_stage"]["number"],
        10
    );
    assert_eq!(final_failure["role_results"][0]["state"], "succeeded");
    assert!(!final_root.join("stages/200-later/up-ran").exists());
    assert_eq!(
        fs::read_to_string(final_root.join("roles.log")).unwrap(),
        "first-up\nfirst-verify\n"
    );
    let stored: (i64, Option<i64>) = rusqlite::Connection::open(
        final_root.join(".control_tower/state.sqlite3"),
    )
    .unwrap()
    .query_row(
        "SELECT completed_stage_count, pending_stage_index FROM workbench_state WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .unwrap();
    assert_eq!(stored, (0, Some(0)));
    drop(final_events);
    let mut final_reconnect = NetworkResponse::open(
        &host,
        "/api/workspaces/final-fault/events",
        "GET",
        None,
        true,
        Some(&origin),
        None,
    )
    .await;
    let final_snapshot = final_reconnect.sse_event().await;
    assert!(final_snapshot.contains("\"state\":\"stopped\""));
    assert!(final_snapshot.contains("\"accepted_stage\":null"));
    assert!(final_snapshot.contains("\"pending_transition\":{\"direction\":\"up\""));
    server.abort();
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
    let connection = rusqlite::Connection::open(root.join(".control_tower/state.sqlite3")).unwrap();
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
    let connection = rusqlite::Connection::open(root.join(".control_tower/state.sqlite3")).unwrap();
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
