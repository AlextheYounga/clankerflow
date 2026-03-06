use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum AgentStatus {
    #[sea_orm(string_value = "PREPARING")]
    Preparing,
    #[sea_orm(string_value = "LAUNCHED")]
    Launched,
    #[sea_orm(string_value = "RUNNING")]
    Running,
    #[sea_orm(string_value = "COMPLETED")]
    Completed,
    #[sea_orm(string_value = "FAILED")]
    Failed,
    #[sea_orm(string_value = "KILLED")]
    Killed,
}

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum AgentEnv {
    #[sea_orm(string_value = "HOST")]
    Host,
    #[sea_orm(string_value = "CONTAINER")]
    Container,
}

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum TicketStatus {
    #[sea_orm(string_value = "OPEN")]
    Open,
    #[sea_orm(string_value = "IN_PROGRESS")]
    InProgress,
    #[sea_orm(string_value = "QA_REVIEW")]
    QaReview,
    #[sea_orm(string_value = "QA_CHANGES_REQUESTED")]
    QaChangesRequested,
    #[sea_orm(string_value = "CLOSED")]
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum WorkflowRunStatus {
    #[sea_orm(string_value = "PENDING")]
    Pending,
    #[sea_orm(string_value = "RUNNING")]
    Running,
    #[sea_orm(string_value = "COMPLETED")]
    Completed,
    #[sea_orm(string_value = "FAILED")]
    Failed,
    #[sea_orm(string_value = "CANCELLED")]
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum NodeExecutionStatus {
    #[sea_orm(string_value = "PENDING")]
    Pending,
    #[sea_orm(string_value = "RUNNING")]
    Running,
    #[sea_orm(string_value = "SUCCEEDED")]
    Succeeded,
    #[sea_orm(string_value = "FAILED")]
    Failed,
}
