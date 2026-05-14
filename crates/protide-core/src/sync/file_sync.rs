use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use uuid::Uuid;

use super::types::{CrdtEntry, NodeId};

/// BYOB file sync — watches a `.protide/` folder and synchronizes individual `.crdt` files.
///
/// Each CRDT entry is stored as a separate JSON file keyed by UUID.
/// This eliminates merge conflicts at the file level since each request
/// has its own file. Dropbox/Google Drive/OneDrive handle the transport.
pub struct FileSync {
    /// Root of the .protide sync folder
    root: PathBuf,
    _node_id: NodeId,
    _event_tx: Sender<FileSyncEvent>,
    /// Receiver for file system events
    event_rx: Receiver<FileSyncEvent>,
    /// File watcher
    _watcher: Option<RecommendedWatcher>,
    /// Known entry filenames (to detect deletes)
    known_entries: HashSet<Uuid>,
}

/// Events from the file sync backend
#[derive(Debug, Clone)]
pub enum FileSyncEvent {
    /// A new or updated CRDT entry was found on disk
    EntryReceived(CrdtEntry),
    /// A CRDT entry was deleted on disk
    EntryDeleted(Uuid),
    /// An error occurred
    Error(String),
}

impl FileSync {
    /// Open or create a .protide sync folder at the given path.
    ///
    /// The `root` should be an existing directory that is synced by
    /// Dropbox, Google Drive, OneDrive, or a git repository.
    pub fn open(root: &Path, node_id: NodeId) -> Result<Self, String> {
        let protide_dir = root.join(".protide");
        let entries_dir = protide_dir.join("entries");
        fs::create_dir_all(&entries_dir)
            .map_err(|e| format!("Failed to create .protide directory: {}", e))?;

        // Write our node ID to the folder
        let node_id_path = protide_dir.join("node_id");
        if !node_id_path.exists() {
            fs::write(&node_id_path, &node_id.0)
                .map_err(|e| format!("Failed to write node_id: {}", e))?;
        }

        let (event_tx, event_rx) = mpsc::channel::<FileSyncEvent>();

        // Scan existing entries
        let known_entries = Self::scan_existing(&entries_dir);

        // Set up file watcher
        let watcher_tx = event_tx.clone();
        let mut watcher = notify::recommended_watcher(move |result: Result<Event, notify::Error>| {
            if let Ok(event) = result
                && Self::is_crdt_event(&event) {
                    for path in &event.paths {
                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                if let Some(entry) = Self::read_entry(path) {
                                    let _ = watcher_tx.send(FileSyncEvent::EntryReceived(entry));
                                }
                            }
                            EventKind::Remove(_) => {
                                if let Some(uuid) = Self::path_to_uuid(path) {
                                    let _ = watcher_tx.send(FileSyncEvent::EntryDeleted(uuid));
                                }
                            }
                            _ => {}
                        }
                    }
                }
        })
        .map_err(|e| format!("Failed to create file watcher: {}", e))?;

        watcher
            .watch(&entries_dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch entries directory: {}", e))?;

        Ok(Self {
            root: protide_dir,
            _node_id: node_id,
            _event_tx: event_tx,
            event_rx,
            _watcher: Some(watcher),
            known_entries,
        })
    }

    /// Write a CRDT entry to disk as an individual `.crdt` file
    pub fn write_entry(&self, entry: &CrdtEntry) -> Result<(), String> {
        let path = self.entry_path(&entry.id);
        let json = serde_json::to_string(entry)
            .map_err(|e| format!("Failed to serialize entry: {}", e))?;
        // Atomic write: write to temp file, then rename
        let tmp_path = self.entry_path_tmp(&entry.id);
        fs::write(&tmp_path, &json)
            .map_err(|e| format!("Failed to write entry: {}", e))?;
        fs::rename(&tmp_path, &path)
            .map_err(|e| format!("Failed to finalize entry: {}", e))?;
        Ok(())
    }

    /// Delete a CRDT entry from disk
    pub fn delete_entry(&self, id: &Uuid) -> Result<(), String> {
        let path = self.entry_path(id);
        match fs::remove_file(&path) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("Failed to delete entry: {}", e)),
        }
    }

    /// Poll for file system events (non-blocking)
    pub fn poll_events(&mut self) -> Vec<FileSyncEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.event_rx.try_recv() {
            match &evt {
                FileSyncEvent::EntryReceived(entry) => {
                    self.known_entries.insert(entry.id);
                }
                FileSyncEvent::EntryDeleted(id) => {
                    self.known_entries.remove(id);
                }
                _ => {}
            }
            events.push(evt);
        }
        events
    }

    /// Read all entries currently on disk (for initial sync)
    pub fn read_all_entries(&self) -> Vec<CrdtEntry> {
        let entries_dir = self.root.join("entries");
        Self::scan_existing(&entries_dir)
            .iter()
            .filter_map(|uuid| Self::read_entry(&self.entry_path(uuid)))
            .collect()
    }

    /// Get the .protide folder path
    pub fn root(&self) -> &Path {
        &self.root
    }

    // --- Private helpers ---

    fn entry_path(&self, id: &Uuid) -> PathBuf {
        self.root.join("entries").join(format!("{}.crdt", id))
    }

    fn entry_path_tmp(&self, id: &Uuid) -> PathBuf {
        self.root.join("entries").join(format!("{}.crdt.tmp", id))
    }

    fn read_entry(path: &Path) -> Option<CrdtEntry> {
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn scan_existing(entries_dir: &Path) -> HashSet<Uuid> {
        let mut uuids = HashSet::new();
        if let Ok(dir) = fs::read_dir(entries_dir) {
            for entry in dir.flatten() {
                let path = entry.path();
                if let Some(uuid) = Self::path_to_uuid(&path) {
                    uuids.insert(uuid);
                }
            }
        }
        uuids
    }

    fn path_to_uuid(path: &Path) -> Option<Uuid> {
        let stem = path.file_stem()?.to_str()?;
        // Skip temp files
        if path.extension().and_then(|e| e.to_str()) == Some("tmp") {
            return None;
        }
        Uuid::parse_str(stem).ok()
    }

    fn is_crdt_event(event: &notify::Event) -> bool {
        event.paths.iter().any(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e == "crdt" || e == "tmp")
                .unwrap_or(false)
        })
    }
}

