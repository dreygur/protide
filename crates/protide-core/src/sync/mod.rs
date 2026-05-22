//! Local-First Sync Engine for Protide.
//!
//! Provides decentralized, zero-infrastructure collaboration through:
//! - **BYOB** (Bring Your Own Backend): File-based sync via Dropbox/Drive/GitHub
//! - **P2P**: Direct libp2p connections with mDNS + Gossipsub
//! - **Live Probe**: UDP broadcast for real-time activity on local network
//! - **PAKE**: Password-authenticated key exchange for secure P2P pairing
//!
//! # Architecture
//!
//! Each Protide instance is a node in a swarm. Changes are represented as
//! CRDT entries (LWW registers) and propagated to peers via the configured
//! transport layer. Conflicts are resolved automatically by wall-clock LWW (last-write-wins by timestamp).
//!
//! The sync engine is "Git-inspired" - each node maintains a local HEAD state,
//! and remote changes arrive as "commits" that are fast-forward merged.

pub mod crdt;
pub mod file_sync;
pub mod live_probe;
pub mod pake;
pub mod p2p;
pub mod types;

mod engine;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

#[allow(unused_imports)]
use log::{debug, info};

pub use crdt::{CrdtStore, MergeResult};
pub use types::*;

use crate::sync::file_sync::{FileSync, FileSyncEvent};

fn push_entry_event(entry: types::CrdtEntry, events: &mut Vec<SyncEvent>) {
    if entry.data_type == DataType::WorkspaceFile {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&entry.data) {
            events.push(SyncEvent::FileReceived {
                relative_path: v["path"].as_str().unwrap_or_default().to_string(),
                content: v["content"].as_str().unwrap_or_default().to_string(),
                deleted: v["deleted"].as_bool().unwrap_or(false),
            });
        }
    } else {
        events.push(SyncEvent::EntryReceived(entry));
    }
}

/// The master sync engine - coordinates all backends and exposes a unified
/// event stream for the UI to consume.
pub struct SyncEngine {
    /// Our node identity
    node_id: NodeId,
    /// Configuration
    config: SyncConfig,
    /// CRDT state store
    pub store: CrdtStore,
    /// File-based sync backend (BYOB)
    file_sync: Option<FileSync>,
    /// P2P sync backend
    #[cfg(feature = "p2p-sync")]
    p2p_sync: Option<p2p::P2PSync>,
    /// Live probe for UDP broadcast
    live_probe: Option<live_probe::LiveProbe>,
    /// Event channel for the UI to consume
    event_tx: Sender<SyncEvent>,
    event_rx: Receiver<SyncEvent>,
    /// Number of events processed
    event_count: u64,
    /// Pending PAKE handshake state (Bob's side - stored until Alice's response arrives)
    #[cfg(feature = "pake-auth")]
    pake_pending: Option<spake2::Spake2<spake2::Ed25519Group>>,
    /// The pairing code used for the pending handshake
    #[cfg(feature = "pake-auth")]
    pake_pending_code: String,
}

impl SyncEngine {
    /// Create a new sync engine with the given configuration.
    pub fn new(config: SyncConfig) -> Self {
        let node_id = match &config.node_id_path {
            Some(path) => types::load_or_create_node_id(path),
            None => NodeId::new(),
        };
        let store = CrdtStore::new(node_id.clone());
        let (event_tx, event_rx) = mpsc::channel::<SyncEvent>();

        Self {
            node_id,
            config,
            store,
            file_sync: None,
            live_probe: None,
            event_tx,
            event_rx,
            event_count: 0,
            #[cfg(feature = "p2p-sync")]
            p2p_sync: None,
            #[cfg(feature = "pake-auth")]
            pake_pending: None,
            #[cfg(feature = "pake-auth")]
            pake_pending_code: String::new(),
        }
    }

