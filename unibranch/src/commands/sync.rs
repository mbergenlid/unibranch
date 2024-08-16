use crate::git::{local_commit::MainCommit, GitRepo};

#[derive(clap::Parser, Default)]
pub struct Options {
    #[arg(short, long)]
    cont: bool,
}

//TODO: Rename to 'update' or 'sync' or something

pub fn execute(options: Options, repo: GitRepo) -> anyhow::Result<()> {
    repo.remote().fetch()?;

    let mut parent_commit = repo.base_commit()?;
    for original_commit in repo.unpushed_commits().unwrap() {
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
