use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0002_create_workflow_runs_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkflowRuns::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkflowRuns::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WorkflowRuns::WorkflowId)
                            .big_integer()
                            .null(),
                    )
                    .col(ColumnDef::new(WorkflowRuns::Pid).big_integer().null())
                    .col(ColumnDef::new(WorkflowRuns::Env).string_len(32).not_null())
                    .col(
                        ColumnDef::new(WorkflowRuns::Status)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowRuns::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowRuns::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowRuns::CompletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(WorkflowRuns::Table, WorkflowRuns::WorkflowId)
                            .to(Workflows::Table, Workflows::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkflowRuns::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum WorkflowRuns {
    Table,
    Id,
    WorkflowId,
    Pid,
    Env,
    Status,
    CreatedAt,
    UpdatedAt,
    CompletedAt,
}

// Referenced for the FK definition
#[derive(Iden)]
enum Workflows {
    Table,
    Id,
}
