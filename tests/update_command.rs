mod common;

use git2::Oid;
use indoc::indoc;
use pretty_assertions::assert_eq;
use sc::commands::cherry_pick;

use crate::common::RemoteRepo;

fn push_options(commit_ref: Option<Oid>) -> cherry_pick::Options {
    cherry_pick::Options {
        dry_run: false,
        rebase: false,
        commit_ref: commit_ref.map(|id| format!("{}", id)),
    }
}

#[test]
fn test_update_a_diff() {
    let remote = RemoteRepo::new();
    let repo = remote.clone();

    let repo = repo
        .create_file("File1", "Hello world!")
        .commit_all("commit1")
        .push();

    let repo = repo
        .append_file("File1", "Another Hello, World!")
        .commit_all("commit2");

    let current_dir = repo.local_repo_dir.path();

    let commit = repo.find_commit(0).id();
    cherry_pick::execute(push_options(Some(commit)), current_dir).unwrap();

    let remote_head = repo.ls_remote_heads("commit2");
    assert!(!remote_head.stdout.is_empty());

    let actual_diff = String::from_utf8(repo.diff("origin/commit2", "origin/master").stdout)
        .expect("Output of diff is not valid UTF-8");
    let expected_diff = indoc! {"
        diff --git a/File1 b/File1
        index e8151f3..cd08755 100644
        --- a/File1
        +++ b/File1
        @@ -1,2 +1 @@
         Hello world!
        -Another Hello, World!
    "};
    assert_eq!(actual_diff, expected_diff);

    let repo = repo
        .append_file("File1", "Some PR review fixes")
        .commit_all_amend();

    let head = repo.find_commit(0).id();
    cherry_pick::execute(push_options(Some(head)), repo.local_repo_dir.path()).unwrap();

    //Verify the diff now.
    let actual_diff = String::from_utf8(repo.diff("origin/commit2", "origin/master").stdout)
        .expect("Output of diff is not valid UTF-8");
    let expected_diff = indoc! {"
        diff --git a/File1 b/File1
        index 17b687d..cd08755 100644
        --- a/File1
        +++ b/File1
        @@ -1,3 +1 @@
         Hello world!
        -Another Hello, World!
        -Some PR review fixes
    "};
    assert_eq!(actual_diff, expected_diff);

    let actual_diff = String::from_utf8(repo.diff("origin/commit2", "origin/commit2^").stdout)
        .expect("Output of diff is not valid UTF-8");
    let expected_diff = indoc! {"
        diff --git a/File1 b/File1
        index 17b687d..e8151f3 100644
        --- a/File1
        +++ b/File1
        @@ -1,3 +1,2 @@
         Hello world!
         Another Hello, World!
        -Some PR review fixes
    "};
    assert_eq!(actual_diff, expected_diff);
}

#[test]
fn test_a_more_complex_update() {
    let remote = RemoteRepo::new();
    let repo = remote.clone();

    let repo = repo
        .create_file("File1", "Hello world!")
        .commit_all("commit1")
        .push();

    let repo = repo
        .create_file("File2", "Completely unrelated changes in another file")
        .commit_all("unrelated commit");

    let repo = repo
        .append_file("File1", "Another Hello, World!")
        .commit_all("commit2");

    let current_dir = repo.local_repo_dir.path();

    let commit = repo.find_commit(0).id();
    cherry_pick::execute(push_options(Some(commit)), current_dir).unwrap();

    let remote_head = repo.ls_remote_heads("commit2");
    assert!(!remote_head.stdout.is_empty());

    let actual_diff = String::from_utf8(repo.diff("origin/commit2", "origin/master").stdout)
        .expect("Output of diff is not valid UTF-8");
    let expected_diff = indoc! {"
        diff --git a/File1 b/File1
        index e8151f3..cd08755 100644
        --- a/File1
        +++ b/File1
        @@ -1,2 +1 @@
         Hello world!
        -Another Hello, World!
    "};
    assert_eq!(actual_diff, expected_diff);

    let repo = repo
        .append_file("File1", "Some PR review fixes")
        .commit_all_amend();

    let unrelated_commit = repo.find_commit(1).id();
    let repo = repo
        .append_file(
            "File2",
            "More unrelated changes belonging to unrelated commit",
        )
        .commit_all_fixup(unrelated_commit);

    let head = repo.find_commit(0).id();
    cherry_pick::execute(push_options(Some(head)), repo.local_repo_dir.path()).unwrap();

    //Verify the diff now.
    let actual_diff = String::from_utf8(repo.diff("origin/commit2", "origin/master").stdout)
        .expect("Output of diff is not valid UTF-8");
    let expected_diff = indoc! {"
        diff --git a/File1 b/File1
        index 17b687d..cd08755 100644
        --- a/File1
        +++ b/File1
        @@ -1,3 +1 @@
         Hello world!
        -Another Hello, World!
        -Some PR review fixes
    "};
    assert_eq!(actual_diff, expected_diff);

    let actual_diff = String::from_utf8(repo.diff("origin/commit2", "origin/commit2^").stdout)
        .expect("Output of diff is not valid UTF-8");
    let expected_diff = indoc! {"
        diff --git a/File1 b/File1
        index 17b687d..e8151f3 100644
        --- a/File1
        +++ b/File1
        @@ -1,3 +1,2 @@
         Hello world!
         Another Hello, World!
        -Some PR review fixes
    "};
    assert_eq!(actual_diff, expected_diff);
}

#[ignore = "Not implemented yet"]
#[allow(dead_code)]
fn test_branch_updated_on_remote() {
    let remote = RemoteRepo::new();
    let repo = remote.clone();

    let repo = repo
        .create_file("File1", "Hello world!")
        .commit_all("commit1")
        .push();

    let repo = repo
        .create_file("File2", "Completely unrelated changes in another file")
        .commit_all("unrelated commit");

    let repo = repo
        .append_file("File1", "Another Hello, World!")
        .commit_all("commit2");

    let current_dir = repo.local_repo_dir.path();

    let commit = repo.find_commit(0).id();
    cherry_pick::execute(push_options(Some(commit)), current_dir).unwrap();

    let actual_diff = String::from_utf8(repo.diff("origin/commit2", "origin/master").stdout)
        .expect("Output of diff is not valid UTF-8");
    let expected_diff = indoc! {"
        diff --git a/File1 b/File1
        index e8151f3..cd08755 100644
        --- a/File1
        +++ b/File1
        @@ -1,2 +1 @@
         Hello world!
        -Another Hello, World!
    "};
    assert_eq!(actual_diff, expected_diff);

    //Someone else commits and pushes on branch origin/commit2
    let other_checkout = remote.clone();

    other_checkout
        .checkout("commit2")
        .create_file("File3", "Some other changes someone else decided to do")
        .commit_all("other user change")
        .push();

    repo.fetch();
}
