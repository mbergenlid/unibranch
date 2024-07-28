use std::{path::Path, process::Command};

use anyhow::Context;

use crate::git::GitRepo;


pub fn execute<P>(repo_dir: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let repo = GitRepo::open(repo_dir.as_ref()).context("Opening git repository")?;

    Command::new("git")
        .current_dir(repo_dir.as_ref())
        .arg("fetch")
        .status()
        .context("git fetch")?;

    let head = repo.head()?;
    repo.update(head)
}
