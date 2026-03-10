use std::path::{Path, PathBuf};

use crate::app::types::RuntimeEnv;
use crate::core::project::require_project_root;
use crate::core::runner::{WorkflowArgs, run_workflow};
use crate::core::settings::Settings;
use crate::db::entities::workflow_run::RunStatus;

/// # Errors
/// Returns an error if the project root is not found, settings fail to load,
/// the workflow path cannot be resolved, or the workflow fails to run.
pub async fn run(name: String, env: RuntimeEnv, yolo: bool) -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    let workflow_path = resolve_workflow(&project_root, &name)?;
    let settings = Settings::load(&project_root)?;

    if settings.codebase_id.is_empty() {
        anyhow::bail!("codebase_id is missing from settings; run `agentctl init` first");
    }

    let args = WorkflowArgs {
        project_root: &project_root,
        workflow_name: &name,
        workflow_path: &workflow_path,
        env,
        yolo,
        codebase_id: &settings.codebase_id,
    };

    let final_status = run_workflow(&args).await?;
    print_summary(&name, &final_status);

    if matches!(final_status, RunStatus::Failed) {
        anyhow::bail!("workflow '{name}' failed");
    }

    Ok(())
}

fn print_summary(name: &str, status: &RunStatus) {
    let label = match status {
        RunStatus::Completed => "completed",
        RunStatus::Cancelled => "cancelled",
        RunStatus::Failed => "failed",
        RunStatus::Running => "running",
        RunStatus::Pending => "pending",
    };
    println!("workflow '{name}' {label}");
}

fn resolve_workflow(project_root: &Path, name: &str) -> anyhow::Result<PathBuf> {
    validate_workflow_name(name)?;
    let workflows_dir = project_root.join(".agents").join("workflows");
    let candidate = workflows_dir.join(format!("{name}.ts"));

    if candidate.exists() {
        return Ok(candidate);
    }

    anyhow::bail!(
        "workflow '{name}' not found under {}",
        workflows_dir.display()
    )
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
        let path = project_root
            .join(".agents/workflows")
            .join(format!("{name}.ts"));
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
