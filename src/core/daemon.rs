mod process;
mod protocol;
mod store;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, BufReader};
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

pub async fn launch_workflow(
    project_root: &Path,
    workflow_name: &str,
    workflow_path: &Path,
    env: &str,
    yolo: bool,
) -> Result<String> {
    let db = connect().await?;
    let run_id = new_run_id();
    let workflow_env = parse_runtime_env(env)?;
    let workflow_id = upsert_workflow(&db, workflow_name, workflow_path).await?;
    create_run(&db, &run_id, workflow_id, workflow_env).await?;

    let executable = std::env::current_exe()
        .map_err(|error| anyhow!("failed to resolve executable path: {error}"))?;

    let mut worker = std::process::Command::new(&executable);
    worker
        .arg("_run")
        .arg("--run-id")
        .arg(&run_id)
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
    run_id: &str,
    workflow_path: &Path,
    env: &str,
    yolo: bool,
) -> Result<()> {
    let db = connect().await?;
    let mut child = spawn_runner(project_root).await?;
    let mut runner_stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("Node runner stdin unavailable"))?;
    let runner_stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Node runner stdout unavailable"))?;

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
    write_message(&mut runner_stdin, &start_run_message).await?;

    let mut lines = BufReader::new(runner_stdout).lines();
    loop {
        if is_stop_requested(&db, run_id).await? {
            send_cancel(&mut runner_stdin, run_id).await;
            set_status(&db, run_id, WorkflowRunStatus::Cancelled).await?;
        }

        let Some(line) = lines
            .next_line()
            .await
            .map_err(|error| anyhow!("error reading from Node runner: {error}"))?
        else {
            break;
        };

        let loop_control = handle_runner_line(&db, &mut runner_stdin, run_id, &line).await?;
        if matches!(loop_control, LoopControl::Stop) {
            break;
        }
    }

    send_shutdown(&mut runner_stdin).await;
    drop(runner_stdin);
    wait_for_child(child).await
}

pub fn runner_js_path(project_root: &Path) -> PathBuf {
    project_root.join(".agents/.agentctl/lib/runner.js")
}

async fn spawn_runner(project_root: &Path) -> Result<tokio::process::Child> {
    let node_bin = resolve_node_bin()?;
    let runner_path = runner_js_path(project_root);

    Command::new(node_bin)
        .arg(runner_path)
        .current_dir(project_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| anyhow!("failed to spawn Node runtime: {error}"))
}

async fn handle_runner_line(
    db: &sea_orm::DatabaseConnection,
    runner_stdin: &mut tokio::process::ChildStdin,
    run_id: &str,
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
            if let Err(error) = write_message(runner_stdin, &response).await {
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
    run_id: &str,
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

fn new_run_id() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("run_{ms}")
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
            "capability": "agent_run",
            "params": { "prompt": "hello" }
        });

        let (capability, params, request_id) = parse_capability_request_payload(&payload).unwrap();

        assert_eq!(capability, "agent_run");
        assert_eq!(request_id, "req_123");
        assert_eq!(params["prompt"], "hello");
    }

    #[test]
    fn parse_capability_request_payload_requires_request_id() {
        let payload = json!({
            "capability": "agent_run",
            "params": { "prompt": "hello" }
        });

        let error = parse_capability_request_payload(&payload).unwrap_err();

        assert!(error.to_string().contains("payload.request_id"));
    }
}
