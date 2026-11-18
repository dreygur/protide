use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Unique node identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// First 8 characters of the id, for display. Node ids are read from a
    /// user-writable file (`load_or_create_node_id`), so this must not assume
    /// a full-length UUID.
    pub fn short(&self) -> &str {
        let end = self
            .0
            .char_indices()
            .nth(8)
            .map_or(self.0.len(), |(i, _)| i);
        &self.0[..end]
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of data being synced
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    Collection,
    Request,
    Environment,
    EnvironmentState,
    CollectionMeta,
    /// Actual workspace file content (path + bytes encoded as JSON)
    WorkspaceFile,
}

/// A single CRDT entry - the atomic unit of sync
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtEntry {
    /// Globally unique ID for this entry
    pub id: Uuid,
    /// What kind of data this is
    pub data_type: DataType,
    /// Serialized data payload (JSON)
    pub data: String,
    /// wall-clock timestamp (ms since epoch); local writes monotonically increment via max(), cross-machine clock skew is a known LWW limitation
    pub timestamp: u64,
    /// Node that authored this version
    pub node_id: String,
    /// Whether this entry is deleted (tombstone)
    pub deleted: bool,
    /// Schema version for future-proofing
    pub version: u32,
}

impl CrdtEntry {
    pub fn new(data_type: DataType, data: String, node_id: &NodeId) -> Self {
        Self {
            id: Uuid::new_v4(),
            data_type,
            data,
            timestamp: timestamp_now(),
            node_id: node_id.0.clone(),
            deleted: false,
            version: 1,
        }
    }

    /// Merge with another entry (LWW - latest timestamp wins, NodeId breaks ties)
    pub fn merge(&self, other: &Self) -> Self {
        if other.timestamp > self.timestamp
            || (other.timestamp == self.timestamp && other.node_id > self.node_id)
        {
            other.clone()
        } else {
            self.clone()
        }
    }
}

/// Events emitted by the sync engine for the UI to consume
#[derive(Debug, Clone)]
pub enum SyncEvent {
    /// A CRDT entry was received and applied
    EntryReceived(CrdtEntry),
    /// A peer joined the swarm
    PeerJoined(String),
    /// A peer left the swarm
    PeerLeft(String),
    /// Live probe activity
    LiveActivity(LiveActivity),
    /// A sync backend became ready/unready
    BackendStatus { backend: SyncBackend, ready: bool },
    /// Error occurred
    SyncError(String),
    /// PAKE handshake completed successfully - both sides derived the shared key
    HandshakeComplete { peer_id: String, peer_name: String },
    /// PAKE handshake failed (wrong code or crypto error)
    HandshakeFailed { reason: String },
    /// Internal P2P diagnostic log (mDNS discovery, PAKE steps, DHT events)
    P2PDiagnostic(String),
    /// Our own libp2p listen multiaddress - logged once on startup
    LocalAddr(String),
    /// A workspace file arrived from a peer - write to disk and refresh
    FileReceived {
        relative_path: String,
        content: String,
        deleted: bool,
    },
}

/// Live activity from a peer (response console output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveActivity {
    pub node_id: String,
    pub node_name: String,
    pub request_name: String,
    pub status: u16,
    pub time_ms: u64,
    pub method: String,
    pub url: String,
}

/// Which sync backend is in use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyncBackend {
    FileSystem,
    P2P,
    LiveProbe,
}

/// Configuration for the sync engine
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Node display name
    pub node_name: String,
    /// Path to the .protide sync folder (for BYOB)
    pub sync_folder: Option<PathBuf>,
    /// Whether to enable P2P sync
    pub p2p_enabled: bool,
    /// Whether to enable live probe
    pub live_probe_enabled: bool,
    /// Port for live probe UDP broadcast
    pub live_probe_port: u16,
    /// PAKE pairing code for secure P2P
    pub pairing_code: Option<String>,
    /// Where to persist this node's identity UUID across restarts
    pub node_id_path: Option<PathBuf>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            node_name: format!(
                "protide-{}",
                Uuid::new_v4()
                    .to_string()
                    .split('-')
                    .next()
                    .unwrap_or("node")
            ),
            sync_folder: None,
            p2p_enabled: false,
            live_probe_enabled: false,
            live_probe_port: 42069,
            pairing_code: None,
            node_id_path: None,
        }
    }
}

