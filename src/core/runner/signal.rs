use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow};
use tokio::process::Child;
use tokio::signal;
use tokio::time::{Instant, sleep};

pub const SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_secs(5);

/// Shared cancellation state between the SIGINT handler and the IPC loop.
pub struct CancelState {
    pub cancelled: AtomicBool,
    pub force_kill: AtomicBool,
}

/// Spawns a background task that listens for Ctrl+C (SIGINT). First signal
/// sets `cancelled` so the IPC loop can request a graceful shutdown from Node.
/// Second signal sets `force_kill` to hard-kill the child process immediately.
pub fn install_signal_handler(cancel: &Arc<CancelState>) {
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

pub async fn wait_for_child(child: &mut Child, cancel: &Arc<CancelState>) -> Result<()> {
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
