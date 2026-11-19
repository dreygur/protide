use log::{debug, warn};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::cell::Cell;
use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use uuid::Uuid;

use super::seal::{open_entry, seal_entry};
use super::types::{CrdtEntry, GroupSecret, NodeId};

/// Context of every entry sealed for a sync folder. It differs from every
/// gossip topic, so a folder seal never opens as a P2P message or the
/// reverse.
const FOLDER_AAD: &str = "protide file sync v1";

/// Associated data of the file of entry `id`: the folder context bound to
/// the id, so a sealed entry never opens under another file name.
fn folder_aad(id: &Uuid) -> String {
    format!("{FOLDER_AAD}|{id}")
}

/// BYOB file sync - watches a `.protide/` folder and synchronizes individual `.crdt` files.
///
/// Each CRDT entry is stored as a separate file keyed by UUID, sealed under
/// the key of the sync group, so anyone else with access to the shared
/// folder can neither read nor inject entries. One file per request
/// eliminates merge conflicts at the file level. Dropbox/Google Drive/OneDrive
/// handle the transport.
pub struct FileSync {
    /// Root of the .protide sync folder
    root: PathBuf,
    _node_id: NodeId,
    /// Receiver for file system changes seen by the watcher
    watch_rx: Receiver<Watched>,
    /// File watcher
    _watcher: Option<RecommendedWatcher>,
    /// Known entry filenames (to detect deletes)
    known_entries: HashSet<Uuid>,
    /// Whether the warning that folder sync needs a group was logged
    warned_no_group: Cell<bool>,
}

/// A change the watcher saw in the entries folder, opened on poll under
/// the group key current at that time.
enum Watched {
    Changed(PathBuf),
    Removed(Uuid),
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

        let (watch_tx, watch_rx) = mpsc::channel::<Watched>();

        // Scan existing entries
        let known_entries = Self::scan_existing(&entries_dir);

