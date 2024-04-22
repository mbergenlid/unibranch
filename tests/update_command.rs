mod common;

use common::TestRepoWithRemote;
use indoc::indoc;
use pretty_assertions::assert_eq;
use stackable_commits::commands::{diff, update};

#[test]
fn test_update_a_diff() {
    let repo = TestRepoWithRemote::new();

    let repo = repo
        .create_file("File1", "Hello world!")
        .commit_all("commit1")
        .push();

    let repo = repo
        .append_file("File1", "Another Hello, World!")
        .commit_all("commit2");

    let current_dir = repo.local_repo_dir.path();

    let commit = repo.find_commit(0).id();
    diff::diff::<&str, _>(Some(&format!("{}", commit)), current_dir).unwrap();

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
    update::update(
        Some(&format!("{}", head)),
        "commit2",
        repo.local_repo_dir.path(),
    )
    .unwrap();

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
