//! SyncEngine method implementations — split from mod.rs to stay within the
//! 333-line file-size limit.

use super::*;

impl SyncEngine {
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
            info!("[PAKE] initiate_handshake called for code: {}", code);
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
                info!("[PAKE] Init packet published on topic: protide-pake-{}", code);
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
