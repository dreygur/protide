use std::collections::HashMap;
use uuid::Uuid;

use super::types::{CrdtEntry, DataType, NodeId, timestamp_now};

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
    pub fn apply_local(&mut self, data_type: DataType, data: String) -> CrdtEntry {
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
            Some(existing) => (
                std::cmp::max(timestamp_now(), existing.timestamp.saturating_add(1)),
                existing.data_type,
            ),
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
                // Ordered by (timestamp, node_id) and then by the payload itself.
                // The payload tie-break is what makes the order *total*: node ids
                // are not guaranteed unique (the id is persisted under config_dir,
                // so a cloned machine image or a synced config directory gives two
                // live nodes the same one). Without it, two same-millisecond edits
                // from the same node id leave each replica holding whichever it saw
                // first, and the divergence never heals. Comparing data/deleted is
                // arbitrary but identical on every replica, which is all convergence
                // requires. Entries equal on all four are genuinely the same write,
                // so re-applying one stays idempotent.
                if (entry.timestamp, &entry.node_id, &entry.data, entry.deleted)
                    > (local.timestamp, &local.node_id, &local.data, local.deleted)
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
        let entries: Vec<CrdtEntry> = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
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
    use std::collections::BTreeMap;

    // ── Convergence harness ──────────────────────────────────────────────
    //
    // A LWW-register CRDT must be a join-semilattice: merging the same set of
    // operations in ANY delivery order must produce the same state on every
    // replica. Gossipsub and cloud-folder sync both reorder and re-deliver
    // messages freely, so anything less is silent data loss.

    fn entry(id: Uuid, ts: u64, node: &str, data: &str, deleted: bool) -> CrdtEntry {
        CrdtEntry {
            id,
            data_type: DataType::Request,
            data: data.into(),
            timestamp: ts,
            node_id: node.into(),
            deleted,
            version: 1,
        }
    }

    fn snapshot(store: &CrdtStore) -> BTreeMap<Uuid, CrdtEntry> {
        store
            .all_entries()
            .into_iter()
            .map(|e| (e.id, e.clone()))
            .collect()
    }

    /// Every ordering of `items` (Heap's algorithm).
    fn permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
        if items.len() <= 1 {
            return vec![items.to_vec()];
        }
        let mut out = Vec::new();
        for i in 0..items.len() {
            let mut rest = items.to_vec();
            let head = rest.remove(i);
            for mut tail in permutations(&rest) {
                tail.insert(0, head.clone());
                out.push(tail);
            }
        }
        out
    }

    /// Merge `ops` into a fresh store in every possible order and assert all
    /// orders agree. Returns the converged state.
    #[track_caller]
    fn assert_converges(ops: &[CrdtEntry]) -> BTreeMap<Uuid, CrdtEntry> {
        let orders = permutations(ops);
        let mut expected: Option<BTreeMap<Uuid, CrdtEntry>> = None;
        for order in &orders {
            let mut store = CrdtStore::new(NodeId("replica".into()));
            for op in order {
                store.merge_remote(op.clone());
            }
            let got = snapshot(&store);
            match &expected {
                None => expected = Some(got),
                Some(first) => assert_eq!(
                    *first,
                    got,
                    "replicas diverged: delivery order {:?} produced a different state",
                    order
                        .iter()
                        .map(|e| (e.timestamp, &e.node_id, &e.data))
                        .collect::<Vec<_>>()
                ),
            }
        }
        expected.expect("no orderings")
    }

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

        assert_eq!(
            store_a.merge_remote(entry_b.clone()),
            MergeResult::Accepted(entry_b.clone())
        );
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
        let updated = store
            .update_local(id, DataType::Request, "user edit".into())
            .expect("entry should still exist");
        assert_eq!(updated.timestamp, u64::MAX);
        assert_eq!(updated.data, "user edit");

        // delete_local has the identical pattern - confirm it too.
        let deleted = store.delete_local(id).expect("entry should still exist");
        assert_eq!(deleted.timestamp, u64::MAX);
        assert!(deleted.deleted);
    }

    // ── Convergence ──────────────────────────────────────────────────────

    /// Concurrent update-vs-delete on the same key, plus a second key edited
    /// concurrently: whatever order the four messages arrive in, every replica
    /// must end up identical.
    #[test]
    fn test_concurrent_update_and_delete_converge_in_every_order() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let ops = vec![
            entry(a, 100, "aaaa", "a-v1", false),
            entry(a, 101, "bbbb", "", true), // b deletes
            entry(b, 100, "aaaa", "b-from-a", false),
            entry(b, 100, "bbbb", "b-from-b", false), // exact tie on b
        ];
        let state = assert_converges(&ops);
        assert!(
            state[&a].deleted,
            "the later tombstone must win over the update"
        );
        assert_eq!(
            state[&b].node_id, "bbbb",
            "an exact timestamp tie must be broken by the higher node id"
        );
    }

    /// Delete then resurrect: a later update must revive a tombstoned entry,
    /// and a late-arriving tombstone must not re-kill it.
    #[test]
    fn test_delete_then_resurrect_converges() {
        let id = Uuid::new_v4();
        let ops = vec![
            entry(id, 100, "aaaa", "original", false),
            entry(id, 200, "aaaa", "", true),
            entry(id, 300, "bbbb", "resurrected", false),
        ];
        let state = assert_converges(&ops);
        assert!(!state[&id].deleted, "the newest write revives the entry");
        assert_eq!(state[&id].data, "resurrected");
    }

    /// The timestamp tie-break must be a deterministic total order and must be
    /// symmetric: merging A into a store holding B and merging B into a store
    /// holding A must both settle on the same winner.
    #[test]
    fn test_timestamp_tie_break_is_deterministic_and_symmetric() {
        let id = Uuid::new_v4();
        let low = entry(id, 500, "aaaa", "from-a", false);
        let high = entry(id, 500, "bbbb", "from-b", false);

        for (first, second) in [(&low, &high), (&high, &low)] {
            let mut store = CrdtStore::new(NodeId("replica".into()));
            store.merge_remote(first.clone());
            store.merge_remote(second.clone());
            assert_eq!(
                store.get(&id).unwrap().data,
                "from-b",
                "the higher node id must always win a timestamp tie"
            );
        }

        // The loser is reported as stale, not silently accepted.
        let mut store = CrdtStore::new(NodeId("replica".into()));
        store.merge_remote(high.clone());
        assert_eq!(store.merge_remote(low), MergeResult::Stale);
    }

    /// Gossipsub re-delivers messages; applying the same entry any number of
    /// times must be a no-op after the first.
    #[test]
    fn test_reapplying_the_same_entry_is_idempotent() {
        let id = Uuid::new_v4();
        let e = entry(id, 100, "aaaa", "value", false);
        let mut store = CrdtStore::new(NodeId("replica".into()));

        assert!(matches!(
            store.merge_remote(e.clone()),
            MergeResult::Accepted(_)
        ));
        for _ in 0..5 {
            assert_eq!(store.merge_remote(e.clone()), MergeResult::Stale);
        }
        assert_eq!(store.len(), 1);
        assert_eq!(store.get(&id).unwrap().data, "value");

        // Duplicates interleaved with a newer write converge too.
        let newer = entry(id, 200, "aaaa", "newer", false);
        let state = assert_converges(&[e.clone(), newer, e]);
        assert_eq!(state[&id].data, "newer");
    }

    /// A peer with a badly skewed clock (far future / far past / zero) must
    /// not break convergence - the far-future write simply wins everywhere.
    #[test]
    fn test_clock_skew_still_converges() {
        let id = Uuid::new_v4();
        let ops = vec![
            entry(id, 0, "aaaa", "epoch", false),
            entry(id, timestamp_now(), "bbbb", "now", false),
            entry(id, u64::MAX - 1, "cccc", "year-292-million", false),
        ];
        let state = assert_converges(&ops);
        assert_eq!(state[&id].data, "year-292-million");
    }

    /// Tombstones must survive a full-snapshot round trip, and re-merging the
    /// same snapshot must change nothing (idempotent full sync).
    #[test]
    fn test_snapshot_roundtrip_is_lossless_and_idempotent() {
        let node = NodeId("aaaa".into());
        let mut source = CrdtStore::new(node.clone());
        let kept = source.apply_local(DataType::Request, "kept".into());
        let killed = source.apply_local(DataType::Environment, "killed".into());
        source.delete_local(killed.id);

        let bytes = source.serialize().unwrap();
        let mut target = CrdtStore::new(NodeId("bbbb".into()));
        assert_eq!(target.deserialize_snapshot(&bytes).unwrap(), 2);
        assert_eq!(snapshot(&target), snapshot(&source));
        assert!(
            target.get(&killed.id).unwrap().deleted,
            "tombstone must survive"
        );
        assert_eq!(target.get(&kept.id).unwrap().data, "kept");

        // Second application accepts nothing and mutates nothing.
        let before = snapshot(&target);
        assert_eq!(target.deserialize_snapshot(&bytes).unwrap(), 0);
        assert_eq!(snapshot(&target), before);
    }

    #[test]
    fn test_deserialize_snapshot_rejects_garbage() {
        let mut store = CrdtStore::new(NodeId("aaaa".into()));
        assert!(store.deserialize_snapshot(b"not json").is_err());
        assert!(store.deserialize_snapshot(b"{}").is_err());
        assert!(store.deserialize_snapshot(b"[]").is_ok());
        assert!(store.is_empty());
    }

    /// Two real replicas performing local edits (wall-clock timestamps, not
    /// hand-written ones) must converge once they exchange entries, no matter
    /// which direction the messages flow first.
    #[test]
    fn test_two_replicas_converge_after_exchanging_local_edits() {
        let mut a = CrdtStore::new(NodeId("aaaa".into()));
        let mut b = CrdtStore::new(NodeId("bbbb".into()));

        // A creates an entry; B learns about it.
        let created = a.apply_local(DataType::Request, "v1".into());
        b.merge_remote(created.clone());

        // Both edit it concurrently, then A deletes a second entry B knows of.
        let a_edit = a
            .update_local(created.id, DataType::Request, "a-v2".into())
            .unwrap();
        let b_edit = b
            .update_local(created.id, DataType::Request, "b-v2".into())
            .unwrap();
        let other = a.apply_local(DataType::Environment, "env".into());
        b.merge_remote(other.clone());
        let removed = a.delete_local(other.id).unwrap();

        // Exchange everything, in opposite orders on each side.
        for op in [&b_edit, &removed] {
            a.merge_remote(op.clone());
        }
        for op in [&removed, &a_edit] {
            b.merge_remote(op.clone());
        }

        assert_eq!(
            snapshot(&a),
            snapshot(&b),
            "replicas must converge after exchange"
        );
        assert!(a.get(&other.id).unwrap().deleted);
        assert!(a.get_by_type(DataType::Environment).is_empty());
        // Concurrent edits at the same millisecond are settled by node id.
        let winner = a.get(&created.id).unwrap();
        assert!(winner.data == "a-v2" || winner.data == "b-v2");
        if a_edit.timestamp == b_edit.timestamp {
            assert_eq!(winner.data, "b-v2", "higher node id wins an exact tie");
        }
    }

    /// A local write must always beat everything the store has already seen,
    /// including a remote entry stamped in the future - otherwise the user's
    /// own edit silently vanishes from their screen on the next sync.
    #[test]
    fn test_local_write_supersedes_a_future_dated_remote_write() {
        let id = Uuid::new_v4();
        let mut store = CrdtStore::new(NodeId("zzzz".into()));
        let future = timestamp_now() + 10_000_000;
        store.merge_remote(entry(id, future, "aaaa", "remote", false));

        let mine = store
            .update_local(id, DataType::Request, "mine".into())
            .unwrap();
        assert!(mine.timestamp > future);
        assert_eq!(store.get(&id).unwrap().data, "mine");

        // And a replica that already had the future remote write accepts ours.
        let mut peer = CrdtStore::new(NodeId("aaaa".into()));
        peer.merge_remote(entry(id, future, "aaaa", "remote", false));
        assert!(matches!(peer.merge_remote(mine), MergeResult::Accepted(_)));
        assert_eq!(peer.get(&id).unwrap().data, "mine");
    }

    /// REGRESSION: `merge_remote` used to order entries only by
    /// `(timestamp, node_id)`. When two *different* payloads carry the same
    /// timestamp AND the same node id, neither wins - each replica keeps
    /// whichever it saw first, so replicas permanently disagree and the
    /// difference never heals.
    ///
    /// Reachable when one node's identity is duplicated: `SyncEngine::new`
    /// loads the node id from `config_dir()/protide/node_id`
    /// (`crates/protide/src/main.rs:147`), so cloning a machine image, syncing
    /// a dotfiles/config directory between laptops, or restoring a backup
    /// gives two live nodes the same `node_id`. Two same-millisecond edits to
    /// the same entry then diverge silently.
    ///
    /// FIXED: `merge_remote` now applies a final deterministic tie-break on the
    /// payload (`data`, then `deleted`), making the order total so both replicas
    /// pick the same winner.
    #[test]
    fn test_identical_timestamp_and_node_id_must_still_converge() {
        let id = Uuid::new_v4();
        assert_converges(&[
            entry(id, 100, "same-node", "left", false),
            entry(id, 100, "same-node", "right", false),
        ]);
    }
}
