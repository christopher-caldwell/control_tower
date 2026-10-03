mod dto;
mod http;
mod movement;
mod runtime;
#[cfg(test)]
mod tests;
mod workspace;

use self::{
    http::router,
    runtime::ObservationStore,
    workspace::{ProjectContext, discover_project},
};
use axum::serve;
use std::{net::TcpListener, path::Path, process::Command, sync::Arc};
use tokio::{net::TcpListener as TokioTcpListener, signal, sync::watch};
use uuid::Uuid;

#[derive(Clone)]
struct ServerContext {
    project: ProjectContext,
    expected_host: String,
    expected_origin: String,
    bootstrap_token: String,
    session_token: String,
    observations: Arc<ObservationStore>,
    shutdown: watch::Sender<bool>,
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
    let (shutdown, _) = watch::channel(false);
    let url = format!("{origin}/#session={bootstrap_token}");
    let context = Arc::new(ServerContext {
        project,
        expected_host: host,
        expected_origin: origin,
        bootstrap_token,
        session_token,
        observations: Arc::new(ObservationStore::new()),
        shutdown: shutdown.clone(),
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
            .with_graceful_shutdown(async move {
                let _ = signal::ctrl_c().await;
                shutdown.send_replace(true);
            })
            .await
            .map_err(|error| format!("the UI server stopped unexpectedly: {error}"))
    })
}
