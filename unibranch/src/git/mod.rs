use std::{
    ffi::CString,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Ok};
use clap::builder::OsStr;
use git2::{Commit, Index, Note, Oid, Repository, RepositoryOpenFlags};

use self::local_commit::{CommitMetadata, MainCommit};

pub mod local_commit;

pub struct GitRepo {
    repo: git2::Repository,
    pub current_branch_name: String,
    path: PathBuf,
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
        let current_branch_name = current_branch_name
            .strip_prefix("refs/heads/")
            .expect("Unknown branch format");
        let current_branch_name = current_branch_name.into();

        drop(head);

        let mut config = repo.config()?;
        config.set_str("notes.rewriteRef", "refs/notes/*")?;
        Ok(GitRepo {
            repo,
            path: path.as_ref().into(),
            current_branch_name,
        })
    }

    pub fn find_local_branch_commit(&self, local_commit: &Commit) -> anyhow::Result<Commit> {
        let note = self.find_note_for_commit(local_commit.id())?;
        let commit_meta_data: CommitMetadata = note
            .as_ref()
            .and_then(|n| n.message().expect("Not valid UTF-8").try_into().ok())
            .unwrap();

        let local_branch_commit = if let Some(remote_commit_id) = commit_meta_data.remote_commit {
            self.repo.find_commit(remote_commit_id)?
        } else {
            self.repo
                .find_branch(
                    &format!("origin/{}", commit_meta_data.remote_branch_name),
                    git2::BranchType::Remote,
                )?
                .get()
                .peel_to_commit()?
        };
        Ok(local_branch_commit)
    }

    pub fn base_commit(&self) -> anyhow::Result<Commit> {
        let remote_ref = format!("refs/remotes/origin/{}", self.current_branch_name);
        let base_commit_id = self.repo.refname_to_id(&remote_ref)?;
        Ok(self.repo.find_commit(base_commit_id)?)
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

    pub fn find_unpushed_commit(&self, commit_ref: &str) -> anyhow::Result<MainCommit> {
        let (obj, _) = self
            .repo
            .revparse_ext(commit_ref)
            .with_context(|| format!("Bad revision '{}'", commit_ref))?;
        let commit = obj.peel_to_commit()?;
        if !self
            .repo
            .graph_descendant_of(commit.id(), self.base_commit()?.id())?
        {
            anyhow::bail!(format!(
                "Commit {} is already pushed to the remote",
                commit.id()
            ));
        }

        Ok(MainCommit::new(self, &self.repo, commit)?)
    }

    pub fn find_meta_data_for_commit(
        &self,
        commit_id: Oid,
    ) -> anyhow::Result<Option<CommitMetadata>> {
        if let Some(note) = self.find_note_for_commit(commit_id)? {
            if let Some(msg) = note.message() {
                let mut commit_meta_data: CommitMetadata = msg.parse()?;
                if commit_meta_data.remote_commit.is_none() {
                    let id = self
                        .repo
                        .find_branch(
                            &format!("origin/{}", commit_meta_data.remote_branch_name),
                            git2::BranchType::Remote,
                        )?
                        .get()
                        .peel_to_commit()?
                        .id();
                    commit_meta_data.remote_commit.replace(id);
                }
                return Ok(Some(commit_meta_data));
            }
        }
        Ok(None)
    }

    pub fn find_note_for_commit(&self, commit_id: Oid) -> anyhow::Result<Option<Note>> {
        let res = self.repo.find_note(None, commit_id);
        if let Err(error) = res {
            match error.code() {
                git2::ErrorCode::NotFound => return Ok(None),
                _ => anyhow::bail!(error),
            }
        }
        let note = res.expect("Already checked for error above");
        Ok(Some(note))
    }

    pub fn save_meta_data(
        &self,
        commit: &Commit,
        meta_data: &CommitMetadata,
    ) -> Result<(), git2::Error> {
        let committer = self.repo.signature().or_else(|_| {
            git2::Signature::now(
                String::from_utf8_lossy(commit.committer().name_bytes()).as_ref(),
                String::from_utf8_lossy(commit.committer().email_bytes()).as_ref(),
            )
        })?;
        self.repo.note(
            &committer,
            &committer,
            None,
            commit.id(),
            &format!("{}", meta_data),
            true,
        )?;
        std::result::Result::Ok(())
    }

    pub fn save_merge_state(&self, _commit1: &Commit, commit2: &Commit) -> anyhow::Result<()> {
        std::fs::create_dir_all(format!("{}/.ubr", self.path.display()))?;
        let mut file =
            std::fs::File::create_new(format!("{}/.ubr/SYNC_MERGE_HEAD", self.path.display()))?;
        file.write_all(format!("{}\n", commit2.id()).as_bytes())?;
        Ok(())
    }

    pub fn rewrite_local_commit(
        &self,
        commit: &Commit,
        config: &CommitMetadata,
    ) -> anyhow::Result<()> {
        let branch = self
            .repo
            .reference_to_annotated_commit(&self.repo.head()?)?;
        let remote = self.repo.reference_to_annotated_commit(
            self.repo
                .find_branch(
                    &format!("origin/{}", &self.current_branch_name),
                    git2::BranchType::Remote,
                )?
                .get(),
        )?;
        let mut rebase = self.repo.rebase(Some(&branch), Some(&remote), None, None)?;

        let committer = self.repo.signature().or_else(|_| {
            git2::Signature::now(
                String::from_utf8_lossy(commit.committer().name_bytes()).as_ref(),
                String::from_utf8_lossy(commit.committer().email_bytes()).as_ref(),
            )
        })?;
        while let Some(op) = rebase.next() {
            let op = op?;
            if op.id() == commit.id() {
                rebase.commit(
                    None,
                    &committer,
                    Some(&format!(
                        "{}\nmeta:\n{}",
                        commit.message().expect("No commmit message"),
                        config,
                    )),
                )?;
            } else {
                rebase.commit(None, &committer, None)?;
            }
        }
        let _ = rebase.finish(None);
        Ok(())
    }

    pub fn unpushed_commits(&self) -> anyhow::Result<Vec<MainCommit>> {
        let mut walk = self.repo.revwalk()?;
        walk.set_sorting(git2::Sort::TOPOLOGICAL.union(git2::Sort::REVERSE))?;
        walk.push_head()?;
        walk.hide(self.base_commit()?.id())?;

        let result: Result<Vec<_>, _> = walk
            .map(|r| self.repo.find_commit(r.expect("whhat")).unwrap())
            .map(|c| MainCommit::new(self, &self.repo, c))
            .collect();
        Ok(result?)
    }

    pub fn cherry_pick_commit(
        &self,
        original_commit: &Commit,
        pr_head: Option<Commit>,
    ) -> anyhow::Result<Option<Commit>> {
        let base_commit = self.repo.find_commit(self.base_commit()?.id())?;
        let complete_index = self
            .repo
            .cherrypick_commit(original_commit, &base_commit, 0, None)
            .context("Cherry picking directly on master")?;

        if complete_index.has_conflicts() {
            anyhow::bail!("There are conflicts");
        }
        let parent_commit = pr_head.unwrap_or(base_commit);
        let diff = self.repo.diff_tree_to_index(
            Some(&parent_commit.tree()?),
            Some(&complete_index),
            None,
        )?;

        let first_pr_commit = {
            let mut walk = self.repo.revwalk()?;
            walk.set_sorting(git2::Sort::TOPOLOGICAL.union(git2::Sort::REVERSE))?;
            walk.push(parent_commit.id())?;
            walk.hide(self.base_commit()?.id())?;
            walk.next().and_then(|r| r.ok())
        };

        if let Some(first_commit_id) = first_pr_commit {
            let first_commit = self.repo.find_commit(first_commit_id)?;
            if first_commit.message() != original_commit.message() {
                println!("Commit message changed, need to update");
            }
        }

        if diff.deltas().len() == 0 {
            println!("Already up to date");
            Ok(None)
        } else {
            let index = self
                .repo
                .apply_to_tree(&parent_commit.tree().unwrap(), &diff, None)
                .context("Apply diff to parent")?;

            if parent_commit.id() == self.base_commit()?.id() {
                Ok(Some(self.commit_index(
                    index,
                    original_commit,
                    parent_commit.id(),
                    original_commit.message().expect("No commit message"),
                )?))
            } else {
                Ok(Some(self.commit_index(
                    index,
                    original_commit,
                    parent_commit.id(),
                    &format!("Fixup! {}", parent_commit.id()),
                )?))
            }
        }
    }

    fn commit_index(
        &self,
        mut index: Index,
        original_commit: &Commit,
        parent: Oid,
        message: &str,
    ) -> anyhow::Result<Commit> {
        if index.has_conflicts() {
            for c in index.conflicts()? {
                let c = c?;
                println!("Conclict {:?}", CString::new(c.our.unwrap().path).unwrap())
            }
            anyhow::bail!(
                "This commit cannot be cherry-picked on {}",
                self.base_commit()?.id()
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
                String::from_utf8_lossy(original_commit.committer().name_bytes()).as_ref(),
                String::from_utf8_lossy(original_commit.committer().email_bytes()).as_ref(),
            )
        })?;

        let author = git2::Signature::now(
            String::from_utf8_lossy(original_commit.author().name_bytes()).as_ref(),
            String::from_utf8_lossy(original_commit.author().email_bytes()).as_ref(),
        )?;
        let cherry_picked_commit = self
            .repo
            .commit(None, &author, &committer, message, &tree, &[&base_commit])
            .context("Committing")?;
        Ok(self.repo.find_commit(cherry_picked_commit)?)
    }

    pub fn update_current_branch(&self, new_head: &Commit) -> anyhow::Result<()> {
        self.repo
            .set_head_detached(new_head.id())
            .context("Detach HEAD before moving the main branch")?;
        self.repo
            .branch(&self.current_branch_name, new_head, true)
            .context("Moving the main branch pointer")?;
        self.repo
            .set_head(&format!("refs/heads/{}", self.current_branch_name))
            .context("Moving HEAD back to main branch")?;
        Ok(())
    }

    pub fn merge(&self, commit1: &Commit, commit2: &Commit) -> anyhow::Result<Oid> {
        let mut merge_index =
            self.repo
                .merge_commits(commit1, commit2, None)?;

        //self.repo.merge_analysis_for_ref
        if merge_index.has_conflicts() {
            for c in merge_index.conflicts()? {
                let c = c?;
                println!("Conclict {:?}", CString::new(c.our.unwrap().path).unwrap())
            }
            //self.repo.set_head_detached(base_commit.id())?;
            //self.repo.merge(
            //    &[&self.repo.find_annotated_commit(remote_commit.id())?],
            //    None,
            //    None,
            //)?;
            //self.git_repo
            //    .save_merge_state(&base_commit, &remote_commit)?;
            anyhow::bail!(
                "Unable to merge {} and {}",
                commit1.id(),
                commit2.id()
            );
        }
        if merge_index.is_empty() {
            anyhow::bail!("Index is empty");
        }
        let tree = merge_index
            .write_tree_to(&self.repo)
            .context("write index to tree")?;
        let oid = self.repo.commit(
            None,
            &self.repo.signature().context("No signature")?,
            &self.repo.signature()?,
            "Merge",
            &self.repo.find_tree(tree)?,
            &[commit1, commit2],
        )?;

        Ok(oid)
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
