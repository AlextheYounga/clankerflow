use clap::{Parser, Subcommand};

use crate::app::commands;
use crate::app::types::{MakeCommands, RuntimeEnv};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_command_uses_expected_defaults() {
        let cli = Cli::try_parse_from(["agentctl", "work", "duos"]).unwrap();

        match cli.command {
            Commands::Work { name, env, yolo } => {
                assert_eq!(name, "duos");
                assert_eq!(env, RuntimeEnv::Host);
                assert!(!yolo);
            }
            _ => panic!("expected work command"),
        }
    }

    #[test]
    fn work_command_parses_container_env() {
        let cli = Cli::try_parse_from(["agentctl", "work", "duos", "--env", "container"]).unwrap();

        match cli.command {
            Commands::Work { env, .. } => assert_eq!(env, RuntimeEnv::Container),
            _ => panic!("expected work command"),
        }
    }

    #[test]
    fn work_command_rejects_invalid_env() {
        let result = Cli::try_parse_from(["agentctl", "work", "duos", "--env", "bad-env"]);

        assert!(result.is_err());
    }

    #[test]
    fn make_worktree_command_parses_branch() {
        let cli = Cli::try_parse_from(["agentctl", "make", "worktree", "feat/new-branch"]).unwrap();

        match cli.command {
            Commands::Make { command } => match command {
                MakeCommands::Worktree { branch } => assert_eq!(branch, "feat/new-branch"),
                _ => panic!("expected make worktree command"),
            },
            _ => panic!("expected make command"),
        }
    }
}
