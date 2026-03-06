use clap::{Parser, Subcommand};

use crate::app::types::{MakeCommands, RuntimeEnv};
use crate::app::commands;

#[derive(Debug, Parser)]
#[command(name = "agentctl", about = "AI workflow orchestration CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize agentctl in current directory
    Init,
    /// Start a workflow run
    Work {
        /// Workflow name (without extension)
        name: String,
        /// Runtime target
        #[arg(short, long, value_enum, default_value_t = RuntimeEnv::Host)]
        env: RuntimeEnv,
        /// Disable safety checks (dangerous; use only in container mode)
        #[arg(long, default_value_t = false)]
        yolo: bool,
    },
    /// TUI for managing workflow runs
    Manage,
    /// Generate project artifacts
    Make {
        #[command(subcommand)]
        command: MakeCommands,
    },
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init => commands::init::run().await,
        Commands::Work { name, env, yolo } => commands::work::run(name, env, yolo).await,
        Commands::Manage => commands::manage::run().await,
        Commands::Make { command } => match command {
            MakeCommands::Ticket => commands::make::ticket().await,
            MakeCommands::Worktree { branch } => commands::make::worktree(branch).await,
        },
    }
}
