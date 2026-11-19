//! PAKE handshake state machine — split out of `engine.rs::poll` so the
//! security-critical three-step exchange can be driven directly in tests
//! without a libp2p swarm.
#![cfg(all(feature = "p2p-sync", feature = "pake-auth"))]

use super::*;

/// Inits answered for one shown code without a verified "confirm" before
/// the code is replaced, so an online guesser gets this many tries per code.
const MAX_FAILED_PAIRINGS: u32 = 5;

impl SyncEngine {
    /// Begin a handshake as Bob for `code`: store the B side state and
    /// return the serialised "init" payload to publish on the code's topic.
    pub(crate) fn start_handshake(&mut self, code: &str) -> Result<Vec<u8>, String> {
        let (msg_b, state_b) = pake::pake_respond(code)?;
        self.pake_pending = Some(state_b);
        let payload = p2p::PakeMsgPayload {
            kind: "init".to_string(),
            node_name: self.config.node_name.clone(),
            pake_bytes: msg_b,
            confirm: None,
        };
        serde_json::to_vec(&payload).map_err(|e| format!("Serialisation error: {}", e))
    }

    /// Replace the shown code so further guesses target a new one. The
    /// swarm derives and subscribes the new topic off this thread and
    /// reports it back; until then every init is ignored.
    fn regenerate_pairing_code(&mut self) {
        let code = pake::generate_pairing_code();
        if let Some(ref p2p) = self.p2p_sync {
            p2p.set_pairing_code(&code);
        }
        self.config.pairing_code = Some(code);
        self.pake_topic = None;
        self.failed_pairings = 0;
    }

