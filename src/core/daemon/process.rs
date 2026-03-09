use std::process::Command;
use std::time::Duration;

use anyhow::{Result, anyhow};
use tokio::process::Child;
use tokio::time::{Instant, sleep};

const CANCEL_GRACE_PERIOD: Duration = Duration::from_secs(5);

pub async fn wait_for_child(mut child: Child) -> Result<()> {
    let deadline = Instant::now() + CANCEL_GRACE_PERIOD;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code().unwrap_or(-1);
                // SIGINT-driven exits often surface as 130; treat that as a
                // controlled cancellation path instead of a hard runtime error.
                if code != 0 && code != 130 {
                    return Err(anyhow!("Node runner exited with status {code}"));
                }
                return Ok(());
            }
            Ok(None) => {}
            Err(error) => return Err(anyhow!("error waiting for Node runner: {error}")),
        }

        if Instant::now() >= deadline {
            // We only force-kill after a grace period so workflow cleanup hooks
            // can run on cancellation, but never block daemon shutdown forever.
            let _ = child.kill().await;
            return Ok(());
        }

        sleep(Duration::from_millis(50)).await;
    }
}

#[cfg(target_family = "unix")]
pub fn detach_process(worker: &mut Command) {
    use std::os::unix::process::CommandExt;

    // SAFETY: setsid() is async-signal-safe and has no preconditions beyond
    // running in a forked child process, which pre_exec guarantees.
    unsafe {
        worker.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
}

#[cfg(not(target_family = "unix"))]
pub fn detach_process(worker: &mut Command) {
    use std::os::windows::process::CommandExt;

    worker.creation_flags(0x00000008);
}
