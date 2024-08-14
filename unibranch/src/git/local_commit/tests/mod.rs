use super::{MainCommit, TrackedCommit};

mod tracked_commit_merge_remote_head;
mod tracked_commit_update_local_branch_head;

fn tracked(commit: MainCommit) -> TrackedCommit {
    match commit {
        MainCommit::UnTracked(_) => panic!("not a TrackedCommit"),
        MainCommit::Tracked(tracked) => tracked,
    }
}
