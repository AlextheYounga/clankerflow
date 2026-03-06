use crate::core::project::require_project_root;
use crate::db::db::connect;

pub async fn run() -> anyhow::Result<()> {
    let _project_root = require_project_root()?;
    connect().await.map_err(|e| anyhow::anyhow!("{}", e))?;

    // TODO: launch Ratatui TUI for monitoring workflow runs
    println!("manage: TUI not yet implemented");
    Ok(())
}
