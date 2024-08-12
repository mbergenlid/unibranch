use clap::{command, Parser, Subcommand};
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
    let cli = Cli::parse();

    match cli.command {
        Commands::Create(config) => create::execute(config, ".")?,
        Commands::Pull(config) => pull::execute(config, ".")?,
        Commands::Push => push::execute(".")?,
    };
    Ok(())
}
