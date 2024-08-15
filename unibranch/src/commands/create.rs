use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Context;

use crate::git::{
    local_commit::MainCommit,
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

    let tracked_commit = untracked_commit.track()?;
    if config.dry_run {
        println!(
            "Dry run mode, will not push {} to remote branch 'origin/{}'",
            tracked_commit.meta_data().remote_commit.unwrap(),
            &tracked_commit.meta_data().remote_branch_name,
        );
        return Ok(());
    }
    Command::new("git")
        .current_dir(repo_dir.as_ref())
        .arg("push")
        .arg("--no-verify")
        .arg("--force-with-lease")
        .arg("--")
        .arg("origin")
        .arg(format!(
            "{}:refs/heads/{}",
            tracked_commit.meta_data().remote_commit.unwrap(),
            &tracked_commit.meta_data().remote_branch_name
        ))
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .status()?;

    Ok(())
}
