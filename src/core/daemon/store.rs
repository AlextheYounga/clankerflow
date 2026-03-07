use std::path::Path;

use anyhow::{Result, anyhow};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::db::entities::event::ActiveModel as EventActive;
use crate::db::entities::workflow::ActiveModel as WorkflowActive;
use crate::db::entities::workflow_run::{
    ActiveModel as WorkflowRunActiveModel, Column as WorkflowRunColumn, Entity as WorkflowRun,
    Model as WorkflowRunModel, WorkflowEnv, WorkflowRunStatus,
};

pub async fn upsert_workflow(db: &DatabaseConnection, name: &str, path: &Path) -> Result<i64> {
    use crate::db::entities::workflow::{Column as WorkflowColumn, Entity as Workflow};

    if let Some(existing_workflow) = Workflow::find()
        .filter(WorkflowColumn::Name.eq(name))
        .one(db)
        .await?
    {
        return Ok(existing_workflow.id);
    }

    let now = chrono::Utc::now();
    let inserted = WorkflowActive {
        name: ActiveValue::Set(name.to_string()),
        save_file: ActiveValue::Set(path.to_string_lossy().to_string()),
        hash: ActiveValue::Set("dev-hash".to_string()),
        version: ActiveValue::Set(1),
        created_at: ActiveValue::Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(inserted.id)
}

pub async fn create_run(
    db: &DatabaseConnection,
    run_id: &str,
    workflow_id: i64,
    env: WorkflowEnv,
) -> Result<()> {
    let now = chrono::Utc::now();

    WorkflowRunActiveModel {
        run_id: ActiveValue::Set(Some(run_id.to_string())),
        workflow_id: ActiveValue::Set(Some(workflow_id)),
        pid: ActiveValue::Set(Some(std::process::id() as i64)),
        env: ActiveValue::Set(env),
        status: ActiveValue::Set(WorkflowRunStatus::Pending),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

pub async fn set_status(
    db: &DatabaseConnection,
    run_id: &str,
    status: WorkflowRunStatus,
) -> Result<()> {
    let run = find_run(db, run_id).await?;
    let now = chrono::Utc::now();
    let completed_at = matches!(
        status,
        WorkflowRunStatus::Completed | WorkflowRunStatus::Failed | WorkflowRunStatus::Cancelled
    )
    .then_some(now);

    WorkflowRunActiveModel {
        id: ActiveValue::Unchanged(run.id),
        status: ActiveValue::Set(status),
        updated_at: ActiveValue::Set(now),
        completed_at: ActiveValue::Set(completed_at),
        ..Default::default()
    }
    .update(db)
    .await?;

    Ok(())
}

pub async fn is_stop_requested(db: &DatabaseConnection, run_id: &str) -> Result<bool> {
    let run = find_run(db, run_id).await?;
    Ok(run.status == WorkflowRunStatus::Cancelled)
}

pub async fn append_run_event(
    db: &DatabaseConnection,
    run_id: &str,
    event_type: &str,
    data: serde_json::Value,
) -> Result<()> {
    let run = find_run(db, run_id).await?;

    EventActive {
        parent_id: ActiveValue::Set(Some(run.id)),
        event_type: ActiveValue::Set(event_type.to_string()),
        data: ActiveValue::Set(Some(data)),
        created_at: ActiveValue::Set(chrono::Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

async fn find_run(db: &DatabaseConnection, run_id: &str) -> Result<WorkflowRunModel> {
    WorkflowRun::find()
        .filter(WorkflowRunColumn::RunId.eq(run_id))
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("workflow run not found: {run_id}"))
}
