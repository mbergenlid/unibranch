use test_repo::{RemoteRepo, TestRepoWithRemote};

use crate::commands::create;

use super::{MainCommit, TrackedCommit};

mod tracked_commit_merge_remote_head;
mod tracked_commit_sync_with_main;
mod tracked_commit_update_local_branch_head;

fn tracked(commit: MainCommit) -> TrackedCommit {
    match commit {
        MainCommit::UnTracked(_) => panic!("not a TrackedCommit"),
        MainCommit::Tracked(tracked) => tracked,
    }
}

fn setup_repo(remote: &RemoteRepo) -> TestRepoWithRemote {
    let local = remote.clone_repo();

    let local = local
        .create_file("file1", "Hello, World!")
        .commit_all("Initial")
        .push();

    let local = local
        .create_file("file2", "another file")
        .commit_all("Commit 1");

    {
        create::execute(create::Options::default(), &local.local_repo_dir).unwrap();
    }
    local
}
