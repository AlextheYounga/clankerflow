use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use agentkata::core::embeds::copy_kit;
use agentkata::core::runner::ipc_loop::{IpcLoopContext, handle_runner_line};
use agentkata::core::runner::protocol::LoopControl;
use agentkata::core::runner::signal::CancelState;
use agentkata::core::runner::store::{create_run, upsert_workflow};
use agentkata::db::connection::connect;
use agentkata::db::entities::event::{Column as EventColumn, Entity as Event};
use agentkata::db::entities::workflow_run::WorkflowEnv;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
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
