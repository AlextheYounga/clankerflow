use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use crate::core::project::get_project_root;
use crate::db::migration::Migrator;

pub fn project_database_path() -> PathBuf {
    let project_root = get_project_root().expect("Failed to find project root");
    project_root.join(".agents/.agentctl/database.db")
}

pub async fn connect() -> Result<DatabaseConnection> {
    let db_path = project_database_path();
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
