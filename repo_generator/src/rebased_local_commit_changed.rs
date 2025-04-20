use test_repo::{RemoteRepo, TestRepoWithRemote};

use indoc::indoc;
use ubr::{commands::create, git::GitRepo};

/// Creates a repository like this:
///
///```text
///
///          Commit * (amended)
///                 |
///                 |
///                 |    * local_branch_head
///                 *<--/------------ (origin/master)
///                 |  /
///                 | /
///                 |/
///    First commit *
///```
pub fn init_repo<'a>(
    remote_repo: &RemoteRepo,
    local_repo: TestRepoWithRemote<'a>,
) -> TestRepoWithRemote<'a> {
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

    let git_repo = GitRepo::open(local_repo.path()).unwrap();
    create::execute(create::Options::default(), git_repo).unwrap();

    let local_repo = local_repo
        .create_file(
            "File1",
            indoc! {"
                Hello World!

                More lines.. + fixup

                This is my very first file
            "},
        )
        .commit_all_amend();

    {
        remote_repo
            .clone_repo()
            .create_file("File2", "Unrelated file from other commit")
            .commit_all("Unrelated commit")
            .push();
    }

    local_repo.pull_rebase()
}
