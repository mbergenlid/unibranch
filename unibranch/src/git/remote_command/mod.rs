use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Context;

use super::local_commit::CommitMetadata;

pub enum RemoteGitCommand<'a> {
    Default(&'a Path),
    Silent(&'a Path),
    DryRun(&'a Path),
}

impl<'a> RemoteGitCommand<'a> {
    pub fn push(&self, meta_data: &CommitMetadata) -> anyhow::Result<()> {
        match self {
            RemoteGitCommand::Default(path) => {
                RemoteGitCommand::push_real(path, meta_data, Stdio::inherit)
            }
            RemoteGitCommand::Silent(path) => {
                RemoteGitCommand::push_real(path, meta_data, Stdio::null)
            }
            RemoteGitCommand::DryRun(_) => Ok(println!(
                "Pushing commit {} to origin/{}",
                meta_data.remote_commit.unwrap(),
                meta_data.remote_branch_name
            )),
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
                meta_data.remote_commit.unwrap(),
                &meta_data.remote_branch_name
            ))
            .stderr(stdio())
            .stdout(stdio())
            .status()?;
        Ok(())
    }

    fn fetch_real<F>(path: &Path, stdio: F) -> anyhow::Result<()>
    where
        F: Fn() -> Stdio,
    {
        Command::new("git")
            .current_dir(path)
            .arg("fetch")
            .stdout(stdio())
            .stderr(stdio())
            .status()
            .context("git fetch")?;
        Ok(())
    }

    pub(crate) fn fetch(&self) -> anyhow::Result<()> {
        match self {
            RemoteGitCommand::Default(path) => RemoteGitCommand::fetch_real(path, Stdio::inherit),
            RemoteGitCommand::Silent(path) => RemoteGitCommand::fetch_real(path, Stdio::null),
            RemoteGitCommand::DryRun(path) => RemoteGitCommand::fetch_real(path, Stdio::inherit),
        }
    }
}
