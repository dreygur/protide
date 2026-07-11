use std::collections::HashMap;
use uuid::Uuid;

use super::types::{timestamp_now, CrdtEntry, DataType, NodeId};

/// In-memory CRDT store - holds the current state merged from all peers
#[derive(Debug, Clone)]
pub struct CrdtStore {
    /// All entries keyed by UUID
    entries: HashMap<Uuid, CrdtEntry>,
    /// Our node identity
    node_id: NodeId,
}

impl CrdtStore {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            entries: HashMap::new(),
            node_id,
        }
    }

    /// Apply a local change, producing a CRDT entry ready for sync
    pub fn apply_local(
        &mut self,
        data_type: DataType,
        data: String,
    ) -> CrdtEntry {
        let entry = CrdtEntry::new(data_type, data, &self.node_id);
        self.entries.insert(entry.id, entry.clone());
        entry
    }

    /// Apply a local update to an existing entry.
    /// Always replaces the local entry since we authored this change.
    pub fn update_local(
        &mut self,
        id: Uuid,
        data_type: DataType,
        data: String,
    ) -> Option<CrdtEntry> {
        let timestamp = match self.entries.get(&id) {
            Some(existing) => std::cmp::max(timestamp_now(), existing.timestamp.saturating_add(1)),
            None => timestamp_now(),
        };
        let entry = CrdtEntry {
            id,
            data_type,
            data,
            timestamp,
            node_id: self.node_id.0.clone(),
            deleted: false,
            version: 1,
        };
        self.entries.insert(id, entry.clone());
        Some(entry)
    }

    /// Mark an entry as deleted locally.
    /// Forces a higher timestamp than existing to ensure tombstone wins.
    pub fn delete_local(&mut self, id: Uuid) -> Option<CrdtEntry> {
        let (timestamp, data_type) = match self.entries.get(&id) {
            Some(existing) => (std::cmp::max(timestamp_now(), existing.timestamp.saturating_add(1)), existing.data_type),
            None => (timestamp_now(), DataType::Request),
        };
        let tombstone = CrdtEntry {
            id,
            data_type,
            data: String::new(),
            timestamp,
            node_id: self.node_id.0.clone(),
            deleted: true,
            version: 1,
        };
        self.entries.insert(id, tombstone.clone());
        Some(tombstone)
    }

    /// Merge a remote entry into our store (LWW)
    pub fn merge_remote(&mut self, entry: CrdtEntry) -> MergeResult {
        let id = entry.id;
        match self.entries.get(&id) {
            Some(local) => {
                if entry.timestamp > local.timestamp
                    || (entry.timestamp == local.timestamp && entry.node_id > local.node_id)
                {
                    self.entries.insert(id, entry.clone());
                    MergeResult::Accepted(entry)
                } else {
                    MergeResult::Stale
                }
            }
            None => {
                self.entries.insert(id, entry.clone());
                MergeResult::Accepted(entry)
            }
        }
    }

    /// Get an entry by ID
    pub fn get(&self, id: &Uuid) -> Option<&CrdtEntry> {
        self.entries.get(id)
    }

    /// Get all non-deleted entries of a given type
    pub fn get_by_type(&self, data_type: DataType) -> Vec<&CrdtEntry> {
        self.entries
            .values()
            .filter(|e| e.data_type == data_type && !e.deleted)
            .collect()
    }

    /// Get all entries (including tombstones) for full sync
    pub fn all_entries(&self) -> Vec<&CrdtEntry> {
        self.entries.values().collect()
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Serialize the full store to JSON bytes
    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        let entries: Vec<&CrdtEntry> = self.entries.values().collect();
        serde_json::to_vec(&entries).map_err(|e| e.to_string())
    }

    /// Deserialize and merge a full snapshot from JSON bytes
    pub fn deserialize_snapshot(&mut self, bytes: &[u8]) -> Result<usize, String> {
        let entries: Vec<CrdtEntry> =
            serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let mut count = 0;
        for entry in entries {
            if let MergeResult::Accepted(_) = self.merge_remote(entry) {
                count += 1;
            }
        }
        Ok(count)
    }
}

