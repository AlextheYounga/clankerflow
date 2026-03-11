use std::fs;
use std::path::{Path, PathBuf};

use agentkata::core::embeds::copy_kit;
use agentkata::db::connection::connect;
use agentkata::db::entities::event::{Column as EventColumn, Entity as Event};
use agentkata::db::entities::workflow::{Column as WorkflowColumn, Entity as Workflow};
use agentkata::db::entities::workflow_run::{Column as WorkflowRunColumn, Entity as WorkflowRun};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;
use tempfile::TempDir;

pub fn setup_project(workflow_name: &str, workflow_source: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    copy_kit(dir.path(), false).unwrap();

    let workflow_path = workflow_path(dir.path(), workflow_name);
    fs::write(workflow_path, workflow_source).unwrap();

    dir
}

pub fn workflow_path(project_root: &Path, workflow_name: &str) -> PathBuf {
    project_root
        .join(".agents/workflows")
        .join(format!("{workflow_name}.mjs"))
}

pub async fn stored_run(
    project_root: &Path,
    workflow_name: &str,
) -> agentkata::db::entities::workflow_run::Model {
    let db = connect(project_root).await.unwrap();
    let workflow = Workflow::find()
        .filter(WorkflowColumn::Name.eq(workflow_name))
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    WorkflowRun::find()
        .filter(WorkflowRunColumn::WorkflowId.eq(workflow.id))
        .one(&db)
        .await
        .unwrap()
        .unwrap()
}

pub async fn event_payloads(project_root: &Path, run_id: i64, event_type: &str) -> Vec<Value> {
    let db = connect(project_root).await.unwrap();
    Event::find()
        .filter(EventColumn::EntityId.eq(run_id))
        .filter(EventColumn::EventType.eq(event_type))
        .all(&db)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| event.data)
        .collect()
}
