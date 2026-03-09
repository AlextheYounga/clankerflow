use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m0004_create_events_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Events::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Events::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Events::EntityId).big_integer().not_null())
                    .col(ColumnDef::new(Events::EntityType).string().not_null())
                    .col(ColumnDef::new(Events::EventType).string().not_null())
                    .col(ColumnDef::new(Events::Data).json().null())
                    .col(
                        ColumnDef::new(Events::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Events::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Events {
    Table,
    Id,
    EntityId,
    EntityType,
    EventType,
    Data,
    CreatedAt,
}
