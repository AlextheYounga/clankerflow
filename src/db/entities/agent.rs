use sea_orm::entity::prelude::*;

use super::enums::{AgentEnv, AgentStatus};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "agents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub execution_id: String,
    pub tmux_session: Option<String>,
    #[sea_orm(column_name = "role")]
    pub label: Option<String>,
    pub prompt: Option<String>,
    pub model: String,
    pub data: Option<Json>,
    pub env: AgentEnv,
    pub status: AgentStatus,
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
                .from(Column::ExecutionId)
                .to(super::workflow_run::Column::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .into(),
        }
    }
}

// Relation definitions for `Agent`.
// - `WorkflowRun`: many-to-one relation to the `workflow_run` entity.
//   Uses `Cascade` so deleting a workflow run will also delete its agents.
impl Related<super::workflow_run::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowRun.def()
    }
}