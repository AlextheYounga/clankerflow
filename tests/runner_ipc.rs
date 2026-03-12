use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use agentkata::core::embeds::copy_kit;
use agentkata::core::runner::ipc_loop::{IpcLoopContext, handle_runner_line};
use agentkata::core::runner::protocol::LoopControl;
use agentkata::core::runner::signal::CancelState;
use agentkata::core::runner::store::{create_run, upsert_workflow};
use agentkata::db::connection::connect;
use agentkata::db::entities::event::{Column as EventColumn, Entity as Event};
use agentkata::db::entities::workflow_run::{RunStatus, WorkflowEnv};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::json;
use tempfile::TempDir;

#[tokio::test]
async fn handle_runner_line_records_parse_error_for_invalid_json() {
    let project = TempDir::new().unwrap();
    copy_kit(project.path(), false).unwrap();
    let ctx = test_ipc_context(&project).await;

    let (control, status) = handle_runner_line(&ctx, "not json").await.unwrap();

    assert_eq!(control, LoopControl::Continue);
    assert_eq!(status, None);

    let errors: Vec<_> = Event::find()
        .filter(EventColumn::EntityId.eq(ctx.run_id))
        .filter(EventColumn::EventType.eq("ipc_parse_error"))
        .all(&ctx.db)
        .await
        .unwrap();

    assert_eq!(errors.len(), 1);
    let error_msg = errors[0]
        .data
        .as_ref()
        .and_then(|d| d.get("error"))
        .and_then(|v| v.as_str())
        .unwrap();
    assert!(
        error_msg.contains("line 1 column"),
        "expected serde parse error, got: {error_msg}"
    );
}

#[tokio::test]
async fn handle_runner_line_persists_run_failed_payload_with_error_details() {
    let project = TempDir::new().unwrap();
    copy_kit(project.path(), false).unwrap();
    let ctx = test_ipc_context(&project).await;

    let message = json!({
        "v": "v1",
        "id": "evt_1",
        "kind": "event",
        "name": "run_failed",
        "payload": {
            "run_id": ctx.run_id,
            "error_code": "WORKFLOW_ERROR",
            "message": "Planner agent failed: fetch failed",
            "details": {},
            "failed_at": "2026-03-12T05:50:57.227Z"
        }
    });

    let (control, status) = handle_runner_line(&ctx, &message.to_string())
        .await
        .unwrap();

    assert_eq!(control, LoopControl::Stop);
    assert_eq!(status, Some(RunStatus::Failed));

    let events = Event::find()
        .filter(EventColumn::EntityId.eq(ctx.run_id))
        .filter(EventColumn::EventType.eq("run_failed"))
        .all(&ctx.db)
        .await
        .unwrap();

    assert_eq!(events.len(), 1);
    let payload = events[0].data.as_ref().unwrap();
    assert_eq!(payload["error_code"], "WORKFLOW_ERROR");
    assert_eq!(payload["message"], "Planner agent failed: fetch failed");
    assert_eq!(payload["failed_at"], "2026-03-12T05:50:57.227Z");
}

async fn test_ipc_context(project: &TempDir) -> IpcLoopContext {
    let db = connect(project.path()).await.unwrap();
    let workflow_id = upsert_workflow(&db, "test", &project.path().join("test.ts"))
        .await
        .unwrap();
    let run_id = create_run(&db, workflow_id, WorkflowEnv::Host)
        .await
        .unwrap();
    let cancel = Arc::new(CancelState {
        cancelled: AtomicBool::new(false),
        force_kill: AtomicBool::new(false),
    });
    IpcLoopContext { db, run_id, cancel }
}
