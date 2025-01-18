use anyhow::Context;
use tracing::{debug, info, span, Level};

use crate::git::{local_commit::MainCommit, GitRepo};

#[derive(clap::Parser, Default)]
pub struct Options {
    #[arg(short, long)]
    pub cont: bool,

    pub commit_ref: Option<String>,
}

///```text
///
///              *
///              |    * (Merge)
///              |   / \
///        c1    *  /   * (remote_branch_head)
///              | * <-/------------------------ cherry-pick c1 local_branch_head (resolve conflicts by accepting theirs)
///              |  \ /
///    (origin)  *   * (local_branch_head)
///              |  /
///              | /
/// (old_origin) *
///```
pub fn execute(options: Options, repo: GitRepo) -> anyhow::Result<()> {
    debug!("Syncing local changes with remote");

    let mut unpushed_commits = repo.unpushed_commits()?;
    let mut parent_commit = if options.cont {
        //Read the current state
        //First finish the ongoing merge
        if options.commit_ref.is_some() {
            anyhow::bail!("Can not call --continue with a reference");
        }
        let tracked_commit = repo.finish_merge()?;

        //
        tracked_commit.commit()
    } else if let Some(c_ref) = options.commit_ref {
        let commit = repo.find_unpushed_commit(&c_ref)?;
        unpushed_commits = vec![commit];
        match repo.find_unpushed_commit(&c_ref)? {
            MainCommit::UnTracked(_) => {
                anyhow::bail!("Commit {} is not tracked so cannot be synced", c_ref)
            }
            MainCommit::Tracked(c) => c.commit().parent(0)?,
        }
    } else {
        repo.base_commit()?
    };

    info!(
        "Base commit {} {}",
        parent_commit.id(),
        parent_commit.summary().unwrap_or("")
    );
    for original_commit in unpushed_commits {
        match original_commit {
            MainCommit::Tracked(tracked_commit) => {
                let _span = span!(
                    Level::INFO,
                    "Tracked",
                    commit = format!("{}", tracked_commit.as_commit().id()),
                    summary = tracked_commit.as_commit().summary()
                )
                .entered();
                let new_parent_1 = tracked_commit
                    .update_local_branch_head()?
                    .merge_remote_head(Some(&parent_commit))?;
                //.sync_with_main()?;

                info!(
                    "Pushing {} to branch {}",
                    new_parent_1.meta_data().remote_commit,
                    new_parent_1.meta_data().remote_branch_name
                );
                repo.remote()
                    .push(new_parent_1.meta_data())
                    .with_context(|| format!("Pushing {}", new_parent_1.meta_data()))?;
                parent_commit = new_parent_1.commit();
            }
            MainCommit::UnTracked(local_commit) => {
                info!(
                    "Untracked commit {} {}",
                    local_commit.as_commit().id(),
                    local_commit.as_commit().message().unwrap_or("")
                );
                let rebased_commit = local_commit.rebase(&parent_commit)?;
                parent_commit = rebased_commit.commit();
            }
        }
    }

    repo.update_current_branch(&parent_commit)?;

    Ok(())
}
