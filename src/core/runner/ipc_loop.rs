use std::sync::Arc;
use std::sync::atomic::Ordering;

use anyhow::{Result, anyhow};
use sea_orm::DatabaseConnection;
use serde_json::json;
use tokio::io::{self as tokio_io, AsyncBufReadExt, AsyncWrite, BufReader};
use tokio::net::TcpStream;

use crate::core::ipc::Message;
use crate::db::entities::workflow_run::RunStatus;

use super::WorkflowArgs;
use super::protocol::{LoopControl, send_cancel, write_message};
use super::signal::CancelState;
use super::store::{append_run_event, create_workflow_session, set_status};

pub struct IpcLoopContext {
    pub db: DatabaseConnection,
    pub run_id: i64,
    pub cancel: Arc<CancelState>,
}

pub async fn send_start_run(
    ipc_write: &mut (impl AsyncWrite + Unpin),
    args: &WorkflowArgs<'_>,
    run_id: i64,
) -> Result<()> {
    let message = Message::command(
        "cmd_start",
        "start_run",
        json!({
            "run_id": run_id,
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

        let (loop_control, status) = handle_runner_line(ctx, &line).await?;
        if let Some(status) = status {
            final_status = status;
        }
        if matches!(loop_control, LoopControl::Stop) {
            break;
        }
    }

    Ok(final_status)
}

pub async fn handle_runner_line(
    ctx: &IpcLoopContext,
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
        "event" => {
            append_run_event(&ctx.db, ctx.run_id, &message.name, message.payload.clone()).await?;
            persist_session_from_event(&ctx.db, ctx.run_id, &message).await?;
            update_run_status_for_event(&ctx.db, ctx.run_id, &message).await
        }
        _ => Ok((LoopControl::Continue, None)),
    }
}

async fn persist_session_from_event(
    db: &DatabaseConnection,
    run_id: i64,
    message: &Message,
) -> Result<()> {
    if let Some(session_id) = session_id_from_event(message) {
        create_workflow_session(db, run_id, session_id).await?;
    }

    Ok(())
}

fn session_id_from_event<'a>(message: &'a Message) -> Option<&'a str> {
    if message.name != "agent_session_started" {
        return None;
    }

    message.payload.get("session_id").and_then(|v| v.as_str())
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
    fn session_id_from_event_reads_agent_session_started_payload() {
        let message = Message::command(
            "evt_1",
            "agent_session_started",
            json!({ "session_id": "sess_abc" }),
        );

        let session_id = session_id_from_event(&message);

        assert_eq!(session_id, Some("sess_abc"));
    }

    #[test]
    fn session_id_from_event_ignores_non_agent_events() {
        let message = Message::command("evt_1", "run_started", json!({ "session_id": "sess_abc" }));

        let session_id = session_id_from_event(&message);

        assert_eq!(session_id, None);
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
