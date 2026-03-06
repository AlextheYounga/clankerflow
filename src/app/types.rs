use clap::{Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RuntimeEnv {
    Host,
    Container,
}

impl RuntimeEnv {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Container => "container",
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum MakeCommands {
    /// Create a new ticket
    Ticket,
    /// Create worktree and related artifacts
    Worktree {
        /// Branch name and worktree folder name
        branch: String,
    },
    /// Validate workflows in .agents/workflows
    Validate,
}
