mod env;
mod ipc_loop;
mod protocol;
mod signal;
mod store;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use std::time::Duration;

use anyhow::{Result, anyhow};
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Child;
use tokio::time::timeout;

use crate::app::types::RuntimeEnv;
use crate::core::codebase_id;
use crate::db::connection::connect;
use crate::db::entities::workflow_run::RunStatus;

use env::{parse_runtime_env, spawn_container_runner, spawn_host_runner};
use ipc_loop::{IpcLoopContext, drive_ipc_loop, send_start_run};
use protocol::send_shutdown;
use signal::{CancelState, install_signal_handler, wait_for_child};
use store::{create_run, upsert_workflow};

/// Parameters for running a workflow synchronously.
pub struct WorkflowArgs<'a> {
    pub project_root: &'a Path,
    pub workflow_name: &'a str,
    pub workflow_path: &'a Path,
    pub env: RuntimeEnv,
    pub yolo: bool,
}

pub struct WorkflowRunner {
    process: RunnerProcess,
    /// Bidirectional TCP IPC channel. Taken by `WorkflowRunner::run` after spawn;
    /// `None` once consumed.
    ipc: Option<TcpStream>,
}

enum RunnerProcess {
    Child(Child),
    #[cfg(test)]
    InProcess,
}

impl WorkflowRunner {
    pub async fn run(args: &WorkflowArgs<'_>) -> Result<RunStatus> {
        let ctx = Self::create_run_context(args).await?;
        let codebase_id = codebase_id::derive(args.project_root);
        let runner = Self::spawn_process(args.project_root, args.env, &codebase_id).await?;
        Self::run_with_context(args, ctx, runner).await
    }

    async fn run_with_context(
        args: &WorkflowArgs<'_>,
        ctx: IpcLoopContext,
        mut runner: Self,
    ) -> Result<RunStatus> {
        let _ = args;
        let ipc = runner.take_ipc_channel()?;
        let (ipc_read, mut ipc_write) = io::split(ipc);

        send_start_run(&mut ipc_write, args, ctx.run_id).await?;
        install_signal_handler(&ctx.cancel);

        let final_status = drive_ipc_loop(&ctx, &mut ipc_write, ipc_read).await?;

        send_shutdown(&mut ipc_write).await;
        drop(ipc_write);
        runner.wait_for_exit(&ctx.cancel).await?;

        Ok(final_status)
    }

    async fn create_run_context(args: &WorkflowArgs<'_>) -> Result<IpcLoopContext> {
        let db = connect(args.project_root).await?;
        let workflow_env = parse_runtime_env(args.env);
        let workflow_id = upsert_workflow(&db, args.workflow_name, args.workflow_path).await?;
        let run_id = create_run(&db, workflow_id, workflow_env).await?;

        println!("workflow started (run id: {run_id})");

        let cancel = Arc::new(CancelState {
            cancelled: AtomicBool::new(false),
            force_kill: AtomicBool::new(false),
        });

        Ok(IpcLoopContext { db, run_id, cancel })
    }

    async fn spawn_process(
        project_root: &Path,
        env: RuntimeEnv,
        codebase_id: &str,
    ) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();

        let child = match env {
            RuntimeEnv::Host => spawn_host_runner(project_root, port)?,
            RuntimeEnv::Container => {
                spawn_container_runner(project_root, codebase_id, port).await?
            }
        };

        let (stream, _) = timeout(Duration::from_secs(30), listener.accept())
            .await
            .map_err(|_| {
                anyhow!(
                    "timed out waiting for runner to connect (is Docker running and the image built?)"
                )
            })?
            .map_err(|e| anyhow!("failed to accept runner connection: {e}"))?;