    /// Process one PAKE handshake message received on a `protide-pake-*`
    /// topic. Pushes UI events onto `events` and returns an optional
    /// `(topic, payload)` for the caller to publish back to the peer.
    ///
    /// The topic name is an Argon2id hash of the pairing code (see
    /// `PairingTopics`) and cannot be reversed, so an "init" is only answered
    /// when it arrives on the cached topic of the code we show, and "resp"
    /// or "confirm" only while a handshake we took part in is pending.
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
        match kind {
            // We are Alice: generate the A-side and finish, but don't trust
            // the handshake yet - SPAKE2's finish() only proves the peer's
            // message was well-formed, not that they used the same code. Send
            // our confirmation blob along with "resp" and wait for Bob's
            // "confirm" before declaring HandshakeComplete.
            "init" => {
                if !self.accepting_pairing || self.pake_topic.as_deref() != Some(topic.as_str()) {
                    return None;
                }
                let code = self.config.pairing_code.clone()?;
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
                        // Counted until a verified "confirm" resets it: a
                        // guesser who learns nothing never sends one.
                        self.failed_pairings += 1;
                        if self.failed_pairings >= MAX_FAILED_PAIRINGS {
                            self.regenerate_pairing_code();
                        }
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
                    self.failed_pairings = 0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::p2p_topic::PairingTopics;

    const CODE: &str = "amber-otter-river-tiger-482";
    const OTHER: &str = "amber-otter-river-tiger-483";

    fn engine(name: &str, pairing_code: Option<&str>) -> SyncEngine {
        SyncEngine::new(SyncConfig {
            node_name: name.to_string(),
            pairing_code: pairing_code.map(str::to_string),
            ..Default::default()
        })
    }

    /// Alice showing `code` in an open pairing window, with its topic
    /// cached as the swarm reports it.
    fn showing(code: &str) -> SyncEngine {
        let mut alice = engine("alice", Some(code));
        alice.pake_topic = Some(PairingTopics::derive(code).pake);
        alice.set_accepting_pairing(true);
        alice
    }

    /// Bob's "init" for `code`, published on that code's topic.
    fn init_for(code: &str) -> (String, Vec<u8>) {
        let init = engine("bob", None).start_handshake(code).unwrap();
        (PairingTopics::derive(code).pake, init)
    }

    /// Feed one published payload into `to` as if gossipsub delivered it.
    fn deliver(
        to: &mut SyncEngine,
        from: &str,
        (topic, data): (String, Vec<u8>),
        events: &mut Vec<SyncEvent>,
    ) -> Option<(String, Vec<u8>)> {
        let msg: p2p::PakeMsgPayload = serde_json::from_slice(&data).unwrap();
        to.handle_pake_msg(
            from.to_string(),
            topic,
            msg.node_name,
            &msg.kind,
            &msg.pake_bytes,
            msg.confirm.as_deref(),
            events,
        )
    }

    /// Bob types `typed` and his init lands on Alice's cached topic.
    /// Messages are pumped by hand until neither side replies.
    fn pair(alice: &mut SyncEngine, typed: &str) -> (Vec<SyncEvent>, Vec<SyncEvent>) {
        let mut bob = engine("bob", None);
        let (mut events_a, mut events_b) = (Vec::new(), Vec::new());
        let init = bob.start_handshake(typed).unwrap();
        let mut next = alice.pake_topic.clone().map(|topic| (topic, init));
        let mut to_alice = true;
        while let Some(msg) = next.take() {
            next = if to_alice {
                deliver(alice, "bob", msg, &mut events_a)
            } else {
                deliver(&mut bob, "alice", msg, &mut events_b)
            };
            to_alice = !to_alice;
        }
        (events_a, events_b)
    }

    fn paired_with(events: &[SyncEvent]) -> Option<&str> {
        events.iter().find_map(|e| match e {
            SyncEvent::HandshakeComplete { peer_name, .. } => Some(peer_name.as_str()),
            _ => None,
        })
    }

    /// True when `alice` neither answers `init` nor reports anything.
    fn ignores(alice: &mut SyncEngine, init: (String, Vec<u8>)) -> bool {
        let mut events = Vec::new();
        deliver(alice, "bob", init, &mut events).is_none() && events.is_empty()
    }

    // REGRESSION: the side showing its code ignored every incoming init,
    // because only the side that typed a code had a pending code set.
    // Each side only reports HandshakeComplete after verifying the other's
    // AEAD confirmation blob, so both completing proves equal session keys.
    #[test]
    fn the_side_showing_its_code_pairs_with_the_side_typing_it() {
        let (events_a, events_b) = pair(&mut showing(CODE), CODE);
        assert_eq!(paired_with(&events_a), Some("bob"));
        assert_eq!(paired_with(&events_b), Some("alice"));
    }

    #[test]
    fn an_init_is_ignored_while_the_pairing_window_is_closed() {
        let mut alice = showing(CODE);
        alice.set_accepting_pairing(false);
        assert!(ignores(&mut alice, init_for(CODE)));
    }

    /// Each answered init is one online guess, so the shown code is
    /// replaced after MAX_FAILED_PAIRINGS of them. The old code then no
    /// longer pairs, and the new one does once the swarm reports its topic.
    #[test]
    fn the_code_changes_after_too_many_failed_pairings() {
        let mut alice = showing(CODE);
        for _ in 1..MAX_FAILED_PAIRINGS {
            pair(&mut alice, OTHER);
        }
        assert_eq!(alice.config.pairing_code.as_deref(), Some(CODE));
        let (events_a, events_b) = pair(&mut alice, OTHER);
        assert_eq!(paired_with(&events_a), None);
        assert_eq!(paired_with(&events_b), None);

        let new_code = alice.config.pairing_code.clone().unwrap();
        assert_ne!(new_code, CODE);
        assert!(ignores(&mut alice, init_for(CODE)));

        alice.pake_topic = Some(PairingTopics::derive(&new_code).pake);
        let (events_a, _) = pair(&mut alice, &new_code);
        assert_eq!(paired_with(&events_a), Some("bob"));
    }

    /// The topic is compared against the cached string only: with nothing
    /// cached yet the init is dropped rather than running the KDF.
    #[test]
    fn an_init_before_the_topic_is_cached_is_ignored() {
        let mut alice = engine("alice", Some(CODE));
        assert!(ignores(&mut alice, init_for(CODE)));
    }

    #[test]
    fn an_init_for_a_code_we_do_not_show_is_ignored() {
        assert!(ignores(&mut showing(CODE), init_for(OTHER)));
    }

    #[test]
    fn a_wrong_code_never_pairs() {
        let (events_a, events_b) = pair(&mut showing(CODE), OTHER);
        assert_eq!(paired_with(&events_a), None);
        assert_eq!(paired_with(&events_b), None);
    }
}
