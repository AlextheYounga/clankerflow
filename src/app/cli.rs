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
        command: crate::types::MakeCommands,
    },

}
