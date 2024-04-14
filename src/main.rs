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
    Diff { commit_ref: Option<String> },
}
const REPO_DIR: &'static str = "/Users/mbergenlid/Development/fun/stackable-prs-demo";

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Diff { commit_ref } => commands::diff::diff(commit_ref.as_ref(), REPO_DIR)?,
    };
    Ok(())
}
