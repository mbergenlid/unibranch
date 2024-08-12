use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Context;

use crate::git::{
    local_commit::{CommitMetadata, MainCommit},
    GitRepo,
};

#[derive(clap::Parser, Default)]
pub struct Options {
    #[arg(short, long)]
    pub dry_run: bool,

    pub commit_ref: Option<String>,
}

pub fn execute<P>(config: Options, repo_dir: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let git_repo = GitRepo::open(repo_dir.as_ref()).context("Opening git repository")?;

    let rev = config.commit_ref.unwrap_or_else(|| "HEAD".to_string());
    let commit = git_repo.find_unpushed_commit(&rev)?;

    let untracked_commit = match commit {
        MainCommit::UnTracked(commit) => commit,
        MainCommit::Tracked(_) => anyhow::bail!("Commit is already tracked"),
    };

    let branch_name = {
        let msg = untracked_commit
            .as_commit()
            .message()
            .unwrap_or("No commit message");
        let title = msg.lines().next().expect("Must have at least one line");
        title
            .replace(
                |c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "-",
            )
            .to_ascii_lowercase()
    };

    let pr_commit = git_repo.find_head_of_remote_branch(&branch_name);
    if pr_commit.is_some() {
        anyhow::bail!(format!("Remote branch '{}' already exist", branch_name));
    }
    match git_repo.cherry_pick_commit(untracked_commit.as_commit(), pr_commit) {
        Ok(Some(cherry_picked_commit)) => {
            let mut cmd = Command::new("git");
            if config.dry_run {
                println!(
                    "Dry run mode, will not push {} to remote branch 'origin/{}'",
                    cherry_picked_commit.id(),
                    branch_name,
                );
                return Ok(());
            }
            let new_meta_data = CommitMetadata {
                remote_branch_name: branch_name.into(),
                remote_commit: Some(cherry_picked_commit.id()),
            };
            git_repo.save_meta_data(untracked_commit.as_commit(), &new_meta_data)?;
            cmd.current_dir(repo_dir.as_ref())
                .arg("push")
                .arg("--no-verify")
                .arg("--force-with-lease")
                .arg("--")
                .arg("origin")
                .arg(format!(
                    "{}:refs/heads/{}",
                    cherry_picked_commit.id(),
                    new_meta_data.remote_branch_name
                ));

            let _exit_status = cmd
                .stderr(Stdio::null())
                .stdout(Stdio::null())
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
