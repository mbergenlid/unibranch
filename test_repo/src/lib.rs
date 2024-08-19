use std::{
    fmt::Display,
    fs::{File, OpenOptions},
    io::Write,
    os::unix::ffi::OsStrExt,
    path::Path,
    process::{Command, Output, Stdio},
};

use git2::{Commit, Oid};
use pretty_assertions::assert_eq;
use tempfile::{tempdir, TempDir};

pub struct RemoteRepo {
    dir: TempDir,
}

impl RemoteRepo {
    pub fn new() -> Self {
        let dir = tempdir().unwrap();
        println!("Remote repo: {}", dir.path().display());
        let _ = git2::Repository::init_bare(dir.path()).unwrap();
        RemoteRepo { dir }
    }

    pub fn clone_repo(&self) -> TestRepoWithRemote {
        let local_repo_dir = tempdir().unwrap();
        println!("Local repo: {}", local_repo_dir.path().display());
        let local_repo = git2::Repository::clone(
            &String::from_utf8_lossy(self.dir.path().as_os_str().as_bytes()),
            local_repo_dir.path(),
        )
        .unwrap();
        TestRepoWithRemote {
            local_repo_dir,
            _remote: self,
            local_repo,
        }
    }
}

impl Default for RemoteRepo {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TestRepoWithRemote<'a> {
    pub local_repo_dir: TempDir,
    _remote: &'a RemoteRepo,
    local_repo: git2::Repository,
}

impl<'a> TestRepoWithRemote<'a> {
    #[allow(dead_code)]
    pub fn head_branch(&self) -> String {
        let current_dir = self.local_repo_dir.path();
        String::from_utf8(
            Command::new("git")
                .current_dir(current_dir)
                .arg("branch")
                .arg("--show-current")
                .output()
                .expect("No stdout from branch --show-current")
                .stdout,
        )
        .expect("")
        .trim()
        .to_string()
    }

    #[allow(dead_code)]
    pub fn head(&self) -> Oid {
        self.local_repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
    }

    #[allow(dead_code)]
    pub fn checkout(self, branch: &str) -> Self {
        let current_dir = self.local_repo_dir.path();
        assert!(Command::new("git")
            .current_dir(current_dir)
            .arg("checkout")
            .arg(branch)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());

        self
    }

