use std::fmt::Display;

pub struct CommitMetadata<'a> {
    pub remote_branch_name: &'a str,
}

impl<'a> Display for CommitMetadata<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("remote-branch: {}\n", self.remote_branch_name))
    }
}
