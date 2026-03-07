use std::time::Duration;

use anyhow::{Result, anyhow};

const CANCEL_GRACE_PERIOD: Duration = Duration::from_secs(5);

pub async fn wait_for_child(mut child: tokio::process::Child) -> Result<()> {
    let deadline = tokio::time::Instant::now() + CANCEL_GRACE_PERIOD;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code().unwrap_or(-1);
                if code != 0 && code != 130 {
                    return Err(anyhow!("Node runner exited with status {code}"));
                }
                return Ok(());
            }
            Ok(None) => {}
            Err(error) => return Err(anyhow!("error waiting for Node runner: {error}")),
        }

        if tokio::time::Instant::now() >= deadline {
            let _ = child.kill().await;
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[cfg(target_family = "unix")]
pub fn detach_process(worker: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;

    unsafe {
        worker.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
}

#[cfg(not(target_family = "unix"))]
pub fn detach_process(worker: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;

    worker.creation_flags(0x00000008);
}
