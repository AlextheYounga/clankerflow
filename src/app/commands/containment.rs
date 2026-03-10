use crate::core::docker::Docker;
use crate::core::project::require_project_root;
use crate::core::settings::Settings;

/// # Errors
/// Returns an error if the project root is not found, settings fail to load,
/// or the container fails to start.
pub async fn up() -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    let settings = Settings::load(&project_root)?;
    Docker::ensure_running(&project_root, &settings.codebase_id).await?;
    println!("container ready");
    Ok(())
}

/// # Errors
/// Returns an error if the project root is not found, settings fail to load,
/// or the container fails to stop.
pub async fn down() -> anyhow::Result<()> {
    let project_root = require_project_root()?;
    let settings = Settings::load(&project_root)?;
    Docker::down(&project_root, &settings.codebase_id).await?;
    println!("container stopped");
    Ok(())
}
