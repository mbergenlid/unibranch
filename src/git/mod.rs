use std::{ffi::CString, path::Path};

use anyhow::{Context, Ok};
use clap::builder::OsStr;
use git2::{Commit, Index, Oid, Repository, RepositoryOpenFlags};

pub struct GitRepo {
    repo: git2::Repository,
    pub base_commit_id: Oid,
}

impl GitRepo {
    pub fn open<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let repo = Repository::open_ext(
            path.as_ref(),
            RepositoryOpenFlags::empty(),
            &[] as &[&OsStr],
        )
        .context("Opening git repository")?;
        let head = repo.head().context("No head")?;
        if !head.is_branch() {
            anyhow::bail!("Detached HEAD");
        }

        let current_branch_name = head.name().expect("Branch must have a name");
        let remote_ref = format!(
            "refs/remotes/origin/{}",
            current_branch_name
                .split("/")
                .last()
                .expect("Must have at least one element")
        );
        let base_commit_id = repo.refname_to_id(&remote_ref)?;

        drop(head);
        Ok(GitRepo {
            repo,
            base_commit_id,
        })
    }

    pub fn base_commit(&self) -> anyhow::Result<Commit> {
        Ok(self.repo.find_commit(self.base_commit_id)?)
    }

    pub fn head(&self) -> anyhow::Result<Commit> {
        Ok(self.repo.head()?.peel_to_commit()?)
    }

    pub fn find_head_of_remote_branch(&self, branch_name: &str) -> Option<Commit> {
        self.repo
            .find_branch(&format!("origin/{}", branch_name), git2::BranchType::Remote)
            .ok()
            .and_then(|b| b.get().peel_to_commit().ok())
    }

    pub fn find_unpushed_commit_by_id(&self, id: Oid) -> anyhow::Result<Commit> {
        let commit: git2::Oid = self
            .unpushed_commits()?
            .into_iter()
            .find(|&oid| oid == id)
            .with_context(|| format!("Unable to find revision {}", id))?;

        Ok(self.repo.find_commit(commit)?)
    }

    fn unpushed_commits(&self) -> anyhow::Result<Vec<Oid>> {
        let mut walk = self.repo.revwalk()?;
        walk.set_sorting(git2::Sort::TOPOLOGICAL.union(git2::Sort::REVERSE))?;
        walk.push_head()?;
        walk.hide(self.base_commit_id)?;

        Ok(walk.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn cherry_pick_commit(
        &self,
        commit: Commit,
        pr_head: Option<Commit>,
    ) -> anyhow::Result<Option<Commit>> {
        //TODO: Ignore empty commits
        //TODO: Commit message should be fixup!
        let index = self.repo.cherrypick_commit(
            &commit,
            &self.repo.find_commit(self.base_commit_id)?,
            0,
            None,
        )?;

        let base_commit = pr_head.unwrap_or_else(|| {
            self.repo
                .find_commit(self.base_commit_id)
                .expect("No commit for base commit id")
        });
        let diff = self
            .repo
            .diff_tree_to_index(Some(&base_commit.tree()?), Some(&index), None)?;

        if diff.deltas().len() == 0 {
            println!("Already up to date");
            Ok(None)
        } else {
            let index = self
                .repo
                .apply_to_tree(&base_commit.tree().unwrap(), &diff, None)?;
            for i in index.iter() {
                dbg!(i);
            }
            Ok(Some(self.commit_index(index, commit, base_commit.id())?))
        }
    }

    fn commit_index(
        &self,
        mut index: Index,
        commit: Commit,
        parent: Oid,
    ) -> anyhow::Result<Commit> {
        if index.has_conflicts() {
            for c in index.conflicts()? {
                let c = c?;
                println!("Conclict {:?}", CString::new(c.our.unwrap().path).unwrap())
            }
            anyhow::bail!(
                "This commit cannot be cherry-picked on {}",
                self.base_commit_id
            );
        }

        let tree = index
            .write_tree_to(&self.repo)
            .context("Write index to tree")?;
        println!("Writing tree of cherry-pick {}", tree);
        let tree = self
            .repo
            .find_tree(tree)
            .context("Can not find tree just created")?;

        let base_commit = self.repo.find_commit(parent)?;
        let committer = self.repo.signature().or_else(|_| {
            git2::Signature::now(
                String::from_utf8_lossy(commit.committer().name_bytes()).as_ref(),
                String::from_utf8_lossy(commit.committer().email_bytes()).as_ref(),
            )
        })?;

        let author = git2::Signature::now(
            String::from_utf8_lossy(commit.author().name_bytes()).as_ref(),
            String::from_utf8_lossy(commit.author().email_bytes()).as_ref(),
        )?;
        let cherry_picked_commit = self
            .repo
            .commit(
                None,
                &author,
                &committer,
                commit.message().expect("No commit message"),
                &tree,
                &[&base_commit],
            )
            .context("Committing")?;
        Ok(self.repo.find_commit(cherry_picked_commit)?)
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use tempfile::tempdir;

    use super::GitRepo;

    #[test]
    fn open_git_repo_from_subdir() {
        let dir = tempdir().unwrap();

        let subdir_path = dir.path().join("dir1");
        std::fs::create_dir_all(subdir_path).unwrap();
        let file_path = dir.path().join("dir1/file1");
        let mut tmp_file = File::create(file_path).unwrap();
        writeln!(tmp_file, "This is a file").unwrap();

        let repo = git2::Repository::init(dir.path()).unwrap();
        assert!(Command::new("git")
            .current_dir(dir.path())
            .arg("add")
            .arg(".")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        assert!(Command::new("git")
            .current_dir(dir.path())
            .arg("commit")
            .arg("-a")
            .arg("-m")
            .arg("Test")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());

        std::fs::create_dir_all(dir.path().join(".git/refs/remotes/origin/")).unwrap();
        let mut ref_file =
            File::create(dir.path().join(".git/refs/remotes/origin/master")).unwrap();
        writeln!(
            ref_file,
            "{}",
            repo.head().unwrap().peel_to_commit().unwrap().id()
        )
        .unwrap();

        let repo = GitRepo::open(dir.path().join("dir1/"));
        assert!(repo.is_ok(), "{:?}", repo.err());
    }
}
