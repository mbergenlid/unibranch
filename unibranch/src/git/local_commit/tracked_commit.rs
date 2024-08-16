use anyhow::Context;
use anyhow::Ok;
use git2::{Branch, Commit, FileFavor, MergeOptions, Oid, Repository};

use super::CommitMetadata;
use super::GitRepo;

#[derive(Clone)]
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
        Ok(self.repo.find_commit(commit_meta_data.remote_commit)?)
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
        let remote_commit = self.repo.find_commit(self.meta_data().remote_commit)?;

        let mut index = self.repo.cherrypick_commit(
            self.as_commit(),
            &remote_commit,
            0,
            Some(MergeOptions::default().file_favor(FileFavor::Theirs)),
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
        let local_branch_head = self.meta_data().remote_commit;
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
            let oid = self
                .git_repo
                .merge(&local_branch_commit, &remote_branch_commit)?;
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
        self.git_repo.save_meta_data(&new_commit, &new_meta_data)?;

        Ok(TrackedCommit::new(
            self.repo,
            self.git_repo,
            new_commit,
            new_meta_data,
        ))
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
    pub fn sync_with_main(mut self) -> anyhow::Result<Self> {
        let local_branch_head = self.meta_data().remote_commit;
        let merge_base = self
            .repo
            .merge_base(local_branch_head, self.as_commit().id())?;
        if dbg!(merge_base) == dbg!(self.git_repo.base_commit()?.id())
            || merge_base == self.commit.id()
        {
            Ok(self)
        } else {
            let local_branch_commit = self.repo.find_commit(local_branch_head)?;
            let merge_oid = self
                .git_repo
                .merge(&self.git_repo.base_commit()?, &local_branch_commit)?;

            let _ = std::mem::replace(&mut self.meta_data.remote_commit, merge_oid);
            self.git_repo
                .save_meta_data(self.as_commit(), &self.meta_data)?;
            Ok(self)
        }
    }

    pub fn update_remote(self, new_remote_head: Oid) -> Self {
        TrackedCommit {
            repo: self.repo,
            git_repo: self.git_repo,
            commit: self.commit,
            meta_data: self.meta_data.update_commit(new_remote_head),
        }
    }
}
