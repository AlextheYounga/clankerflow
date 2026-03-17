use std::io::{Error as IoError, Result as IoResult};

use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::core::ipc::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopControl {
    Continue,
    Stop,
}

/// Serialize and write a single IPC message as one newline-delimited JSON line.
///
/// # Errors
/// Returns an error if serialization fails or if writing to the IPC stream fails.
pub async fn write_message(
    ipc_write: &mut (impl AsyncWrite + Unpin),
    message: &Message,
) -> IoResult<()> {
    let line = serde_json::to_string(message).map_err(|error| IoError::other(error.to_string()))?;
    ipc_write.write_all(line.as_bytes()).await?;
    ipc_write.write_all(b"\n").await?;
    Ok(())
}

pub async fn send_cancel(ipc_write: &mut (impl AsyncWrite + Unpin), run_id: i64) {
    let message = Message::command(
        "cmd_cancel",
        "cancel_run",
        serde_json::json!({ "run_id": run_id, "reason": "user_requested" }),
    );
    // Cancellation is best-effort during teardown; broken pipes are expected
    // when the child has already exited.
    let _ = write_message(ipc_write, &message).await;
}

pub async fn send_shutdown(ipc_write: &mut (impl AsyncWrite + Unpin)) {
    let message = Message::command(
        "cmd_shutdown",
        "shutdown",
        serde_json::json!({ "reason": "run_complete" }),
    );
    // Shutdown is a courtesy signal so Node can drain in-flight work before
    // the parent enforces the grace-period kill.
    let _ = write_message(ipc_write, &message).await;
}
