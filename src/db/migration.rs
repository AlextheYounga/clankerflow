use sea_orm_migration::MigratorTrait;

mod m0001_create_workflows_table;
mod m0002_create_workflow_runs_table;
mod m0003_create_workflow_sessions_table;
mod m0004_create_events_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
        vec![
            Box::new(m0001_create_workflows_table::Migration),
            Box::new(m0002_create_workflow_runs_table::Migration),
            Box::new(m0003_create_workflow_sessions_table::Migration),
            Box::new(m0004_create_events_table::Migration),
        ]
    }
}
