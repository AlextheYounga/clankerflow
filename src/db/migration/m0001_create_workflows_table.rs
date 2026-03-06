use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0001_create_workflows_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Workflows::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Workflows::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Workflows::Name).string().not_null())
                    .col(ColumnDef::new(Workflows::SaveFile).string().not_null())
                    .col(ColumnDef::new(Workflows::Hash).string().not_null())
                    .col(ColumnDef::new(Workflows::Version).integer().not_null())
                    .col(
                        ColumnDef::new(Workflows::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Workflows::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Workflows {
    Table,
    Id,
    Name,
    SaveFile,
    Hash,
    Version,
    CreatedAt,
}
