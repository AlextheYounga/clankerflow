mod protocol;
mod store;

use std::fs::File;
use std::io::{Error as IoError, ErrorKind};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow};
use sea_orm::DatabaseConnection;
use serde_json::json;
use tokio::fs::File as TokioFile;
use tokio::io::{self as tokio_io, AsyncBufReadExt, AsyncWrite, BufReader};
use tokio::process::{Child, Command};
use tokio::signal;
use tokio::time::{Instant, sleep};

use crate::core::capabilities::{CapabilityRequest, dispatch as dispatch_capability};
use crate::core::ipc::Message;
use crate::core::runtime::resolve_node_bin;
use crate::db::connection::connect;
use crate::db::entities::workflow_run::{RunStatus, WorkflowEnv};

use protocol::{
    LoopControl, parse_capability_request_payload, send_cancel, send_shutdown, write_message,
};
use store::{append_run_event, create_run, set_status, upsert_workflow};

const SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_secs(5);

/// Parameters for running a workflow synchronously.
pub struct WorkflowArgs<'a> {
    pub project_root: &'a Path,
    pub workflow_name: &'a str,
    pub workflow_path: &'a Path,
    pub env: &'a str,
    pub yolo: bool,
}

/// Shared cancellation state between the SIGINT handler and the IPC loop.
struct CancelState {
    cancelled: AtomicBool,
    force_kill: AtomicBool,
}

/// Holds DB + run identity for the IPC loop. Avoids passing five separate
/// arguments through every layer.
struct RunContext {
    db: DatabaseConnection,
    run_id: i64,
    cancel: Arc<CancelState>,
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
    let mut child = spawn_runner(args.project_root)?;
    let ipc = child
        .ipc
        .take()
        .ok_or_else(|| anyhow!("IPC channel not available"))?;
    let (ipc_read, mut ipc_write) = tokio_io::split(ipc);

    send_start_run(&mut ipc_write, args).await?;
    install_signal_handler(&ctx.cancel);

    let final_status = drive_ipc_loop(&ctx, &mut ipc_write, ipc_read).await?;

    send_shutdown(&mut ipc_write).await;
    drop(ipc_write);
    wait_for_child(&mut child.child, &ctx.cancel).await?;

    Ok(final_status)
}

async fn setup_run(args: &WorkflowArgs<'_>) -> Result<RunContext> {
    let db = connect().await?;
    let workflow_env = parse_runtime_env(args.env)?;
    let workflow_id = upsert_workflow(&db, args.workflow_name, args.workflow_path).await?;
    let run_id = create_run(&db, workflow_id, workflow_env).await?;

    println!("workflow started (run id: {run_id})");

    let cancel = Arc::new(CancelState {
        cancelled: AtomicBool::new(false),
        force_kill: AtomicBool::new(false),
    });

    Ok(RunContext { db, run_id, cancel })
}

async fn send_start_run(
    ipc_write: &mut (impl AsyncWrite + Unpin),
    args: &WorkflowArgs<'_>,
) -> Result<()> {
    let message = Message::command(
        "cmd_start",
        "start_run",
        json!({
            "run_id": 0, // Node uses its own tracking; DB run_id is Rust-side only
            "workflow_path": args.workflow_path,
            "runtime_env": args.env,
            "yolo": args.yolo,
            "workflow_input": {},
        }),
    );
    write_message(ipc_write, &message).await?;
    Ok(())
}

fn install_signal_handler(cancel: &Arc<CancelState>) {
    let cancel = Arc::clone(cancel);
    tokio::spawn(async move {
        loop {
            if signal::ctrl_c().await.is_err() {
                break;
            }
            if cancel.cancelled.load(Ordering::SeqCst) {
                cancel.force_kill.store(true, Ordering::SeqCst);
                break;
            }
            cancel.cancelled.store(true, Ordering::SeqCst);
        }
    });
}

