use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m0003_create_workflow_sessions_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkflowSessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkflowSessions::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WorkflowSessions::WorkflowRunId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowSessions::OpencodeSessionId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(WorkflowSessions::Label).string().null())
                    .col(ColumnDef::new(WorkflowSessions::Data).json().null())
                    .col(
                        ColumnDef::new(WorkflowSessions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowSessions::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(WorkflowSessions::Table, WorkflowSessions::WorkflowRunId)
                            .to(WorkflowRuns::Table, WorkflowRuns::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkflowSessions::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum WorkflowSessions {
    Table,
    Id,
    WorkflowRunId,
    OpencodeSessionId,
    Label,
    Data,
    CreatedAt,
    UpdatedAt,
}

// Referenced for the FK definition
#[derive(Iden)]
enum WorkflowRuns {
    Table,
    Id,
}
