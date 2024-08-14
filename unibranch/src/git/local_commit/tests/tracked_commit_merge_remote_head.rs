use indoc::indoc;
use test_repo::RemoteRepo;

use pretty_assertions::assert_eq;

use crate::{
    commands::create,
    git::{local_commit::tracked_commit, GitRepo},
};

//          *                                        *
//          |                                        |
//          |                                        |
//          *      * (remote_branch_head)   ===>     *      * (remote_branch_head, local_branch_head)
//          |     /                                  |     /
//          |    /                                   |    /
//       c1 *   * (local_branch_head)                *   *
//          |  /                                     |  /
//          | /                                      | /
// (origin) *                                        *
#[test]
fn should_not_merge_if_remote_commit_is_descendant_of_local() {
    let remote = RemoteRepo::new();
    let local = remote.clone();

    let local = local
        .create_file("file1", "Hello, World!")
        .commit_all("Initial")
        .push();

    let local = local
        .create_file("file2", "another file")
        .commit_all("Commit 1");

    {
        create::execute(create::Options::default(), &local.local_repo_dir).unwrap()
    }

    let remote_branch_head = {
        remote
            .clone()
            .checkout("commit-1")
            .append_file("file2", "Some fixes")
            .commit_all("Fixes")
            .push()
            .head()
    };

    let git_repo = GitRepo::open(&local.local_repo_dir).unwrap();
    let local = local.fetch();

    let tracked_commit = super::tracked(git_repo.find_unpushed_commit("HEAD").unwrap());

    //When
    let tracked_commit = tracked_commit.merge_remote_head(None).unwrap();
    let rev_str = format!("{}", tracked_commit.as_commit().id());
    assert_eq!(
        tracked_commit.meta_data().remote_commit,
        Some(remote_branch_head)
    );

    local.assert_note(&rev_str, tracked_commit.meta_data());

    let diff = String::from_utf8(local.diff(&format!("{}^", rev_str), &format!("{}", rev_str)).stdout).unwrap();

    let expected_diff = indoc! {"
        diff --git a/file2 b/file2
        new file mode 100644
        index 0000000..684bd12
        --- /dev/null
        +++ b/file2
        @@ -0,0 +1,2 @@
        +another file
        +Some fixes
    "};

    assert_eq!(diff, expected_diff);
}

//
//          *                                        *
//          |                                        |
//          |                                        |
//          *      * (local_branch_head)   ===>     *      * (local_branch_head, remote_branch_head)
//          |     /                                  |     /
//          |    /                                   |    /
//       c1 *   * (remote_branch_head)               *   *
//          |  /                                     |  /
//          | /                                      | /
// (origin) *                                        *
#[test]
fn should_not_merge_if_local_commit_is_descendant_of_remote() {
    let remote = RemoteRepo::new();
    let local = remote.clone();

    let local = local
        .create_file("file1", "Hello, World!")
        .commit_all("Initial")
        .push();

    let local = local
        .create_file("file2", "another file")
        .commit_all("Commit 1");

    {
        // Create initial PR
        create::execute(create::Options::default(), &local.local_repo_dir).unwrap()
    }

    let git_repo = GitRepo::open(&local.local_repo_dir).unwrap();
    let local = {
        // Make a new local commit

        let local = local.append_file("file2", "some fixes").commit_all_amend();

        let tracked_commit = super::tracked(git_repo.find_unpushed_commit("HEAD").unwrap());
        tracked_commit.update_local_branch_head().unwrap();
        local
    };

    let _local = local.fetch();

    let tracked_commit = super::tracked(git_repo.find_unpushed_commit("HEAD").unwrap());
    let local_branch_head = tracked_commit.meta_data().remote_commit;
    let tracked_commit = tracked_commit.merge_remote_head(None).unwrap();

    assert_eq!(tracked_commit.meta_data().remote_commit, local_branch_head);
}

//
//          *                                        *
//          |                                        |    * (local_branch_head)
//          |                                        |   / \
//          *      * (remote_branch_head)  ===>      *  /   *
//          | * <-/- (local_branch_head)             | *   /
//          |  \ /                                   |  \ /
//       c1 *   *                                    *   *
//          |  /                                     |  /
//          | /                                      | /
// (origin) *                                        *
#[test]
fn test_merge() {
    let remote = RemoteRepo::new();
    let local = remote.clone();

    let local = local
        .create_file("file1", "Hello, World!")
        .commit_all("Initial")
        .push();

    let local = local
        .create_file("file2", "another file")
        .commit_all("Commit 1");

    {
        // Create initial PR
        create::execute(create::Options::default(), &local.local_repo_dir).unwrap()
    }

    let git_repo = GitRepo::open(&local.local_repo_dir).unwrap();
    let (local, tracked_commit) = {
        // Make a new local commit

        let local = local.append_file("file2", "some fixes").commit_all_amend();

        let tracked_commit = super::tracked(git_repo.find_unpushed_commit("HEAD").unwrap());
        let tracked_commit = tracked_commit.update_local_branch_head().unwrap();
        (local, tracked_commit)
    };

    let remote_branch_head = {
        remote
            .clone()
            .checkout("commit-1")
            .create_file("file3", "Some fixes in file3")
            .commit_all("Fixes")
            .push()
            .head()
    };

    let _local = local.fetch();

    let tracked_commit = tracked_commit.merge_remote_head(None).unwrap();

    //    assert_eq!(tracked_commit.meta_data().remote_commit, local_branch_head);
}
