use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Result, anyhow};
use tokio::process::{Child, Command};

use crate::app::types::RuntimeEnv;
use crate::core::docker::Docker;
use crate::core::runtime::resolve_node_bin;
use crate::db::entities::workflow_run::WorkflowEnv;

pub fn parse_runtime_env(env: RuntimeEnv) -> WorkflowEnv {
    match env {
        RuntimeEnv::Host => WorkflowEnv::Host,
        RuntimeEnv::Container => WorkflowEnv::Container,
    }
}

pub fn spawn_container_runner(project_root: &Path, codebase_id: &str, port: u16) -> Result<Child> {
    let container_id = Docker::ensure_running(project_root, codebase_id)?;
    let runner_path = "/workspace/.agents/.agentctl/lib/runner.js";

    Command::new("docker")
        .args(["exec"])
        .args(["-e", &format!("AGENTCTL_IPC_PORT={port}")])
        .args(["-e", "AGENTCTL_CONTAINER=1"])
        .arg(&container_id)
        .args(["node", runner_path])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow!("failed to exec in container: {e}"))
}

pub fn spawn_host_runner(project_root: &Path, port: u16) -> Result<Child> {
    let node_bin = resolve_node_bin()?;
    let runner_path = js_path(project_root);

    Command::new(node_bin)
        .arg(runner_path)
        .current_dir(project_root)
        .env("AGENTCTL_IPC_PORT", port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow!("failed to spawn Node runtime: {e}"))
}

#[must_use]
fn js_path(project_root: &Path) -> PathBuf {
    project_root.join(".agents/.agentctl/lib/runner.js")
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
