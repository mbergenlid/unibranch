use clap::{command, Parser, Subcommand};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;
use ubr::commands::{create, pull, push};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Create(create::Options),
    Pull(pull::Options),
    Push,
}

fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(false)
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    let cli = Cli::parse();


    match cli.command {
        Commands::Create(config) => create::execute(config, ".")?,
        Commands::Pull(config) => pull::execute(config, ".")?,
        Commands::Push => push::execute(".")?,
    };
    Ok(())
}
