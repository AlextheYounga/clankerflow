use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Result, anyhow};

pub struct Docker;

impl Docker {
    /// Returns `true` if `docker compose` is available on `PATH`.
    #[must_use]
    pub fn is_available() -> bool {
        Command::new("docker")
            .args(["compose", "version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    /// Build the agent container image.
    ///
    /// # Errors
    /// Returns an error if `docker compose build` fails.
    pub fn build(project_root: &Path, codebase_id: &str) -> Result<()> {
        let status = compose_command(project_root)
            .env("CODEBASE_ID", codebase_id)
            .args(["build"])
            .status()
            .map_err(|e| anyhow!("failed to run docker compose build: {e}"))?;

        if !status.success() {
            return Err(anyhow!("docker compose build failed"));
        }
        Ok(())
    }

    /// Start the agent container in detached mode.
    ///
    /// Ensures `~/.config/opencode` exists first to prevent Docker from
    /// creating it as a root-owned directory.
    ///
    /// # Errors
    /// Returns an error if `docker compose up` fails.
    pub fn up(project_root: &Path, codebase_id: &str) -> Result<()> {
        ensure_opencode_config_dir();

        let status = compose_command(project_root)
            .env("CODEBASE_ID", codebase_id)
            .args(["up", "-d"])
            .status()
            .map_err(|e| anyhow!("failed to run docker compose up: {e}"))?;

        if !status.success() {
            return Err(anyhow!("docker compose up failed"));
        }
        Ok(())
    }

    /// Stop and remove the agent container.
    ///
    /// # Errors
    /// Returns an error if `docker compose down` fails.
    pub fn down(project_root: &Path, codebase_id: &str) -> Result<()> {
        let status = compose_command(project_root)
            .env("CODEBASE_ID", codebase_id)
            .args(["down"])
            .status()
            .map_err(|e| anyhow!("failed to run docker compose down: {e}"))?;

        if !status.success() {
            return Err(anyhow!("docker compose down failed"));
        }
        Ok(())
    }

    /// Check whether the agent container is currently running.
    ///
    /// # Errors
    /// Returns an error if the `docker compose ps` command fails to execute.
    pub fn is_running(project_root: &Path, codebase_id: &str) -> Result<bool> {
        let output = compose_command(project_root)
            .env("CODEBASE_ID", codebase_id)
            .args(["ps", "-q", "--status=running"])
            .output()
            .map_err(|e| anyhow!("failed to run docker compose ps: {e}"))?;

        Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
    }

    /// Return the container ID of the running agent service.
    ///
    /// # Errors
    /// Returns an error if no running container is found.
    pub fn get_container_id(project_root: &Path, codebase_id: &str) -> Result<String> {
        let output = compose_command(project_root)
            .env("CODEBASE_ID", codebase_id)
            .args(["ps", "-q", "--status=running", "agent"])
            .output()
            .map_err(|e| anyhow!("failed to get container id: {e}"))?;

        let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if id.is_empty() {
            return Err(anyhow!("no running agent container found"));
        }
        Ok(id)
    }

    /// Idempotently ensure the container is running. Builds and starts if
    /// needed, then returns the container ID.
    ///
    /// # Errors
    /// Returns an error if build, start, or container ID retrieval fails.
    pub fn ensure_running(project_root: &Path, codebase_id: &str) -> Result<String> {
        if !Self::is_running(project_root, codebase_id)? {
            Self::build(project_root, codebase_id)?;
            Self::up(project_root, codebase_id)?;
        }
        Self::get_container_id(project_root, codebase_id)
    }
}

fn compose_file_path(project_root: &Path) -> PathBuf {
    project_root.join(".agents/.agentctl/docker/agent.docker-compose.yaml")
}

fn compose_args(project_root: &Path) -> Vec<String> {
    vec![
        "-f".to_string(),
        compose_file_path(project_root)
            .to_string_lossy()
            .to_string(),
    ]
}

fn compose_command(project_root: &Path) -> Command {
    let mut cmd = Command::new("docker");
    cmd.args(["compose"]);
    cmd.args(compose_args(project_root));
    cmd
}

fn ensure_opencode_config_dir() {
    if let Some(home) = env::var_os("HOME") {
        let config_dir = PathBuf::from(home).join(".config/opencode");
        let _ = fs::create_dir_all(config_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn compose_file_path_returns_correct_path() {
        let root = Path::new("/home/user/project");

        let path = compose_file_path(root);

        assert_eq!(
            path,
            PathBuf::from("/home/user/project/.agents/.agentctl/docker/agent.docker-compose.yaml")
        );
    }

    #[test]
    fn compose_args_contains_file_flag_and_path() {
        let root = Path::new("/home/user/project");

        let args = compose_args(root);

        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "-f");
        assert!(args[1].ends_with("agent.docker-compose.yaml"));
    }
}
