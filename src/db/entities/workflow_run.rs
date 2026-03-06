use sea_orm::entity::prelude::*;

use super::enums::WorkflowRunStatus;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "workflow_runs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub workflow_id: Option<String>,
    pub pid: Option<String>,
    pub status: WorkflowRunStatus,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub completed_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Agents,
    Workflow,
}

// Relation definitions for `WorkflowRun`.
// - `Agents`: one-to-many relation to the `agent` entity.
// - `Workflow`: optional many-to-one relation to the `workflow` entity
//   (sets the foreign key to NULL when the workflow is deleted).
impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Agents => Entity::has_many(super::agent::Entity).into(),
            Self::Workflow => Entity::belongs_to(super::workflow::Entity)
                .from(Column::WorkflowId)
                .to(super::workflow::Column::Id)
                .on_delete(ForeignKeyAction::SetNull)
                .into(),
        }
    }
}

// Related impl for `Agent`: enables using helpers like `Entity::find_related(agent::Entity)`
// to fetch agents associated with a workflow run.
impl Related<super::agent::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Agents.def()
    }
}

// Related impl for `Workflow`: enables traversing from a workflow run to its parent workflow.
impl Related<super::workflow::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Workflow.def()
    }
}