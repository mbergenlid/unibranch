use test_repo::{RemoteRepo, TestRepoWithRemote};

use indoc::indoc;
use ubr::commands::create;

/// Creates a repository like this:
///
///
///          Commit * (amended)
///                 |
///                 |
///                 |    * remote-commit
///                 |   /
///                 |  /
///                 | /
///                 |/
///    First commit * <-----------(origin/master)
pub fn init_repo(local_repo: TestRepoWithRemote) {
    let local_repo = local_repo
        .create_file(
            "File1",
            indoc! {"
                Hello World!

                This is my very first file
                "},
        )
        .commit_all("First commit")
        .push()
        .create_file(
            "File1",
            indoc! {"
                Hello World!

                More lines..

                This is my very first file
            "},
        )
        .commit_all("add more lines");

    let git_repo = ubr::git::GitRepo::open(local_repo.path()).unwrap();
    create::execute(
        create::Options {
            force: false,
            commit_ref: None,
        },
        git_repo,
    )
    .unwrap();

    local_repo
        .create_file(
            "File1",
            indoc! {"
                Hello World!

                More lines + some fixes..

                This is my very first file
            "},
        )
        .commit_all_amend();
}
