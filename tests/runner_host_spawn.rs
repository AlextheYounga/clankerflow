mod support;

use clankerflow::app::types::RuntimeEnv;
use clankerflow::core::runner::{WorkflowArgs, WorkflowEngine};
use clankerflow::db::entities::workflow_run::RunStatus;
use serde_json::Value;

use support::{event_payloads, setup_project, stored_run, workflow_path};

const SUCCESS_WORKFLOW: &str = r#"
export const meta = {
  id: "demo",
  name: "Demo Workflow",
  runtime: "host",
};

export default async function demoWorkflow(_ctx, tools) {
  tools.log.info("workflow log from integration test");
}
"#;

#[tokio::test]
async fn run_persists_workflow_progress_from_spawned_host_runner() {
    let project = setup_project("demo", SUCCESS_WORKFLOW);
    let workflow_path = workflow_path(project.path(), "demo");

    unsafe {
        std::env::set_var("CLANKERFLOW_HOST_RUNNER_BUNDLE", test_runner_bundle());
    }

    let args = WorkflowArgs {
        project_root: project.path(),
        workflow_name: "demo",
        workflow_path: &workflow_path,
        env: RuntimeEnv::Host,
        yolo: true,
    };

    let final_status = WorkflowEngine::run(&args).await.unwrap();
    let run = stored_run(project.path(), "demo").await;
    let run_started = event_payloads(project.path(), run.id, "run_started").await;
    let step_started = event_payloads(project.path(), run.id, "step_started").await;
    let logs = event_payloads(project.path(), run.id, "log").await;
    let step_finished = event_payloads(project.path(), run.id, "step_finished").await;
    let run_finished = event_payloads(project.path(), run.id, "run_finished").await;

    assert_eq!(final_status, RunStatus::Completed);
    assert_eq!(run.status, RunStatus::Completed);
    assert!(run.completed_at.is_some());
    assert_eq!(run_started.len(), 1);
    assert_eq!(step_started.len(), 1);
    assert_eq!(step_finished.len(), 1);
    assert_eq!(run_finished.len(), 1);
    assert!(logs.iter().any(|payload| {
        payload
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("workflow log from integration test"))
    }));
}

fn test_runner_bundle() -> &'static str {
    env!("CLANKERFLOW_TEST_RUNNER_BUNDLE")
}
