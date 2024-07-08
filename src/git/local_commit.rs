use std::{borrow::Cow, error::Error, fmt::Display};

use git2::Commit;
use itertools::Itertools;

pub struct LocalCommit<'repo> {
    commit: Commit<'repo>,
}

impl<'repo> LocalCommit<'repo> {
    pub fn new(commit: Commit<'repo>) -> Self {
        LocalCommit { commit }
    }

    pub fn meta_data(&self) -> Option<CommitMetadata> {
        self.commit.message().and_then(|msg| msg.try_into().ok())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CommitMetadata<'a> {
    pub remote_branch_name: Cow<'a, str>,
}

impl<'a> CommitMetadata<'a> {
    pub fn is_modified(&self) -> bool {
        match self.remote_branch_name {
            Cow::Borrowed(_) => false,
            Cow::Owned(_) => true,
        }
    }
}

impl<'a> Display for CommitMetadata<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("remote-branch: {}\n", self.remote_branch_name))
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

impl<'a> TryFrom<&'a str> for CommitMetadata<'a> {
    type Error = MetaDataError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let mut remote_branch_name = None;
        for line in value.lines() {
            if let Some((key, value)) = line.splitn(2, ':').collect_tuple() {
                if key == "remote-branch" {
                    remote_branch_name = Some(value.trim());
                }
            }
        }
        remote_branch_name
            .map(|name| CommitMetadata {
                remote_branch_name: Cow::Borrowed(name),
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
                remote_branch_name: Cow::Borrowed("branch_name")
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
}