        // Set up file watcher
        let mut watcher =
            notify::recommended_watcher(move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result
                    && Self::is_crdt_event(&event)
                {
                    for path in &event.paths {
                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                let _ = watch_tx.send(Watched::Changed(path.clone()));
                            }
                            EventKind::Remove(_) => {
                                if let Some(uuid) = Self::path_to_uuid(path) {
                                    let _ = watch_tx.send(Watched::Removed(uuid));
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
            watch_rx,
            _watcher: Some(watcher),
            known_entries,
            warned_no_group: Cell::new(false),
        })
    }

    /// Write a CRDT entry to disk as an individual `.crdt` file, sealed
    /// under the key of `secret`. A node without a group has no key and
    /// writes nothing; the first such write logs why.
    pub fn write_entry(
        &self,
        secret: Option<&GroupSecret>,
        entry: &CrdtEntry,
    ) -> Result<(), String> {
        let Some(secret) = secret else {
            if !self.warned_no_group.replace(true) {
                warn!(
                    "[FileSync] Folder sync needs a sync group: nothing is written to the \
                     shared folder until this node pairs with a group"
                );
            }
            return Ok(());
        };
        let aad = folder_aad(&entry.id);
        let data = seal_entry(secret, &aad, entry).ok_or("Failed to seal entry")?;
        let path = self.entry_path(&entry.id);
        // Atomic write: write to temp file, then rename
        let tmp_path = self.entry_path_tmp(&entry.id);
        fs::write(&tmp_path, &data).map_err(|e| format!("Failed to write entry: {}", e))?;
        fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to finalize entry: {}", e))?;
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

    /// Poll for file system events (non-blocking), opening changed files
    /// under the key of `secret`
    pub fn poll_events(&mut self, secret: Option<&GroupSecret>) -> Vec<FileSyncEvent> {
        let mut events = Vec::new();
        while let Ok(seen) = self.watch_rx.try_recv() {
            match seen {
                Watched::Changed(path) => {
                    if let Some(entry) = Self::open_file(&path, secret) {
                        self.known_entries.insert(entry.id);
                        events.push(FileSyncEvent::EntryReceived(entry));
                    }
                }
                Watched::Removed(id) => {
                    self.known_entries.remove(&id);
                    events.push(FileSyncEvent::EntryDeleted(id));
                }
            }
        }
        events
    }

    /// Read all entries currently on disk that open under the key of
    /// `secret` (for initial sync)
    pub fn read_all_entries(&self, secret: Option<&GroupSecret>) -> Vec<CrdtEntry> {
        let entries_dir = self.root.join("entries");
        Self::scan_existing(&entries_dir)
            .iter()
            .filter_map(|uuid| Self::open_file(&self.entry_path(uuid), secret))
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

    /// The entry in the file at `path`, opened under the key of `secret`
    /// for the id its name carries. A file that does not open, or holds
    /// another entry, is skipped unread: it was sealed by another group,
    /// altered, renamed, or written in the clear by an older version.
    fn open_file(path: &Path, secret: Option<&GroupSecret>) -> Option<CrdtEntry> {
        let id = Self::path_to_uuid(path)?;
        let data = fs::read(path).ok()?;
        let entry = secret
            .and_then(|s| open_entry(s, &folder_aad(&id), &data))
            .filter(|entry| entry.id == id);
        if entry.is_none() {
            debug!("[FileSync] Skipped a sync file that did not open");
        }
        entry
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
    use super::super::seal::NONCE_LEN;
    use super::*;
    use crate::sync::crdt::CrdtStore;
    use crate::sync::{DataType, SyncConfig, SyncEngine, SyncEvent};
    use crate::test_support::TempDir;

    const FIELD_VALUE: &str = "https://internal.example/token-4f2a";
    const GROUP: GroupSecret = GroupSecret([1; 32]);
    const OTHER_GROUP: GroupSecret = GroupSecret([2; 32]);

    fn folder() -> TempDir {
        TempDir::new("protide_file_sync_test")
    }

    fn node(tmp: &TempDir) -> FileSync {
        FileSync::open(tmp.path(), NodeId::new()).unwrap()
    }

    fn request_entry() -> CrdtEntry {
        let data = format!(r#"{{"url":"{FIELD_VALUE}"}}"#);
        CrdtStore::new(NodeId::new()).apply_local(DataType::Request, data)
    }

    /// Paths of every file in the entries folder.
    fn entry_files(sync: &FileSync) -> Vec<PathBuf> {
        let dir = fs::read_dir(sync.root().join("entries")).unwrap();
        dir.flatten().map(|e| e.path()).collect()
    }

    /// An engine on the folder of `tmp`, in the group of `secret` if any.
    fn engine(tmp: &TempDir, secret: Option<GroupSecret>) -> SyncEngine {
        let mut engine = SyncEngine::new(SyncConfig {
            sync_folder: Some(tmp.path().to_path_buf()),
            group_secret: secret,
            ..Default::default()
        });
        engine.init().unwrap();
        engine
    }

    #[test]
    fn two_nodes_in_the_same_group_share_entries_through_the_folder() {
        let tmp = folder();
        let (alice, bob) = (node(&tmp), node(&tmp));
        let entry = request_entry();
        alice.write_entry(Some(&GROUP), &entry).unwrap();

        let entries = bob.read_all_entries(Some(&GROUP));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, entry.id);
        assert_eq!(entries[0].data, entry.data);
    }

    #[test]
    fn an_engine_loads_the_entries_another_member_wrote() {
        let tmp = folder();
        let mut alice = engine(&tmp, Some(GROUP));
        let entry = alice.apply_local_change(DataType::Request, FIELD_VALUE.into());
        let bob = engine(&tmp, Some(GROUP));
        assert_eq!(bob.store.get(&entry.id).unwrap().data, FIELD_VALUE);
    }

    /// REGRESSION: entries were written as plaintext JSON, readable by
    /// anyone with access to the shared folder.
    #[test]
    fn the_folder_holds_nothing_another_group_or_a_loner_can_read() {
        let tmp = folder();
        let alice = node(&tmp);
        alice.write_entry(Some(&GROUP), &request_entry()).unwrap();

        let plain = FIELD_VALUE.as_bytes();
        for path in entry_files(&alice) {
            let data = fs::read(path).unwrap();
            assert!(!data.windows(plain.len()).any(|w| w == plain));
            assert!(serde_json::from_slice::<CrdtEntry>(&data).is_err());
        }
        let outsider = node(&tmp);
        assert!(outsider.read_all_entries(Some(&OTHER_GROUP)).is_empty());
        assert!(outsider.read_all_entries(None).is_empty());
        let mut loner = engine(&tmp, None);
        assert!(loner.store.is_empty());
        assert!(loner.poll().is_empty());
    }

    #[test]
    fn files_from_another_group_or_in_the_clear_are_rejected() {
        let tmp = folder();
        let (alice, mallory) = (node(&tmp), node(&tmp));
        mallory
            .write_entry(Some(&OTHER_GROUP), &request_entry())
            .unwrap();
        let legacy = request_entry();
        let json = serde_json::to_vec(&legacy).unwrap();
        fs::write(alice.entry_path(&legacy.id), json).unwrap();

        assert_eq!(entry_files(&alice).len(), 2);
        assert!(alice.read_all_entries(Some(&GROUP)).is_empty());
    }

    #[test]
    fn a_tampered_file_is_rejected() {
        let tmp = folder();
        let sync = node(&tmp);
        let entry = request_entry();
        sync.write_entry(Some(&GROUP), &entry).unwrap();
        let path = sync.entry_path(&entry.id);
        let intact = fs::read(&path).unwrap();

        for i in [0, NONCE_LEN, intact.len() - 1] {
            let mut tampered = intact.clone();
            tampered[i] ^= 1;
            fs::write(&path, &tampered).unwrap();
            assert!(sync.read_all_entries(Some(&GROUP)).is_empty(), "byte {i}");
        }
        fs::write(&path, &intact).unwrap();
        assert_eq!(sync.read_all_entries(Some(&GROUP)).len(), 1);
    }

    /// REGRESSION: with a fixed associated data, a sealed entry copied to
    /// the file name of another entry still opened.
    #[test]
    fn a_sealed_file_copied_to_another_name_is_rejected() {
        let tmp = folder();
        let sync = node(&tmp);
        let (entry, other) = (request_entry(), request_entry());
        sync.write_entry(Some(&GROUP), &entry).unwrap();
        let sealed = fs::read(sync.entry_path(&entry.id)).unwrap();
        fs::remove_file(sync.entry_path(&entry.id)).unwrap();
        fs::write(sync.entry_path(&other.id), sealed).unwrap();
        assert!(sync.read_all_entries(Some(&GROUP)).is_empty());
    }

    /// The sealed file of `id` currently in the folder of `tmp`.
    fn sealed_file(tmp: &TempDir, id: &Uuid) -> Vec<u8> {
        fs::read(node(tmp).entry_path(id)).unwrap()
    }

    /// Put `sealed` back as the file of `id` and merge whatever the folder
    /// then holds into `to`, returning the events the merge reported.
    fn replay(tmp: &TempDir, to: &mut SyncEngine, id: &Uuid, sealed: &[u8]) -> Vec<SyncEvent> {
        let reader = node(tmp);
        fs::write(reader.entry_path(id), sealed).unwrap();
        let entries = reader.read_all_entries(Some(&GROUP));
        assert_eq!(entries.len(), 1, "the replayed file opens");
        let mut events = Vec::new();
        for entry in entries {
            to.merge_remote_entry(entry, &mut events);
        }
        events
    }

    #[test]
    fn replaying_an_older_sealed_version_changes_nothing() {
        let tmp = folder();
        let mut alice = engine(&tmp, Some(GROUP));
        let id = alice.apply_local_change(DataType::Request, "v1".into()).id;
        let v1 = sealed_file(&tmp, &id);
        alice.update_local_change(id, DataType::Request, "v2".into());

        let mut bob = engine(&tmp, Some(GROUP));
        assert!(replay(&tmp, &mut bob, &id, &v1).is_empty());
        assert_eq!(bob.store.get(&id).unwrap().data, "v2");
    }

    #[test]
    fn replaying_a_version_from_before_a_delete_changes_nothing() {
        let tmp = folder();
        let mut alice = engine(&tmp, Some(GROUP));
        let id = alice.apply_local_change(DataType::Request, "v1".into()).id;
        let live = sealed_file(&tmp, &id);
        alice.delete_local_change(id);

        let mut bob = engine(&tmp, Some(GROUP));
        assert!(bob.store.get(&id).unwrap().deleted);
        assert!(replay(&tmp, &mut bob, &id, &live).is_empty());
        assert!(bob.store.get(&id).unwrap().deleted);
    }

    #[test]
    fn a_node_without_a_group_writes_nothing() {
        let tmp = folder();
        let sync = node(&tmp);
        sync.write_entry(None, &request_entry()).unwrap();
        sync.write_entry(None, &request_entry()).unwrap();
        assert!(entry_files(&sync).is_empty());

        let mut loner = engine(&tmp, None);
        loner.apply_local_change(DataType::Request, FIELD_VALUE.into());
        assert!(entry_files(&sync).is_empty());
    }

    #[test]
    fn test_delete_entry() {
        let tmp = folder();
        let sync = node(&tmp);
        let entry = request_entry();

        sync.write_entry(Some(&GROUP), &entry).unwrap();
        assert_eq!(sync.read_all_entries(Some(&GROUP)).len(), 1);

        sync.delete_entry(&entry.id).unwrap();
        assert_eq!(sync.read_all_entries(Some(&GROUP)).len(), 0);
    }
}
