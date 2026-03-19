use std::io::ErrorKind;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Result, anyhow};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::time::{Instant, sleep, timeout};

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 4096;
pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:4096";

const PROBE_TIMEOUT: Duration = Duration::from_millis(400);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Ensure an `OpenCode` server is reachable at the default local address.
///
/// If nothing is listening, this starts `opencode serve` in the background and
/// waits until the socket is reachable.
///
/// # Errors
/// Returns an error when the `opencode` binary is missing, fails to start, or
/// does not become reachable before timeout.
pub async fn ensure_running() -> Result<()> {
    if is_reachable().await {
        return Ok(());
    }

    let mut child = spawn_server()?;
    wait_until_reachable(&mut child).await
}

async fn wait_until_reachable(child: &mut Child) -> Result<()> {
    let deadline = Instant::now() + STARTUP_TIMEOUT;

    while Instant::now() < deadline {
        if is_reachable().await {
            return Ok(());
        }

        if let Some(status) = child.try_wait()? {
            return Err(anyhow!(
                "OpenCode server exited before becoming ready (status: {status})"
            ));
        }

        sleep(STARTUP_POLL_INTERVAL).await;
    }

    Err(anyhow!(
        "timed out waiting for OpenCode server at {DEFAULT_BASE_URL}"
    ))
}

fn spawn_server() -> Result<Child> {
    Command::new("opencode")
        .args(serve_args())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(map_spawn_error)
}

fn serve_args() -> [String; 5] {
    [
        "serve".to_string(),
        "--hostname".to_string(),
        DEFAULT_HOST.to_string(),
        "--port".to_string(),
        DEFAULT_PORT.to_string(),
    ]
}

fn map_spawn_error(error: std::io::Error) -> anyhow::Error {
    if error.kind() == ErrorKind::NotFound {
        anyhow!("`opencode` binary not found on PATH; install OpenCode CLI to run workflows")
    } else {
        anyhow!("failed to start OpenCode server: {error}")
    }
}

async fn is_reachable() -> bool {
    let target = (DEFAULT_HOST, DEFAULT_PORT);
    matches!(
        timeout(PROBE_TIMEOUT, TcpStream::connect(target)).await,
        Ok(Ok(_))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serve_args_include_default_host_and_port() {
        let args = serve_args();

        assert_eq!(args[0], "serve");
        assert_eq!(args[1], "--hostname");
        assert_eq!(args[2], DEFAULT_HOST);
        assert_eq!(args[3], "--port");
        assert_eq!(args[4], DEFAULT_PORT.to_string());
    }

    #[test]
    fn map_spawn_error_reports_missing_binary_clearly() {
        let missing = std::io::Error::from(ErrorKind::NotFound);
        let error = map_spawn_error(missing);

        assert!(error.to_string().contains("binary not found"));
    }
}
