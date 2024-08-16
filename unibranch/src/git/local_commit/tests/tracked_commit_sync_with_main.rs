use test_repo::RemoteRepo;

use crate::git::GitRepo;

use super::setup_repo;
//
//
//                         * (Merge with 'main') <---- Produces this merge
//                 *      /  \
//                 |     /    * (Merge)
//                 |    /    / \
//           c1    *   /    /   * (remote_branch_head)
//                 |  /    * <-/------------------------(local_branch_head)
//                 | /      \ /
//     (origin)    *         *
//                 |        /
//                 |       /
//                 *------/
//

//
//
//
//                 *
//                 |
//                 |
//           c1    *   * (local_branch_head)
//                 |  /
//                 | /
//     (origin)    *
//
// Assume that meta_data.remote_head is up-to-date.
#[test]
fn nothing_should_happen_if_origin_has_not_changed() {
    let remote = RemoteRepo::new();
    let local = setup_repo(&remote);

    let git_repo = GitRepo::open(local.local_repo_dir.path()).unwrap();
    let tracked_commit = super::tracked(git_repo.find_unpushed_commit("HEAD").unwrap());
    local.print_log();

    let new_tracked_commit = tracked_commit.clone().sync_with_main().unwrap();

    assert_eq!(
        new_tracked_commit.as_commit().id(),
        tracked_commit.as_commit().id()
    );
}

//
//
//
//                 *
//                 |
//                 |    -* (new_local_branch_head
//           c1    *   /  \
//                 |  /    \
//                 | /      \
//     (origin)    *         * (original_local_branch_head)
//                 |        /
//                 |       /
//                 *------/
//
#[test]
fn new_changes_main_should_be_merged_in() {
    let remote = RemoteRepo::new();
    let local = setup_repo(&remote);

    {
        remote
            .clone_repo()
            .create_file("file3", "Some other feature")
            .commit_all("other feature")
            .push();
    }

    let local = local.pull_rebase();
    local.print_log();

    let git_repo = GitRepo::open(local.local_repo_dir.path()).unwrap();

    let tracked_commit = super::tracked(git_repo.find_unpushed_commit("HEAD").unwrap());

    let new_tracked_commit = tracked_commit.clone().sync_with_main().unwrap();

    local.assert_tracked_commit_in_sync(
        new_tracked_commit.as_commit().id(),
        new_tracked_commit.meta_data().remote_commit,
    );
}
