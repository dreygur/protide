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
        let mut watcher = notify::recommended_watcher(
            move |result: NotifyResult<Event>| {
                if let Ok(event) = result {
                    use notify::EventKind::*;
                    for path in event.paths {
                        let is_dir = path.is_dir();
                        let evt = match event.kind {
                            Create(_) => {
                                if is_dir { WorkspaceEvent::DirCreated(path) }
                                else { WorkspaceEvent::FileCreated(path) }
                            }
                            Modify(_) => WorkspaceEvent::FileModified(path),
                            Remove(_) => {
                                if is_dir { WorkspaceEvent::DirDeleted(path) }
                                else { WorkspaceEvent::FileDeleted(path) }
                            }
                            _ => continue,
                        };
                        let _ = watcher_tx.send(evt);
                    }
                }
            },
        )
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
        scan_dir(&self.root, &self.root, 0, &mut entries);
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

fn scan_dir(root: &Path, dir: &Path, depth: usize, out: &mut Vec<CollectionEntry>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };

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
            out.push(CollectionEntry { path: path.clone(), name, is_dir: true, depth });
            scan_dir(root, &path, depth + 1, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("http") {
            out.push(CollectionEntry { path, name, is_dir: false, depth });
        }
    }
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
    path.starts_with(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_scan_empty_dir() {
        let tmp = std::env::temp_dir().join("api_dash_workspace_test");
        let _ = fs::create_dir_all(&tmp);
        let ws = Workspace {
            root: tmp.clone(),
            _watcher: None,
            rx: None,
        };
        let entries = ws.scan();
        assert!(entries.iter().all(|e| e.path.extension().map(|x| x == "http").unwrap_or(true)));
        let _ = fs::remove_dir_all(&tmp);
    }
}
