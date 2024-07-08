mod common;

use common::RemoteRepo;
use git2::Oid;
use indoc::indoc;
use sc::commands::cherry_pick;

use pretty_assertions::assert_eq;

fn push_options(commit_ref: Option<Oid>) -> cherry_pick::Options {
    cherry_pick::Options {
        dry_run: false,
        rebase: false,
        commit_ref: commit_ref.map(|id| format!("{}", id)),
    }
}

#[test]
fn basic_test() {
    let remote = RemoteRepo::new();
    let repo = remote.clone();

    let repo = repo
        .create_file("File1", "Hello world!")
        .commit_all("commit1")
        .push();

    let repo = repo
        .append_file("File1", "Another Hello, World!")
        .commit_all("commit2");

    let repo = repo
        .create_file("File2", "Yet another Hello, World!")
        .commit_all("commit3");

    let current_dir = repo.local_repo_dir.path();

    cherry_pick::execute(push_options(None), current_dir).unwrap();

    let remote_head = repo.ls_remote_heads("commit3");
    assert!(!remote_head.stdout.is_empty());

    let output = String::from_utf8(repo.diff("origin/commit3", "origin/master").stdout)
        .expect("Output of diff is not valid UTF-8");
    let expected_diff = indoc! {"
        diff --git a/File2 b/File2
        deleted file mode 100644
        index 9dd1272..0000000
        --- a/File2
        +++ /dev/null
        @@ -1 +0,0 @@
        -Yet another Hello, World!
    "};
    assert_eq!(output, expected_diff);

    assert_eq!(
        repo.find_note("head"),
        indoc! {"
            remote-branch: commit3
        "},
    );
}

#[test]
fn test_diff_from_not_head_commit() {
    let remote = RemoteRepo::new();
    let repo = remote.clone();

    let repo = repo
        .create_file("File1", "Hello world!")
        .commit_all("commit1")
        .push();

    let repo = repo
        .append_file("File1", "Another Hello, World!")
        .commit_all("commit2");

    let repo = repo
        .create_file("File2", "Yet another Hello, World!")
        .commit_all("commit3");

    let current_dir = repo.local_repo_dir.path();

    let commit = repo.find_commit(1).id();
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

    assert_eq!(
        repo.find_note("head^"),
        indoc! {"
            remote-branch: commit2
        "},
    );
}
