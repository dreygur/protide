//! PAKE handshake state machine — split out of `engine.rs::poll` so the
//! security-critical three-step exchange can be driven directly in tests
//! without a libp2p swarm.
#![cfg(all(feature = "p2p-sync", feature = "pake-auth"))]

use super::*;

impl SyncEngine {
    /// Process one PAKE handshake message received on a `protide-pake-*`
    /// topic. Pushes UI events onto `events` and returns an optional
    /// `(topic, payload)` for the caller to publish back to the peer.
    ///
    /// Messages are only acted on while a handshake we started is pending
    /// (`pake_pending_code` non-empty) - the topic name is a one-way hash of
    /// the pairing code (see `p2p::topic_hash`) and cannot be reversed, so the
    /// code we typed in ourselves is the only one we can use.
    pub(crate) fn handle_pake_msg(
        &mut self,
        from: String,
        topic: String,
        node_name: String,
        kind: &str,
        pake_bytes: &[u8],
        confirm: Option<&[u8]>,
        events: &mut Vec<SyncEvent>,
    ) -> Option<(String, Vec<u8>)> {
        let code = self.pake_pending_code.clone();
        if code.is_empty() {
            return None;
        }
        match kind {
            // We are Alice: generate the A-side and finish, but don't trust
            // the handshake yet - SPAKE2's finish() only proves the peer's
            // message was well-formed, not that they used the same code. Send
            // our confirmation blob along with "resp" and wait for Bob's
            // "confirm" before declaring HandshakeComplete.
            "init" => {
                let (msg_a, state_a) = pake::pake_initiate(&code).ok()?;
                let session_a = match pake::pake_finish(state_a, pake_bytes) {
                    Ok(s) => s,
                    Err(_) => {
                        info!("[PAKE] Handshake mismatch on 'init' from peer {}", from);
                        events.push(SyncEvent::HandshakeFailed {
                            reason: "PAKE mismatch".to_string(),
                        });
                        return None;
                    }
                };
                match pake::confirm_blob(&session_a) {
                    Ok(confirm_a) => {
                        self.pake_pending_alice = Some(session_a);
                        self.reply(topic, "resp", msg_a, confirm_a)
                    }
                    Err(e) => {
                        events.push(SyncEvent::HandshakeFailed {
                            reason: format!("Failed to build confirmation: {}", e),
                        });
                        None
                    }
                }
            }
            // We are Bob: finish with Alice's message, then verify her
            // confirmation blob before trusting the handshake. Only once that
            // verifies do we declare HandshakeComplete and send our own
            // confirmation back so Alice can too.
            "resp" => {
                let state_b = self.pake_pending.take()?;
                let session_b = pake::pake_finish(state_b, pake_bytes).ok();
                let confirmed = session_b
                    .as_ref()
                    .is_some_and(|s| confirm.is_some_and(|blob| pake::verify_confirm(s, blob)));
                if !confirmed {
                    info!("[PAKE] Handshake mismatch on 'resp' from peer {}", from);
                    events.push(SyncEvent::HandshakeFailed {
                        reason: "PAKE mismatch".to_string(),
                    });
                    return None;
                }
                info!("[PAKE] Handshake complete (resp) with peer {}", from);
                events.push(SyncEvent::HandshakeComplete {
                    peer_id: from,
                    peer_name: node_name,
                });
                let confirm_b = pake::confirm_blob(&session_b?).ok()?;
                self.reply(topic, "confirm", Vec::new(), confirm_b)
            }
            // We are Alice: Bob has (claimed to have) proven he derived the
            // same key as us.
            "confirm" => {
                let session_a = self.pake_pending_alice.take()?;
                if confirm.is_some_and(|blob| pake::verify_confirm(&session_a, blob)) {
                    info!("[PAKE] Handshake complete (confirm) with peer {}", from);
                    events.push(SyncEvent::HandshakeComplete {
                        peer_id: from,
                        peer_name: node_name,
                    });
                } else {
                    info!("[PAKE] Handshake mismatch on 'confirm' from peer {}", from);
                    events.push(SyncEvent::HandshakeFailed {
                        reason: "PAKE mismatch".to_string(),
                    });
                }
                None
            }
            _ => None,
        }
    }

    /// Serialise an outgoing handshake message for `topic`.
    fn reply(
        &self,
        topic: String,
        kind: &str,
        pake_bytes: Vec<u8>,
        confirm: Vec<u8>,
    ) -> Option<(String, Vec<u8>)> {
        let payload = p2p::PakeMsgPayload {
            kind: kind.to_string(),
            node_name: self.config.node_name.clone(),
            pake_bytes,
            confirm: Some(confirm),
        };
        serde_json::to_vec(&payload).ok().map(|data| (topic, data))
    }
}
