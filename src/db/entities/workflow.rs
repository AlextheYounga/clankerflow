use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "workflows")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub save_file: String,
    pub hash: String,
    pub version: i32,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    WorkflowRuns,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::WorkflowRuns => Entity::has_many(super::workflow_run::Entity).into(),
        }
    }
}

// Relation definitions for `Workflow`.
// - `WorkflowRuns`: one-to-many relation to the `workflow_run` entity.
//   This allows fetching runs belonging to a workflow.
impl Related<super::workflow_run::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowRuns.def()
    }
}
