use std::{ffi::CString, path::Path};

use anyhow::{Context, Ok};
use git2::{Commit, Oid, Repository};

pub struct GitRepo {
    repo: git2::Repository,
    pub base_commit_id: Oid,
}

impl GitRepo {
    pub fn open<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let repo = Repository::open(path.as_ref()).context("Opening git repository")?;
        //TODO: make 'master' configurable
        let master_ref = format!("refs/remotes/origin/{}", "master");
        let base_commit_id = repo.refname_to_id(&master_ref)?;

        Ok(GitRepo {
            repo,
            base_commit_id,
        })
    }

    pub fn find_unpushed_commit_by_id(&self, id: Oid) -> anyhow::Result<Commit> {
        let commit: git2::Oid = self
            .unpushed_commits()?
            .into_iter()
            .find(|&oid| oid == id)
            .with_context(|| format!("Unable to find revision {}", id))?;

        Ok(self.repo.find_commit(commit)?)
    }

    pub fn unpushed_commits(&self) -> anyhow::Result<Vec<Oid>> {
        let mut walk = self.repo.revwalk()?;
        walk.set_sorting(git2::Sort::TOPOLOGICAL.union(git2::Sort::REVERSE))?;
        walk.push_head()?;
        walk.hide(self.base_commit_id)?;

        Ok(walk.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn cherry_pick_commit(&self, commit: Commit) -> anyhow::Result<Oid> {
        //Need to cherry-pick the commit on top off master
        let mut index = self.repo.cherrypick_commit(
            &commit,
            &self.repo.find_commit(self.base_commit_id)?,
            0,
            None,
        )?;
        if index.has_conflicts() {
            for c in index.conflicts()? {
                let c = c?;
                println!("Conclict {:?}", CString::new(c.our.unwrap().path).unwrap())
            }
            anyhow::bail!(
                "This commit cannot be cherry-picked on {}",
                self.base_commit_id
            );
        }

        let tree = index
            .write_tree_to(&self.repo)
            .context("Write index to tree")?;
        println!("Writing tree of cherry-pick {}", tree);
        let tree = self
            .repo
            .find_tree(tree)
            .context("Can not find tree just created")?;
        let committer = self.repo.signature().or_else(|_| {
            git2::Signature::now(
                String::from_utf8_lossy(commit.committer().name_bytes()).as_ref(),
                String::from_utf8_lossy(commit.committer().email_bytes()).as_ref(),
            )
        })?;

        let author = git2::Signature::now(
            String::from_utf8_lossy(commit.author().name_bytes()).as_ref(),
            String::from_utf8_lossy(commit.author().email_bytes()).as_ref(),
        )?;
        let base_commit = self.repo.find_commit(self.base_commit_id)?;
        let cherry_picked_commit = self
            .repo
            .commit(
                None,
                &author,
                &committer,
                commit.message().expect("No commit message"),
                &tree,
                &[&base_commit],
            )
            .context("Committing")?;
        Ok(cherry_picked_commit)
    }
}
