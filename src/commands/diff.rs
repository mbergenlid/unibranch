use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Context;
use git2::Repository;

use crate::git::GitRepo;

pub fn diff<T, P>(commit_ref: Option<T>, repo_dir: P) -> anyhow::Result<()>
where
    T: AsRef<str>,
    P: AsRef<Path>,
{
    let repo = Repository::open(repo_dir.as_ref()).context("Opening git repository")?;
    let git_repo = GitRepo::open(repo_dir.as_ref())?;

    let commit_oid = match commit_ref {
        Some(commit_ref) => commit_ref
            .as_ref()
            .parse()
            .with_context(|| format!("Invalid OID: {}", commit_ref.as_ref()))?,
        None => repo
            .head()?
            .target()
            .expect("HEAD does not point to a valid commit"),
    };

    let commit = git_repo.find_unpushed_commit_by_id(commit_oid)?;
    let msg = commit.message().unwrap_or("No commit message");
    let title = msg.lines().next().expect("Must have at least one line");
    let branch_name = title.replace(' ', "-").to_ascii_lowercase();

    let base = git_repo.base_commit_id;

    let commit_id = if commit.parent(0).context("Has no parent")?.id() == base {
        //We are right on master
        println!("Creating branch for commit '{}' ({})", title, &branch_name);
        commit.id()
    } else {
        //Need to cherry-pick the commit on top off master
        let cherry_picked_commit = git_repo.cherry_pick_commit(commit)?;
        println!(
            "Creating branch for cherry-picked commit '{}' ({})",
            cherry_picked_commit, &branch_name
        );
        cherry_picked_commit
    };

    let mut cmd = Command::new("git");
    cmd.arg(format!("--git-dir={}/.git", repo_dir.as_ref().display()))
        .arg("push")
        .arg("--no-verify")
        .arg("--")
        .arg("origin")
        .arg(format!("{}:refs/heads/{}", commit_id, &branch_name));

    let exit_status = cmd
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn()?
        .wait()?;

    println!("{}", exit_status);
    Ok(())
}
