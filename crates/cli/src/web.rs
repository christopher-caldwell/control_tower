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
    workspace::{WorkspaceContext, discover_workspace},
};
use axum::serve;
use std::{net::TcpListener, path::Path, sync::Arc};
use tokio::net::TcpListener as TokioTcpListener;

#[derive(Clone)]
struct ServerContext {
    workspace: WorkspaceContext,
    observations: Arc<ObservationStore>,
}

fn startup_message(url: &str) -> String {
    format!("Control Tower UI: \x1b]8;;{url}\x1b\\{url}\x1b]8;;\x1b\\")
}

pub(super) fn run_from_current_directory() -> Result<(), String> {
    let workspace_root = std::env::current_dir()
        .map_err(|error| format!("cannot determine the launch Workspace directory: {error}"))?;
    run_from(&workspace_root)
}

fn run_from(workspace_root: &Path) -> Result<(), String> {
    let workspace = discover_workspace(workspace_root)?;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| format!("cannot bind the loopback UI listener: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("cannot prepare the UI listener: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("cannot inspect the UI listener: {error}"))?;
    let url = format!("http://127.0.0.1:{}", address.port());
    let context = Arc::new(ServerContext {
        workspace,
        observations: Arc::new(ObservationStore::new()),
    });

    println!("{}", startup_message(&url));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("cannot start the UI runtime: {error}"))?;
    runtime.block_on(async move {
        let listener = TokioTcpListener::from_std(listener)
            .map_err(|error| format!("cannot start the UI listener: {error}"))?;
        serve(listener, router(context))
            .await
            .map_err(|error| format!("the UI server stopped unexpectedly: {error}"))
    })
}
