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

    pub fn short(&self) -> &str {
        &self.0[..8]
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
}

/// A single CRDT entry — the atomic unit of sync
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtEntry {
    /// Globally unique ID for this entry
    pub id: Uuid,
    /// What kind of data this is
    pub data_type: DataType,
    /// Serialized data payload (JSON)
    pub data: String,
    /// Lamport timestamp (milliseconds since epoch)
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

    pub fn tombstone(id: Uuid, node_id: &NodeId) -> Self {
        Self {
            id,
            data_type: DataType::Request,
            data: String::new(),
            timestamp: timestamp_now(),
            node_id: node_id.0.clone(),
            deleted: true,
            version: 1,
        }
    }

    /// Merge with another entry (LWW — latest timestamp wins, NodeId breaks ties)
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
    /// PAKE handshake completed successfully — both sides derived the shared key
    HandshakeComplete { peer_id: String, peer_name: String },
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
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            node_name: format!("protide-{}", Uuid::new_v4().to_string().split('-').next().unwrap_or("node")),
            sync_folder: None,
            p2p_enabled: false,
            live_probe_enabled: false,
            live_probe_port: 42069,
            pairing_code: None,
        }
    }
}

pub(crate) fn timestamp_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
