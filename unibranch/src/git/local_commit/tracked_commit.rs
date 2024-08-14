use std::ffi::CString;

use anyhow::Context;
use anyhow::Ok;
use git2::{Branch, Commit, FileFavor, MergeOptions, Oid, Repository};


use super::CommitMetadata;
use super::GitRepo;

pub struct TrackedCommit<'repo> {
    repo: &'repo Repository,
    git_repo: &'repo GitRepo,
    commit: Commit<'repo>,
    meta_data: CommitMetadata<'repo>,
}

impl<'repo> TrackedCommit<'repo> {
    pub fn new(
        repo: &'repo Repository,
        git_repo: &'repo GitRepo,
        commit: Commit<'repo>,
        meta_data: CommitMetadata<'repo>,
    ) -> Self {
        Self {
            repo,
            git_repo,
            commit,
            meta_data,
        }
    }

    pub fn remote_branch(&self) -> anyhow::Result<Branch> {
        let remote_branch = self
            .repo
            .find_branch(
                &format!("origin/{}", self.meta_data.remote_branch_name),
                git2::BranchType::Remote,
            )
            .context("Find the remote branch")?;
        Ok(remote_branch)
    }

    pub fn local_branch_head(&self) -> anyhow::Result<Commit> {
        let commit_meta_data = &self.meta_data;
        let local_branch_commit = if let Some(remote_commit_id) = commit_meta_data.remote_commit {
            self.repo.find_commit(remote_commit_id)?
        } else {
            self.repo
                .find_branch(
                    &format!("origin/{}", commit_meta_data.remote_branch_name),
                    git2::BranchType::Remote,
                )?
                .get()
                .peel_to_commit()?
        };
        Ok(local_branch_commit)
    }

    pub fn as_commit(&self) -> &Commit {
        &self.commit
    }

