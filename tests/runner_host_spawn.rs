mod support;

use agentkata::app::types::RuntimeEnv;
use agentkata::core::runner::{WorkflowArgs, WorkflowRunner};
use agentkata::db::entities::workflow_run::RunStatus;
use serde_json::Value;

use support::{event_payloads, setup_project, stored_run, workflow_path};

// Exercises the real failure path: agent.run calls the OpenCode SDK against a
// port with nothing listening, so fetch rejects, agent.run returns ok:false,
// and the workflow re-throws as "Planner agent failed: fetch failed".
const AGENT_FETCH_ERROR_WORKFLOW: &str = r#"
export const meta = {
  id: "agent-fetch-error",
  name: "Agent Fetch Error",
  runtime: "host",
};

export default async function run(_ctx, tools) {
  const result = await tools.agent.run({ prompt: "do something" });
  if (!result.ok) throw new Error(`Planner agent failed: ${result.error}`);
}
"#;

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
        std::env::set_var("AGENTKATA_HOST_RUNNER_BUNDLE", test_runner_bundle());
    }

    let args = WorkflowArgs {
        project_root: project.path(),
        workflow_name: "demo",
        workflow_path: &workflow_path,
        env: RuntimeEnv::Host,
        yolo: true,
    };

    let final_status = WorkflowRunner::run(&args).await.unwrap();
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

#[tokio::test]
async fn run_records_failed_status_and_error_details_when_agent_cannot_reach_opencode() {
    let project = setup_project("agent-fetch-error", AGENT_FETCH_ERROR_WORKFLOW);
    let workflow_path = workflow_path(project.path(), "agent-fetch-error");

    unsafe {
        std::env::set_var("AGENTKATA_HOST_RUNNER_BUNDLE", test_runner_bundle());
    }

    let args = WorkflowArgs {
        project_root: project.path(),
        workflow_name: "agent-fetch-error",
        workflow_path: &workflow_path,
        env: RuntimeEnv::Host,
        yolo: false,
    };

    let final_status = WorkflowRunner::run(&args).await.unwrap();
    let run = stored_run(project.path(), "agent-fetch-error").await;
    let run_failed = event_payloads(project.path(), run.id, "run_failed").await;

    assert_eq!(final_status, RunStatus::Failed);
    assert_eq!(run.status, RunStatus::Failed);
    assert!(run.completed_at.is_some());

    assert_eq!(run_failed.len(), 1);
    let payload = &run_failed[0];
    assert_eq!(payload["error_code"], "WORKFLOW_ERROR");
    assert!(
        payload["message"]
            .as_str()
            .is_some_and(|m| m.contains("ECONNREFUSED")),
        "expected ECONNREFUSED cause detail in run_failed message, got: {payload}"
    );
}

fn test_runner_bundle() -> &'static str {
    env!("AGENTKATA_TEST_RUNNER_BUNDLE")
}
