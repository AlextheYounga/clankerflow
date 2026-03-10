use std::io::ErrorKind;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use anyhow::{Result, anyhow};
use sea_orm::DatabaseConnection;
use serde_json::json;
use tokio::io::{self as tokio_io, AsyncBufReadExt, AsyncWrite, BufReader};
use tokio::net::TcpStream;

use crate::core::capabilities::{CapabilityRequest, dispatch as dispatch_capability};
use crate::core::ipc::Message;
use crate::db::entities::workflow_run::RunStatus;

use super::WorkflowArgs;
use super::protocol::{LoopControl, parse_capability_request_payload, send_cancel, write_message};
use super::signal::CancelState;
use super::store::{append_run_event, create_workflow_session, set_status};

pub struct IpcLoopContext {
    pub db: DatabaseConnection,
    pub run_id: i64,
    pub cancel: Arc<CancelState>,
    pub server_url: String,
}

pub async fn send_start_run(
    ipc_write: &mut (impl AsyncWrite + Unpin),
    args: &WorkflowArgs<'_>,
) -> Result<()> {
    let message = Message::command(
        "cmd_start",
        "start_run",
        json!({
            "run_id": 0, // Node uses its own tracking; DB run_id is Rust-side only
            "workflow_path": args.workflow_path,
            "runtime_env": args.env.as_str(),
            "yolo": args.yolo,
            "workflow_input": {},
        }),
    );
    write_message(ipc_write, &message).await?;
    Ok(())
}

pub async fn drive_ipc_loop(
    ctx: &IpcLoopContext,
    ipc_write: &mut (impl AsyncWrite + Unpin),
    ipc_read: tokio_io::ReadHalf<TcpStream>,
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

        let (loop_control, status) = handle_runner_line(ctx, ipc_write, &line).await?;
        if let Some(status) = status {
            final_status = status;
        }
        if matches!(loop_control, LoopControl::Stop) {
            break;
        }
    }

    Ok(final_status)
}

async fn handle_runner_line(
    ctx: &IpcLoopContext,
    ipc_write: &mut (impl AsyncWrite + Unpin),
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
                &ctx.db,
                ctx.run_id,
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
            let response = dispatch_capability(request_id, &request, &ctx.server_url);
            if capability == "session_run"
                && let Some(session_id) =
                    response.payload.get("session_id").and_then(|v| v.as_str())
            {
                create_workflow_session(&ctx.db, ctx.run_id, session_id).await?;
            }
            if let Err(error) = write_message(ipc_write, &response).await
                && error.kind() != ErrorKind::BrokenPipe
            {
                return Err(anyhow!("failed to write capability response: {error}"));
            }
            Ok((LoopControl::Continue, None))
        }
        "event" => {
            append_run_event(&ctx.db, ctx.run_id, &message.name, message.payload.clone()).await?;
            update_run_status_for_event(&ctx.db, ctx.run_id, &message).await
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
}
