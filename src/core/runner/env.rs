use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::{env, ffi::OsString};

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
        .args(["-e", &format!("AGENTKATA_IPC_PORT={port}")])
        .args(["-e", "AGENTKATA_CONTAINER=1"])
        .arg(&container_id)
        .args([tsx_bin, "--tsconfig", tsconfig_path, runner_path])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow!("failed to exec in container: {e}"))
}

pub fn spawn_host_runner(project_root: &Path, port: u16) -> Result<Child> {
    let executable = host_runner_executable(project_root);
    let runner_path = runner_entry_path(project_root);
    let tsconfig_path = project_root.join(".agents/tsconfig.json");

    let mut command = Command::new(executable.program);
    command.args(executable.args);

    if let Some(runner_path) = runner_path {
        command
            .args(["--tsconfig", tsconfig_path.to_str().unwrap_or(".")])
            .arg(runner_path);
    }

    command
        .current_dir(project_root)
        .env("AGENTKATA_IPC_PORT", port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow!("failed to spawn tsx runtime: {e}"))
}

struct HostRunnerExecutable {
    program: OsString,
    args: Vec<OsString>,
}

fn host_runner_executable(project_root: &Path) -> HostRunnerExecutable {
    if let Ok(bundle_path) = env::var("AGENTKATA_HOST_RUNNER_BUNDLE") {
        return HostRunnerExecutable {
            program: OsString::from("node"),
            args: vec![OsString::from(bundle_path)],
        };
    }

    HostRunnerExecutable {
        program: tsx_bin_path(project_root).into_os_string(),
        args: Vec::new(),
    }
}

fn runner_entry_path(project_root: &Path) -> Option<PathBuf> {
    if env::var_os("AGENTKATA_HOST_RUNNER_BUNDLE").is_some() {
        return None;
    }

    Some(runner_ts_path(project_root))
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
