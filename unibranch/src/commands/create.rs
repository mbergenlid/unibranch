use crate::git::{local_commit::MainCommit, GitRepo};

#[derive(clap::Parser, Default)]
pub struct Options {
    #[arg(short, long)]
    pub force: bool,
    pub commit_ref: Option<String>,
    #[arg(short, long)]
    pub name: Option<String>,
}

impl Options {
    pub fn with_force(mut self) -> Self {
        self.force = true;
        self
    }
    pub fn with_name<T: Into<String>>(mut self, name: T) -> Self {
        self.name.replace(name.into());
        self
    }

    pub fn with_commit_ref<T: Into<String>>(mut self, name: T) -> Self {
        self.commit_ref.replace(name.into());
        self
    }
}

pub fn execute(config: Options, git_repo: GitRepo) -> anyhow::Result<()> {
    let rev = config.commit_ref.unwrap_or_else(|| "HEAD".to_string());
    let commit = git_repo.find_unpushed_commit(&rev)?;

    let untracked_commit = match commit {
        MainCommit::UnTracked(commit) => commit,
        MainCommit::Tracked(tracked) => {
            if !config.force {
                anyhow::bail!("Commit is already tracked");
            }

            tracked.untrack()?
        }
    };

    let tracked_commit = untracked_commit.track(config.name)?;
    git_repo.remote().push(tracked_commit.meta_data())?;

    Ok(())
}
