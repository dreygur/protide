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
//! transport layer. Conflicts are resolved automatically by Lamport timestamp.
//!
//! The sync engine is "Git-inspired" - each node maintains a local HEAD state,
//! and remote changes arrive as "commits" that are fast-forward merged.

pub mod crdt;
pub mod file_sync;
pub mod live_probe;
pub mod pake;
pub mod p2p;
pub mod types;

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
        let node_id = NodeId::new();
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

    /// Apply a local change to the CRDT store and propagate to all backends.
    pub fn apply_local_change(&mut self, data_type: DataType, data: String) -> types::CrdtEntry {
        let entry = self.store.apply_local(data_type, data);

        // Write to file sync
        if let Some(ref fs) = self.file_sync {
            let _ = fs.write_entry(&entry);
        }

        // Broadcast via P2P
        #[cfg(feature = "p2p-sync")]
        if let Some(ref mut p2p) = self.p2p_sync {
            let _ = p2p.broadcast_entry(&entry);
        }

        self.event_count += 1;
        entry
    }

    /// Update an existing entry locally
    pub fn update_local_change(&mut self, id: uuid::Uuid, data_type: DataType, data: String) -> Option<types::CrdtEntry> {
        let entry = self.store.update_local(id, data_type, data)?;

        if let Some(ref fs) = self.file_sync {
            let _ = fs.write_entry(&entry);
        }

        #[cfg(feature = "p2p-sync")]
        if let Some(ref mut p2p) = self.p2p_sync {
            let _ = p2p.broadcast_entry(&entry);
        }

        self.event_count += 1;
        Some(entry)
    }

    /// Delete an entry locally
    pub fn delete_local_change(&mut self, id: uuid::Uuid) -> Option<types::CrdtEntry> {
        let entry = self.store.delete_local(id)?;

        if let Some(ref fs) = self.file_sync {
            let _ = fs.delete_entry(&id);
            // Also write the tombstone
            let _ = fs.write_entry(&entry);
        }

        #[cfg(feature = "p2p-sync")]
        if let Some(ref mut p2p) = self.p2p_sync {
            let _ = p2p.broadcast_entry(&entry);
        }

        self.event_count += 1;
        Some(entry)
    }

    /// Broadcast a workspace file change (create/modify/delete) to all P2P peers.
    pub fn broadcast_workspace_file(&mut self, workspace_root: &std::path::Path, file_path: &std::path::Path, content: String, deleted: bool) {
        let rel = file_path.strip_prefix(workspace_root).unwrap_or(file_path);
        let payload = serde_json::json!({
            "path": rel.to_string_lossy(),
            "content": content,
            "deleted": deleted,
        }).to_string();
        self.apply_local_change(DataType::WorkspaceFile, payload);
    }

    /// Broadcast a live activity event via UDP
    pub fn broadcast_live_activity(
        &self,
        request_name: &str,
        status: u16,
        time_ms: u64,
        method: &str,
        url: &str,
    ) {
        if let Some(ref lp) = self.live_probe {
            let _ = lp.broadcast(request_name, status, time_ms, method, url);
        }
    }

    /// Poll all backends for incoming events and drain them into the event channel.
    pub fn poll(&mut self) -> Vec<SyncEvent> {
        let mut events = Vec::new();

        // Poll file sync events
        if let Some(ref mut fs) = self.file_sync {
            for fs_event in fs.poll_events() {
                match fs_event {
                    FileSyncEvent::EntryReceived(entry) => {
                        match self.store.merge_remote(entry.clone()) {
                            MergeResult::Accepted(_) => {
                                push_entry_event(entry, &mut events);
                            }
                            MergeResult::Stale => {}
                        }
                    }
                    FileSyncEvent::EntryDeleted(_id) => {
                        // Re-create the entry as a tombstone via remote merge
                        // (the actual delete event will be handled when the tombstone is read)
                    }
                    FileSyncEvent::Error(e) => {
                        events.push(SyncEvent::SyncError(e));
                    }
                }
            }
        }

        // Poll P2P events - two-phase: read all events, then send any PAKE responses
        #[cfg(feature = "p2p-sync")]
        {
            let p2p_events: Vec<_> = self.p2p_sync.as_ref()
                .map(|p| p.poll_events())
                .unwrap_or_default();

            // (topic, serialised PakeMsgPayload) to publish after processing
            let mut pake_resps: Vec<(String, Vec<u8>)> = Vec::new();

            for p2p_event in p2p_events {
                match p2p_event {
                    p2p::P2PEvent::EntryReceived(entry) => {
                        match self.store.merge_remote(entry.clone()) {
                            MergeResult::Accepted(_) => {
                                push_entry_event(entry, &mut events);
                            }
                            MergeResult::Stale => {}
                        }
                    }
                    p2p::P2PEvent::PeerJoined(peer) => {
                        info!("[mDNS] Discovered peer: {}", peer);
                        events.push(SyncEvent::P2PDiagnostic(
                            format!("[mDNS] Discovered peer: {}", peer)
                        ));
                        events.push(SyncEvent::PeerJoined(peer.to_string()));
                    }
                    p2p::P2PEvent::PeerLeft(peer) => {
                        events.push(SyncEvent::PeerLeft(peer.to_string()));
                    }
                    p2p::P2PEvent::Error(e) => {
                        events.push(SyncEvent::SyncError(e));
                    }
                    p2p::P2PEvent::LocalAddr(addr) => {
                        events.push(SyncEvent::LocalAddr(addr));
                    }
                    p2p::P2PEvent::PakeMsg { from, topic, node_name, kind, pake_bytes } => {
                        info!("[PAKE] Received '{}' from {} on topic {}", kind, from, topic);
                        events.push(SyncEvent::P2PDiagnostic(
                            format!("[PAKE] Received '{}' from {} on {}", kind, from, topic)
                        ));
                        #[cfg(feature = "pake-auth")]
                        {
                            let code = topic.strip_prefix("protide-pake-").unwrap_or("");
                            match kind.as_str() {
                                "init" => {
                                    // We are Alice: generate A-side, finish immediately, send resp
                                    if let Ok((msg_a, state_a)) = pake::pake_initiate(code) {
                                        if pake::pake_finish(state_a, &pake_bytes).is_ok() {
                                            info!("[PAKE] Handshake complete (init) with peer {}", from);
                                            events.push(SyncEvent::HandshakeComplete {
                                                peer_id: from.to_string(),
                                                peer_name: node_name.clone(),
                                            });
                                        } else {
                                            info!("[PAKE] Handshake mismatch on 'init' from peer {}", from);
                                            events.push(SyncEvent::HandshakeFailed {
                                                reason: "PAKE Mismatch".to_string(),
                                            });
                                        }
                                        let resp = p2p::PakeMsgPayload {
                                            kind: "resp".to_string(),
                                            node_name: self.config.node_name.clone(),
                                            pake_bytes: msg_a,
                                        };
                                        if let Ok(data) = serde_json::to_vec(&resp) {
                                            pake_resps.push((topic, data));
                                        }
                                    }
                                }
                                "resp" => {
                                    // We are Bob: finish with Alice's message
                                    if let Some(state_b) = self.pake_pending.take() {
                                        if pake::pake_finish(state_b, &pake_bytes).is_ok() {
                                            info!("[PAKE] Handshake complete (resp) with peer {}", from);
                                            events.push(SyncEvent::HandshakeComplete {
                                                peer_id: from.to_string(),
                                                peer_name: node_name,
                                            });
                                        } else {
                                            info!("[PAKE] Handshake mismatch on 'resp' from peer {}", from);
                                            events.push(SyncEvent::HandshakeFailed {
                                                reason: "PAKE Mismatch".to_string(),
                                            });
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        // Without pake-auth, handshake messages are silently ignored
                        #[cfg(not(feature = "pake-auth"))]
                        { let _ = (from, topic, node_name, kind, pake_bytes); }
                    }
                }
            }

            // Send PAKE responses (requires mutable borrow - done after the read loop)
            for (topic, data) in pake_resps {
                if let Some(ref p2p) = self.p2p_sync {
                    p2p.publish_on_topic(&topic, data);
                }
            }
        }

        // Poll live probe events
        if let Some(ref lp) = self.live_probe {
            for (_addr, activity) in lp.drain_activities() {
                events.push(SyncEvent::LiveActivity(activity));
            }
        }

        self.event_count += events.len() as u64;
        events
    }

    /// Drain pending sync events (for the UI to consume)
    pub fn drain_events(&self) -> Vec<SyncEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.event_rx.try_recv() {
            events.push(evt);
        }
        events
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

    /// Initiate a PAKE handshake with a peer who published the given `code`.
    ///
    /// - Subscribes to the `protide-pake-{code}` gossipsub topic.
    /// - Broadcasts Bob's SPAKE2 public key as an "init" message.
    /// - Stores Bob's state so it can be finished when Alice's "resp" arrives.
    ///
    /// Requires the `full-sync` feature (`p2p-sync` + `pake-auth`).
    /// Without those features this is a no-op that always returns `Ok(())`.
    pub fn initiate_handshake(&mut self, _code: &str) -> Result<(), String> {
        #[cfg(all(feature = "p2p-sync", feature = "pake-auth"))]
        {
            let code = _code;
            println!("[PAKE] initiate_handshake called for code: {}", code);
            // Bob calls pake_respond to generate his B-side key
            let (msg_b, state_b) = pake::pake_respond(code)?;
            self.pake_pending = Some(state_b);
            self.pake_pending_code = code.to_string();

            if let Some(ref p2p) = self.p2p_sync {
                p2p.subscribe_pake_topic(code);
                let payload = p2p::PakeMsgPayload {
                    kind: "init".to_string(),
                    node_name: self.config.node_name.clone(),
                    pake_bytes: msg_b,
                };
                let data = serde_json::to_vec(&payload)
                    .map_err(|e| format!("Serialisation error: {}", e))?;
                p2p.publish_on_pake_topic(code, data);
                println!("[PAKE] Init packet published on topic: protide-pake-{}", code);
                info!("[PAKE] Initiation packet sent for code: {}", code);
                let _ = self.event_tx.send(SyncEvent::P2PDiagnostic(
                    format!("[PAKE] Initiation packet sent for code: {}", code)
                ));
            }
        }
        Ok(())
    }

    /// Perform a periodic tick - call this from a timer (e.g., every 1 second)
    pub fn tick(&mut self) -> Vec<SyncEvent> {
        self.poll()
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