    /// Initialize all configured backends. Call this after creating the engine.
    pub fn init(&mut self) -> Result<(), String> {
        let node_id = self.node_id.clone();
        let event_tx = self.event_tx.clone();

        // Initialize file sync (BYOB)
        if let Some(ref sync_folder) = self.config.sync_folder {
            match FileSync::open(sync_folder, node_id.clone()) {
                Ok(fs) => {
                    // Load existing entries from disk
                    let entries = fs.read_all_entries();
                    for entry in &entries {
                        self.store.merge_remote(entry.clone());
                    }
                    self.file_sync = Some(fs);
                    let _ = event_tx.send(SyncEvent::BackendStatus {
                        backend: SyncBackend::FileSystem,
                        ready: true,
                    });
                }
                Err(e) => {
                    let _ = event_tx.send(SyncEvent::SyncError(format!("File sync init failed: {}", e)));
                }
            }
        }

        // Initialize Live Probe
        if self.config.live_probe_enabled {
            match live_probe::LiveProbe::start(
                node_id.clone(),
                self.config.node_name.clone(),
                Some(self.config.live_probe_port),
            ) {
                Ok(lp) => {
                    self.live_probe = Some(lp);
                    let _ = event_tx.send(SyncEvent::BackendStatus {
                        backend: SyncBackend::LiveProbe,
                        ready: true,
                    });
                }
                Err(e) => {
                    let _ = event_tx.send(SyncEvent::SyncError(format!("Live probe init failed: {}", e)));
                }
            }
        }

        // Initialize P2P with pairing code for topic scoping
        #[cfg(feature = "p2p-sync")]
        if self.config.p2p_enabled {
            let pairing_code = self.config.pairing_code.clone().unwrap_or_default();
            match p2p::P2PSync::start(node_id.clone(), None, &pairing_code) {
                Ok(p2p) => {
                    self.p2p_sync = Some(p2p);
                    let _ = event_tx.send(SyncEvent::BackendStatus {
                        backend: SyncBackend::P2P,
                        ready: true,
                    });
                }
                Err(e) => {
                    let _ = event_tx.send(SyncEvent::SyncError(format!("P2P init failed: {}", e)));
                }
            }
        }

        Ok(())
    }

    /// Get our node ID
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Get the sync configuration
    pub fn config(&self) -> &SyncConfig {
        &self.config
    }

    /// Get the number of events processed
    pub fn event_count(&self) -> u64 {
        self.event_count
    }

    /// Check if file sync is active
    pub fn is_file_sync_active(&self) -> bool {
        self.file_sync.is_some()
    }

    /// Check if P2P is active
    #[cfg(feature = "p2p-sync")]
    pub fn is_p2p_active(&self) -> bool {
        self.p2p_sync.is_some()
    }

    /// Check if P2P is active (non-feature-gated fallback)
    #[cfg(not(feature = "p2p-sync"))]
    pub fn is_p2p_active(&self) -> bool {
        false
    }

    /// Get the sync folder path if file sync is active
    pub fn sync_folder_path(&self) -> Option<PathBuf> {
        self.config.sync_folder.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_create_and_apply() {
        let config = SyncConfig::default();
        let mut engine = SyncEngine::new(config);

        let entry = engine.apply_local_change(
            DataType::Request,
            r#"{"url":"https://test.com"}"#.into(),
        );

        assert_eq!(engine.store.len(), 1);
        assert_eq!(entry.data_type, DataType::Request);
    }

    #[test]
    fn test_engine_update_and_delete() {
        let config = SyncConfig::default();
        let mut engine = SyncEngine::new(config);

        let entry = engine.apply_local_change(
            DataType::Request,
            r#"{"url":"https://test.com"}"#.into(),
        );

        // Update
        let updated = engine.update_local_change(
            entry.id,
            DataType::Request,
            r#"{"url":"https://updated.com"}"#.into(),
        );
        assert!(updated.is_some());
        assert_eq!(
            engine.store.get(&entry.id).unwrap().data,
            r#"{"url":"https://updated.com"}"#
        );

        // Delete
        let deleted = engine.delete_local_change(entry.id);
        assert!(deleted.is_some());
        assert!(engine.store.get(&entry.id).unwrap().deleted);
    }

    #[test]
    fn test_engine_poll_returns_events() {
        let config = SyncConfig::default();
        let mut engine = SyncEngine::new(config);
        let events = engine.tick();
        // No backends enabled, so no events expected
        assert!(events.is_empty());
    }
}