/// Determine the default sync folder path.
/// Checks common cloud storage locations.
pub fn default_sync_folder() -> Option<PathBuf> {
    let home = dirs::home_dir()?;

    // Check common cloud sync folders
    let candidates = [
        home.join("Dropbox"),
        home.join("Library").join("CloudStorage").join("OneDrive"),
        home.join("OneDrive"),
        home.join("Google Drive"),
        home.join("GoogleDrive"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }

    // Fall back to home directory
    Some(home)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::crdt::CrdtStore;
    use crate::sync::DataType;

    #[test]
    fn test_write_and_read_entry() {
        let tmp = std::env::temp_dir().join("protide_file_sync_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let node = NodeId::new();
        let sync = FileSync::open(&tmp, node).unwrap();

        let mut store = CrdtStore::new(NodeId::new());
        let entry = store.apply_local(DataType::Request, r#"{"url":"https://api.example.com"}"#.into());

        sync.write_entry(&entry).unwrap();

        let entries = sync.read_all_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, entry.id);
        assert_eq!(entries[0].data, r#"{"url":"https://api.example.com"}"#);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_delete_entry() {
        let tmp = std::env::temp_dir().join("protide_file_sync_del_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let node = NodeId::new();
        let sync = FileSync::open(&tmp, node).unwrap();

        let mut store = CrdtStore::new(NodeId::new());
        let entry = store.apply_local(DataType::Request, "data".into());

        sync.write_entry(&entry).unwrap();
        assert_eq!(sync.read_all_entries().len(), 1);

        sync.delete_entry(&entry.id).unwrap();
        assert_eq!(sync.read_all_entries().len(), 0);

        let _ = fs::remove_dir_all(&tmp);
    }
}
