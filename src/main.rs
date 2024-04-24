use clap::{command, Parser, Subcommand};
use stackable_commits::commands;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Push { commit_ref: Option<String> },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Push { commit_ref } => commands::push(commit_ref.as_ref(), ".")?,
    };
    Ok(())
}
