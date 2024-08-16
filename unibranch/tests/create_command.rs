use git2::Oid;
use indoc::indoc;
use test_repo::{RemoteRepo, TestRepoWithRemote};
use ubr::{commands::create, git::GitRepo};

use pretty_assertions::assert_eq;

fn create_options(commit_ref: Option<Oid>) -> create::Options {
    create::Options {
        commit_ref: commit_ref.map(|id| format!("{}", id)),
    }
}

fn git_repo(value: &TestRepoWithRemote) -> GitRepo {
    GitRepo::open(value.local_repo_dir.path()).unwrap()
}

#[test]
fn basic_test() {
    let remote = RemoteRepo::new();
    let repo = remote.clone_repo();

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

    create::execute(create_options(None), git_repo(&repo)).unwrap();

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
    let repo = remote.clone_repo();

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

    let commit = repo.find_commit(1).id();
    create::execute(create_options(Some(commit)), git_repo(&repo)).unwrap();

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
    let repo = remote.clone_repo();

    let repo = repo
        .create_file("File1", "Hello world!")
        .commit_all("commit1")
        .push();

    let repo = repo
        .append_file("File1", "Another Hello, World!")
        .commit_all("commit2");

    let current_dir = repo.local_repo_dir.path();

    create::execute(create_options(None), GitRepo::open(current_dir).unwrap()).unwrap();

    let result = create::execute(create_options(None), GitRepo::open(current_dir).unwrap());
    assert!(result.is_err());
}

#[test]
fn should_fail_if_remote_branch_already_exists() {
    let remote = RemoteRepo::new();
    let repo = remote.clone_repo();

    let repo = repo
        .create_file("File1", "Hello world!")
        .commit_all("commit1")
        .push();

    let repo = repo
        .append_file("File1", "Another Hello, World!")
        .commit_all("commit2");

    create::execute(create_options(None), git_repo(&repo)).unwrap();

    let repo = repo
        .create_file("File2", "Another Hello, World!")
        .commit_all("commit2");

    let result = create::execute(create_options(None), git_repo(&repo));
    assert!(result.is_err());
    assert_eq!(
        format!("{}", result.unwrap_err()),
        "Remote branch 'commit2' already exist"
    );
}
