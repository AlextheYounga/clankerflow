use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "workflow_sessions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub workflow_run_id: i64,
    pub opencode_session_id: String,
    pub label: Option<String>,
    pub data: Option<Json>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    WorkflowRun,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::WorkflowRun => Entity::belongs_to(super::workflow_run::Entity)
                .from(Column::WorkflowRunId)
                .to(super::workflow_run::Column::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .into(),
        }
    }
}

impl Related<super::workflow_run::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowRun.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
