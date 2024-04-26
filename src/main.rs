use clap::{command, Parser, Subcommand};
use spr::commands;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Push(commands::PushOptions),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Push(config) => commands::push(config, ".")?,
    };
    Ok(())
}
