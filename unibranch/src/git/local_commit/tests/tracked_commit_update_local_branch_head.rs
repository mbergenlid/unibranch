use crate::{
    commands::create,
    git::{local_commit::MainCommit, GitRepo},
};
use indoc::indoc;
use pretty_assertions::assert_eq;
use test_repo::{RemoteRepo, TestRepoWithRemote};

use super::tracked;

fn git_repo(value: &TestRepoWithRemote) -> GitRepo {
    GitRepo::open(value.path()).unwrap()
}

#[test]
fn test_simple_update() {
    let remote = RemoteRepo::new();
    let local = remote.clone_repo();

    let local = local
        .create_file("file1", "Hello, World!")
        .commit_all("Initial")
        .push();

    let local = local
        .create_file("file2", "Hello, World!")
        .commit_all("Commit 1");

    create::execute(create::Options::default(), git_repo(&local)).unwrap();

    let local = local
        .create_file("file3", "Yaay, no conflicts")
        .commit_all_amend();

    let git_repo = GitRepo::open(local.path()).unwrap();

    let commit = git_repo.find_unpushed_commit("HEAD").unwrap();
    let tracked_commit = match commit {
        MainCommit::UnTracked(_) => panic!("Not a main commit"),
        MainCommit::Tracked(c) => c,
    };
    let tracked_commit = tracked_commit.update_local_branch_head().unwrap();

    let new_commit = format!("{}", tracked_commit.meta_data().remote_commit);

    let actual_diff = String::from_utf8(local.diff("origin/master", &new_commit).stdout)
        .expect("Output of diff is not valid UTF-8");
    let expected_diff = indoc! {"
        diff --git a/file2 b/file2
        new file mode 100644
        index 0000000..8ab686e
        --- /dev/null
        +++ b/file2
        @@ -0,0 +1 @@
        +Hello, World!
        diff --git a/file3 b/file3
        new file mode 100644
        index 0000000..c76ab66
        --- /dev/null
        +++ b/file3
        @@ -0,0 +1 @@
        +Yaay, no conflicts
    "};
    assert_eq!(actual_diff, expected_diff);

    let new_tracked_commit = tracked(git_repo.find_unpushed_commit("HEAD").unwrap());
    assert_eq!(
        new_tracked_commit.meta_data().remote_commit,
        tracked_commit.meta_data().remote_commit
    );
}

#[test]
fn test_update_local() {
    let remote = RemoteRepo::new();
    let local = remote.clone_repo();

    let local = local
        .create_file("file1", "Hello, World!")
        .commit_all("Initial")
        .push();

    let local = local
        .create_file("file2", "Hello, World!")
        .commit_all("Commit 1");

    create::execute(create::Options::default(), git_repo(&local)).unwrap();

    let local = local
        .create_file("file2", "Hello, Conflicting World!")
        .commit_all_amend();

    let git_repo = GitRepo::open(local.path()).unwrap();

    let commit = git_repo.find_unpushed_commit("HEAD").unwrap();
    let tracked_commit = match commit {
        MainCommit::UnTracked(_) => panic!("Not a main commit"),
        MainCommit::Tracked(c) => c,
    };
    let tracked_commit = tracked_commit.update_local_branch_head().unwrap();

    let new_commit = format!("{}", tracked_commit.meta_data().remote_commit);

    let actual_diff = String::from_utf8(local.diff("origin/master", &new_commit).stdout)
        .expect("Output of diff is not valid UTF-8");
    let expected_diff = indoc! {"
        diff --git a/file2 b/file2
        new file mode 100644
        index 0000000..11032e3
        --- /dev/null
        +++ b/file2
        @@ -0,0 +1 @@
        +Hello, Conflicting World!
    "};
    assert_eq!(actual_diff, expected_diff);

    let new_tracked_commit = tracked(git_repo.find_unpushed_commit("HEAD").unwrap());
    assert_eq!(
        new_tracked_commit.meta_data().remote_commit,
        tracked_commit.meta_data().remote_commit
    );
}

#[test]
fn test_update_with_a_rebase_first() {
    let remote = RemoteRepo::new();
    let local = remote.clone_repo();

    let local = local
        .create_file("file1", "Hello, World!")
        .commit_all("Initial")
        .push();

    let local = local
        .create_file("file2", "Hello, World!")
        .commit_all("Commit 1");

    create::execute(create::Options::default(), git_repo(&local)).unwrap();

    {
        let other_local = remote.clone_repo();
        other_local
            .create_file("unrelated_file", "Unrelated")
            .commit_all("Other commit")
            .push();
    }

    let local = local
        .create_file("file2", "Hello, Conflicting World!")
        .commit_all_amend();

    let local = local.pull_rebase();

    //     Commit 1 *
    //              |
    //              |   * remote-commit
    // Other commit *<-/-----------------(origin/master)
    //              | /
    //      Initial *
    let git_repo = GitRepo::open(local.path()).unwrap();

    let commit = git_repo.find_unpushed_commit("HEAD").unwrap();
    let tracked_commit = match commit {
        MainCommit::UnTracked(_) => panic!("Not a main commit"),
        MainCommit::Tracked(c) => c,
    };
    let tracked_commit = tracked_commit.update_local_branch_head().unwrap();

    let new_commit = format!("{}", tracked_commit.meta_data().remote_commit);

    let actual_diff = String::from_utf8(local.diff("origin/master", &new_commit).stdout)
        .expect("Output of diff is not valid UTF-8");
    let expected_diff = indoc! {"
        diff --git a/file2 b/file2
        new file mode 100644
        index 0000000..11032e3
        --- /dev/null
        +++ b/file2
        @@ -0,0 +1 @@
        +Hello, Conflicting World!
    "};
    assert_eq!(actual_diff, expected_diff);

    let new_tracked_commit = tracked(git_repo.find_unpushed_commit("HEAD").unwrap());
    assert_eq!(
        new_tracked_commit.meta_data().remote_commit,
        tracked_commit.meta_data().remote_commit
    );
}

#[test]
fn nothing_should_happen_if_no_changes() {
    let remote = RemoteRepo::new();
    let local = remote.clone_repo();

    let local = local
        .create_file("file1", "Hello, World!")
        .commit_all("Initial")
        .push();

    let local = local
        .create_file("file2", "Hello, World!")
        .commit_all("Commit 1");

    create::execute(create::Options::default(), git_repo(&local)).unwrap();

    let git_repo = GitRepo::open(local.path()).unwrap();

    let tracked_commit = tracked(git_repo.find_unpushed_commit("HEAD").unwrap());
    let original_branch_head = tracked_commit.meta_data().remote_commit;
    let new_tracked_commit = tracked_commit.update_local_branch_head().unwrap();

    assert_eq!(
        new_tracked_commit.meta_data().remote_commit,
        original_branch_head
    );
}

#[test]
fn should_separate_fixup_and_merge_with_main() {
    let remote = RemoteRepo::new();
    let local = remote.clone_repo();
    let local = repo_generator::rebased_local_commit_changed::init_repo(&remote, local);

    let git_repo = GitRepo::open(local.path()).unwrap();

    let tracked_commit = tracked(git_repo.find_unpushed_commit("HEAD").unwrap());
    let new_tracked_commit = tracked_commit.update_local_branch_head().unwrap();

    let expected_diff = indoc! {"
        diff --git a/File1 b/File1
        index 7da932b..78b5315 100644
        --- a/File1
        +++ b/File1
        @@ -1,6 +1,6 @@
         Hello World!
         
        -More lines..
        +More lines.. + fixup
         
         This is my very first file
         
   "};
    local.assert_diff(
        &format!("{}^", new_tracked_commit.meta_data().remote_commit),
        &format!("{}", new_tracked_commit.meta_data().remote_commit),
        expected_diff,
    );

    local.assert_diff(
        &format!("{}^^", new_tracked_commit.meta_data().remote_commit),
        &format!("{}^", new_tracked_commit.meta_data().remote_commit),
        indoc! {"
            diff --git a/File2 b/File2
            new file mode 100644
            index 0000000..1b59044
            --- /dev/null
            +++ b/File2
            @@ -0,0 +1 @@
            +Unrelated file from other commit
        "},
    );

    let local = local.checkout(&format!("{}", new_tracked_commit.meta_data().remote_commit));

    local.assert_log(vec!["Fixup!", "Sync with main!"]);
}
