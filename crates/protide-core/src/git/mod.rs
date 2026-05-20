//! Git integration for Protide collections.
//! Wraps git2 with a simple, error-string interface for UI consumption.

pub mod branch;
pub mod remote;
pub mod stash;
pub mod status;

use std::path::{Path, PathBuf};

pub use branch::{BranchInfo, ConflictEntry};
pub use stash::StashEntry;
pub use status::{FileStatus, StatusEntry};

/// A git repository handle.
pub struct GitRepo {
    pub(crate) repo: git2::Repository,
}

impl GitRepo {
    /// Open the git repository at or above `path`.
    pub fn open(path: &Path) -> Result<Self, String> {
        git2::Repository::discover(path)
            .map(|repo| Self { repo })
            .map_err(|e| format!("Not a git repository: {}", e))
    }

    /// Initialize a new git repository at `path`.
    pub fn init(path: &Path) -> Result<Self, String> {
        git2::Repository::init(path)
            .map(|repo| Self { repo })
            .map_err(|e| format!("Failed to initialize git repository: {}", e))
    }

    /// Clone a remote repository into `dest`.
    pub fn clone_repo(url: &str, dest: &Path) -> Result<Self, String> {
        git2::Repository::clone(url, dest)
            .map(|repo| Self { repo })
            .map_err(|e| format!("Clone failed: {}", e))
    }

    /// Root directory of the working tree.
    pub fn workdir(&self) -> Option<PathBuf> {
        self.repo.workdir().map(|p| p.to_path_buf())
    }

    /// Commit staged changes. Returns the new commit OID as hex string.
    pub fn commit(&self, message: &str, author_name: &str, author_email: &str) -> Result<String, String> {
        let sig = git2::Signature::now(author_name, author_email)
            .map_err(|e| format!("Signature error: {}", e))?;

        let mut index = self.repo.index()
            .map_err(|e| format!("Index error: {}", e))?;
        let tree_id = index.write_tree()
            .map_err(|e| format!("Write tree error: {}", e))?;
        let tree = self.repo.find_tree(tree_id)
            .map_err(|e| format!("Find tree error: {}", e))?;

        let parent = self.repo.head().ok()
            .and_then(|h| h.peel_to_commit().ok());

        let parents: Vec<&git2::Commit> = parent.iter().collect();
        let oid = self.repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .map_err(|e| format!("Commit failed: {}", e))?;

        Ok(oid.to_string())
    }

    /// Stage all modified and new tracked files (equivalent to `git add -u && git add .`).
    pub fn stage_all(&self) -> Result<(), String> {
        let mut index = self.repo.index()
            .map_err(|e| format!("Index error: {}", e))?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| format!("Stage all failed: {}", e))?;
        index.write()
            .map_err(|e| format!("Index write failed: {}", e))
    }

    /// Unstage all staged changes (equivalent to `git reset HEAD`).
    pub fn unstage_all(&self) -> Result<(), String> {
        let head = self.repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        if let Some(commit) = head {
            self.repo.reset_default(Some(commit.as_object()), ["*"].iter())
                .map_err(|e| format!("Unstage failed: {}", e))
        } else {
            // No HEAD yet (initial commit) — just clear the index
            let mut index = self.repo.index().map_err(|e| e.to_string())?;
            index.clear().map_err(|e| e.to_string())?;
            index.write().map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(suffix: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("protide_git_{}_{}", suffix,
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn test_init_and_open() {
        let dir = temp_dir("init");
        let repo = GitRepo::init(&dir).expect("init failed");
        assert!(repo.workdir().is_some());

        let opened = GitRepo::open(&dir).expect("open failed");
        assert!(opened.workdir().is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_open_non_repo_fails() {
        let dir = temp_dir("nonrepo");
        let result = GitRepo::open(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_stage_and_commit() {
        use std::io::Write;
        let dir = temp_dir("commit");
        let repo = GitRepo::init(&dir).unwrap();

        std::fs::File::create(dir.join("test.http")).unwrap()
            .write_all(b"GET https://example.com\n").unwrap();

        repo.stage_all().unwrap();
        let oid = repo.commit("initial commit", "Test User", "test@example.com").unwrap();
        assert_eq!(oid.len(), 40); // hex SHA

        let _ = std::fs::remove_dir_all(&dir);
    }
}
