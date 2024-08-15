use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Context;

use crate::git::{local_commit::MainCommit, GitRepo};

#[derive(clap::Parser, Default)]
pub struct Options {
    #[arg(short, long)]
    cont: bool,

    #[arg(short, long)]
    dry_run: bool,
}

//TODO: Rename to 'update' or 'sync' or something

pub fn execute<P>(options: Options, repo_dir: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let repo = GitRepo::open(repo_dir.as_ref()).context("Opening git repository")?;

    Command::new("git")
        .current_dir(repo_dir.as_ref())
        .arg("fetch")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("git fetch")?;

    let mut parent_commit = repo.base_commit()?;
    for original_commit in repo.unpushed_commits().unwrap() {
        match original_commit {
            MainCommit::Tracked(tracked_commit) => {
                let new_parent_1 = tracked_commit
                    .update_local_branch_head()?
                    .merge_remote_head(Some(&parent_commit))?
                    .sync_with_main()?;

                if !options.dry_run {
                    Command::new("git")
                        .current_dir(repo_dir.as_ref())
                        .arg("push")
                        .arg("--no-verify")
                        .arg("--force-with-lease")
                        .arg("--")
                        .arg("origin")
                        .arg(format!(
                            "{}:refs/heads/{}",
                            new_parent_1.local_branch_head()?.id(),
                            new_parent_1.meta_data().remote_branch_name
                        ))
                        .stderr(Stdio::null())
                        .stdout(Stdio::null())
                        .status()?;
                } else {
                    println!(
                        "Dry-run: Push {} as new head of remote branch {}",
                        new_parent_1.local_branch_head()?.id(),
                        new_parent_1.meta_data().remote_branch_name
                    );
                }
                parent_commit = new_parent_1.commit();
            }
            MainCommit::UnTracked(local_commit) => {
                let rebased_commit = local_commit.rebase(&parent_commit)?;
                parent_commit = rebased_commit.commit();
            }
        }
    }

    if !options.dry_run {
        repo.update_current_branch(&parent_commit)?;
    } else {
        println!("Dry-run: Update HEAD to {}", parent_commit.id());
    }

    Ok(())
}
