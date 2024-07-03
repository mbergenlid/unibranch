use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Context;

use crate::git::GitRepo;

#[derive(clap::Parser)]
pub struct Options {
    #[arg(short, long)]
    pub dry_run: bool,
    #[arg(short, long)]
    pub rebase: bool,

    pub commit_ref: Option<String>,
}

//TODO: Put reference to the remote branch name in the local commit
pub fn execute<P>(config: Options, repo_dir: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let git_repo = GitRepo::open(repo_dir.as_ref()).context("Opening git repository")?;

    let commit_oid = match &config.commit_ref {
        Some(commit_ref) => commit_ref
            .parse()
            .with_context(|| format!("Invalid OID: {}", commit_ref))?,
        None => git_repo
            .head()
            .context("HEAD does not point to a valid commit")?
            .id(),
    };

    let commit = git_repo.find_unpushed_commit_by_id(commit_oid)?;
    let msg = commit.message().unwrap_or("No commit message");
    let title = msg.lines().next().expect("Must have at least one line");
    let branch_name = title
        .replace(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'), "-")
        .to_ascii_lowercase();

    let pr_commit = if config.rebase {
        None
    } else {
        git_repo.find_head_of_remote_branch(&branch_name)
    };
    match git_repo.cherry_pick_commit(commit, pr_commit) {
        Ok(Some(cherry_picked_commit)) => {
            let mut cmd = Command::new("git");
            if config.dry_run {
                println!(
                    "Dry run mode, will not push {} to remote branch 'origin/{}'",
                    cherry_picked_commit.id(),
                    branch_name
                );
                return Ok(());
            }
            cmd
                .current_dir(repo_dir.as_ref())
                .arg("push")
                .arg("--no-verify")
                .arg("--force-with-lease")
                .arg("--")
                .arg("origin")
                .arg(format!(
                    "{}:refs/heads/{}",
                    cherry_picked_commit.id(),
                    &branch_name
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
