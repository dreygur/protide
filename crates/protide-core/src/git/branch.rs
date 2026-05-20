//! Branch management and merge conflict resolution.

use std::path::PathBuf;
use super::GitRepo;

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

#[derive(Debug, Clone)]
pub struct ConflictEntry {
    pub path: PathBuf,
    pub ours: String,
    pub theirs: String,
}

impl GitRepo {
    /// List all local and remote branches.
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>, String> {
        let branches = self.repo.branches(None)
            .map_err(|e| format!("Branch list failed: {}", e))?;

        let mut result = Vec::new();
        for branch in branches {
            let (branch, branch_type) = branch.map_err(|e| e.to_string())?;
            let name = branch.name().ok().flatten().unwrap_or("?").to_string();
            let is_current = branch.is_head();
            let is_remote = branch_type == git2::BranchType::Remote;
            result.push(BranchInfo { name, is_current, is_remote });
        }

        Ok(result)
    }

    /// Create a new local branch from HEAD.
    pub fn create_branch(&self, name: &str) -> Result<(), String> {
        let head = self.repo.head()
            .and_then(|h| h.peel_to_commit())
            .map_err(|e| format!("HEAD not found: {}", e))?;
        self.repo.branch(name, &head, false)
            .map(|_| ())
            .map_err(|e| format!("Create branch failed: {}", e))
    }

    /// Checkout (switch to) a local branch by name.
    pub fn checkout_branch(&self, name: &str) -> Result<(), String> {
        let refname = format!("refs/heads/{}", name);
        let obj = self.repo.revparse_single(&refname)
            .map_err(|e| format!("Branch '{}' not found: {}", name, e))?;

        self.repo.checkout_tree(&obj, None)
            .map_err(|e| format!("Checkout tree failed: {}", e))?;
        self.repo.set_head(&refname)
            .map_err(|e| format!("Set HEAD failed: {}", e))
    }

    /// Fast-forward merge a branch into HEAD (no-op if already up to date).
    pub fn merge_ff(&self, branch_name: &str) -> Result<(), String> {
        let refname = format!("refs/heads/{}", branch_name);
        let reference = self.repo.find_reference(&refname)
            .map_err(|e| format!("Branch ref not found: {}", e))?;
        let ann_commit = self.repo.reference_to_annotated_commit(&reference)
            .map_err(|e| format!("Annotated commit error: {}", e))?;

        let (analysis, _) = self.repo.merge_analysis(&[&ann_commit])
            .map_err(|e| format!("Merge analysis failed: {}", e))?;

        if analysis.is_up_to_date() {
            return Ok(());
        }
        if !analysis.is_fast_forward() {
            return Err("Merge requires a non-fast-forward strategy; not supported here.".to_string());
        }

        let target_id = reference.target().ok_or("No target OID")?;
        let mut head_ref = self.repo.find_reference("HEAD")
            .map_err(|e| format!("HEAD ref error: {}", e))?;
        head_ref.set_target(target_id, "fast-forward merge")
            .map_err(|e| format!("Set target failed: {}", e))?;
        self.repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(|e| format!("Checkout head failed: {}", e))
    }

    /// Return conflicted file entries with ours/theirs content.
    pub fn merge_conflicts(&self) -> Result<Vec<ConflictEntry>, String> {
        let index = self.repo.index()
            .map_err(|e| format!("Index error: {}", e))?;

        if !index.has_conflicts() {
            return Ok(Vec::new());
        }

        let workdir = self.repo.workdir().ok_or("Bare repository")?;
        let mut entries = Vec::new();

        for conflict in index.conflicts().map_err(|e| e.to_string())? {
            let conflict = conflict.map_err(|e| e.to_string())?;
            let path = conflict.our
                .as_ref()
                .and_then(|e| std::str::from_utf8(&e.path).ok())
                .map(PathBuf::from)
                .unwrap_or_default();

            let full_path = workdir.join(&path);
            let content = std::fs::read_to_string(&full_path).unwrap_or_default();

            let (ours, theirs) = split_conflict_markers(&content);
            entries.push(ConflictEntry { path, ours, theirs });
        }

        Ok(entries)
    }
}

/// Split `<<<<<<<` / `=======` / `>>>>>>>` conflict markers into (ours, theirs).
fn split_conflict_markers(content: &str) -> (String, String) {
    let mut ours = String::new();
    let mut theirs = String::new();
    let mut state = 0u8; // 0=before, 1=ours, 2=theirs

    for line in content.lines() {
        if line.starts_with("<<<<<<<") { state = 1; continue; }
        if line.starts_with("=======") { state = 2; continue; }
        if line.starts_with(">>>>>>>") { state = 0; continue; }
        match state {
            1 => { ours.push_str(line); ours.push('\n'); }
            2 => { theirs.push_str(line); theirs.push('\n'); }
            _ => {}
        }
    }

    (ours, theirs)
}
