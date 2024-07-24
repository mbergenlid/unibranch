mod common;

use common::{RemoteRepo, TestRepoWithRemote};
use indoc::indoc;
use pretty_assertions::assert_eq;
use sc::{commands::cherry_pick, git::GitRepo};

fn init_repo(remote: &RemoteRepo) -> TestRepoWithRemote {
    let test_repo = remote.clone();

    let test_repo = test_repo
        .create_file("File1", "Hello world!")
        .commit_all("commit1")
        .push();
    let test_repo = test_repo
        .append_file("File1", "Hello again")
        .commit_all("commit2")
        .append_file("File1", "Hello another time")
        .commit_all("commit3")
        .append_file("File1", "More")
        .commit_all("commit4")
        .append_file("File1", "more")
        .commit_all("commit5");

    test_repo
}

#[test]
fn find_commit_by_head() {
    let remote_repo = RemoteRepo::new();
    let test_repo = init_repo(&remote_repo);
    let repo = GitRepo::open(test_repo.local_repo_dir.path()).unwrap();

    assert_eq!(
        repo.find_unpushed_commit("HEAD").unwrap().id(),
        test_repo.find_commit(0).id()
    );
}

#[test]
fn find_commit_by_parent_head() {
    let remote_repo = RemoteRepo::new();
    let test_repo = init_repo(&remote_repo);
    let repo = GitRepo::open(test_repo.local_repo_dir.path()).unwrap();

    assert_eq!(
        repo.find_unpushed_commit("HEAD^").unwrap().id(),
        test_repo.find_commit(1).id()
    );
}

#[test]
fn find_commit_by_parent_ancestors() {
    let remote_repo = RemoteRepo::new();
    let test_repo = init_repo(&remote_repo);
    let repo = GitRepo::open(test_repo.local_repo_dir.path()).unwrap();

    assert_eq!(
        repo.find_unpushed_commit("HEAD~2").unwrap().id(),
        test_repo.find_commit(2).id()
    );
}

#[test]
fn find_commit_from_commit() {
    let remote_repo = RemoteRepo::new();
    let test_repo = init_repo(&remote_repo);
    let repo = GitRepo::open(test_repo.local_repo_dir.path()).unwrap();

    assert_eq!(
        repo.find_unpushed_commit(&format!("{}^", test_repo.find_commit(1).id()))
            .unwrap()
            .id(),
        test_repo.find_commit(2).id()
    );
    let short_hash = &format!("{}", test_repo.find_commit(1).id())[0..6];
    assert_eq!(
        repo.find_unpushed_commit(&format!("{}^", short_hash))
            .unwrap()
            .id(),
        test_repo.find_commit(2).id()
    );
}

#[test]
fn find_an_already_pushed_commit() {
    let remote_repo = RemoteRepo::new();
    let test_repo = init_repo(&remote_repo);
    let repo = GitRepo::open(test_repo.local_repo_dir.path()).unwrap();

    assert!(repo.find_unpushed_commit("HEAD~4").is_err());
}

#[test]
fn update_commit_from_remote() {
    let remote_repo = RemoteRepo::new();
    let local_repo = remote_repo
        .clone()
        .create_file("File1", "Hello, World!")
        .commit_all("commit1")
        .push()
        .append_file("File1", "Some more changes")
        .commit_all("pr commit");

    let repo = GitRepo::open(local_repo.local_repo_dir.path()).unwrap();

    //Create a PR from local repo
    cherry_pick::execute(
        cherry_pick::Options {
            dry_run: false,
            rebase: false,
            commit_ref: Some("HEAD".to_string()),
        },
        &local_repo.local_repo_dir,
    )
    .expect("Unable to create initial PR");

    let another_local_clone = remote_repo.clone();

    let _another_local_clone = another_local_clone
        .checkout("pr-commit")
        .append_file("File1", "Remote fixes")
        .commit_all("Fixup")
        .push();

    let local_repo = local_repo.fetch();
    let origin_diff = String::from_utf8(local_repo.diff("origin/pr-commit", "HEAD^").stdout)
        .expect("Getting diff");
    assert_eq!(
        origin_diff,
        indoc! {"
            diff --git a/File1 b/File1
            index 6a56b5e..8ab686e 100644
            --- a/File1
            +++ b/File1
            @@ -1,3 +1 @@
             Hello, World!
            -Some more changes
            -Remote fixes
        "}
    );

    repo.update(local_repo.find_commit(0)).unwrap();

    let local_commit_diff =
        String::from_utf8(local_repo.diff("master", "master^").stdout).expect("Getting diff");
    assert_eq!(
        local_commit_diff,
        indoc! {"
            diff --git a/File1 b/File1
            index 6a56b5e..8ab686e 100644
            --- a/File1
            +++ b/File1
            @@ -1,3 +1 @@
             Hello, World!
            -Some more changes
            -Remote fixes
        "},
        "Local 'master' commit hasn't been updated with the remote changes"
    );

    assert_eq!(local_repo.head_branch(), "master");
}
