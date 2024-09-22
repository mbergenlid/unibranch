use indoc::{formatdoc, indoc};
use pretty_assertions::assert_eq;
use test_repo::{RemoteRepo, TestRepoWithRemote};
use ubr::{
    commands::{create, sync},
    git::{GitRepo, SyncState},
};

fn git_repo(value: &TestRepoWithRemote) -> GitRepo {
    GitRepo::open(value.local_repo_dir.path()).unwrap()
}

#[test]
fn test_merge_conflict_from_remote() {
    let remote_repo = RemoteRepo::new();
    let local_repo = remote_repo
        .clone_repo()
        .create_file("File1", "Hello, World!")
        .commit_all("commit1")
        .push()
        .append_file("File1", "Starting on a new feature")
        .commit_all("feature 1");

    //Create a PR from local repo
    create::execute(
        create::Options {
            commit_ref: Some("HEAD".to_string()),
            force: false,
        },
        git_repo(&local_repo),
    )
    .expect("Unable to create initial PR");

    let remote_head = {
        let another_local_clone = remote_repo.clone_repo();

        another_local_clone
            .checkout("feature-1")
            .append_file("File1", "Some remote fixes")
            .commit_all("Fixup")
            .push()
            .head()
    };

    let local_repo = local_repo
        .append_file("File1", "Some local fixes")
        .commit_all_amend();

    let expected_main_commit_id = local_repo.head();
    let expected_main_parent_id = local_repo.find_commit(1).id();

    let result = sync::execute(sync::Options::default(), git_repo(&local_repo));
    assert!(result.is_err());
    let expected_error_message = formatdoc! {"
        Unable to merge local commit ({local}) with commit from remote ({remote})
        Once all the conflicts has been resolved, run 'ubr sync --continue'
        ",
        local = local_repo.head(),
        remote = remote_head
    };
    assert_eq!(format!("{}", result.unwrap_err()), expected_error_message);

    let sync_state = serde_json::from_reader::<_, SyncState>(
        std::fs::File::open(
            local_repo
                .local_repo_dir
                .path()
                .join(".ubr/SYNC_MERGE_HEAD"),
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        sync_state,
        SyncState {
            remote_commit_id: remote_head.into(),
            main_commit_id: expected_main_commit_id.into(),
            main_commit_parent_id: expected_main_parent_id.into(),
            main_branch_name: "master".to_string()
        }
    );

    let local_repo = local_repo
        .create_file(
            "File1",
            "Hello, World!\nStarting on a new feature\nSome local/remote fixes",
        )
        .add_all();

    {
        let resolved_file = String::from_utf8(
            std::fs::read(local_repo.local_repo_dir.path().join("File1")).unwrap(),
        )
        .unwrap();

        assert_eq!(
            resolved_file,
            "Hello, World!\nStarting on a new feature\nSome local/remote fixes\n"
        );
    }

    sync::execute(sync::Options { cont: true }, git_repo(&local_repo)).expect("Should succeed");

    local_repo.assert_diff(
        "master^",
        "master",
        indoc! {"
        diff --git a/File1 b/File1
        index 8ab686e..7eb283b 100644
        --- a/File1
        +++ b/File1
        @@ -1 +1,3 @@
         Hello, World!
        +Starting on a new feature
        +Some local/remote fixes
        "},
    );
}

//#[test]
//fn test_merge_conflict_with_main() {
//    todo!()
//}

#[test]
fn test_merge_conflict_in_the_middle_of_sync() {
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

    //Create a PR from local repo
    create::execute(
        create::Options {
            commit_ref: Some("HEAD".to_string()),
            force: false,
        },
        git_repo(&local_repo),
    )
    .expect("Unable to create initial PR");

    let remote_head = {
        let another_local_clone = remote_repo.clone_repo();

        another_local_clone
            .checkout("feature-1")
            .append_file("File1", "Some remote fixes")
            .commit_all("Fixup")
            .push()
            .head()
    };

    let local_repo = local_repo
        .append_file("File1", "Some local fixes")
        .commit_all_amend();

    let expected_main_commit_id = local_repo.head();
    let expected_main_parent_id = local_repo.find_commit(1).id();

    let result = sync::execute(sync::Options::default(), git_repo(&local_repo));
    assert!(result.is_err());

    let expected_error_message = formatdoc! {"
        Unable to merge local commit ({local}) with commit from remote ({remote})
        Once all the conflicts has been resolved, run 'ubr sync --continue'
        ",
        local = local_repo.head(),
        remote = remote_head
    };
    assert_eq!(format!("{}", result.unwrap_err()), expected_error_message);

    let sync_state = serde_json::from_reader::<_, SyncState>(
        std::fs::File::open(
            local_repo
                .local_repo_dir
                .path()
                .join(".ubr/SYNC_MERGE_HEAD"),
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        sync_state,
        SyncState {
            remote_commit_id: remote_head.into(),
            main_commit_id: expected_main_commit_id.into(),
            main_commit_parent_id: expected_main_parent_id.into(),
            main_branch_name: "master".to_string()
        }
    );

    //Resolve the conflict
    let local_repo = local_repo
        .create_file(
            "File1",
            "Hello, World!\nStarting on a new feature\nSome local/remote fixes",
        )
        .add_all();

    {
        //Assert that resolution was succesful
        let resolved_file = String::from_utf8(
            std::fs::read(local_repo.local_repo_dir.path().join("File1")).unwrap(),
        )
        .unwrap();

        assert_eq!(
            resolved_file,
            "Hello, World!\nStarting on a new feature\nSome local/remote fixes\n"
        );
    }

    sync::execute(sync::Options { cont: true }, git_repo(&local_repo)).expect("Should succeed");

    local_repo.assert_diff(
        "master^",
        "master",
        indoc! {"
        diff --git a/File1 b/File1
        index 8ab686e..7eb283b 100644
        --- a/File1
        +++ b/File1
        @@ -1 +1,3 @@
         Hello, World!
        +Starting on a new feature
        +Some local/remote fixes
        "},
    );
}

#[test]
fn test_merge_conflict_in_the_middle_of_sync_2() {
    let remote_repo = RemoteRepo::new();
    let local_repo = remote_repo
        .clone_repo()
        .create_file("File1", "Hello, World!")
        .commit_all("commit1")
        .push()
        .create_file("File2", "Unrelated feature")
        .commit_all("unrelated commit 1")
        .append_file("File1", "Starting on a new feature")
        .commit_all("feature 1");

    //Create a PR from local repo
    create::execute(
        create::Options {
            commit_ref: Some("HEAD".to_string()),
            force: false,
        },
        git_repo(&local_repo),
    )
    .expect("Unable to create initial PR");

    let remote_head = {
        let another_local_clone = remote_repo.clone_repo();

        another_local_clone
            .checkout("feature-1")
            .append_file("File1", "Some remote fixes")
            .commit_all("Fixup")
            .push()
            .head()
    };

    let local_repo = local_repo
        .append_file("File1", "Some local fixes")
        .commit_all_amend();

    let expected_main_commit_id = local_repo.head();
    let expected_main_parent_id = local_repo.find_commit(1).id();

    let local_repo = local_repo
        .create_file("File3", "Another unrelated feature")
        .commit_all("unrelated commit 2");

    let result = sync::execute(sync::Options::default(), git_repo(&local_repo));
    assert!(result.is_err());

    let expected_error_message = formatdoc! {"
        Unable to merge local commit ({local}) with commit from remote ({remote})
        Once all the conflicts has been resolved, run 'ubr sync --continue'
        ",
        local = local_repo.head(),
        remote = remote_head
    };
    assert_eq!(format!("{}", result.unwrap_err()), expected_error_message);

    let sync_state = serde_json::from_reader::<_, SyncState>(
        std::fs::File::open(
            local_repo
                .local_repo_dir
                .path()
                .join(".ubr/SYNC_MERGE_HEAD"),
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        sync_state,
        SyncState {
            remote_commit_id: remote_head.into(),
            main_commit_id: expected_main_commit_id.into(),
            main_commit_parent_id: expected_main_parent_id.into(),
            main_branch_name: "master".to_string()
        }
    );

    //Resolve the conflict
    let local_repo = local_repo
        .create_file(
            "File1",
            "Hello, World!\nStarting on a new feature\nSome local/remote fixes",
        )
        .add_all();

    {
        //Assert that resolution was succesful
        let resolved_file = String::from_utf8(
            std::fs::read(local_repo.local_repo_dir.path().join("File1")).unwrap(),
        )
        .unwrap();

        assert_eq!(
            resolved_file,
            "Hello, World!\nStarting on a new feature\nSome local/remote fixes\n"
        );
    }

    sync::execute(sync::Options { cont: true }, git_repo(&local_repo)).expect("Should succeed");

    local_repo.assert_diff(
        "master^^",
        "master^",
        indoc! {"
        diff --git a/File1 b/File1
        index 8ab686e..7eb283b 100644
        --- a/File1
        +++ b/File1
        @@ -1 +1,3 @@
         Hello, World!
        +Starting on a new feature
        +Some local/remote fixes
        "},
    );

    local_repo.assert_diff(
        "master^",
        "master",
        indoc! {"
        diff --git a/File3 b/File3
        new file mode 100644
        index 0000000..cbf4353
        --- /dev/null
        +++ b/File3
        @@ -0,0 +1 @@
        +Another unrelated feature
        "},
    );
}
