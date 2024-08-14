use test_repo::RemoteRepo;

use ubr::{
    commands::{create, pull},
    git::local_commit::CommitMetadata,
};

use indoc::indoc;
use pretty_assertions::assert_eq;

#[test]
fn update_commit_from_remote() {
    let remote_repo = RemoteRepo::new();
    let local_repo = remote_repo
        .clone_repo()
        .create_file("File1", "Hello, World!")
        .commit_all("commit1")
        .push()
        .append_file("File1", "Some more changes")
        .commit_all("pr commit");

    //Create a PR from local repo
    create::execute(
        create::Options {
            dry_run: false,
            commit_ref: Some("HEAD".to_string()),
        },
        &local_repo.local_repo_dir,
    )
    .expect("Unable to create initial PR");

    let another_local_clone = remote_repo.clone_repo();

    let another_local_clone = another_local_clone
        .checkout("pr-commit")
        .append_file("File1", "Remote fixes")
        .commit_all("Fixup")
        .push();

    pull::execute(pull::Options::default(), &local_repo.local_repo_dir)
        .expect("Error while running pull command");

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

    local_repo.assert_note(
        "HEAD",
        &CommitMetadata {
            remote_branch_name: std::borrow::Cow::Owned("pr-commit".to_string()),
            remote_commit: Some(
                another_local_clone
                    .rev_parse("pr-commit")
                    .parse()
                    .expect("Not a valid object id"),
            ),
        },
    );
}

#[test]
fn update_commit_from_remote_with_local_changes() {
    let remote_repo = RemoteRepo::new();
    let local_repo = remote_repo
        .clone_repo()
        .create_file("File1", "Hello, World!")
        .commit_all("commit1")
        .push()
        .append_file("File1", "Some more changes")
        .commit_all("pr commit");

    //Create a PR from local repo
    create::execute(
        create::Options {
            dry_run: false,
            commit_ref: Some("HEAD".to_string()),
        },
        &local_repo.local_repo_dir,
    )
    .expect("Unable to create initial PR");

    let local_repo = local_repo
        .create_file("File2", "Some other changes")
        .commit_all_amend();

    {
        let another_local_clone = remote_repo.clone_repo();

        let _another_local_clone = another_local_clone
            .checkout("pr-commit")
            .append_file("File1", "Remote fixes")
            .commit_all("Fixup")
            .push();
    }

    let local_commit_diff =
        String::from_utf8(local_repo.diff("master^", "master").stdout).expect("Getting diff");
    assert_eq!(
        local_commit_diff,
        indoc! {"
            diff --git a/File1 b/File1
            index 8ab686e..3c34bd3 100644
            --- a/File1
            +++ b/File1
            @@ -1 +1,2 @@
             Hello, World!
            +Some more changes
            diff --git a/File2 b/File2
            new file mode 100644
            index 0000000..9eed636
            --- /dev/null
            +++ b/File2
            @@ -0,0 +1 @@
            +Some other changes
        "},
        "Pre update validation"
    );

    //Perform the actual update
    pull::execute(pull::Options::default(), &local_repo.local_repo_dir)
        .expect("Unable to perform pull command");

    let local_commit_diff =
        String::from_utf8(local_repo.diff("master^", "master").stdout).expect("Getting diff");
    assert_eq!(
        local_commit_diff,
        indoc! {"
            diff --git a/File1 b/File1
            index 8ab686e..6a56b5e 100644
            --- a/File1
            +++ b/File1
            @@ -1 +1,3 @@
             Hello, World!
            +Some more changes
            +Remote fixes
            diff --git a/File2 b/File2
            new file mode 100644
            index 0000000..9eed636
            --- /dev/null
            +++ b/File2
            @@ -0,0 +1 @@
            +Some other changes
        "},
        "Local 'master' commit hasn't been updated with the remote changes"
    );

    assert_eq!(local_repo.head_branch(), "master");

    local_repo.assert_note(
        "HEAD",
        &CommitMetadata {
            remote_branch_name: std::borrow::Cow::Owned("pr-commit".to_string()),
            remote_commit: Some(
                local_repo
                    .rev_parse("origin/pr-commit")
                    .parse()
                    .expect("Not a valid object id"),
            ),
        },
    );
}

#[test]
fn sync_multiple_commits() {
    let remote_repo = RemoteRepo::new();
    let local_repo = remote_repo
        .clone_repo()
        .create_file("File1", "Hello, World!")
        .commit_all("commit1")
        .push()
        .append_file("File1", "Some more changes")
        .commit_all("first pr")
        .create_file("File2", "Unrelated feature")
        .commit_all("second pr");

    //second pr
    create::execute(
        create::Options {
            dry_run: false,
            commit_ref: Some("HEAD".to_string()),
        },
        &local_repo.local_repo_dir,
    )
    .unwrap();

    //first pr
    create::execute(
        create::Options {
            dry_run: false,
            commit_ref: Some("HEAD^".to_string()),
        },
        &local_repo.local_repo_dir,
    )
    .unwrap();

    let another_local_clone = remote_repo.clone_repo();

    let another_local_clone = another_local_clone
        .checkout("first-pr")
        .append_file("File1", "Remote fixes")
        .commit_all("Fixup")
        .push();
    let _another_local_clone = another_local_clone
        .checkout("second-pr")
        .append_file("File2", "Remote fixes")
        .commit_all("Fixup")
        .push()
        .show("HEAD^");

    pull::execute(pull::Options::default(), &local_repo.local_repo_dir).unwrap();

    let second_pr_diff =
        String::from_utf8(local_repo.diff("master^", "master").stdout).expect("Getting diff");
    assert_eq!(
        second_pr_diff,
        indoc! {"
            diff --git a/File2 b/File2
            new file mode 100644
            index 0000000..d9e3866
            --- /dev/null
            +++ b/File2
            @@ -0,0 +1,2 @@
            +Unrelated feature
            +Remote fixes
        "},
        "Local 'master' of second commit hasn't been updated with the remote changes"
    );

    let first_pr_diff =
        String::from_utf8(local_repo.diff("master^^", "master^").stdout).expect("Getting diff");
    assert_eq!(
        first_pr_diff,
        indoc! {"
            diff --git a/File1 b/File1
            index 8ab686e..6a56b5e 100644
            --- a/File1
            +++ b/File1
            @@ -1 +1,3 @@
             Hello, World!
            +Some more changes
            +Remote fixes
        "},
        "Local 'master' of first commit hasn't been updated with the remote changes"
    );
}

#[test]
fn test_update_after_rebase_of_main() {}
