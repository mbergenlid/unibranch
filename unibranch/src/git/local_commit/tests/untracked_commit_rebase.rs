use test_repo::RemoteRepo;

use crate::git::{local_commit::MainCommit, GitRepo};

#[test]
fn test_rebase() {
    let remote_repo = RemoteRepo::new();
    let local_repo = remote_repo
        .clone_repo()
        .create_file("File1", "Hello, World!")
        .commit_all("commit1")
        .push()
        .create_file("File2", "Unrelated feature")
        .commit_all("unrelated commit")
        .append_file("File1", "Starting on a new feature")
        .commit_all("feature 1");

    let git_repo = GitRepo::open(local_repo.local_repo_dir.path()).unwrap();
    let untracked_commit = match git_repo.find_unpushed_commit("HEAD^").unwrap() {
        MainCommit::UnTracked(c) => c,
        MainCommit::Tracked(_) => panic!("Expected an untracked commit"),
    };

    let original_id = untracked_commit.as_commit().id();

    let rebased = untracked_commit
        .rebase(&git_repo.base_commit().unwrap())
        .unwrap();
    assert_eq!(rebased.as_commit().id(), original_id);
}
