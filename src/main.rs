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
    Diff {
        commit_ref: Option<String>,
    },
    Update {
        commit_ref: Option<String>,
        branch_commit: Option<String>,
    },
}
const REPO_DIR: &str = "/tmp/test-repo";

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Diff { commit_ref } => commands::diff::diff(commit_ref.as_ref(), REPO_DIR)?,
        Commands::Update {
            commit_ref,
            branch_commit,
        } => commands::update::update(commit_ref.as_ref(), branch_commit.as_ref(), REPO_DIR)?,
    };
    Ok(())
}
