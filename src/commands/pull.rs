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

    let original_commit = repo.head()?;

    let local_branch_head = repo.find_local_branch_commit(&original_commit)?;
    //First, update 'local' branch with local changes.
    let local_branch_head = repo
        .cherry_pick_commit(&original_commit, Some(local_branch_head.clone()))?
        .unwrap_or(local_branch_head);


    repo.update(original_commit, &local_branch_head)
}
