//! Git status and diff operations.

use std::path::PathBuf;
use super::GitRepo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    New,
    Modified,
    Deleted,
    Renamed,
    Conflicted,
    Untracked,
}

#[derive(Debug, Clone)]
pub struct StatusEntry {
    pub path: PathBuf,
    pub status: FileStatus,
    pub staged: bool,
}

impl GitRepo {
    /// Return working tree and index status.
    pub fn status(&self) -> Result<Vec<StatusEntry>, String> {
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);

        let statuses = self.repo.statuses(Some(&mut opts))
            .map_err(|e| format!("Status failed: {}", e))?;

        let entries = statuses.iter().filter_map(|e| {
            let path = PathBuf::from(e.path()?);
            let st = e.status();
            let (file_status, staged) = classify_status(st)?;
            Some(StatusEntry { path, status: file_status, staged })
        }).collect();

        Ok(entries)
    }

    /// Return unified diff for a file (index vs working tree, or HEAD vs index if staged).
    pub fn diff_file(&self, path: &std::path::Path) -> Result<String, String> {
        let path_str = path.to_str().ok_or("non-UTF8 path")?;

        // Try index vs workdir diff first
        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.pathspec(path_str);

        let diff = self.repo.diff_index_to_workdir(None, Some(&mut diff_opts))
            .map_err(|e| format!("Diff failed: {}", e))?;

        let mut result = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                ' ' => " ",
                _ => "",
            };
            result.push_str(prefix);
            if let Ok(s) = std::str::from_utf8(line.content()) {
                result.push_str(s);
            }
            true
        }).map_err(|e| format!("Diff print failed: {}", e))?;

        if result.is_empty() {
            // Try HEAD vs index (staged changes)
            let head = self.repo.head().ok().and_then(|h| h.peel_to_tree().ok());
            let staged_diff = self.repo.diff_tree_to_index(head.as_ref(), None, Some(&mut diff_opts))
                .map_err(|e| format!("Staged diff failed: {}", e))?;
            staged_diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
                let prefix = match line.origin() {
                    '+' => "+",
                    '-' => "-",
                    ' ' => " ",
                    _ => "",
                };
                result.push_str(prefix);
                if let Ok(s) = std::str::from_utf8(line.content()) {
                    result.push_str(s);
                }
                true
            }).map_err(|e| format!("Staged diff print failed: {}", e))?;
        }

        Ok(result)
    }
}

fn classify_status(st: git2::Status) -> Option<(FileStatus, bool)> {
    if st.is_conflicted() {
        return Some((FileStatus::Conflicted, false));
    }
    if st.is_index_new() { return Some((FileStatus::New, true)); }
    if st.is_index_modified() { return Some((FileStatus::Modified, true)); }
    if st.is_index_deleted() { return Some((FileStatus::Deleted, true)); }
    if st.is_index_renamed() { return Some((FileStatus::Renamed, true)); }
    if st.is_wt_new() { return Some((FileStatus::Untracked, false)); }
    if st.is_wt_modified() { return Some((FileStatus::Modified, false)); }
    if st.is_wt_deleted() { return Some((FileStatus::Deleted, false)); }
    if st.is_wt_renamed() { return Some((FileStatus::Renamed, false)); }
    None
}
