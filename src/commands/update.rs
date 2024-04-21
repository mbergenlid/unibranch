use std::{ffi::CString, path::Path};

use anyhow::Context;
use git2::Repository;

pub fn update<T, P>(
    commit_ref: Option<T>,
    branch_commit: Option<T>,
    repo_dir: P,
) -> anyhow::Result<()>
where
    T: AsRef<str>,
    P: AsRef<Path>,
{
    let repo = Repository::open(repo_dir.as_ref()).context("Opening git repository")?;

    let commit_oid = commit_ref.unwrap().as_ref().parse()?;
    let commit = repo.find_commit(commit_oid)?;

    let branch_commit = repo.find_commit(branch_commit.unwrap().as_ref().parse()?)?;
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
    Ok(())
}
