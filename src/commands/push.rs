use std::path::Path;
use crate::git::GitRepo;
use anyhow::Context;
use std::process::Command;

pub fn execute<P>(repo_dir: P) -> anyhow::Result<()> where P: AsRef<Path> {
    let git_repo = GitRepo::open(repo_dir.as_ref()).context("Opening git repository")?;

    let user =  std::env::var("USER").context("No $USER env variable")?;
    let branch_name = format!("{}/{}", user, git_repo.current_branch_name);

    Command::new("git")
        .current_dir(repo_dir.as_ref())
        .arg("push")
        .arg("--no-verify")
        .arg("--force-with-lease")
        .arg("--")
        .arg("origin")
        .arg(format!(
            "{}:refs/heads/{}",
            git_repo.head()?.id(),
            &branch_name
        ))
        .status()?;
    Ok(())
}
