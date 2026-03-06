use clap::Parser;

use agentctl::app::cli::{Cli, run};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run(Cli::parse()).await
}
