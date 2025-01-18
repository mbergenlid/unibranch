use std::{
    path::Path,
    process::{Command, Stdio},
};

use super::local_commit::CommitMetadata;

pub enum RemoteGitCommand<'a> {
    Default(&'a Path),
    Silent(&'a Path),
    DryRun(&'a Path),
}

impl RemoteGitCommand<'_> {
    pub fn push(&self, meta_data: &CommitMetadata) -> anyhow::Result<()> {
        match self {
            RemoteGitCommand::Default(path) => {
                RemoteGitCommand::push_real(path, meta_data, Stdio::inherit)
            }
            RemoteGitCommand::Silent(path) => {
                RemoteGitCommand::push_real(path, meta_data, Stdio::null)
            }
            RemoteGitCommand::DryRun(_) => {
                println!(
                    "Pushing commit {} to origin/{}",
                    meta_data.remote_commit, meta_data.remote_branch_name
                );
                Ok(())
            }
        }
    }

    fn push_real<F>(path: &Path, meta_data: &CommitMetadata, stdio: F) -> anyhow::Result<()>
    where
        F: Fn() -> Stdio,
    {
        Command::new("git")
            .current_dir(path)
            .arg("push")
            .arg("--no-verify")
            .arg("--force-with-lease")
            .arg("--")
            .arg("origin")
            .arg(format!(
                "{}:refs/heads/{}",
                meta_data.remote_commit, &meta_data.remote_branch_name
            ))
            .stderr(stdio())
            .stdout(stdio())
            .status()?;
        Ok(())
    }
}