    #[allow(dead_code)]
    pub fn run_command(&self) -> Command {
        let current_dir = self.local_repo_dir.path();
        let mut command = Command::new("git");

        command
            .current_dir(current_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        command
    }

    pub fn create_file<P>(self, path: P, content: &str) -> Self
    where
        P: AsRef<Path>,
    {
        let file_path = self.local_repo_dir.path().join(path);
        let mut tmp_file = File::create(file_path).unwrap();
        writeln!(tmp_file, "{}", content).unwrap();
        self
    }

    #[allow(dead_code)]
    pub fn append_file<P>(self, path: P, content: &str) -> Self
    where
        P: AsRef<Path>,
    {
        let file_path = self.local_repo_dir.path().join(path);
        let mut tmp_file = OpenOptions::new().append(true).open(file_path).unwrap();
        writeln!(tmp_file, "{}", content).unwrap();
        self
    }

    pub fn add_all(self) -> Self {
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
        self
    }

    pub fn commit_all(self, msg: &str) -> Self {
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

    #[allow(dead_code)]
    pub fn commit_all_amend(self) -> Self {
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
            .arg("--amend")
            .arg("--no-edit")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        self
    }

    #[allow(dead_code)]
    pub fn commit_all_amend_with_message(self, message: &str) -> Self {
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
            .arg("--amend")
            .arg("-m")
            .arg(message)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        self
    }

    #[allow(dead_code)]
    pub fn commit_all_fixup(self, fixup_commit: Oid) -> Self {
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
            .arg(&format!("--fixup={}", fixup_commit))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        assert!(Command::new("git")
            .current_dir(current_dir)
            .arg("-c")
            .arg("sequence.editor=:")
            .arg("rebase")
            .arg("-i")
            .arg("--autosquash")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .unwrap()
            .success());
        self
    }

    pub fn push(self) -> Self {
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

    #[allow(dead_code)]
    pub fn pull_rebase(self) -> Self {
        let current_dir = self.local_repo_dir.path();

        assert!(Command::new("git")
            .current_dir(current_dir)
            .arg("pull")
            .arg("--rebase")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        self
    }

    #[allow(dead_code)]
    pub fn fetch(self) -> Self {
        let current_dir = self.local_repo_dir.path();

        assert!(Command::new("git")
            .current_dir(current_dir)
            .arg("fetch")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        self
    }

    #[allow(dead_code)]
    pub fn fetch_ref(self, rev: &str) -> Self {
        let current_dir = self.local_repo_dir.path();

        assert!(Command::new("git")
            .current_dir(current_dir)
            .arg("fetch")
            .arg(rev)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        self
    }

    #[allow(dead_code)]
    pub fn ls_remote_heads(&self, name: &str) -> Output {
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

    #[allow(dead_code)]
    pub fn diff(&self, ref1: &str, ref2: &str) -> Output {
        let current_dir = self.local_repo_dir.path();
        Command::new("git")
            .current_dir(current_dir)
            .arg("diff")
            .arg(ref1)
            .arg(ref2)
            .output()
            .unwrap()
    }

    #[allow(dead_code)]
    pub fn show(&self, rev: &str) {
        let current_dir = self.local_repo_dir.path();
        let out = String::from_utf8(
            Command::new("git")
                .current_dir(current_dir)
                .arg("show")
                .arg(rev)
                .output()
                .unwrap()
                .stdout,
        )
        .expect("git show is not valid UTF-8");
        println!("{}", out);
    }

    #[allow(dead_code)]
    pub fn print_log(&self) {
        let current_dir = self.local_repo_dir.path();
        let out = String::from_utf8(
            Command::new("git")
                .current_dir(current_dir)
                .arg("log")
                .output()
                .unwrap()
                .stdout,
        )
        .expect("git log is not valid UTF-8");
        println!("{}", out);
    }

    #[allow(dead_code)]
    pub fn find_note(&self, rev: &str) -> String {
        let current_dir = self.local_repo_dir.path();

        let out = Command::new("git")
            .current_dir(current_dir)
            .arg("notes")
            .arg("show")
            .arg(rev)
            .output()
            .unwrap();
        String::from_utf8(out.stdout).expect("Output is not valid UTF-8")
    }

    pub fn find_commit(&self, ancestors: u32) -> Commit {
        let head = self.local_repo.head().unwrap();

        let mut commit = head.peel_to_commit().unwrap();

        for _ in 0..ancestors {
            commit = commit.parent(0).unwrap();
        }

        commit
    }

    #[allow(dead_code)]
    pub fn find_commit_by_reference(&self, reference: &str) -> Commit {
        self.local_repo
            .find_reference(reference)
            .unwrap()
            .peel_to_commit()
            .unwrap()
    }

    #[allow(dead_code)]
    pub fn rev_parse(&self, rev: &str) -> String {
        let current_dir = self.local_repo_dir.path();

        let out = Command::new("git")
            .current_dir(current_dir)
            .arg("rev-parse")
            .arg(rev)
            .output()
            .unwrap();
        String::from_utf8(out.stdout)
            .expect("Output is not valid UTF-8")
            .trim()
            .to_string()
    }

    #[allow(dead_code)]
    pub fn assert_log(&self, messages: Vec<&str>) {
        for (index, expected_message) in messages.into_iter().enumerate() {
            let local_commit = &self.find_commit(index as u32);
            let actual_message = local_commit.message().unwrap();
            assert_eq!(actual_message, expected_message);
        }
    }

    #[allow(dead_code)]
    pub fn assert_note<D: Display>(&self, rev: &str, expected_note: D) {
        let note = self.find_note(rev);
        assert_eq!(
            note,
            format!("{}", expected_note),
            "comparing note for commit {}",
            rev
        )
    }

    #[allow(dead_code)]
    pub fn assert_diff(&self, rev1: &str, rev2: &str, expected: &str) {
        let actual = String::from_utf8(self.diff(rev1, rev2).stdout).unwrap();
        assert_eq!(actual, expected, "comparing diff {} -> {}", rev1, rev2,)
    }

    pub fn assert_tracked_commit_in_sync(&self, tracked_commit_id: Oid, remote_head: Oid) {
        let rev1 = &format!("{}^", tracked_commit_id);
        let rev2 = &format!("{}", tracked_commit_id);
        let main_diff = String::from_utf8(self.diff(rev1, rev2).stdout).unwrap();

        self.assert_diff("origin/master", &format!("{}", remote_head), &main_diff);
    }
}
