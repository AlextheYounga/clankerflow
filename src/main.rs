use clap::Parser;

use agentctl::app::cli::{run, Cli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run(Cli::parse()).await
}
