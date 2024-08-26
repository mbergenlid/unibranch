use crate::git::{local_commit::MainCommit, GitRepo};

#[derive(clap::Parser, Default)]
pub struct Options {
    #[arg(short, long)]
    pub cont: bool,
}

pub fn execute(options: Options, repo: GitRepo) -> anyhow::Result<()> {
    repo.remote().fetch()?;

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
    for original_commit in unpushed_commits {
        match original_commit {
            MainCommit::Tracked(tracked_commit) => {
                let new_parent_1 = tracked_commit
                    .update_local_branch_head()?
                    .merge_remote_head(Some(&parent_commit))?
                    .sync_with_main()?;

                repo.remote().push(new_parent_1.meta_data())?;
                parent_commit = new_parent_1.commit();
            }
            MainCommit::UnTracked(local_commit) => {
                let rebased_commit = local_commit.rebase(&parent_commit)?;
                parent_commit = rebased_commit.commit();
            }
        }
    }

    repo.update_current_branch(&parent_commit)?;

    Ok(())
}