async fn drive_ipc_loop(
    ctx: &RunContext,
    ipc_write: &mut (impl AsyncWrite + Unpin),
    ipc_read: tokio_io::ReadHalf<TokioFile>,
) -> Result<RunStatus> {
    let mut lines = BufReader::new(ipc_read).lines();
    let mut cancel_sent = false;
    let mut final_status = RunStatus::Completed;

    loop {
        if ctx.cancel.force_kill.load(Ordering::SeqCst) {
            set_status(&ctx.db, ctx.run_id, RunStatus::Cancelled).await?;
            final_status = RunStatus::Cancelled;
            break;
        }

        if ctx.cancel.cancelled.load(Ordering::SeqCst) && !cancel_sent {
            send_cancel(ipc_write, ctx.run_id).await;
            cancel_sent = true;
        }

        let Some(line) = lines
            .next_line()
            .await
            .map_err(|error| anyhow!("error reading from Node runner: {error}"))?
        else {
            break;
        };

        let (loop_control, status) =
            handle_runner_line(&ctx.db, ipc_write, ctx.run_id, &line).await?;
        if let Some(status) = status {
            final_status = status;
        }
        if matches!(loop_control, LoopControl::Stop) {
            break;
        }
    }

    Ok(final_status)
}

#[must_use]
pub fn js_path(project_root: &Path) -> PathBuf {
    project_root.join(".agents/.agentctl/lib/runner.js")
}

struct NodeRunner {
    child: Child,
    /// Bidirectional Node IPC channel (`NODE_CHANNEL_FD=3`).
    /// Taken by `run_workflow` after spawn; `None` once consumed.
    ipc: Option<TokioFile>,
}

fn spawn_runner(project_root: &Path) -> Result<NodeRunner> {
    let node_bin = resolve_node_bin()?;
    let runner_path = js_path(project_root);

    let (parent_ipc_fd, child_ipc_fd) = make_socketpair()?;
    let child_ipc_raw = child_ipc_fd.as_raw_fd();

    let mut command = Command::new(node_bin);
    command
        .arg(runner_path)
        .current_dir(project_root)
        .env("NODE_CHANNEL_FD", "3")
        .env("NODE_CHANNEL_SERIALIZATION_MODE", "json")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    // SAFETY: pre_exec runs after fork, before exec. child_ipc_raw is valid and
    // dup2/close are async-signal-safe.
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(child_ipc_raw, 3) == -1 {
                return Err(IoError::last_os_error());
            }
            libc::close(child_ipc_raw);
            Ok(())
        });
    }

    let child = command
        .spawn()
        .map_err(|error| anyhow!("failed to spawn Node runtime: {error}"))?;

    drop(child_ipc_fd);

    // SAFETY: parent_ipc_fd is a valid open file descriptor produced by
    // make_socketpair; we consume it via into_raw_fd so it is not double-closed.
    let ipc = TokioFile::from_std(unsafe { File::from_raw_fd(parent_ipc_fd.into_raw_fd()) });

    Ok(NodeRunner {
        child,
        ipc: Some(ipc),
    })
}

fn make_socketpair() -> Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0i32; 2];
    // SAFETY: fds is a valid 2-element array; socketpair fills it.
    let ret = unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_CLOEXEC,
            0,
            fds.as_mut_ptr(),
        )
    };
    if ret == -1 {
        return Err(anyhow!("socketpair failed: {}", IoError::last_os_error()));
    }
    // SAFETY: socketpair succeeded; fds[0] and fds[1] are valid open file descriptors.
    let left = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    // SAFETY: socketpair succeeded; fds[0] and fds[1] are valid open file descriptors.
    let right = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    Ok((left, right))
}

async fn handle_runner_line(
    db: &DatabaseConnection,
    ipc_write: &mut (impl AsyncWrite + Unpin),
    run_id: i64,
    line: &str,
) -> Result<(LoopControl, Option<RunStatus>)> {
    let trimmed_line = line.trim();
    if trimmed_line.is_empty() {
        return Ok((LoopControl::Continue, None));
    }

    let message: Message = match serde_json::from_str(trimmed_line) {
        Ok(message) => message,
        Err(error) => {
            append_run_event(
                db,
                run_id,
                "ipc_parse_error",
                json!({ "error": error.to_string() }),
            )
            .await?;
            return Ok((LoopControl::Continue, None));
        }
    };

    match message.kind.as_str() {
        "request" => {
            let (capability, params, request_id) =
                parse_capability_request_payload(&message.payload)?;
            let request = CapabilityRequest { capability, params };
            let response = dispatch_capability(request_id, &request);
            if let Err(error) = write_message(ipc_write, &response).await
                && error.kind() != ErrorKind::BrokenPipe
            {
                return Err(anyhow!("failed to write capability response: {error}"));
            }
            Ok((LoopControl::Continue, None))
        }
        "event" => {
            append_run_event(db, run_id, &message.name, message.payload.clone()).await?;
            update_run_status_for_event(db, run_id, &message).await
        }
        _ => Ok((LoopControl::Continue, None)),
    }
}

