use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use crate::core::project::get_project_root;
use crate::db::migration::Migrator;

fn project_database_path() -> Option<PathBuf> {
    get_project_root().map(|root| root.join(".agents/.agentkata/database.db"))
}

/// # Errors
/// Returns an error if the project root cannot be determined, the database
/// directory or file cannot be created, the connection cannot be established,
/// or migrations fail.
pub async fn connect() -> Result<DatabaseConnection> {
    let db_path = project_database_path()
        .ok_or_else(|| anyhow!("project root not found; run `kata init` first"))?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create DB directory: {}", parent.display()))?;
    }
    if !db_path.exists() {
        fs::File::create(&db_path)
            .with_context(|| format!("failed to create DB file: {}", db_path.display()))?;
    }
    let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());
    let conn = Database::connect(&db_url)
        .await
        .context("failed to connect to database")?;
    Migrator::up(&conn, None)
        .await
        .context("failed to run migrations")?;
    Ok(conn)
}
