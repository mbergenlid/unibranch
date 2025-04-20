use test_repo::{RemoteRepo, TestRepoWithRemote};

use indoc::indoc;
use ubr::{commands::create, git::GitRepo};

/// Creates a repository like this:
///
///```text
///
///          Commit *
///                 |      * remote-commit
///                 |     /
///                 |    * local-branch-head
///                 |   /
///                 |  /
///                 | /
///                 |/
///    First commit * <--------------- (origin/master)
///```
pub fn init_repo(remote_repo: &RemoteRepo, local_repo: TestRepoWithRemote) {
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

    {
        remote_repo
            .clone_repo()
            .checkout("add-more-lines")
            .create_file(
                "File1",
                indoc! {"
                    Hello World!

                    More lines.. + fixup

                    This is my very first file
                "},
            )
            .commit_all("Fixup from remote")
            .push();
    }
}
