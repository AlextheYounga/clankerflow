use crate::core::codebase_id;
use crate::core::docker::Docker;
use crate::core::project::require_project_root;

/// # Errors
/// Returns an error if the project root is not found or the container fails to
/// start.
pub async fn up() -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    let project_id = codebase_id::derive(&project_root);
    Docker::ensure_running(&project_root, &project_id).await?;
    println!("container ready");
    Ok(())
}

/// # Errors
/// Returns an error if the project root is not found or the container fails to
/// stop.
pub async fn down() -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    let project_id = codebase_id::derive(&project_root);
    Docker::down(&project_root, &project_id).await?;
    println!("container stopped");
    Ok(())
}
