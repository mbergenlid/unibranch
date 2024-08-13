use test_repo::RemoteRepo;
use git2::Oid;
use indoc::indoc;
use ubr::commands::create;

use pretty_assertions::assert_eq;

fn create_options(commit_ref: Option<Oid>) -> create::Options {
    create::Options {
        dry_run: false,
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

    create::execute(create_options(None), current_dir).unwrap();

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

    let expected_note = indoc! {"
            remote-branch: commit3
            remote-commit: {}
        "};
    let expected_note = expected_note.replacen(
        "{}",
        &format!(
            "{}",
            repo.find_commit_by_reference("refs/remotes/origin/commit3")
                .id()
        ),
        1,
    );
    assert_eq!(repo.find_note("head"), expected_note,);
}

#[test]
fn test_create_from_not_head_commit() {
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
    create::execute(create_options(Some(commit)), current_dir).unwrap();

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

    let expected_note = indoc! {"
            remote-branch: commit2
            remote-commit: {}
        "};
    let expected_note = expected_note.replacen(
        "{}",
        &format!(
            "{}",
            repo.find_commit_by_reference("refs/remotes/origin/commit2")
                .id()
        ),
        1,
    );
    assert_eq!(repo.find_note("head^"), expected_note,);
}

#[test]
fn should_not_be_able_to_call_create_for_same_commit_twice() {
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

    create::execute(create_options(None), current_dir).unwrap();

    let result = create::execute(create_options(None), current_dir);
    assert!(result.is_err());
}

#[test]
fn should_fail_if_remote_branch_already_exists() {
    let remote = RemoteRepo::new();
    let repo = remote.clone();

    let repo = repo
        .create_file("File1", "Hello world!")
        .commit_all("commit1")
        .push();

    let repo = repo
        .append_file("File1", "Another Hello, World!")
        .commit_all("commit2");

    create::execute(create_options(None), repo.local_repo_dir.path()).unwrap();

    let repo = repo
        .create_file("File2", "Another Hello, World!")
        .commit_all("commit2");

    let result = create::execute(create_options(None), repo.local_repo_dir.path());
    assert!(result.is_err());
    assert_eq!(
        format!("{}", result.unwrap_err()),
        "Remote branch 'commit2' already exist"
    );
}
