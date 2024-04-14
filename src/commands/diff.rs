use std::{
    ffi::CString,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Context;
use git2::Repository;

pub fn diff<T, P>(commit_ref: Option<T>, repo_dir: P) -> anyhow::Result<()>
where
    T: AsRef<str>,
    P: AsRef<Path>,
{
    let repo = Repository::open(repo_dir.as_ref()).context("Opening git repository")?;

    let master_ref = format!("refs/remotes/origin/master");

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

    let mut walk = repo.revwalk()?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL.union(git2::Sort::REVERSE))?;
    walk.push_head()?;
    walk.hide_ref(&master_ref)?;

    let commits = walk.collect::<Result<Vec<_>, _>>()?;
    let commit: git2::Oid = commits
        .into_iter()
        .find(|&oid| oid == commit_oid)
        .with_context(|| format!("Unable to find revision {}", commit_oid))?;

    let commit = repo.find_commit(commit)?;
    let msg = commit.message().unwrap_or("No commit message");
    let title = msg.lines().next().expect("Must have at least one line");
    let branch_name = title.replace(" ", "-").to_ascii_lowercase();

    let base = repo.refname_to_id(&master_ref)?;

    let commit_id = if commit.parent(0).context("Has no parent")?.id() == base {
        //We are right on master
        println!("Creating branch for commit '{}' ({})", title, &branch_name);
        commit.id()
    } else {
        //Need to cherry-pick the commit on top off master
        let mut index = repo.cherrypick_commit(&commit, &repo.find_commit(base)?, 0, None)?;
        if index.has_conflicts() {
            for c in index.conflicts()? {
                let c = c?;
                println!("Conclict {:?}", CString::new(c.our.unwrap().path).unwrap())
            }
            anyhow::bail!("This commit cannot be cherry-picked on {base}");
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
        let base_commit = repo.find_commit(base)?;
        let cherry_picked_commit = repo
            .commit(
                None,
                &author,
                &committer,
                commit.message().expect("No commit message"),
                &tree,
                &[&base_commit],
            )
            .context("Committing")?;
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
