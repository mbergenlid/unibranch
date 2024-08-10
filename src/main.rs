use clap::{command, Parser, Subcommand};
use sc::commands::{create, pull, push};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Create(create::Options),
    Pull,
    Push,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create(config) => create::execute(config, ".")?,
        Commands::Pull => pull::execute(".")?,
        Commands::Push => push::execute(".")?,
    };
    Ok(())
}
