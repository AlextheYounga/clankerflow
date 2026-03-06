use crate core::project::get_project_root;
use crate::db::migration::Migrator;
use sea_orm::{Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;

pub fn project_database_path() -> PathBuf {
	project_root = get_project_root().expect("Failed to find project root");
    project_root.join(".agents/.agentctl/database.db")
}

pub async fn connect() -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let db_path = project_database_path();
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

async fn run_migrations(db: &DatabaseConnection) -> Result<(), DbErr> {
    Migrator::up(db, None).await
}
