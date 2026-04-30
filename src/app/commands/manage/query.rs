use anyhow::{Result, anyhow};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::app::commands::manage::state::{EventRow, RunRow};
use crate::core::tmux;
use crate::db::entities::{event, workflow, workflow_run, workflow_session};

/// # Errors
/// Returns an error if database reads or tmux inspection fail.
pub async fn load_runs(db: &DatabaseConnection, project_key: &str) -> Result<Vec<RunRow>> {
    let runs = workflow_run::Entity::find()
        .order_by_desc(workflow_run::Column::CreatedAt)
        .all(db)
        .await?;
    let windows = tmux::list_project_windows(project_key)?;

    let mut rows = Vec::with_capacity(runs.len());
    for run in runs {
        let workflow_name = load_workflow_name(db, run.workflow_id).await?;
        let session_ids = load_session_ids(db, run.id).await?;
        let tmux_windows = windows_for_run(&windows, run.id);

        rows.push(RunRow {
            run_id: run.id,
            workflow_name,
            status: format!("{:?}", run.status),
            env: format!("{:?}", run.env),
            pid: run.pid,
            created_at: run.created_at,
            updated_at: run.updated_at,
            completed_at: run.completed_at,
            session_ids,
            tmux_windows,
        });
    }

    Ok(rows)
}

/// # Errors
/// Returns an error if database reads fail.
pub async fn load_events(db: &DatabaseConnection, run_id: i64) -> Result<Vec<EventRow>> {
    let events = event::Entity::find()
        .filter(event::Column::EntityType.eq("workflow_run"))
        .filter(event::Column::EntityId.eq(run_id))
        .order_by_desc(event::Column::CreatedAt)
        .all(db)
        .await?;

    Ok(events
        .into_iter()
        .take(25)
        .map(|record| EventRow {
            created_at: record.created_at,
            summary: summarize_event_payload(record.data.as_ref()),
            event_type: record.event_type,
        })
        .collect())
}

async fn load_workflow_name(db: &DatabaseConnection, workflow_id: Option<i64>) -> Result<String> {
    let Some(workflow_id) = workflow_id else {
        return Ok("unknown".to_string());
    };

    let workflow = workflow::Entity::find_by_id(workflow_id).one(db).await?;
    workflow
        .map(|record| record.name)
        .ok_or_else(|| anyhow!("missing workflow record for id {workflow_id}"))
}

async fn load_session_ids(db: &DatabaseConnection, run_id: i64) -> Result<Vec<String>> {
    let sessions = workflow_session::Entity::find()
        .filter(workflow_session::Column::WorkflowRunId.eq(run_id))
        .order_by_asc(workflow_session::Column::CreatedAt)
        .all(db)
        .await?;

    Ok(sessions
        .into_iter()
        .map(|record| record.opencode_session_id)
        .collect())
}

fn windows_for_run(windows: &[tmux::SessionWindow], run_id: i64) -> Vec<String> {
    let needle = format!("__{run_id}__");
    windows
        .iter()
        .filter(|window| window.window_name.contains(&needle))
        .map(|window| window.window_name.clone())
        .collect()
}

fn summarize_event_payload(payload: Option<&sea_orm::JsonValue>) -> String {
    let Some(payload) = payload else {
        return "no payload".to_string();
    };

    if let Some(message) = payload.get("message").and_then(serde_json::Value::as_str) {
        return message.to_string();
    }
    if let Some(session_id) = payload
        .get("session_id")
        .and_then(serde_json::Value::as_str)
    {
        return format!("session {session_id}");
    }
    if let Some(status) = payload.get("status").and_then(serde_json::Value::as_str) {
        return status.to_string();
    }

    payload.to_string()
}
