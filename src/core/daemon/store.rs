use std::path::Path;
use std::process;

use anyhow::{Result, anyhow};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::db::entities::event::ActiveModel as EventActive;
use crate::db::entities::workflow::ActiveModel as WorkflowActive;
use crate::db::entities::workflow_run::{
    ActiveModel as WorkflowRunActive, Entity as WorkflowRun, RunStatus, WorkflowEnv,
};

pub async fn upsert_workflow(db: &DatabaseConnection, name: &str, path: &Path) -> Result<i64> {
    use crate::db::entities::workflow::{Column as WorkflowColumn, Entity as Workflow};

    if let Some(existing) = Workflow::find()
        .filter(WorkflowColumn::Name.eq(name))
        .one(db)
        .await?
    {
        return Ok(existing.id);
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
    workflow_id: i64,
    env: WorkflowEnv,
) -> Result<i64> {
    let now = chrono::Utc::now();

    let inserted = WorkflowRunActive {
        workflow_id: ActiveValue::Set(Some(workflow_id)),
        pid: ActiveValue::Set(Some(i64::from(process::id()))),
        env: ActiveValue::Set(env),
        status: ActiveValue::Set(RunStatus::Pending),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(inserted.id)
}

pub async fn set_status(db: &DatabaseConnection, id: i64, status: RunStatus) -> Result<()> {
    let now = chrono::Utc::now();
    // `completed_at` is only meaningful for terminal states; keeping it null for
    // active states avoids misleading durations in downstream queries.
    let completed_at = matches!(
        status,
        RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled
    )
    .then_some(now);

    WorkflowRunActive {
        id: ActiveValue::Unchanged(id),
        status: ActiveValue::Set(status),
        updated_at: ActiveValue::Set(now),
        completed_at: ActiveValue::Set(completed_at),
        ..Default::default()
    }
    .update(db)
    .await?;

    Ok(())
}

pub async fn is_stop_requested(db: &DatabaseConnection, id: i64) -> Result<bool> {
    let run = WorkflowRun::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| anyhow!("workflow run not found: {id}"))?;
    // Cancellation is represented as desired state in the DB so any worker loop
    // can observe it, even if the original triggering process is gone.
    Ok(run.status == RunStatus::Cancelled)
}

pub async fn append_run_event(
    db: &DatabaseConnection,
    id: i64,
    event_type: &str,
    data: serde_json::Value,
) -> Result<()> {
    EventActive {
        entity_id: ActiveValue::Set(id),
        entity_type: ActiveValue::Set("workflow_run".to_string()),
        event_type: ActiveValue::Set(event_type.to_string()),
        data: ActiveValue::Set(Some(data)),
        created_at: ActiveValue::Set(chrono::Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}
