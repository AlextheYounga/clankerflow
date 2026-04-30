use std::path::{Path, PathBuf};

use colored::Colorize;

use crate::app::commands::manage;
use crate::app::output;
use crate::app::types::RuntimeEnv;
use crate::core::project::require_project_root;
use crate::core::runner::{WorkflowArgs, WorkflowEngine};
use crate::db::entities::workflow_run::RunStatus;

/// # Errors
/// Returns an error if the project root is not found, the workflow path cannot
/// be resolved, or the workflow fails to run.
pub async fn run(name: String, env: RuntimeEnv, yolo: bool) -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    let workflow_path = resolve_workflow(&project_root, &name)?;

    let args =
        WorkflowArgs { project_root: &project_root, workflow_name: &name, workflow_path: &workflow_path, env, yolo };

    let launch = WorkflowEngine::launch_in_tmux(&args).await?;
    println!(
        "{} workflow '{}' (run id: {}, tmux: {}:{})",
        output::success("Launched"),
        output::path(&name),
        launch.run_id,
        output::path(&launch.project_session_name),
        output::path(&launch.run_window_name)
    );

    if let Err(error) = manage::run().await {
        eprintln!("{} failed to open clankerflow manager: {error}", output::warning("Warning"));
    }

    Ok(())
}

/// # Errors
/// Returns an error if the project root or workflow path are invalid, or if the
/// workflow worker fails while running.
pub async fn run_worker(worker: WorkerArgs) -> anyhow::Result<()> {
    let project_root = PathBuf::from(&worker.project_root);
    let workflow_path = PathBuf::from(&worker.workflow_path);

    let args = WorkflowArgs {
        project_root: &project_root,
        workflow_name: &worker.workflow_name,
        workflow_path: &workflow_path,
        env: worker.env,
        yolo: worker.yolo,
    };

    let final_status = WorkflowEngine::run_existing(&args, worker.run_id).await?;
    print_summary(&worker.workflow_name, &final_status);

    if matches!(final_status, RunStatus::Failed) {
        anyhow::bail!("workflow '{}' failed", worker.workflow_name);
    }

    Ok(())
}

pub struct WorkerArgs {
    pub run_id: i64,
    pub workflow_name: String,
    pub workflow_path: String,
    pub env: RuntimeEnv,
    pub project_root: String,
    pub yolo: bool,
}

fn print_summary(name: &str, status: &RunStatus) {
    let label = match status {
        RunStatus::Completed => output::success("completed"),
        RunStatus::Cancelled => output::warning("cancelled"),
        RunStatus::Failed => "failed".bright_red().bold().to_string(),
        RunStatus::Running => output::action("running"),
        RunStatus::Pending => output::warning("pending"),
    };
    println!("workflow '{}' {label}", output::path(&name));
}

fn resolve_workflow(project_root: &Path, name: &str) -> anyhow::Result<PathBuf> {
    validate_workflow_name(name)?;
    let workflows_dir = project_root.join(".agents").join("workflows");
    let candidate = workflows_dir.join(format!("{name}.ts"));

    if candidate.exists() {
        return Ok(candidate);
    }

    anyhow::bail!("workflow '{name}' not found under {}", workflows_dir.display())
}

fn validate_workflow_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        anyhow::bail!("workflow name cannot be empty");
    }

    if name.contains('/') || name.contains('\\') || name.contains("..") {
        anyhow::bail!("workflow name contains unsafe path characters: '{name}'");
    }

    Ok(())
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".agents/workflows")).unwrap();
        dir
    }

    fn write_workflow(project_root: &Path, name: &str) {
        let path = project_root.join(".agents/workflows").join(format!("{name}.ts"));
        fs::write(path, "export default async () => {};").unwrap();
    }

    #[test]
    fn resolves_ts_workflow() {
        let dir = setup();
        write_workflow(dir.path(), "duos");

        let resolved = resolve_workflow(dir.path(), "duos").unwrap();

        assert_eq!(resolved, dir.path().join(".agents/workflows/duos.ts"));
    }

    #[test]
    fn resolve_error_includes_workflows_directory() {
        let dir = setup();

        let err = resolve_workflow(dir.path(), "missing").unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains(".agents/workflows"));
    }

    #[test]
    fn rejects_unsafe_workflow_names() {
        let dir = setup();

        let slash = resolve_workflow(dir.path(), "../escape");
        let backslash = resolve_workflow(dir.path(), "..\\escape");
        let nested = resolve_workflow(dir.path(), "nested/name");

        assert!(slash.is_err());
        assert!(backslash.is_err());
        assert!(nested.is_err());
    }

    #[test]
    fn print_summary_labels_match_status() {
        // Verify no panics and correct output labels for all variants.
        print_summary("test", &RunStatus::Completed);
        print_summary("test", &RunStatus::Cancelled);
        print_summary("test", &RunStatus::Failed);
        print_summary("test", &RunStatus::Running);
        print_summary("test", &RunStatus::Pending);
    }
}