    pub fn commit(self) -> Commit<'repo> {
        self.commit
    }

    pub fn meta_data(&self) -> &CommitMetadata {
        &self.meta_data
    }

    //
    // Apply the diff between this commit and the self.meta_data.remote_commit
    // and return the new TrackedCommit
    //
    //
    //              *
    //              |    * (Merge)
    //              |   / \
    //              *  /   * (remote_branch_head)
    //              | * <-/------------------------ cherry-pick c1 local_branch_head (resolve conflicts by accepting theirs)
    //              |  \ /
    //        c1    *   * (local_branch_head)
    //              |  /
    //              | /
    //  (origin)    *
    pub fn update_local_branch_head(self) -> Result<Self, git2::Error> {
        let remote_commit = self
            .repo
            .find_commit(self.meta_data().remote_commit.unwrap())?;

        let mut index = self.repo.cherrypick_commit(
            self.as_commit(),
            &remote_commit,
            0,
            Some(&MergeOptions::default().file_favor(FileFavor::Theirs)),
        )?;
        assert!(!index.has_conflicts());
        if index.is_empty() {
            return std::result::Result::Ok(self);
        }
        let tree_id = index.write_tree_to(self.repo)?;
        if tree_id == remote_commit.tree()?.id() {
            return std::result::Result::Ok(self);
        }
        let tree = self.repo.find_tree(tree_id)?;

        let new_commit = {
            let signature = self.as_commit().author();
            self.repo.commit(
                None,
                &signature,
                &signature,
                "Fixup!",
                &tree,
                &[&remote_commit],
            )?
        };

        let new_meta = self.meta_data.update_commit(new_commit);
        self.git_repo.save_meta_data(&self.commit, &new_meta)?;
        std::result::Result::Ok(TrackedCommit {
            repo: self.repo,
            git_repo: self.git_repo,
            commit: self.commit,
            meta_data: new_meta,
        })
    }

    //
    // Merge remote_branch_head with local_branch_head unless remote_branch_head any
    // of those are a direct dependant on the other.
    //
    // Will not update from remote.
    //
    //
    //                 *
    //                 |    * (Merge) <---- Produces this merge unless.
    //                 |   / \
    //                 *  /   * (remote_branch_head)
    //                 | * <-/------------------------ (local_branch_head)
    //                 |  \ /
    //           c1    *   *
    //                 |  /
    //                 | /
    //     (origin)    *
    pub fn merge_remote_head(self, new_parent: Option<&Commit>) -> anyhow::Result<Self> {
        let remote_branch_commit = self.remote_branch()?.get().peel_to_commit()?; 
        let remote_branch_head = remote_branch_commit.id();
        let local_branch_head = self.meta_data().remote_commit.unwrap();
        let merge_base = self
            .repo
            .merge_base(local_branch_head, remote_branch_head)?;

        let new_remote_commit = if merge_base == local_branch_head {
            self.repo.find_commit(remote_branch_head)?
        } else if merge_base == remote_branch_head {
            drop(remote_branch_commit);
            return Ok(self);
        } else {
            let local_branch_commit = self.repo.find_commit(local_branch_head)?;
            let mut remote_index =
                self.repo
                    .merge_commits(&local_branch_commit, &remote_branch_commit, None)?;

            //self.repo.merge_analysis_for_ref
            if remote_index.has_conflicts() {
                for c in remote_index.conflicts()? {
                    let c = c?;
                    println!("Conclict {:?}", CString::new(c.our.unwrap().path).unwrap())
                }
                //self.repo.set_head_detached(base_commit.id())?;
                //self.repo.merge(
                //    &[&self.repo.find_annotated_commit(remote_commit.id())?],
                //    None,
                //    None,
                //)?;
                //self.git_repo
                //    .save_merge_state(&base_commit, &remote_commit)?;
                anyhow::bail!(
                    "Unable to merge {} and {}",
                    local_branch_commit.id(),
                    remote_branch_commit.id()
                );
            }
            if remote_index.is_empty() {
                anyhow::bail!("Index is empty");
            }
            let tree = remote_index
                .write_tree_to(self.repo)
                .context("write index to tree")?;
            let oid = self.repo.commit(
                None,
                &self.repo.signature().context("No signature")?,
                &self.repo.signature()?,
                "Merge",
                &self.repo.find_tree(tree)?,
                &[&local_branch_commit, &remote_branch_commit],
            )?;
            self.repo.find_commit(oid)?
        };

        let new_remote_tree = new_remote_commit.tree()?;
        let diff = self.repo.diff_tree_to_tree(
            Some(&self.git_repo.base_commit()?.tree()?),
            Some(&new_remote_tree),
            None,
        )?;

        let parent_commit = if let Some(parent) = new_parent {
            parent.clone()
        } else {
            self.commit.parent(0)?
        };
        let mut index = self
            .repo
            .apply_to_tree(&parent_commit.tree()?, &diff, None)?;
        let tree_id = index.write_tree_to(self.repo)?;
        let tree = self.repo.find_tree(tree_id)?;

        let new_commit = {
            let signature = self.as_commit().author();
            self.repo.commit(
                None,
                &signature,
                &signature,
                self.commit.message().expect("Not valid UTF-8"),
                &tree,
                &[&parent_commit],
            )?
        };

        drop(remote_branch_commit);
        let new_commit = self.repo.find_commit(new_commit)?;
        let new_meta_data = self.meta_data.update_commit(new_remote_commit.id());
        self.git_repo.save_meta_data(
            &new_commit,
            &new_meta_data,
        )?;

        Ok(TrackedCommit::new(&self.repo, &self.git_repo, new_commit, new_meta_data))
    }

    //
    //
    //                         * (Merge with 'main') <---- Produces this merge
    //                 *      /  \
    //                 |     /    * (Merge)
    //                 |    /    / \
    //           c1    *   /    /   * (remote_branch_head)
    //                 |  /    * <-/------------------------(local_branch_head)
    //                 | /      \ /
    //     (origin)    *         *
    //                 |        /
    //                 |       /
    //                 *------/
    //
    pub fn sync_with_main(self) -> Result<Self, git2::Error> {
        todo!()
    }

    pub fn update_remote(self, new_remote_head: Oid) -> Self {
        TrackedCommit {
            repo: self.repo,
            git_repo: self.git_repo,
            commit: self.commit,
            meta_data: self.meta_data.update_commit(new_remote_head),
        }
    }

    pub fn update(self, new_parent: &Commit) -> anyhow::Result<Self> {
        //Add local changes first.
        let base_commit = self.local_branch_head()?;

        //Update "local" version of remote with the actual remote
        let new_remote_commit: Commit<'repo> = {
            let remote_branch = self.remote_branch()?;
            let remote_commit = remote_branch.get().peel_to_commit()?;

            let merge_base = self.repo.merge_base(base_commit.id(), remote_commit.id())?;
            if merge_base == base_commit.id() {
                //No need to merge as base_commit doesn't contain any commits that aren't already
                //in remote_commit
                self.repo.find_commit(remote_commit.id())?
            } else if merge_base == remote_commit.id() {
                self.repo.find_commit(base_commit.id())?
            } else {
                let mut remote_index =
                    self.repo
                        .merge_commits(&base_commit, &remote_commit, None)?;

                //self.repo.merge_analysis_for_ref
                if remote_index.has_conflicts() {
                    for c in remote_index.conflicts()? {
                        let c = c?;
                        println!("Conclict {:?}", CString::new(c.our.unwrap().path).unwrap())
                    }
                    self.repo.set_head_detached(base_commit.id())?;
                    self.repo.merge(
                        &[&self.repo.find_annotated_commit(remote_commit.id())?],
                        None,
                        None,
                    )?;
                    self.git_repo
                        .save_merge_state(&base_commit, &remote_commit)?;
                    anyhow::bail!(
                        "Unable to merge {} and {}",
                        base_commit.id(),
                        remote_commit.id()
                    );
                }
                if remote_index.is_empty() {
                    anyhow::bail!("Index is empty");
                }
                let tree = remote_index
                    .write_tree_to(self.repo)
                    .context("write index to tree")?;
                let oid = self.repo.commit(
                    None,
                    &self.repo.signature().context("No signature")?,
                    &self.repo.signature()?,
                    "Merge",
                    &self.repo.find_tree(tree)?,
                    &[&base_commit, &remote_commit],
                )?;
                self.repo.find_commit(oid)?
            }
        };

        let new_remote_tree = new_remote_commit.tree()?;
        let diff = self.repo.diff_tree_to_tree(
            Some(&self.git_repo.base_commit()?.tree()?),
            Some(&new_remote_tree),
            None,
        )?;

        let index = self.repo.apply_to_tree(&new_parent.tree()?, &diff, None)?;

        let new_commit = self.git_repo.commit_index(
            index,
            &self.commit,
            new_parent.id(),
            self.commit.message().expect("Not valid UTF-8 message"),
        )?;

        drop(base_commit);
        let new_meta_data = self.meta_data.update_commit(new_remote_commit.id());
        self.git_repo.save_meta_data(&new_commit, &new_meta_data)?;

        Ok(TrackedCommit {
            repo: self.repo,
            git_repo: self.git_repo,
            commit: new_commit,
            meta_data: new_meta_data,
        })
    }
}
