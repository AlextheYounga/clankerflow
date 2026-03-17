use std::path::Path;
use std::process;

use anyhow::Result;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::db::entities::event::ActiveModel as EventActive;
use crate::db::entities::workflow::ActiveModel as WorkflowActive;
use crate::db::entities::workflow_run::{ActiveModel as WorkflowRunActive, RunStatus, WorkflowEnv};
use crate::db::entities::workflow_session::ActiveModel as WorkflowSessionActive;

/// Find or create a workflow record for a workflow path.
///
/// # Errors
/// Returns an error if database read/insert operations fail.
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

/// Create a workflow run record in `pending` state.
///
/// # Errors
/// Returns an error if inserting the run row fails.
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

/// Update the run status and timestamps for a workflow run.
///
/// # Errors
/// Returns an error if updating the run row fails.
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

/// Append an event record for a workflow run.
///
/// # Errors
/// Returns an error if inserting the event row fails.
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

/// Persist an association between a workflow run and an `OpenCode` session.
///
/// # Errors
/// Returns an error if inserting the session row fails.
pub async fn create_workflow_session(
    db: &DatabaseConnection,
    run_id: i64,
    opencode_session_id: &str,
) -> Result<()> {
    let now = chrono::Utc::now();

    WorkflowSessionActive {
        workflow_run_id: ActiveValue::Set(run_id),
        opencode_session_id: ActiveValue::Set(opencode_session_id.to_string()),
        label: ActiveValue::Set(None),
        data: ActiveValue::Set(None),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}
