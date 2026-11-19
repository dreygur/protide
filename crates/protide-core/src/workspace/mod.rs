//! Workspace management - loading and watching .http file collections

use notify::{Event, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

/// Change event from file system watcher
#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    FileCreated(PathBuf),
    FileModified(PathBuf),
    FileDeleted(PathBuf),
    DirCreated(PathBuf),
    DirDeleted(PathBuf),
}

/// Scanned collection entry
#[derive(Debug, Clone)]
pub struct CollectionEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
}

/// Active workspace with optional file watcher
pub struct Workspace {
    pub root: PathBuf,
    _watcher: Option<RecommendedWatcher>,
    pub rx: Option<Receiver<WorkspaceEvent>>,
}

impl Workspace {
    /// Open a workspace at `root`, starting a file watcher.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, String> {
        let root = root.as_ref().to_path_buf();
        let (tx, rx) = mpsc::channel::<WorkspaceEvent>();

        let watcher_tx = tx.clone();
        let mut watcher = notify::recommended_watcher(move |result: NotifyResult<Event>| {
            if let Ok(event) = result {
                use notify::EventKind::*;
                for path in event.paths {
                    let is_dir = path.is_dir();
                    let evt = match event.kind {
                        Create(_) => {
                            if is_dir {
                                WorkspaceEvent::DirCreated(path)
                            } else {
                                WorkspaceEvent::FileCreated(path)
                            }
                        }
                        Modify(_) => WorkspaceEvent::FileModified(path),
                        Remove(_) => {
                            if is_dir {
                                WorkspaceEvent::DirDeleted(path)
                            } else {
                                WorkspaceEvent::FileDeleted(path)
                            }
                        }
                        _ => continue,
                    };
                    let _ = watcher_tx.send(evt);
                }
            }
        })
        .map_err(|e| format!("Failed to create file watcher: {}", e))?;

        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch directory: {}", e))?;

        Ok(Self {
            root,
            _watcher: Some(watcher),
            rx: Some(rx),
        })
    }

    /// Scan the workspace and return a flat list of all entries (sorted: dirs first).
    pub fn scan(&self) -> Vec<CollectionEntry> {
        let mut entries = Vec::new();
        scan_dir(&self.root, 0, &mut entries);
        entries
    }

    /// Poll for pending file system events (non-blocking).
    pub fn poll_events(&self) -> Vec<WorkspaceEvent> {
        let mut events = Vec::new();
        if let Some(rx) = &self.rx {
            while let Ok(evt) = rx.try_recv() {
                events.push(evt);
            }
        }
        events
    }
}

fn scan_dir(dir: &Path, depth: usize, out: &mut Vec<CollectionEntry>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    let mut children: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    children.sort_by(|a, b| {
        let a_dir = a.path().is_dir();
        let b_dir = b.path().is_dir();
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in children {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            out.push(CollectionEntry {
                path: path.clone(),
                name,
                is_dir: true,
                depth,
            });
            scan_dir(&path, depth + 1, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("http") {
            out.push(CollectionEntry {
                path,
                name,
                is_dir: false,
                depth,
            });
        }
    }
}

/// Whether `rel` begins with Protide's own state folder. Compared case
/// insensitively because macOS and Windows treat `.PROTIDE` as the same folder.
fn starts_in_data_dir(rel: &Path) -> bool {
    rel.components().next().is_some_and(|first| {
        first
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(crate::storage::DATA_DIR)
    })
}

/// Format a `Duration` debounce: collect events within `debounce` interval.
/// Returns whether any of the given paths are within `root`.
pub fn is_relevant(event: &WorkspaceEvent, root: &Path) -> bool {
    let path = match event {
        WorkspaceEvent::FileCreated(p)
        | WorkspaceEvent::FileModified(p)
        | WorkspaceEvent::FileDeleted(p)
        | WorkspaceEvent::DirCreated(p)
        | WorkspaceEvent::DirDeleted(p) => p,
    };
    // Protide's own state files (environments with secrets, history) must never
    // be broadcast to peers or trigger a tree rescan.
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    !starts_in_data_dir(rel)
}

/// Where a file a peer sent may be written, or `None`. Only plain names are
/// allowed (no `..`, root or `.`), never under `.protide`, and never through
/// a symlinked directory, so a peer cannot write outside the workspace.
pub fn synced_file_path(root: &Path, relative: &str) -> Option<PathBuf> {
    let rel = Path::new(relative);
    // Windows drops trailing dots and spaces and reads ':' as a stream, so a
    // name could compare unequal here yet open `.protide` or escape there.
    let portable = |name: &std::ffi::OsStr| {
        let name = name.to_string_lossy();
        !name.ends_with(['.', ' ']) && !name.contains([':', '\\'])
    };
    let plain = rel.components().all(|c| match c {
        std::path::Component::Normal(name) => portable(name),
        _ => false,
    });
    if relative.is_empty() || !plain || starts_in_data_dir(rel) {
        return None;
    }

    let mut current = root.to_path_buf();
    for component in rel.components() {
        current.push(component);
        let is_link = std::fs::symlink_metadata(&current)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        if is_link {
            return None;
        }
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn test_scan_empty_dir() {
        let tmp = TempDir::new("protide_workspace_test");
        let ws = Workspace {
            root: tmp.path().to_path_buf(),
            _watcher: None,
            rx: None,
        };
        let entries = ws.scan();
        assert!(
            entries
                .iter()
                .all(|e| e.path.extension().map(|x| x == "http").unwrap_or(true))
        );
    }

    #[test]
    fn protide_state_files_are_not_relevant_events() {
        let root = Path::new("/ws");
        let modified = |p: &str| WorkspaceEvent::FileModified(PathBuf::from(p));
        assert!(is_relevant(&modified("/ws/a.http"), root));
        assert!(!is_relevant(
            &modified("/ws/.protide/environments.json"),
            root
        ));
        assert!(!is_relevant(&modified("/other/a.http"), root));
        assert!(
            !is_relevant(&modified("/ws/.PROTIDE/x"), root),
            "case insensitive filesystems treat this as the same folder"
        );
        assert!(is_relevant(&modified("/ws/sub/.protide/a.http"), root));
    }

    /// REGRESSION: a peer's relative path was joined onto the workspace
    /// unchecked, so "../../.ssh/authorized_keys" wrote anywhere on disk.
    #[test]
    fn a_synced_path_cannot_leave_the_workspace() {
        let tmp = TempDir::new("synced_path");
        let root = tmp.path();
        assert_eq!(
            synced_file_path(root, "api/users.http"),
            Some(root.join("api/users.http"))
        );
        for hostile in [
            "../escape.http",
            "api/../../escape.http",
            "/etc/passwd",
            "",
            ".protide/environments.json",
            ".PROTIDE/history.json",
            "./a.http",
            // Windows strips trailing dots and spaces, and reads ':' as a stream
            ".protide./environments.json",
            ".protide /environments.json",
            "a.http:stream",
            "sub\\..\\..\\escape.http",
        ] {
            assert_eq!(synced_file_path(root, hostile), None, "{hostile}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_synced_path_cannot_go_through_a_symlink() {
        let tmp = TempDir::new("synced_symlink");
        let outside = TempDir::new("synced_symlink_target");
        std::os::unix::fs::symlink(outside.path(), tmp.path().join("link")).unwrap();
        assert_eq!(synced_file_path(tmp.path(), "link/a.http"), None);
    }
}
