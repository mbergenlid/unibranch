use std::path::PathBuf;

use clap::{command, Parser, Subcommand};

mod local_commit_changed;
mod rebase_with_conflict;
mod rebased_local_commit_changed;
mod rebased_local_commit_unchanged;
mod remote_branch_changed_local_unchanged;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long)]
    target: PathBuf,

    #[arg(short, long)]
    force: bool,
}

#[derive(Subcommand)]
enum Commands {
    RebasedLocalCommitUnchanged,
    LocalCommitChanged,
    RemoteBranchChangedLocalUnchanged,
    RebasedLocalCommitChanged,
    RebaseWithConflict,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if !cli.target.exists() {
        anyhow::bail!("target directory {} does not exist", cli.target.display());
    }

    if !cli.target.is_dir() {
        anyhow::bail!("target {} must be a directory", cli.target.display());
    }

    let remote_dir = cli.target.join("remote");
    if remote_dir.exists() {
        if cli.force {
            std::fs::remove_dir_all(&remote_dir)?;
        } else {
            anyhow::bail!(
                "remote dir {} already exists, run with '--force' to overwrite",
                remote_dir.display()
            );
        }
    }
    let remote_repo = test_repo::RemoteRepo::new_in(remote_dir);
    let local_dir = cli.target.join("local");
    if local_dir.exists() {
        if cli.force {
            std::fs::remove_dir_all(&local_dir)?;
        } else {
            anyhow::bail!(
                "local dir {} already exists, run with '--force' to overwrite",
                local_dir.display()
            )
        }
    }
    let local_repo = remote_repo.clone_repo_into(local_dir);

    match cli.command {
        Commands::RebasedLocalCommitUnchanged => {
            rebased_local_commit_unchanged::init_repo(&remote_repo, local_repo)
        }
        Commands::LocalCommitChanged => local_commit_changed::init_repo(local_repo),
        Commands::RemoteBranchChangedLocalUnchanged => {
            remote_branch_changed_local_unchanged::init_repo(&remote_repo, local_repo)
        }
        Commands::RebasedLocalCommitChanged => {
            rebased_local_commit_changed::init_repo(&remote_repo, local_repo)
        }
        Commands::RebaseWithConflict => rebase_with_conflict::init_repo(&remote_repo, local_repo),
    };
    Ok(())
}
