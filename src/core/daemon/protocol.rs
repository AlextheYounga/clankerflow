use anyhow::{Result, anyhow};
use serde_json::Value;
use tokio::io::AsyncWriteExt;

use crate::core::ipc::IpcMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopControl {
    Continue,
    Stop,
}

pub fn parse_capability_request_payload(payload: &Value) -> Result<(&str, &Value, &str)> {
    let capability = payload
        .get("capability")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("capability_request missing payload.capability"))?;
    let params = payload
        .get("params")
        .ok_or_else(|| anyhow!("capability_request missing payload.params"))?;
    let request_id = payload
        .get("request_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("capability_request missing payload.request_id"))?;

    Ok((capability, params, request_id))
}

pub async fn write_message(
    runner_stdin: &mut tokio::process::ChildStdin,
    message: &IpcMessage,
) -> std::io::Result<()> {
    let line =
        serde_json::to_string(message).map_err(|error| std::io::Error::other(error.to_string()))?;
    runner_stdin.write_all(line.as_bytes()).await?;
    runner_stdin.write_all(b"\n").await?;
    Ok(())
}

pub async fn send_cancel(runner_stdin: &mut tokio::process::ChildStdin, run_id: &str) {
    let message = IpcMessage::command(
        "cmd_cancel",
        "cancel_run",
        serde_json::json!({
            "run_id": run_id,
            "reason": "user_requested",
        }),
    );
    let _ = write_message(runner_stdin, &message).await;
}

pub async fn send_shutdown(runner_stdin: &mut tokio::process::ChildStdin) {
    let message = IpcMessage::command(
        "cmd_shutdown",
        "shutdown",
        serde_json::json!({ "reason": "run_complete" }),
    );
    let _ = write_message(runner_stdin, &message).await;
}
