use anyhow::Result;
use clap::Parser;
use toran::cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli::run(cli).await
}