        Ok(Self {
            process: RunnerProcess::Child(child),
            ipc: Some(stream),
        })
    }

    async fn wait_for_exit(&mut self, cancel: &Arc<CancelState>) -> Result<()> {
        match &mut self.process {
            RunnerProcess::Child(child) => wait_for_child(child, cancel).await,
            #[cfg(test)]
            RunnerProcess::InProcess => Ok(()),
        }
    }

    fn take_ipc_channel(&mut self) -> Result<TcpStream> {
        self.ipc
            .take()
            .ok_or_else(|| anyhow!("IPC channel not available"))
    }

    #[cfg(test)]
    async fn run_with_runner(args: &WorkflowArgs<'_>, runner: Self) -> Result<RunStatus> {
        let ctx = Self::create_run_context(args).await?;
        Self::run_with_context(args, ctx, runner).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    use serde_json::Value;
    use std::fs;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;
    use tempfile::TempDir;

    use crate::db::entities::event::{Column as EventColumn, Entity as Event};
    use crate::db::entities::workflow::{Column as WorkflowColumn, Entity as Workflow};
    use crate::db::entities::workflow_run::{Column as WorkflowRunColumn, Entity as WorkflowRun};
    use crate::db::entities::workflow_session::Entity as WorkflowSession;

    fn setup_project() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".agents/workflows")).unwrap();
        fs::write(
            dir.path().join(".agents/workflows/demo.ts"),
            "export default async () => {};\n",
        )
        .unwrap();
        dir
    }

    fn in_process_runner(ipc: TcpStream) -> WorkflowRunner {
        WorkflowRunner {
            process: RunnerProcess::InProcess,
            ipc: Some(ipc),
        }
    }

    async fn stored_run(project_root: &Path) -> crate::db::entities::workflow_run::Model {
        let db = connect(project_root).await.unwrap();
        let workflow = Workflow::find()
            .filter(WorkflowColumn::Name.eq("demo"))
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

    async fn event_payloads(project_root: &Path, run_id: i64, event_type: &str) -> Vec<Value> {
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

    async fn stored_sessions(
        project_root: &Path,
    ) -> Vec<crate::db::entities::workflow_session::Model> {
        let db = connect(project_root).await.unwrap();
        WorkflowSession::find().all(&db).await.unwrap()
    }

    #[tokio::test]
    async fn run_persists_workflow_progress_from_runner_events() {
        let project = setup_project();
        let workflow_path = project.path().join(".agents/workflows/demo.ts");
        let args = WorkflowArgs {
            project_root: project.path(),
            workflow_name: "demo",
            workflow_path: &workflow_path,
            env: RuntimeEnv::Host,
            yolo: true,
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let fake_runner = tokio::spawn(async move {
            let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            let (read_half, mut write_half) = io::split(stream);
            let mut lines = BufReader::new(read_half).lines();

            let start = lines.next_line().await.unwrap().unwrap();
            let message: crate::core::ipc::Message = serde_json::from_str(&start).unwrap();
            assert_eq!(message.name, "start_run");

            for (name, payload) in [
                ("run_started", serde_json::json!({})),
                (
                    "agent_session_started",
                    serde_json::json!({ "session_id": "sess_123" }),
                ),
                ("run_finished", serde_json::json!({ "status": "COMPLETED" })),
            ] {
                let event = crate::core::ipc::Message {
                    v: "v1".to_string(),
                    id: format!("evt_{name}"),
                    kind: "event".to_string(),
                    name: name.to_string(),
                    payload,
                };
                let line = serde_json::to_string(&event).unwrap();
                write_half.write_all(line.as_bytes()).await.unwrap();
                write_half.write_all(b"\n").await.unwrap();
            }

            let shutdown = lines.next_line().await.unwrap().unwrap();
            let message: crate::core::ipc::Message = serde_json::from_str(&shutdown).unwrap();
            assert_eq!(message.name, "shutdown");
        });

        let (stream, _) = listener.accept().await.unwrap();
        let final_status = WorkflowRunner::run_with_runner(&args, in_process_runner(stream))
            .await
            .unwrap();
        fake_runner.await.unwrap();
        let run = stored_run(project.path()).await;
        let run_started = event_payloads(project.path(), run.id, "run_started").await;
        let session_started =
            event_payloads(project.path(), run.id, "agent_session_started").await;
        let run_finished = event_payloads(project.path(), run.id, "run_finished").await;
        let sessions = stored_sessions(project.path()).await;

        assert_eq!(final_status, RunStatus::Completed);
        assert_eq!(run.status, RunStatus::Completed);
        assert!(run.completed_at.is_some());
        assert_eq!(run_started.len(), 1);
        assert_eq!(session_started.len(), 1);
        assert_eq!(run_finished.len(), 1);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].workflow_run_id, run.id);
        assert_eq!(sessions[0].opencode_session_id, "sess_123");
    }

    #[tokio::test]
    async fn run_records_ipc_parse_errors_from_runner_output() {
        let project = setup_project();
        let workflow_path = project.path().join(".agents/workflows/demo.ts");
        let args = WorkflowArgs {
            project_root: project.path(),
            workflow_name: "demo",
            workflow_path: &workflow_path,
            env: RuntimeEnv::Host,
            yolo: true,
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let fake_runner = tokio::spawn(async move {
            let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            let (read_half, mut write_half) = io::split(stream);
            let mut lines = BufReader::new(read_half).lines();

            lines.next_line().await.unwrap().unwrap();
            write_half.write_all(b"not json\n").await.unwrap();
        });

        let (stream, _) = listener.accept().await.unwrap();
        let final_status = WorkflowRunner::run_with_runner(&args, in_process_runner(stream))
            .await
            .unwrap();
        fake_runner.await.unwrap();
        let run = stored_run(project.path()).await;
        let parse_errors = event_payloads(project.path(), run.id, "ipc_parse_error").await;

        assert_eq!(final_status, RunStatus::Completed);
        assert_eq!(run.status, RunStatus::Pending);
        assert!(run.completed_at.is_none());
        assert_eq!(parse_errors.len(), 1);
        assert!(
            parse_errors[0]
                .get("error")
                .and_then(Value::as_str)
                .is_some_and(|error| error.contains("line 1 column"))
        );
    }
}
