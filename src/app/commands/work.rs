use std::path::PathBuf;

use crate::app::types::RuntimeEnv;
use crate::core::project::require_project_root;

pub async fn run(name: String, env: RuntimeEnv, yolo: bool) -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    let workflow_path = resolve_workflow(&project_root, &name)?;

    println!(
        "Starting workflow '{}' (env={}, yolo={})",
        name,
        env.as_str(),
        yolo
    );
    println!("  workflow: {}", workflow_path.display());

    // TODO: spawn Node runtime, pass workflow path and env over IPC
    anyhow::bail!("workflow execution not yet implemented")
}

fn resolve_workflow(project_root: &PathBuf, name: &str) -> anyhow::Result<PathBuf> {
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
