use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0005_add_run_id_to_workflow_runs"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkflowRuns::Table)
                    .add_column(ColumnDef::new(WorkflowRuns::RunId).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkflowRuns::Table)
                    .drop_column(WorkflowRuns::RunId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum WorkflowRuns {
    Table,
    RunId,
}
