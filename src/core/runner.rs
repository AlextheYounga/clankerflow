mod env;
pub mod ipc_loop;
pub mod protocol;
pub mod signal;
pub mod store;

use std::env::current_exe;
use std::path::Path;
use std::process;
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
use crate::core::opencode::{Gateway, server};
use crate::core::tmux;
use crate::db::connection::connect;
use crate::db::entities::workflow_run::RunStatus;

use env::{parse_runtime_env, spawn_container_runner, spawn_host_runner};
use ipc_loop::{Context, drive, send_start_run};
use protocol::send_shutdown;
use signal::{CancelState, install_signal_handler, wait_for_child};
use store::{create_run, set_pid, upsert_workflow};

/// Parameters for running a workflow synchronously.
pub struct WorkflowArgs<'a> {
    pub project_root: &'a Path,
    pub workflow_name: &'a str,
    pub workflow_path: &'a Path,
    pub env: RuntimeEnv,
    pub yolo: bool,
}

pub struct RunLaunch {
    pub run_id: i64,
    pub project_session_name: String,
    pub run_window_name: String,
}

pub struct WorkflowEngine {
    process: RunnerProcess,
    /// Bidirectional TCP IPC channel. Taken by `WorkflowEngine::run` after spawn;
    /// `None` once consumed.
    ipc: Option<TcpStream>,
}

enum RunnerProcess {
    Child(Child),
}

impl WorkflowEngine {
    /// Prepare a run record without executing the runner.
    ///
    /// # Errors
    /// Returns an error if database operations fail.
    pub async fn prepare_run(args: &WorkflowArgs<'_>) -> Result<i64> {
        let db = connect(args.project_root).await?;
        let workflow_env = parse_runtime_env(args.env);
        let workflow_id = upsert_workflow(&db, args.workflow_name, args.workflow_path).await?;
        let run_id = create_run(&db, workflow_id, workflow_env).await?;
        Ok(run_id)
    }

    /// Launch a workflow inside the project tmux session.
    ///
    /// # Errors
    /// Returns an error if the run record cannot be prepared or tmux setup fails.
    pub async fn launch_in_tmux(args: &WorkflowArgs<'_>) -> Result<RunLaunch> {
        let run_id = Self::prepare_run(args).await?;
        let codebase_id = codebase_id::derive(args.project_root);
        let project_session_name = tmux::project_session_name(&codebase_id);
        let command = worker_command(args, run_id)?;
        let run_window_name = tmux::create_run_window(
            args.workflow_name,
            tmux::WindowTarget { session_name: &project_session_name, run_id, work_dir: args.project_root },
            &command,
        )?;

        Ok(RunLaunch { run_id, project_session_name, run_window_name })
    }

    /// Run a workflow to completion and return its final status.
    ///
    /// # Errors
    /// Returns an error if runner setup fails, IPC communication fails, or
    /// process/database operations fail during execution.
    pub async fn run(args: &WorkflowArgs<'_>) -> Result<RunStatus> {
        let run_id = Self::prepare_run(args).await?;
        Self::run_existing(args, run_id).await
    }

    /// Run an already-created workflow to completion and return its final status.
    ///
    /// # Errors
    /// Returns an error if runner setup fails, IPC communication fails, or
    /// process/database operations fail during execution.
    pub async fn run_existing(args: &WorkflowArgs<'_>, run_id: i64) -> Result<RunStatus> {
        let ctx = Self::create_run_context(args, run_id).await?;
        let codebase_id = codebase_id::derive(args.project_root);
        let runner = Self::spawn_process(args.project_root, args.env, &codebase_id).await?;
        Self::run_with_context(args, ctx, runner).await
    }

    async fn run_with_context(args: &WorkflowArgs<'_>, ctx: Context, mut runner: Self) -> Result<RunStatus> {
        let _ = args;
        let ipc = runner.take_ipc_channel()?;
        let (ipc_read, mut ipc_write) = io::split(ipc);

        send_start_run(&mut ipc_write, args, ctx.run_id).await?;
        install_signal_handler(&ctx.cancel);

        let final_status = drive(&ctx, &mut ipc_write, ipc_read).await?;

        send_shutdown(&mut ipc_write).await;
        drop(ipc_write);
        runner.wait_for_exit(&ctx.cancel).await?;

        Ok(final_status)
    }

    async fn create_run_context(args: &WorkflowArgs<'_>, run_id: i64) -> Result<Context> {
        let db = connect(args.project_root).await?;

        println!("workflow started (run id: {run_id})");
        set_pid(&db, run_id, i64::from(process::id())).await?;

        let cancel = Arc::new(CancelState { cancelled: AtomicBool::new(false), force_kill: AtomicBool::new(false) });

        server::ensure_running().await?;
        let opencode = Gateway::from_project_root(args.project_root)?;
        let project_session_name = tmux::project_session_name(&codebase_id::derive(args.project_root));

        Ok(Context {
            db,
            run_id,
            cancel,
            opencode,
            project_session_name,
            project_root: args.project_root.to_path_buf(),
        })
    }

    async fn spawn_process(project_root: &Path, env: RuntimeEnv, codebase_id: &str) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();

        let child = match env {
            RuntimeEnv::Host => spawn_host_runner(project_root, port)?,
            RuntimeEnv::Container => spawn_container_runner(project_root, codebase_id, port).await?,
        };

        let (stream, _) = timeout(Duration::from_secs(30), listener.accept())
            .await
            .map_err(|_| anyhow!("timed out waiting for runner to connect (is Docker running and the image built?)"))?
            .map_err(|e| anyhow!("failed to accept runner connection: {e}"))?;

        Ok(Self { process: RunnerProcess::Child(child), ipc: Some(stream) })
    }

    async fn wait_for_exit(&mut self, cancel: &Arc<CancelState>) -> Result<()> {
        match &mut self.process {
            RunnerProcess::Child(child) => wait_for_child(child, cancel).await,
        }
    }

    fn take_ipc_channel(&mut self) -> Result<TcpStream> {
        self.ipc.take().ok_or_else(|| anyhow!("IPC channel not available"))
    }
}

fn worker_command(args: &WorkflowArgs<'_>, run_id: i64) -> Result<String> {
    let executable = current_exe().map_err(|error| anyhow!("failed to resolve clankerflow executable: {error}"))?;

    let mut parts = vec![
        shell_escape(executable.to_string_lossy().as_ref()),
        "_run".to_string(),
        "--run-id".to_string(),
        run_id.to_string(),
        "--workflow-name".to_string(),
        shell_escape(args.workflow_name),
        "--workflow-path".to_string(),
        shell_escape(args.workflow_path.to_string_lossy().as_ref()),
        "--env".to_string(),
        args.env.as_str().to_string(),
        "--project-root".to_string(),
        shell_escape(args.project_root.to_string_lossy().as_ref()),
    ];

    if args.yolo {
        parts.push("--yolo".to_string());
    }

    Ok(parts.join(" "))
}

fn shell_escape(value: &str) -> String {
    let escaped = value.replace('"', "\\\"");
    format!("\"{escaped}\"")
}