async fn update_run_status_for_event(
    db: &DatabaseConnection,
    run_id: i64,
    message: &Message,
) -> Result<(LoopControl, Option<RunStatus>)> {
    match message.name.as_str() {
        "run_started" => {
            set_status(db, run_id, RunStatus::Running).await?;
            Ok((LoopControl::Continue, Some(RunStatus::Running)))
        }
        "run_finished" => {
            let status = run_finished_status(message);
            set_status(db, run_id, status.clone()).await?;
            Ok((LoopControl::Stop, Some(status)))
        }
        "run_failed" => {
            set_status(db, run_id, RunStatus::Failed).await?;
            Ok((LoopControl::Stop, Some(RunStatus::Failed)))
        }
        _ => Ok((LoopControl::Continue, None)),
    }
}

fn parse_runtime_env(env: &str) -> Result<WorkflowEnv> {
    match env {
        "host" => Ok(WorkflowEnv::Host),
        "container" => Ok(WorkflowEnv::Container),
        other => Err(anyhow!("unknown runtime env: {other}")),
    }
}

fn run_finished_status(message: &Message) -> RunStatus {
    match message
        .payload
        .get("status")
        .and_then(|value| value.as_str())
    {
        Some("CANCELLED") => RunStatus::Cancelled,
        _ => RunStatus::Completed,
    }
}

async fn wait_for_child(child: &mut Child, cancel: &Arc<CancelState>) -> Result<()> {
    let deadline = Instant::now() + SHUTDOWN_GRACE_PERIOD;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code().unwrap_or(-1);
                // SIGINT-driven exits surface as 130; treat as controlled cancellation.
                if code != 0 && code != 130 {
                    return Err(anyhow!("Node runner exited with status {code}"));
                }
                return Ok(());
            }
            Ok(None) => {}
            Err(error) => return Err(anyhow!("error waiting for Node runner: {error}")),
        }

        if cancel.force_kill.load(Ordering::SeqCst) || Instant::now() >= deadline {
            let _ = child.kill().await;
            return Ok(());
        }

        sleep(Duration::from_millis(50)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_capability_request_payload_reads_nested_fields() {
        let payload = json!({
            "request_id": "req_123",
            "capability": "session_run",
            "params": { "prompt": "hello" }
        });

        let (capability, params, request_id) = parse_capability_request_payload(&payload).unwrap();

        assert_eq!(capability, "session_run");
        assert_eq!(request_id, "req_123");
        assert_eq!(params["prompt"], "hello");
    }

    #[test]
    fn parse_capability_request_payload_requires_request_id() {
        let payload = json!({
            "capability": "session_run",
            "params": { "prompt": "hello" }
        });

        let error = parse_capability_request_payload(&payload).unwrap_err();

        assert!(error.to_string().contains("payload.request_id"));
    }

    #[test]
    fn run_finished_status_returns_cancelled_for_cancelled_payload() {
        let message = Message::command("evt_1", "run_finished", json!({ "status": "CANCELLED" }));

        assert!(matches!(
            run_finished_status(&message),
            RunStatus::Cancelled
        ));
    }

    #[test]
    fn run_finished_status_defaults_to_completed() {
        let message = Message::command("evt_1", "run_finished", json!({}));

        assert!(matches!(
            run_finished_status(&message),
            RunStatus::Completed
        ));
    }

    #[test]
    fn parse_runtime_env_accepts_valid_values() {
        assert!(matches!(
            parse_runtime_env("host").unwrap(),
            WorkflowEnv::Host
        ));
        assert!(matches!(
            parse_runtime_env("container").unwrap(),
            WorkflowEnv::Container
        ));
    }

    #[test]
    fn parse_runtime_env_rejects_unknown() {
        let error = parse_runtime_env("cloud").unwrap_err();

        assert!(error.to_string().contains("unknown runtime env"));
    }
}