/// Result of merging a remote entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeResult {
    /// Entry was accepted (newer than local)
    Accepted(CrdtEntry),
    /// Entry was stale (local is newer)
    Stale,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_apply_and_get() {
        let node = NodeId::new();
        let mut store = CrdtStore::new(node);
        let entry = store.apply_local(DataType::Request, r#"{"url":"https://example.com"}"#.into());
        assert_eq!(store.len(), 1);
        assert_eq!(store.get(&entry.id).unwrap().data_type, DataType::Request);
    }

    #[test]
    fn test_lww_merge_newer_wins() {
        let node_a = NodeId("aaaa".into());
        let id = Uuid::new_v4();

        let mut store_a = CrdtStore::new(node_a);

        let entry_b = CrdtEntry {
            id,
            data_type: DataType::Request,
            data: "from_b".into(),
            timestamp: 200,
            node_id: "bbbb".into(),
            deleted: false,
            version: 1,
        };

        assert_eq!(store_a.merge_remote(entry_b.clone()), MergeResult::Accepted(entry_b.clone()));
        assert_eq!(store_a.get(&id).unwrap().data, "from_b");
    }

    #[test]
    fn test_lww_merge_stale() {
        let node_a = NodeId("aaaa".into());
        let id = Uuid::new_v4();

        let mut store_a = CrdtStore::new(node_a);

        let local = CrdtEntry {
            id,
            data_type: DataType::Request,
            data: "local_data".into(),
            timestamp: 300,
            node_id: "aaaa".into(),
            deleted: false,
            version: 1,
        };
        store_a.merge_remote(local);

        let stale = CrdtEntry {
            id,
            data_type: DataType::Request,
            data: "stale_data".into(),
            timestamp: 100,
            node_id: "bbbb".into(),
            deleted: false,
            version: 1,
        };

        assert_eq!(store_a.merge_remote(stale), MergeResult::Stale);
        assert_eq!(store_a.get(&id).unwrap().data, "local_data");
    }

    #[test]
    fn test_tombstone() {
        let node = NodeId::new();
        let mut store = CrdtStore::new(node.clone());
        let entry = store.apply_local(DataType::Request, "data".into());
        assert_eq!(store.len(), 1);

        store.delete_local(entry.id);
        assert!(store.get(&entry.id).unwrap().deleted);
        assert!(store.get_by_type(DataType::Request).is_empty());
    }

    /// A malicious or corrupt remote peer can broadcast a CrdtEntry with
    /// `timestamp: u64::MAX` for any id (merge_remote has no upper-bound
    /// validation — "higher timestamp always wins"). `update_local`/
    /// `delete_local` used to compute `existing.timestamp + 1` unchecked,
    /// which overflowed and panicked in debug builds (or silently wrapped in
    /// release, corrupting LWW ordering) the next time the local user
    /// touched that entry. They now use `saturating_add(1)`, so a
    /// `u64::MAX` timestamp just pins the entry there instead of crashing or
    /// wrapping — a real but non-exploitable degenerate case.
    #[test]
    fn test_remote_max_timestamp_then_local_update_does_not_overflow() {
        let node_a = NodeId("aaaa".into());
        let id = Uuid::new_v4();
        let mut store = CrdtStore::new(node_a);

        // Attacker-controlled entry arriving over gossipsub / file sync.
        // merge_remote has no upper-bound validation on timestamp, so this
        // is accepted as-is.
        let malicious = CrdtEntry {
            id,
            data_type: DataType::Request,
            data: "evil".into(),
            timestamp: u64::MAX,
            node_id: "attacker".into(),
            deleted: false,
            version: 1,
        };
        let result = store.merge_remote(malicious);
        assert!(matches!(result, MergeResult::Accepted(_)));

        // The local user now edits that same entry (e.g. tweaks the URL).
        // This must not panic or wrap; the entry just saturates at u64::MAX.
        let updated = store.update_local(id, DataType::Request, "user edit".into())
            .expect("entry should still exist");
        assert_eq!(updated.timestamp, u64::MAX);
        assert_eq!(updated.data, "user edit");

        // delete_local has the identical pattern - confirm it too.
        let deleted = store.delete_local(id).expect("entry should still exist");
        assert_eq!(deleted.timestamp, u64::MAX);
        assert!(deleted.deleted);
    }
}
