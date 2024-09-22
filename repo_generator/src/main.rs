use std::path::PathBuf;

use clap::{command, Parser, Subcommand};
use indoc::indoc;

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
    Generate,
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
        Commands::Generate => local_repo
            .create_file(
                "File1",
                indoc! {"
                Hello World!

                This is my very first file
                "},
            )
            .commit_all("First commit")
            .push()
            .create_file(
                "File1",
                indoc! {"
            Hello World!

            More lines..

            This is my very first file
            "},
            ).commit_all("add more lines"),
    };
    Ok(())
}
