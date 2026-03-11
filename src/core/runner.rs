mod env;
pub mod ipc_loop;
pub mod protocol;
pub mod signal;
pub mod store;

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
        }
    }

    fn take_ipc_channel(&mut self) -> Result<TcpStream> {
        self.ipc
            .take()
            .ok_or_else(|| anyhow!("IPC channel not available"))
    }
}
