mod process;
mod protocol;
mod store;

use std::env;
use std::fs::File;
use std::io::{Error as IoError, ErrorKind};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};

use anyhow::{Result, anyhow};
use serde_json::json;
use tokio::fs::File as TokioFile;
use tokio::io::{self as tokio_io, AsyncBufReadExt, AsyncWrite, BufReader};
use tokio::process::{Child, Command};

use crate::core::capabilities::{CapabilityRequest, dispatch as dispatch_capability};
use crate::core::ipc::Message;
use crate::core::runtime::resolve_node_bin;
use crate::db::connection::connect;
use crate::db::entities::workflow_run::{WorkflowEnv, RunStatus};

use process::{detach_process, wait_for_child};
use protocol::{
    LoopControl, parse_capability_request_payload, send_cancel, send_shutdown, write_message,
};
use store::{append_run_event, create_run, is_stop_requested, set_status, upsert_workflow};

struct NodeRunner {
    child: Child,
    /// Bidirectional Node IPC channel (`NODE_CHANNEL_FD=3`)
    ipc: TokioFile,
}

/// Parameters for launching a new workflow run (used by `launch_workflow`).
pub struct WorkflowArgs<'a> {
    pub project_root: &'a Path,
    pub workflow_name: &'a str,
    pub workflow_path: &'a Path,
    pub env: &'a str,
    pub yolo: bool,
}

/// Parameters for pumping an already-created workflow run (used by `pump_workflow`).
pub struct PumpArgs<'a> {
    pub project_root: &'a Path,
    pub run_id: i64,
    pub workflow_path: &'a Path,
    pub env: &'a str,
    pub yolo: bool,
}

/// # Errors
/// Returns an error if the database connection fails, the workflow cannot be
/// registered, the run record cannot be created, or the worker process fails
/// to spawn.
pub async fn launch_workflow(args: &WorkflowArgs<'_>) -> Result<i64> {
    let db = connect().await?;
    let workflow_env = parse_runtime_env(args.env)?;
    let workflow_id = upsert_workflow(&db, args.workflow_name, args.workflow_path).await?;
    let run_id = create_run(&db, workflow_id, workflow_env).await?;

    let executable = env::current_exe()
        .map_err(|error| anyhow!("failed to resolve executable path: {error}"))?;

    let mut worker = StdCommand::new(&executable);
    worker
        .arg("_run")
        .arg("--run-id")
        .arg(run_id.to_string())
        .arg("--workflow-path")
        .arg(args.workflow_path)
        .arg("--env")
        .arg(args.env)
        .arg("--project-root")
        .arg(args.project_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if args.yolo {
        worker.arg("--yolo");
    }

    detach_process(&mut worker);
    worker
        .spawn()
        .map_err(|error| anyhow!("failed to spawn workflow worker: {error}"))?;

    Ok(run_id)
}

/// # Errors
/// Returns an error if the database connection fails, the Node runner cannot
/// be spawned, IPC communication fails, or the child process exits with an
/// unexpected status code.
pub async fn pump_workflow(args: &PumpArgs<'_>) -> Result<()> {
    let db = connect().await?;
    let NodeRunner { child, ipc } = spawn_runner(args.project_root)?;
    let (ipc_read, mut ipc_write) = tokio_io::split(ipc);

    let start_run_message = Message::command(
        "cmd_start",
        "start_run",
        json!({
            "run_id": args.run_id,
            "workflow_path": args.workflow_path,
            "runtime_env": args.env,
            "yolo": args.yolo,
            "workflow_input": {},
        }),
    );
    write_message(&mut ipc_write, &start_run_message).await?;

    let mut lines = BufReader::new(ipc_read).lines();
    loop {
        if is_stop_requested(&db, args.run_id).await? {
            send_cancel(&mut ipc_write, args.run_id).await;
            set_status(&db, args.run_id, RunStatus::Cancelled).await?;
        }

        let Some(line) = lines
            .next_line()
            .await
            .map_err(|error| anyhow!("error reading from Node runner: {error}"))?
        else {
            break;
        };

        let loop_control = handle_runner_line(&db, &mut ipc_write, args.run_id, &line).await?;
        if matches!(loop_control, LoopControl::Stop) {
            break;
        }
    }

    send_shutdown(&mut ipc_write).await;
    drop(ipc_write);
    wait_for_child(child).await
}

#[must_use]
pub fn runner_js_path(project_root: &Path) -> PathBuf {
    project_root.join(".agents/.agentctl/lib/runner.js")
}

fn spawn_runner(project_root: &Path) -> Result<NodeRunner> {
    let node_bin = resolve_node_bin()?;
    let runner_path = runner_js_path(project_root);

    // Node's built-in IPC channel is bound from NODE_CHANNEL_FD (fd 3).
    // We provide one bidirectional Unix socket endpoint for the child and keep
    // the other endpoint in Rust.
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
                return Err(IoError::last_os_error()); // async-signal-safe
            }
            libc::close(child_ipc_raw);
            Ok(())
        });
    }

    let child = command
        .spawn()
        .map_err(|error| anyhow!("failed to spawn Node runtime: {error}"))?;

    // Drop the child endpoint in the parent so EOF propagates correctly.
    drop(child_ipc_fd);

    // SAFETY: parent_ipc_fd is a valid open file descriptor produced by
    // make_socketpair; we consume it via into_raw_fd so it is not double-closed.
    let ipc = TokioFile::from_std(unsafe {
        File::from_raw_fd(parent_ipc_fd.into_raw_fd())
    });

    Ok(NodeRunner { child, ipc })
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
    db: &sea_orm::DatabaseConnection,
    ipc_write: &mut (impl AsyncWrite + Unpin),
    run_id: i64,
    line: &str,
) -> Result<LoopControl> {
    let trimmed_line = line.trim();
    if trimmed_line.is_empty() {
        return Ok(LoopControl::Continue);
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
            return Ok(LoopControl::Continue);
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
            Ok(LoopControl::Continue)
        }
        "event" => {
            append_run_event(db, run_id, &message.name, message.payload.clone()).await?;
            update_run_status_for_event(db, run_id, &message).await
        }
        _ => Ok(LoopControl::Continue),
    }
}

async fn update_run_status_for_event(
    db: &sea_orm::DatabaseConnection,
    run_id: i64,
    message: &Message,
) -> Result<LoopControl> {
    match message.name.as_str() {
        "run_started" => {
            set_status(db, run_id, RunStatus::Running).await?;
            Ok(LoopControl::Continue)
        }
        "run_finished" => {
            set_status(db, run_id, run_finished_status(message)).await?;
            Ok(LoopControl::Stop)
        }
        "run_failed" => {
            set_status(db, run_id, RunStatus::Failed).await?;
            Ok(LoopControl::Stop)
        }
        _ => Ok(LoopControl::Continue),
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
}
