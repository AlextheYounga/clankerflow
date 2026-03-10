use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Result, anyhow};
use tokio::process::{Child, Command};

use crate::app::types::RuntimeEnv;
use crate::core::docker::Docker;
use crate::db::entities::workflow_run::WorkflowEnv;

pub fn parse_runtime_env(env: RuntimeEnv) -> WorkflowEnv {
    match env {
        RuntimeEnv::Host => WorkflowEnv::Host,
        RuntimeEnv::Container => WorkflowEnv::Container,
    }
}

pub async fn spawn_container_runner(
    project_root: &Path,
    codebase_id: &str,
    port: u16,
) -> Result<Child> {
    let container_id = Docker::ensure_running(project_root, codebase_id).await?;
    let runner_path = "/workspace/.agents/.agentkata/lib/src/runner.ts";
    let tsconfig_path = "/workspace/.agents/tsconfig.json";
    let tsx_bin = "/workspace/.agents/.agentkata/lib/node_modules/.bin/tsx";

    Command::new("docker")
        .args(["exec"])
        .args(["-e", &format!("AGENTCTL_IPC_PORT={port}")])
        .args(["-e", "AGENTCTL_CONTAINER=1"])
        .arg(&container_id)
        .args([tsx_bin, "--tsconfig", tsconfig_path, runner_path])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow!("failed to exec in container: {e}"))
}

pub fn spawn_host_runner(project_root: &Path, port: u16) -> Result<Child> {
    let tsx_bin = tsx_bin_path(project_root);
    let runner_path = runner_ts_path(project_root);
    let tsconfig_path = project_root.join(".agents/tsconfig.json");

    Command::new(tsx_bin)
        .args(["--tsconfig", tsconfig_path.to_str().unwrap_or(".")])
        .arg(runner_path)
        .current_dir(project_root)
        .env("AGENTCTL_IPC_PORT", port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow!("failed to spawn tsx runtime: {e}"))
}

#[must_use]
fn tsx_bin_path(project_root: &Path) -> PathBuf {
    project_root.join(".agents/.agentkata/lib/node_modules/.bin/tsx")
}

#[must_use]
fn runner_ts_path(project_root: &Path) -> PathBuf {
    project_root.join(".agents/.agentkata/lib/src/runner.ts")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_runtime_env_maps_host() {
        assert!(matches!(
            parse_runtime_env(RuntimeEnv::Host),
            WorkflowEnv::Host
        ));
    }

    #[test]
    fn parse_runtime_env_maps_container() {
        assert!(matches!(
            parse_runtime_env(RuntimeEnv::Container),
            WorkflowEnv::Container
        ));
    }
}
