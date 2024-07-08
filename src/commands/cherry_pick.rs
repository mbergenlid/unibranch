use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Context;

use crate::git::{local_commit::CommitMetadata, GitRepo};

#[derive(clap::Parser)]
pub struct Options {
    #[arg(short, long)]
    pub dry_run: bool,
    #[arg(short, long)]
    pub rebase: bool,

    pub commit_ref: Option<String>,
}

pub fn execute<P>(config: Options, repo_dir: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let git_repo = GitRepo::open(repo_dir.as_ref()).context("Opening git repository")?;

    let commit = if let Some(rev) = config.commit_ref {
        git_repo.find_unpushed_commit(&rev)?
    } else {
        git_repo
            .head()
            .context("HEAD does not point to a valid commit")?
    };

    let meta_data_note = git_repo.find_note_for_commit(commit.id())?;

    let meta_data: CommitMetadata = meta_data_note
        .as_ref()
        .and_then(|n| n.message().expect("Not valid UTF-8").try_into().ok())
        .unwrap_or_else(|| {
            let msg = commit.message().unwrap_or("No commit message");
            let title = msg.lines().next().expect("Must have at least one line");
            let branch_name = title
                .replace(
                    |c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                    "-",
                )
                .to_ascii_lowercase();
            CommitMetadata {
                remote_branch_name: std::borrow::Cow::Owned(branch_name),
            }
        });

    let pr_commit = if config.rebase {
        None
    } else {
        git_repo.find_head_of_remote_branch(&meta_data.remote_branch_name)
    };
    match git_repo.cherry_pick_commit(commit.clone(), pr_commit) {
        Ok(Some(cherry_picked_commit)) => {
            let mut cmd = Command::new("git");
            if config.dry_run {
                println!(
                    "Dry run mode, will not push {} to remote branch 'origin/{}'",
                    cherry_picked_commit.id(),
                    meta_data.remote_branch_name
                );
                return Ok(());
            }
            if meta_data.is_modified() {
                git_repo.save_meta_data(&commit, &meta_data)?;
            }
            cmd.current_dir(repo_dir.as_ref())
                .arg("push")
                .arg("--no-verify")
                .arg("--force-with-lease")
                .arg("--")
                .arg("origin")
                .arg(format!(
                    "{}:refs/heads/{}",
                    cherry_picked_commit.id(),
                    meta_data.remote_branch_name
                ));

            let _exit_status = cmd
                .stderr(Stdio::inherit())
                .stdout(Stdio::inherit())
                .spawn()?
                .wait()?;
        }
        Ok(None) => todo!(),
        Err(e) => {
            eprintln!("{:?}", e);
            eprintln!("Diff doesn't apply cleanly on master")
        }
    };

    Ok(())
}
