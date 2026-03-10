use clap::{Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RuntimeEnv {
    Host,
    Container,
}

impl RuntimeEnv {
    #[must_use]
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
    /// Create a git worktree and a paired ticket
    Worktree {
        /// Branch name (used as worktree folder name and ticket branch)
        branch: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ContainmentCommands {
    /// Build and start the agent container
    Up,
    /// Stop and remove the agent container
    Down,
}
