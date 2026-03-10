use clap::{Parser, Subcommand};

use crate::app::commands;
use crate::app::types::{ContainmentCommands, MakeCommands, RuntimeEnv};

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
        /// Run inside a container with dangerous mode enabled
        #[arg(long, default_value_t = false)]
        containment: bool,
    },
    /// Open the `OpenCode` web UI for this project
    Manage,
    /// Generate project artifacts
    Make {
        #[command(subcommand)]
        command: MakeCommands,
    },
    /// Manage containment containers
    Containment {
        #[command(subcommand)]
        command: ContainmentCommands,
    },
}

/// # Errors
/// Returns an error if the dispatched subcommand fails.
pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init => commands::init::run().await,
        Commands::Work {
            name,
            env,
            yolo,
            containment,
        } => {
            let (effective_env, effective_yolo) = resolve_work_flags(env, yolo, containment)?;
            commands::work::run(name, effective_env, effective_yolo).await
        }
        Commands::Manage => commands::manage::run(),
        Commands::Make { command } => match command {
            MakeCommands::Ticket => commands::make::ticket(),
            MakeCommands::Worktree { branch } => commands::make::worktree(&branch),
        },
        Commands::Containment { command } => match command {
            ContainmentCommands::Up => commands::containment::up(),
            ContainmentCommands::Down => commands::containment::down(),
        },
    }
}

fn resolve_work_flags(
    env: RuntimeEnv,
    yolo: bool,
    containment: bool,
) -> anyhow::Result<(RuntimeEnv, bool)> {
    if !containment {
        return Ok((env, yolo));
    }

    if env != RuntimeEnv::Host || yolo {
        anyhow::bail!("--containment cannot be combined with --env or --yolo");
    }

    Ok((RuntimeEnv::Container, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_command_uses_expected_defaults() {
        let cli = Cli::try_parse_from(["agentctl", "work", "duos"]).unwrap();

        match cli.command {
            Commands::Work {
                name,
                env,
                yolo,
                containment,
            } => {
                assert_eq!(name, "duos");
                assert_eq!(env, RuntimeEnv::Host);
                assert!(!yolo);
                assert!(!containment);
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
    fn work_command_parses_containment_flag() {
        let cli = Cli::try_parse_from(["agentctl", "work", "duos", "--containment"]).unwrap();

        match cli.command {
            Commands::Work { containment, .. } => assert!(containment),
            _ => panic!("expected work command"),
        }
    }

    #[test]
    fn containment_resolves_to_container_and_yolo() {
        let (env, yolo) = resolve_work_flags(RuntimeEnv::Host, false, true).unwrap();

        assert_eq!(env, RuntimeEnv::Container);
        assert!(yolo);
    }

    #[test]
    fn containment_conflicts_with_explicit_env() {
        let result = resolve_work_flags(RuntimeEnv::Container, false, true);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("--containment"));
    }

    #[test]
    fn containment_conflicts_with_explicit_yolo() {
        let result = resolve_work_flags(RuntimeEnv::Host, true, true);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("--containment"));
    }

    #[test]
    fn containment_up_command_parses() {
        let cli = Cli::try_parse_from(["agentctl", "containment", "up"]).unwrap();

        assert!(matches!(
            cli.command,
            Commands::Containment {
                command: ContainmentCommands::Up
            }
        ));
    }

    #[test]
    fn containment_down_command_parses() {
        let cli = Cli::try_parse_from(["agentctl", "containment", "down"]).unwrap();

        assert!(matches!(
            cli.command,
            Commands::Containment {
                command: ContainmentCommands::Down
            }
        ));
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
