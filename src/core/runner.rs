mod env;
mod ipc_loop;
mod protocol;
mod signal;
mod store;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::{Result, anyhow};
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Child;

use crate::app::types::RuntimeEnv;
use crate::core::settings::Settings;
use crate::db::connection::connect;
use crate::db::entities::workflow_run::RunStatus;

use env::{parse_runtime_env, spawn_container_runner, spawn_host_runner};
use ipc_loop::{IpcLoopContext, drive_ipc_loop, send_start_run};
use protocol::send_shutdown;
use signal::{CancelState, install_signal_handler, wait_for_child};
use store::{create_run, upsert_workflow};

const DEFAULT_OPENCODE_URL: &str = "http://127.0.0.1:4096";

/// Parameters for running a workflow synchronously.
pub struct WorkflowArgs<'a> {
    pub project_root: &'a Path,
    pub workflow_name: &'a str,
    pub workflow_path: &'a Path,
    pub env: RuntimeEnv,
    pub yolo: bool,
    pub codebase_id: &'a str,
}

struct NodeRunner {
    child: Child,
    /// Bidirectional TCP IPC channel. Taken by `run_workflow` after spawn;
    /// `None` once consumed.
    ipc: Option<TcpStream>,
}

/// Run a workflow synchronously, blocking until it completes or is cancelled.
///
/// Creates DB records, spawns Node, runs the IPC loop in the foreground, and
/// returns the final run status. Ctrl+C sends a cancel to Node and waits for
/// graceful shutdown; a second Ctrl+C force-kills immediately.
///
/// # Errors
/// Returns an error if the database connection fails, the workflow cannot be
/// registered, the run record cannot be created, or the Node runtime fails.
pub async fn run_workflow(args: &WorkflowArgs<'_>) -> Result<RunStatus> {
    let ctx = setup_run(args).await?;
    let mut runner = spawn_runner(args.project_root, args.env, args.codebase_id).await?;
    let ipc = runner
        .ipc
        .take()
        .ok_or_else(|| anyhow!("IPC channel not available"))?;
    let (ipc_read, mut ipc_write) = io::split(ipc);

    send_start_run(&mut ipc_write, args).await?;
    install_signal_handler(&ctx.cancel);

    let final_status = drive_ipc_loop(&ctx, &mut ipc_write, ipc_read).await?;

    send_shutdown(&mut ipc_write).await;
    drop(ipc_write);
    wait_for_child(&mut runner.child, &ctx.cancel).await?;

    Ok(final_status)
}

async fn setup_run(args: &WorkflowArgs<'_>) -> Result<IpcLoopContext> {
    let db = connect().await?;
    let workflow_env = parse_runtime_env(args.env);
    let workflow_id = upsert_workflow(&db, args.workflow_name, args.workflow_path).await?;
    let run_id = create_run(&db, workflow_id, workflow_env).await?;

    let server_url = Settings::load(args.project_root)
        .ok()
        .and_then(|s| s.opencode)
        .and_then(|o| o.server_url)
        .unwrap_or_else(|| DEFAULT_OPENCODE_URL.to_string());

    println!("workflow started (run id: {run_id})");

    let cancel = Arc::new(CancelState {
        cancelled: AtomicBool::new(false),
        force_kill: AtomicBool::new(false),
    });

    Ok(IpcLoopContext {
        db,
        run_id,
        cancel,
        server_url,
    })
}

async fn spawn_runner(
    project_root: &Path,
    env: RuntimeEnv,
    codebase_id: &str,
) -> Result<NodeRunner> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    let child = match env {
        RuntimeEnv::Host => spawn_host_runner(project_root, port)?,
        RuntimeEnv::Container => spawn_container_runner(project_root, codebase_id, port)?,
    };

    let (stream, _) = listener.accept().await?;

    Ok(NodeRunner {
        child,
        ipc: Some(stream),
    })
}

#[cfg(test)]
mod tests {
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn tcp_listener_binds_and_accepts_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let connect_handle = tokio::spawn(async move {
            TcpStream::connect(format!("127.0.0.1:{port}"))
                .await
                .unwrap()
        });

        let (server_stream, _) = listener.accept().await.unwrap();
        let client_stream = connect_handle.await.unwrap();

        assert!(server_stream.peer_addr().is_ok());
        assert!(client_stream.peer_addr().is_ok());
    }
}
