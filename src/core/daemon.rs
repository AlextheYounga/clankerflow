mod process;
mod protocol;
mod store;

use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Result, anyhow};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWrite, BufReader};
use tokio::process::Command;

use crate::core::capabilities::{CapabilityRequest, dispatch as dispatch_capability};
use crate::core::ipc::IpcMessage;
use crate::core::runtime::resolve_node_bin;
use crate::db::db::connect;
use crate::db::entities::workflow_run::WorkflowRunStatus;

use process::{detach_process, wait_for_child};
use protocol::{
    LoopControl, parse_capability_request_payload, send_cancel, send_shutdown, write_message,
};
use store::{append_run_event, create_run, is_stop_requested, set_status, upsert_workflow};

struct NodeRunner {
    child: tokio::process::Child,
    /// Bidirectional Node IPC channel (NODE_CHANNEL_FD=3)
    ipc: tokio::fs::File,
}

pub async fn launch_workflow(
    project_root: &Path,
    workflow_name: &str,
    workflow_path: &Path,
    env: &str,
    yolo: bool,
) -> Result<i64> {
    let db = connect().await?;
    let workflow_env = parse_runtime_env(env)?;
    let workflow_id = upsert_workflow(&db, workflow_name, workflow_path).await?;
    let run_id = create_run(&db, workflow_id, workflow_env).await?;

    let executable = std::env::current_exe()
        .map_err(|error| anyhow!("failed to resolve executable path: {error}"))?;

    let mut worker = std::process::Command::new(&executable);
    worker
        .arg("_run")
        .arg("--run-id")
        .arg(run_id.to_string())
        .arg("--workflow-path")
        .arg(workflow_path)
        .arg("--env")
        .arg(env)
        .arg("--project-root")
        .arg(project_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if yolo {
        worker.arg("--yolo");
    }

    detach_process(&mut worker);
    worker
        .spawn()
        .map_err(|error| anyhow!("failed to spawn workflow worker: {error}"))?;

    Ok(run_id)
}

pub async fn pump_workflow(
    project_root: &Path,
    run_id: i64,
    workflow_path: &Path,
    env: &str,
    yolo: bool,
) -> Result<()> {
    let db = connect().await?;
    let NodeRunner { child, ipc } = spawn_runner(project_root)?;
    let (ipc_read, mut ipc_write) = tokio::io::split(ipc);

    let start_run_message = IpcMessage::command(
        "cmd_start",
        "start_run",
        json!({
            "run_id": run_id,
            "workflow_path": workflow_path,
            "runtime_env": env,
            "yolo": yolo,
            "workflow_input": {},
        }),
    );
    write_message(&mut ipc_write, &start_run_message).await?;

    let mut lines = BufReader::new(ipc_read).lines();
    loop {
        if is_stop_requested(&db, run_id).await? {
            send_cancel(&mut ipc_write).await;
            set_status(&db, run_id, WorkflowRunStatus::Cancelled).await?;
        }

        let Some(line) = lines
            .next_line()
            .await
            .map_err(|error| anyhow!("error reading from Node runner: {error}"))?
        else {
            break;
        };

        let loop_control = handle_runner_line(&db, &mut ipc_write, run_id, &line).await?;
        if matches!(loop_control, LoopControl::Stop) {
            break;
        }
    }

    send_shutdown(&mut ipc_write).await;
    drop(ipc_write);
    wait_for_child(child).await
}

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
                return Err(std::io::Error::last_os_error());
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

    let ipc = tokio::fs::File::from_std(unsafe {
        std::fs::File::from_raw_fd(parent_ipc_fd.into_raw_fd())
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
        return Err(anyhow!(
            "socketpair failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: socketpair succeeded; fds are valid open file descriptors.
    let left = unsafe { OwnedFd::from_raw_fd(fds[0]) };
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

    let message: IpcMessage = match serde_json::from_str(trimmed_line) {
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
            let response = dispatch_capability(request_id, request);
            if let Err(error) = write_message(ipc_write, &response).await {
                use std::io::ErrorKind;
                if error.kind() != ErrorKind::BrokenPipe {
                    return Err(anyhow!("failed to write capability response: {error}"));
                }
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
    message: &IpcMessage,
) -> Result<LoopControl> {
    match message.name.as_str() {
        "run_started" => {
            set_status(db, run_id, WorkflowRunStatus::Running).await?;
            Ok(LoopControl::Continue)
        }
        "run_finished" => {
            set_status(db, run_id, run_finished_status(message)).await?;
            Ok(LoopControl::Stop)
        }
        "run_failed" => {
            set_status(db, run_id, WorkflowRunStatus::Failed).await?;
            Ok(LoopControl::Stop)
        }
        _ => Ok(LoopControl::Continue),
    }
}

fn parse_runtime_env(env: &str) -> Result<crate::db::entities::workflow_run::WorkflowEnv> {
    match env {
        "host" => Ok(crate::db::entities::workflow_run::WorkflowEnv::Host),
        "container" => Ok(crate::db::entities::workflow_run::WorkflowEnv::Container),
        other => Err(anyhow!("unknown runtime env: {other}")),
    }
}

fn run_finished_status(message: &IpcMessage) -> WorkflowRunStatus {
    match message
        .payload
        .get("status")
        .and_then(|value| value.as_str())
    {
        Some("CANCELLED") => WorkflowRunStatus::Cancelled,
        _ => WorkflowRunStatus::Completed,
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
