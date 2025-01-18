use std::{borrow::Cow, error::Error, fmt::Display, str::FromStr};

use git2::{Commit, Oid, Repository};
use itertools::Itertools;

use super::GitRepo;

#[cfg(test)]
mod tests;

mod tracked_commit;
pub use tracked_commit::TrackedCommit;
mod untracked_commit;
pub use untracked_commit::UnTrackedCommit;

#[derive(Debug)]
pub enum MainCommit<'repo> {
    UnTracked(UnTrackedCommit<'repo>),
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
                    return Ok(MainCommit::UnTracked(UnTrackedCommit::new(
                        repo, git_repo, commit,
                    )))
                }
                _ => return Err(error),
            }
        }
        let note = res.expect("Already checked for error above");
        if let Some(meta_data) = note
            .message()
            .and_then(|m| m.parse::<CommitMetadata>().ok())
        {
            Ok(MainCommit::Tracked(TrackedCommit::new(
                repo, git_repo, commit, meta_data,
            )))
        } else {
            Ok(MainCommit::UnTracked(UnTrackedCommit::new(
                repo, git_repo, commit,
            )))
        }
    }

    pub fn id(&self) -> Oid {
        match self {
            MainCommit::UnTracked(c) => c.as_commit().id(),
            MainCommit::Tracked(c) => c.as_commit().id(),
        }
    }

    pub fn message(&self) -> Option<&str> {
        match self {
            MainCommit::UnTracked(c) => c.as_commit().message(),
            MainCommit::Tracked(c) => c.as_commit().message(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CommitMetadata<'a> {
    pub remote_branch_name: Cow<'a, str>,
    pub remote_commit: Oid,
}

impl CommitMetadata<'_> {
    pub fn update_commit(mut self, oid: Oid) -> Self {
        self.remote_commit = oid;
        self
    }
}

impl Display for CommitMetadata<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("remote-branch: {}\n", self.remote_branch_name))?;
        f.write_fmt(format_args!("remote-commit: {}\n", self.remote_commit))?;
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
        if let (Some(branch), Some(commit)) = (remote_branch_name, remote_commit_id) {
            Ok(CommitMetadata {
                remote_branch_name: Cow::Owned(branch.to_string()),
                remote_commit: commit,
            })
        } else {
            Err(MetaDataError)
        }
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
        if let (Some(remote_branch_name), Some(remote_commit)) =
            (remote_branch_name, remote_commit_id)
        {
            Ok(CommitMetadata {
                remote_branch_name: Cow::Owned(remote_branch_name.to_string()),
                remote_commit,
            })
        } else {
            Err(MetaDataError)
        }
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

        let meta_data = TryInto::<CommitMetadata>::try_into(commit_msg);
        assert!(meta_data.is_err(),)
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
                remote_commit: "6ec67b364e67bbd74c66fc8f0cbb95e6ac155d84".parse().unwrap(),
            }
        )
    }

    #[test]
    fn test_parse_with_invalid_remote_commit() {
        let msg = indoc! {"
            remote-branch: branch_name
            remote-commit: Invrlid
        "};
        let meta_data = TryInto::<CommitMetadata>::try_into(msg);
        assert!(meta_data.is_err())
    }
}
