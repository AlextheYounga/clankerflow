use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub parent_id: Option<String>,
    #[sea_orm(column_name = "type")]
    pub event_type: String,
    pub data: Option<Json>,
    pub created_at: DateTimeUtc,
}
