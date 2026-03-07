use std::path::PathBuf;

use crate::app::types::RuntimeEnv;
use crate::core::daemon::launch_workflow;
use crate::core::project::require_project_root;
use crate::core::settings::Settings;

pub async fn run(name: String, env: RuntimeEnv, yolo: bool) -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    let workflow_path = resolve_workflow(&project_root, &name)?;
    let settings = Settings::load(&project_root)?;

    if settings.codebase_id.is_empty() {
        anyhow::bail!("codebase_id is missing from settings; run `agentctl init` first");
    }

    let run_id = launch_workflow(&project_root, &name, &workflow_path, env.as_str(), yolo).await?;
    println!("workflow started: {run_id}");
    Ok(())
}

fn resolve_workflow(project_root: &PathBuf, name: &str) -> anyhow::Result<PathBuf> {
    validate_workflow_name(name)?;
    let workflows_dir = project_root.join(".agents").join("workflows");

    for ext in ["js", "ts"] {
        let candidate = workflows_dir.join(format!("{}.{}", name, ext));
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "workflow '{}' not found under {}",
        name,
        workflows_dir.display()
    )
}

fn validate_workflow_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        anyhow::bail!("workflow name cannot be empty");
    }

    if name.contains('/') || name.contains('\\') || name.contains("..") {
        anyhow::bail!("workflow name contains unsafe path characters: '{}'", name);
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

    fn write_workflow(project_root: &Path, name: &str, ext: &str) {
        let path = project_root
            .join(".agents/workflows")
            .join(format!("{}.{}", name, ext));
        fs::write(path, "export default async () => {};").unwrap();
    }

    #[test]
    fn resolves_js_workflow() {
        let dir = setup();
        write_workflow(dir.path(), "duos", "js");

        let resolved = resolve_workflow(&dir.path().to_path_buf(), "duos").unwrap();

        assert_eq!(resolved, dir.path().join(".agents/workflows/duos.js"));
    }

    #[test]
    fn resolves_ts_workflow_when_js_missing() {
        let dir = setup();
        write_workflow(dir.path(), "duos", "ts");

        let resolved = resolve_workflow(&dir.path().to_path_buf(), "duos").unwrap();

        assert_eq!(resolved, dir.path().join(".agents/workflows/duos.ts"));
    }

    #[test]
    fn prefers_js_when_js_and_ts_exist() {
        let dir = setup();
        write_workflow(dir.path(), "duos", "js");
        write_workflow(dir.path(), "duos", "ts");

        let resolved = resolve_workflow(&dir.path().to_path_buf(), "duos").unwrap();

        assert_eq!(resolved, dir.path().join(".agents/workflows/duos.js"));
    }

    #[test]
    fn resolve_error_includes_workflows_directory() {
        let dir = setup();

        let err = resolve_workflow(&dir.path().to_path_buf(), "missing").unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains(".agents/workflows"));
    }

    #[test]
    fn rejects_unsafe_workflow_names() {
        let dir = setup();

        let slash = resolve_workflow(&dir.path().to_path_buf(), "../escape");
        let backslash = resolve_workflow(&dir.path().to_path_buf(), "..\\escape");
        let nested = resolve_workflow(&dir.path().to_path_buf(), "nested/name");

        assert!(slash.is_err());
        assert!(backslash.is_err());
        assert!(nested.is_err());
    }
}
