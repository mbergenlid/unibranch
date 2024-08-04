use std::{borrow::Cow, error::Error, fmt::Display, str::FromStr};

use anyhow::Context;
use git2::{Branch, Commit, Oid, Repository};
use itertools::Itertools;

use super::GitRepo;

pub enum MainCommit<'repo> {
    UnTracked(LocalCommit<'repo>),
    Tracked(TrackedCommit<'repo>),
}

impl<'repo> MainCommit<'repo> {
    pub fn new(
        git_repo: &'repo GitRepo,
        repo: &'repo Repository,
        commit: Commit<'repo>,
    ) -> Result<MainCommit<'repo>, git2::Error> {
        let res = repo.find_note(None, commit.id());
        if let Err(error) = res {
            match error.code() {
                git2::ErrorCode::NotFound => {
                    return Ok(MainCommit::UnTracked(LocalCommit { _repo: repo, commit }))
                }
                _ => return Err(error),
            }
        }
        let note = res.expect("Already checked for error above");
        if let Some(meta_data) = note
            .message()
            .and_then(|m| m.parse::<CommitMetadata>().ok())
        {
            Ok(MainCommit::Tracked(TrackedCommit {
                repo,
                git_repo,
                commit,
                meta_data,
            }))
        } else {
            Ok(MainCommit::UnTracked(LocalCommit { _repo: repo, commit }))
        }
    }
}

pub struct LocalCommit<'repo> {
    _repo: &'repo Repository,
    commit: Commit<'repo>,
}

pub struct TrackedCommit<'repo> {
    repo: &'repo Repository,
    git_repo: &'repo GitRepo,
    commit: Commit<'repo>,
    meta_data: CommitMetadata<'repo>,
}

impl<'repo> LocalCommit<'repo> {
    pub fn new(repo: &'repo Repository, commit: Commit<'repo>) -> Self {
        LocalCommit { _repo: repo, commit }
    }

    pub fn as_commit(&self) -> &Commit {
        &self.commit
    }
}

impl<'repo> TrackedCommit<'repo> {
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
            } else {
                let mut remote_index =
                    self.repo
                        .merge_commits(&base_commit, &remote_commit, None)?;
                if remote_index.has_conflicts() {
                    anyhow::bail!("Index has conflicts");
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
        let new_meta_data = self.meta_data
                .update_commit(new_remote_commit.id());
        self.git_repo.save_meta_data(
            &new_commit,
            &new_meta_data,
        )?;

        Ok(TrackedCommit {
            repo: self.repo,
            git_repo: self.git_repo,
            commit: new_commit,
            meta_data: new_meta_data,
        })
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CommitMetadata<'a> {
    pub remote_branch_name: Cow<'a, str>,
    pub remote_commit: Option<Oid>,
}

impl<'a> CommitMetadata<'a> {
    pub fn update_commit(mut self, oid: Oid) -> Self {
        self.remote_commit.replace(oid);
        self
    }
}

impl<'a> Display for CommitMetadata<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("remote-branch: {}\n", self.remote_branch_name))?;
        if let Some(remote_commit) = self.remote_commit {
            f.write_fmt(format_args!("remote-commit: {}\n", remote_commit))?;
        };
        Ok(())
    }
}

#[derive(Debug)]
pub struct MetaDataError;

impl Display for MetaDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("")
    }
}

impl Error for MetaDataError {}

impl FromStr for CommitMetadata<'static> {
    type Err = MetaDataError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut remote_branch_name = None;
        let mut remote_commit_id = None;
        for line in value.lines() {
            if let Some((key, value)) = line.splitn(2, ':').collect_tuple() {
                if key == "remote-branch" {
                    remote_branch_name = Some(value.trim());
                } else if key == "remote-commit" {
                    remote_commit_id = value.trim().parse::<Oid>().ok();
                }
            }
        }
        remote_branch_name
            .map(|name| CommitMetadata {
                remote_branch_name: Cow::Owned(name.to_string()),
                remote_commit: remote_commit_id,
            })
            .ok_or(MetaDataError)
    }
}

impl<'a> TryFrom<&'a str> for CommitMetadata<'a> {
    type Error = MetaDataError;

    //Implement this using parse..
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let mut remote_branch_name = None;
        let mut remote_commit_id = None;
        for line in value.lines() {
            if let Some((key, value)) = line.splitn(2, ':').collect_tuple() {
                if key == "remote-branch" {
                    remote_branch_name = Some(value.trim());
                } else if key == "remote-commit" {
                    remote_commit_id = value.trim().parse::<Oid>().ok();
                }
            }
        }
        remote_branch_name
            .map(|name| CommitMetadata {
                remote_branch_name: Cow::Borrowed(name),
                remote_commit: remote_commit_id,
            })
            .ok_or(MetaDataError)
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;

    use indoc::indoc;

    use super::CommitMetadata;

    #[test]
    fn test_parse() {
        let commit_msg = indoc! {"
            remote-branch: branch_name
        "};

        let meta_data = TryInto::<CommitMetadata>::try_into(commit_msg).unwrap();
        assert_eq!(
            meta_data,
            CommitMetadata {
                remote_branch_name: Cow::Borrowed("branch_name"),
                remote_commit: None
            }
        )
    }

    #[test]
    fn test_parse_where_there_is_no_meta() {
        let commit_msg = indoc! {"
            other text
        "};

        let meta_data = TryInto::<CommitMetadata>::try_into(commit_msg);
        assert!(meta_data.is_err())
    }

    #[test]
    fn test_parse_with_remote_commit() {
        let msg = indoc! {"
            remote-branch: branch_name
            remote-commit: 6ec67b364e67bbd74c66fc8f0cbb95e6ac155d84
        "};
        let meta_data = TryInto::<CommitMetadata>::try_into(msg).unwrap();
        assert_eq!(
            meta_data,
            CommitMetadata {
                remote_branch_name: Cow::Borrowed("branch_name"),
                remote_commit: Some("6ec67b364e67bbd74c66fc8f0cbb95e6ac155d84".parse().unwrap()),
            }
        )
    }

    #[test]
    fn test_parse_with_invalid_remote_commit() {
        let msg = indoc! {"
            remote-branch: branch_name
            remote-commit: Invalid
        "};
        let meta_data = TryInto::<CommitMetadata>::try_into(msg).unwrap();
        assert_eq!(
            meta_data,
            CommitMetadata {
                remote_branch_name: Cow::Borrowed("branch_name"),
                remote_commit: None,
            }
        )
    }
}
