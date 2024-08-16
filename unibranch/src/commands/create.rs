use crate::git::{local_commit::MainCommit, GitRepo};

#[derive(clap::Parser, Default)]
pub struct Options {

    pub commit_ref: Option<String>,
}

pub fn execute(config: Options, git_repo: GitRepo) -> anyhow::Result<()> {
    let rev = config.commit_ref.unwrap_or_else(|| "HEAD".to_string());
    let commit = git_repo.find_unpushed_commit(&rev)?;

    let untracked_commit = match commit {
        MainCommit::UnTracked(commit) => commit,
        MainCommit::Tracked(_) => anyhow::bail!("Commit is already tracked"),
    };

    let tracked_commit = untracked_commit.track()?;
    git_repo.remote().push(tracked_commit.meta_data())?;

    Ok(())
}
