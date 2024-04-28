use clap::{command, Parser, Subcommand};
use spr::commands::{cherry_pick, push};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    CherryPick(cherry_pick::Options),
    Push,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let _a = 32;
    match cli.command {
        Commands::CherryPick(config) => cherry_pick::execute(config, ".")?,
        Commands::Push => push::execute(".")?,
    };
    Ok(())
}
