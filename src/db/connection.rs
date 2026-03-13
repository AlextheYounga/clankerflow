use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use crate::db::migration::Migrator;

fn database_path(project_root: &Path) -> PathBuf {
    project_root.join(".agents/.clankerflow/database.db")
}

/// # Errors
/// Returns an error if migrations fail.
pub async fn migrate(conn: &DatabaseConnection) -> Result<()> {
    Migrator::up(conn, None)
        .await
        .context("failed to run migrations")?;
    Ok(())
}

/// # Errors
/// Returns an error if the database directory or file cannot be created, or the
/// connection cannot be established.
pub async fn connect(project_root: &Path) -> Result<DatabaseConnection> {
    let db_path = database_path(project_root);

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
    migrate(&conn).await?;
    Ok(conn)
}
