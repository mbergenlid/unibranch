use anyhow::Context;
use tracing::{debug, info, span, Level};

use crate::git::{local_commit::MainCommit, GitRepo};

#[derive(clap::Parser, Default)]
pub struct Options {
    #[arg(short, long)]
    pub cont: bool,
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

    let unpushed_commits = repo.unpushed_commits()?;
    let mut parent_commit = if options.cont {
        //Read the current state
        //First finish the ongoing merge
        let tracked_commit = repo.finish_merge()?;

        //
        tracked_commit.commit()
    } else {
        repo.base_commit()?
    };
    info!(
        "Base commit {} {}",
        parent_commit.id(),
        parent_commit.message().unwrap_or("")
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

                info!("Pushing {} to branch {}", new_parent_1.as_commit().id(), new_parent_1.meta_data().remote_branch_name);
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
