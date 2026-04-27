use anyhow::Result;
use clap::Parser;
use sandprint::cli::Cli;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    init_logging();
    Cli::parse().run()
}

fn init_logging() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("sandprint=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();
}
