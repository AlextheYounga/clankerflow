use crate::core::constants::{CodebasePathType, codebase_paths};
use crate::db::migration::Migrator;
use sea_orm::{Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;
use std::fs;
use std::path::Path;
use tokio::sync::OnceCell;

static DB: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub async fn run_migrations(db: &DatabaseConnection) -> Result<(), DbErr> {
    Migrator::up(db, None).await
}

pub async fn instantiate_db(
    codebase_path: &Path,
) -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let db_path = codebase_paths(codebase_path, CodebasePathType::DatabasePath);
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !db_path.exists() {
        fs::File::create(&db_path)?;
    }
    let db_url = format!("sqlite:///{}?mode=rwc", db_path.to_string_lossy());
    let conn = Database::connect(&db_url).await?;
    run_migrations(&conn).await?;
    Ok(conn)
}

pub async fn init_db(codebase_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // If already initialized, return early (idempotent)
    if DB.get().is_some() {
        return Ok(());
    }

    let conn = instantiate_db(codebase_path).await?;
    DB.set(conn).map_err(|_| "Database already initialized")?;
    Ok(())
}

pub fn db_instance() -> &'static DatabaseConnection {
    DB.get()
        .expect("DB not initialized - did you call init_db()?")
}