/// Load the persisted NodeId from `path`, or create and save a new one.
pub fn load_or_create_node_id(path: &std::path::Path) -> NodeId {
    if let Ok(s) = std::fs::read_to_string(path) {
        let s = s.trim().to_string();
        if !s.is_empty() {
            return NodeId(s);
        }
    }
    let id = NodeId::new();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, &id.0);
    id
}

pub(crate) fn timestamp_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    fn entry(ts: u64, node: &str, data: &str) -> CrdtEntry {
        CrdtEntry {
            id: Uuid::new_v4(),
            data_type: DataType::Request,
            data: data.into(),
            timestamp: ts,
            node_id: node.into(),
            deleted: false,
            version: 1,
        }
    }

    #[test]
    fn test_node_id_short_never_panics_on_odd_ids() {
        assert_eq!(NodeId::new().short().chars().count(), 8);
        // Node ids come from a file the user can edit or truncate.
        assert_eq!(NodeId(String::new()).short(), "");
        assert_eq!(NodeId("abc".into()).short(), "abc");
        assert_eq!(NodeId("ααααααααα".into()).short(), "αααααααα");
    }

    #[test]
    fn test_node_ids_are_unique() {
        assert_ne!(NodeId::new(), NodeId::default());
    }

    /// The entry wire format is what peers on other Protide versions parse;
    /// renaming a field silently breaks sync, so pin the exact JSON shape.
    #[test]
    fn test_crdt_entry_wire_format_is_stable() {
        let id = Uuid::nil();
        let e = CrdtEntry {
            id,
            data_type: DataType::EnvironmentState,
            data: "{}".into(),
            timestamp: 7,
            node_id: "n1".into(),
            deleted: true,
            version: 1,
        };
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "id": "00000000-0000-0000-0000-000000000000",
                "data_type": "EnvironmentState",
                "data": "{}",
                "timestamp": 7,
                "node_id": "n1",
                "deleted": true,
                "version": 1,
            })
        );
        let back: CrdtEntry = serde_json::from_value(json).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn test_all_data_types_roundtrip() {
        for dt in [
            DataType::Collection,
            DataType::Request,
            DataType::Environment,
            DataType::EnvironmentState,
            DataType::CollectionMeta,
            DataType::WorkspaceFile,
        ] {
            let json = serde_json::to_string(&dt).unwrap();
            assert_eq!(serde_json::from_str::<DataType>(&json).unwrap(), dt);
        }
    }

    /// Malformed or future-version entries from a peer must be a clean parse
    /// error, never a panic.
    #[test]
    fn test_malformed_entries_are_rejected_cleanly() {
        for bad in [
            r#"{"data_type":"UnknownFutureType","id":"00000000-0000-0000-0000-000000000000","data":"","timestamp":1,"node_id":"n","deleted":false,"version":1}"#,
            r#"{"id":"not-a-uuid","data_type":"Request","data":"","timestamp":1,"node_id":"n","deleted":false,"version":1}"#,
            r#"{"id":"00000000-0000-0000-0000-000000000000","data_type":"Request"}"#,
            r#"{"timestamp":-1}"#,
            "null",
            "",
        ] {
            assert!(
                serde_json::from_str::<CrdtEntry>(bad).is_err(),
                "accepted {}",
                bad
            );
        }
    }

    /// `CrdtEntry::merge` duplicates the ordering rule in
    /// `CrdtStore::merge_remote`; if the two ever drift, replicas that sync
    /// through different code paths stop agreeing.
    #[test]
    fn test_entry_merge_matches_store_merge_and_is_commutative() {
        let base = entry(100, "aaaa", "a");
        let cases = [
            (
                base.clone(),
                CrdtEntry {
                    timestamp: 200,
                    data: "b".into(),
                    node_id: "bbbb".into(),
                    ..base.clone()
                },
            ),
            (
                base.clone(),
                CrdtEntry {
                    timestamp: 100,
                    data: "b".into(),
                    node_id: "bbbb".into(),
                    ..base.clone()
                },
            ),
            (
                base.clone(),
                CrdtEntry {
                    timestamp: 50,
                    data: "b".into(),
                    node_id: "bbbb".into(),
                    ..base.clone()
                },
            ),
        ];
        for (left, right) in cases {
            assert_eq!(
                left.merge(&right),
                right.merge(&left),
                "merge must be commutative"
            );
            assert_eq!(left.merge(&left), left, "merge must be idempotent");

            let mut store = crate::sync::CrdtStore::new(NodeId("replica".into()));
            store.merge_remote(left.clone());
            store.merge_remote(right.clone());
            assert_eq!(*store.get(&left.id).unwrap(), left.merge(&right));
        }
    }

    #[test]
    fn test_live_activity_roundtrip() {
        let activity = LiveActivity {
            node_id: "n1".into(),
            node_name: "alice".into(),
            request_name: "Get users".into(),
            status: 204,
            time_ms: 12,
            method: "DELETE".into(),
            url: "https://api.example.com".into(),
        };
        let json = serde_json::to_string(&activity).unwrap();
        let back: LiveActivity = serde_json::from_str(&json).unwrap();
        assert_eq!(back.node_id, activity.node_id);
        assert_eq!(back.status, activity.status);
        assert_eq!(back.url, activity.url);
        // Every field is required - a partial payload is not silently defaulted.
        assert!(serde_json::from_str::<LiveActivity>(r#"{"node_id":"n1"}"#).is_err());
    }

    #[test]
    fn test_load_or_create_node_id_persists_across_calls() {
        let tmp = TempDir::new("protide_node_id");
        let path = tmp.path().join("nested").join("node_id");

        let first = load_or_create_node_id(&path);
        assert!(path.exists(), "missing parent directories must be created");
        assert_eq!(
            load_or_create_node_id(&path),
            first,
            "identity must be stable"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap().trim(), first.0);
    }

    #[test]
    fn test_load_or_create_node_id_handles_damaged_files() {
        let tmp = TempDir::new("protide_node_id_damaged");

        // Trailing newline (what `echo >` leaves behind) must be trimmed.
        let padded = tmp.write("padded", b"  abcd-1234 \n");
        assert_eq!(load_or_create_node_id(&padded).0, "abcd-1234");

        // An empty/whitespace-only file must yield a fresh, persisted id.
        let empty = tmp.write("empty", b"   \n");
        let created = load_or_create_node_id(&empty);
        assert!(!created.0.is_empty());
        assert_eq!(load_or_create_node_id(&empty), created);
    }

    #[test]
    fn test_default_config_is_fully_offline() {
        let config = SyncConfig::default();
        assert!(!config.p2p_enabled);
        assert!(!config.live_probe_enabled);
        assert!(config.sync_folder.is_none());
        assert!(config.pairing_code.is_none());
        assert!(config.node_id_path.is_none());
        assert!(config.node_name.starts_with("protide-"));
        assert_ne!(SyncConfig::default().node_name, config.node_name);
    }

    #[test]
    fn test_timestamp_now_is_monotonic_and_plausible() {
        let t = timestamp_now();
        // Any wall clock sane enough to sync with is past 2020-01-01.
        assert!(t > 1_577_836_800_000, "timestamp {} looks wrong", t);
        assert!(timestamp_now() >= t);
    }
}
