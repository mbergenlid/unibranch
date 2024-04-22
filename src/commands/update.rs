use std::{
    ffi::CString,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Context;
use git2::Repository;

pub fn update<T1, T2, P>(commit_ref: Option<T1>, branch_ref: T2, repo_dir: P) -> anyhow::Result<()>
where
    T1: AsRef<str>,
    T2: AsRef<str>,
    P: AsRef<Path>,
{
    let repo = Repository::open(repo_dir.as_ref()).context("Opening git repository")?;

    let commit_oid = commit_ref.unwrap().as_ref().parse()?;
    let commit = repo.find_commit(commit_oid)?;

    let branch_commit = repo
        .find_reference(&format!("refs/remotes/origin/{}", branch_ref.as_ref()))
        .context("Find reference")?
        .peel_to_commit()
        .context("Peel to commit")?;
    let diff = repo.diff_tree_to_tree(
        branch_commit.tree().ok().as_ref(),
        commit.tree().ok().as_ref(),
        None,
    )?;

    let mut index = repo.apply_to_tree(&branch_commit.tree().unwrap(), &diff, None)?;

    if index.has_conflicts() {
        for c in index.conflicts()? {
            let c = c?;
            println!("Conclict {:?}", CString::new(c.our.unwrap().path).unwrap())
        }
        anyhow::bail!("Failed to update");
    }

    let tree = index.write_tree_to(&repo).context("Write index to tree")?;
    println!("Writing tree of cherry-pick {}", tree);
    let tree = repo
        .find_tree(tree)
        .context("Can not find tree just created")?;
    let committer = repo.signature().or_else(|_| {
        git2::Signature::now(
            String::from_utf8_lossy(commit.committer().name_bytes()).as_ref(),
            String::from_utf8_lossy(commit.committer().email_bytes()).as_ref(),
        )
    })?;

    let author = git2::Signature::now(
        String::from_utf8_lossy(commit.author().name_bytes()).as_ref(),
        String::from_utf8_lossy(commit.author().email_bytes()).as_ref(),
    )?;
    let base_commit = branch_commit;
    let cherry_picked_commit = repo
        .commit(
            None,
            &author,
            &committer,
            &format!(
                "Updates to commit: \"{}\"",
                commit.message().expect("No commit message")
            ),
            &tree,
            &[&base_commit],
        )
        .context("Committing")?;

    println!("Created patch commit {}", cherry_picked_commit);

    let mut cmd = Command::new("git");
    cmd.arg(format!("--git-dir={}/.git", repo_dir.as_ref().display()))
        .arg("push")
        .arg("--no-verify")
        .arg("--")
        .arg("origin")
        .arg(format!(
            "{}:refs/heads/{}",
            cherry_picked_commit,
            branch_ref.as_ref()
        ));

    let exit_status = cmd
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn()?
        .wait()?;

    println!("{}", exit_status);
    Ok(())
}
