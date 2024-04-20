use git2::Commit;
use indoc::indoc;
use stackable_commits::commands::diff;

use pretty_assertions::assert_eq;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::{tempdir, TempDir};

#[test]
fn basic_test() {
    let repo = TestRepoWithRemote::new();

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

    diff::diff::<&str, _>(None, current_dir).unwrap();

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
}

#[test]
fn test_diff_from_not_head_commit() {
    let repo = TestRepoWithRemote::new();

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
    diff::diff::<&str, _>(Some(&format!("{}", commit)), current_dir).unwrap();

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
}

struct TestRepoWithRemote {
    local_repo_dir: TempDir,
    _remote_repo_dir: TempDir,
    local_repo: git2::Repository,
}

impl TestRepoWithRemote {
    fn new() -> Self {
        let remote_repo_dir = tempdir().unwrap();
        println!("Remote repo: {}", remote_repo_dir.path().display());
        let _ = git2::Repository::init_bare(remote_repo_dir.path()).unwrap();

        let local_repo_dir = tempdir().unwrap();
        println!("Local repo: {}", local_repo_dir.path().display());
        let local_repo = git2::Repository::clone(
            &String::from_utf8_lossy(remote_repo_dir.path().as_os_str().as_bytes()),
            local_repo_dir.path(),
        )
        .unwrap();
        TestRepoWithRemote {
            local_repo_dir,
            _remote_repo_dir: remote_repo_dir,
            local_repo,
        }
    }

    fn create_file<P>(self, path: P, content: &str) -> Self
    where
        P: AsRef<Path>,
    {
        let file_path = self.local_repo_dir.path().join(path);
        let mut tmp_file = File::create(file_path).unwrap();
        writeln!(tmp_file, "{}", content).unwrap();
        self
    }

    fn append_file<P>(self, path: P, content: &str) -> Self
    where
        P: AsRef<Path>,
    {
        let file_path = self.local_repo_dir.path().join(path);
        let mut tmp_file = OpenOptions::new()
            .append(true)
            .write(true)
            .open(file_path)
            .unwrap();
        writeln!(tmp_file, "{}", content).unwrap();
        self
    }

    fn commit_all(self, msg: &str) -> Self {
        let current_dir = self.local_repo_dir.path();
        assert!(Command::new("git")
            .current_dir(current_dir)
            .arg("add")
            .arg(".")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        assert!(Command::new("git")
            .current_dir(current_dir)
            .arg("commit")
            .arg("-a")
            .arg("-m")
            .arg(msg)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        self
    }

    fn push(self) -> Self {
        let current_dir = self.local_repo_dir.path();

        assert!(Command::new("git")
            .current_dir(current_dir)
            .arg("push")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        self
    }

    fn ls_remote_heads(&self, name: &str) -> Output {
        let current_dir = self.local_repo_dir.path();
        Command::new("git")
            .current_dir(current_dir)
            .arg("ls-remote")
            .arg("--heads")
            .arg("origin")
            .arg(name)
            .output()
            .unwrap()
    }

    fn diff(&self, ref1: &str, ref2: &str) -> Output {
        let current_dir = self.local_repo_dir.path();
        Command::new("git")
            .current_dir(current_dir)
            .arg("diff")
            .arg(ref1)
            .arg(ref2)
            .output()
            .unwrap()
    }

    fn find_commit(&self, ancestors: u32) -> Commit {
        let head = self.local_repo.head().unwrap();

        let mut commit = head.peel_to_commit().unwrap();

        for _ in 0..ancestors {
            commit = commit.parent(0).unwrap();
        }

        commit
    }
}
