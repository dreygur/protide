//! PAKE handshake state machine — split out of `engine.rs::poll` so the
//! security-critical three-step exchange can be driven directly in tests
//! without a libp2p swarm.
#![cfg(all(feature = "p2p-sync", feature = "pake-auth"))]

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::pake::Role;
use super::*;

/// Length of one guess budget window. Every answered init is one online
/// guess at the shown code.
const GUESS_WINDOW: Duration = Duration::from_secs(60);

/// Inits answered per peer in one window. A peer over it is ignored until
/// the window ends, so one peer cannot force rotations on honest users.
const MAX_GUESSES_PER_PEER: u32 = 3;

/// Inits answered across all peers in one window, however many PeerIds an
/// attacker makes up. The guess that reaches it replaces the code, and
/// further inits wait for the next window. It also bounds the per peer
/// table, since every tracked peer has spent a guess.
const MAX_GUESSES_PER_WINDOW: u32 = 10;

/// How long a pending responder session accepts its peer's "confirm".
/// Matches the UI's handshake timeout.
const PENDING_PAIRING_TIMEOUT: Duration = Duration::from_secs(10);

/// Most pending responder sessions held at once; the oldest is dropped to
/// make room, so no peer can fill the table and lock others out. Every
/// pending session cost a guess, so one window's budget always fits and
/// an honest session is never evicted by the guesses that follow it.
const MAX_PENDING_PAIRINGS: usize = MAX_GUESSES_PER_WINDOW as usize;

/// The responder's session while it waits for its peer's "confirm".
pub(super) struct PendingResponder {
    session: pake::PakeSession,
    since: Instant,
}

impl PendingResponder {
    fn is_live(&self) -> bool {
        self.since.elapsed() < PENDING_PAIRING_TIMEOUT
    }
}

/// Guesses answered in the current window, in total and per peer.
pub(super) struct GuessWindow {
    start: Instant,
    total: u32,
    per_peer: HashMap<String, u32>,
}

impl Default for GuessWindow {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            total: 0,
            per_peer: HashMap::new(),
        }
    }
}

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
            group: None,
        };
        serde_json::to_vec(&payload).map_err(|e| format!("Serialisation error: {}", e))
    }

    /// Replace the shown code so further guesses target a new one. The
    /// swarm derives and subscribes the new topic off this thread and
    /// reports it back; until then every init is ignored. Sessions already
    /// pending keep their keys, so peers whose init came first can still
    /// confirm.
    fn regenerate_pairing_code(&mut self) {
        let code = pake::generate_pairing_code();
        if let Some(ref p2p) = self.p2p_sync {
            p2p.set_pairing_code(&code);
        }
        self.config.pairing_code = Some(code);
        self.pake_topic = None;
    }

    /// Spend one guess for `peer`, or refuse it when that peer or all peers
    /// together are out of budget for the current window.
    fn take_guess(&mut self, peer: &str) -> bool {
        if self.guesses.start.elapsed() >= GUESS_WINDOW {
            self.guesses = GuessWindow::default();
        }
        let window = &mut self.guesses;
        let spent = window.per_peer.get(peer).copied().unwrap_or(0);
        if window.total >= MAX_GUESSES_PER_WINDOW || spent >= MAX_GUESSES_PER_PEER {
            return false;
        }
        window.per_peer.insert(peer.to_string(), spent + 1);
        window.total += 1;
        true
    }

    /// Hold `session` for `peer` until it confirms, replacing that peer's
    /// own earlier session and dropping expired ones, then the oldest when
    /// the table is full.
    fn hold_pending(&mut self, peer: String, session: pake::PakeSession) {
        let pending = &mut self.pending_responders;
        pending.retain(|_, p| p.is_live());
        if pending.len() >= MAX_PENDING_PAIRINGS && !pending.contains_key(&peer) {
            let oldest = pending
                .iter()
                .min_by_key(|(_, p)| p.since)
                .map(|(k, _)| k.clone());
            if let Some(oldest) = oldest {
                pending.remove(&oldest);
            }
        }
        let since = Instant::now();
        pending.insert(peer, PendingResponder { session, since });
    }

    /// Adopt the group secret the responder shared and move CRDT sync to
    /// the group's topic, so both peers hear each other's entries.
    fn join_group(&mut self, secret: GroupSecret) {
        self.config.group_secret = Some(secret);
        if let Some(ref p2p) = self.p2p_sync {
            p2p.set_sync_topic(self.sync_topic());
        }
    }

    /// Process one PAKE handshake message received on a `protide-pake-*`
    /// topic. Pushes UI events onto `events` and returns an optional
    /// `(topic, payload)` for the caller to publish back to the peer.
    ///
    /// The topic name is an Argon2id hash of the pairing code (see
    /// `p2p_topic::pake_topic`) and cannot be reversed, so an "init" is only answered
    /// when it arrives on the cached topic of the code we show, and "resp"
    /// or "confirm" only while a handshake we took part in is pending.
    pub(crate) fn handle_pake_msg(
        &mut self,
        from: String,
        topic: String,
        msg: p2p::PakeMsgPayload,
        events: &mut Vec<SyncEvent>,
    ) -> Option<(String, Vec<u8>)> {
        let (node_name, pake_bytes) = (msg.node_name, msg.pake_bytes.as_slice());
        let (confirm, group) = (msg.confirm.as_deref(), msg.group.as_deref());
        match msg.kind.as_str() {
            // We are Alice: generate the A-side and finish, but don't trust
            // the handshake yet - SPAKE2's finish() only proves the peer's
            // message was well-formed, not that they used the same code. Send
            // our confirmation blob along with "resp" and wait for Bob's
            // "confirm" before declaring HandshakeComplete.
            "init" => {
                if !self.accepting_pairing
                    || self.pake_topic.as_deref() != Some(topic.as_str())
                    || !self.take_guess(&from)
                {
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
                let confirm_a = pake::confirm_blob(&session_a, Role::Responder);
                self.hold_pending(from, session_a);
                if self.guesses.total >= MAX_GUESSES_PER_WINDOW {
                    self.regenerate_pairing_code();
                }
                self.reply(topic, "resp", msg_a, Some(confirm_a), None)
            }
            // We are Bob: finish with Alice's message, then verify her
            // confirmation blob before trusting the handshake. Only once that
            // verifies do we send our own confirmation, and keep the session
            // to open the group Alice releases once she verified it.
            "resp" => {
                let state_b = self.pake_pending.take()?;
                let session_b = pake::pake_finish(state_b, pake_bytes).ok()?;
                if !confirm
                    .is_some_and(|blob| pake::verify_confirm(&session_b, Role::Responder, blob))
                {
                    info!("[PAKE] Handshake mismatch on 'resp' from peer {}", from);
                    events.push(SyncEvent::HandshakeFailed {
                        reason: "PAKE mismatch".to_string(),
                    });
                    return None;
                }
                let confirm_b = pake::confirm_blob(&session_b, Role::Initiator);
                self.pending_initiator = Some((from, session_b));
                self.reply(topic, "confirm", Vec::new(), Some(confirm_b), None)
            }
            // We are Bob: Alice verified our confirm and released her group
            // secret, sealed. Only the peer whose "resp" we verified counts;
            // a seal that fails to open under that key was tampered with.
            "group" => {
                let (_, session) = self.pending_initiator.take_if(|(peer, _)| *peer == from)?;
                match group.map(|blob| pake::open_group_secret(&session, blob)) {
                    Some(None) => {
                        info!("[PAKE] Group secret from peer {} did not open", from);
                        events.push(SyncEvent::HandshakeFailed {
                            reason: "PAKE mismatch".to_string(),
                        });
                        return None;
                    }
                    Some(Some(secret)) => self.join_group(secret),
                    None => {}
                }
                info!("[PAKE] Handshake complete (group) with peer {}", from);
                events.push(SyncEvent::HandshakeComplete {
                    peer_id: from,
                    peer_name: node_name,
                });
                None
            }
            // We are Alice: Bob has (claimed to have) proven he derived the
            // same key as us. A confirm is only checked against the session
            // of the peer that sent it, while it is live and the window open.
            // Only a verified confirm releases our group secret, sealed.
            "confirm" => {
                if !self.accepting_pairing {
                    return None;
                }
                let pending = self.pending_responders.remove(&from)?;
                if !pending.is_live() {
                    return None;
                }
                if confirm.is_some_and(|blob| {
                    pake::verify_confirm(&pending.session, Role::Initiator, blob)
                }) {
                    info!("[PAKE] Handshake complete (confirm) with peer {}", from);
                    // Single use: a code that paired once never pairs again.
                    self.regenerate_pairing_code();
                    events.push(SyncEvent::HandshakeComplete {
                        peer_id: from,
                        peer_name: node_name,
                    });
                    let sealed = self
                        .config
                        .group_secret
                        .and_then(|s| pake::seal_group_secret(&pending.session, &s).ok());
                    return self.reply(topic, "group", Vec::new(), None, sealed);
                }
                info!("[PAKE] Handshake mismatch on 'confirm' from peer {}", from);
                events.push(SyncEvent::HandshakeFailed {
                    reason: "PAKE mismatch".to_string(),
                });
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
        confirm: Option<Vec<u8>>,
        group: Option<Vec<u8>>,
    ) -> Option<(String, Vec<u8>)> {
        let payload = p2p::PakeMsgPayload {
            kind: kind.to_string(),
            node_name: self.config.node_name.clone(),
            pake_bytes,
            confirm,
            group,
        };
        serde_json::to_vec(&payload).ok().map(|data| (topic, data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::p2p_topic::pake_topic;

    const CODE: &str = "amber-otter-river-tiger-482";
    const OTHER: &str = "amber-otter-river-tiger-483";

    fn engine(name: &str, pairing_code: Option<&str>) -> SyncEngine {
        SyncEngine::new(SyncConfig {
            node_name: name.to_string(),
            pairing_code: pairing_code.map(str::to_string),
            group_secret: Some(pake::generate_group_secret()),
            ..Default::default()
        })
    }

    /// Alice showing `code` in an open pairing window, with its topic
    /// cached as the swarm reports it.
    fn showing(code: &str) -> SyncEngine {
        let mut alice = engine("alice", Some(code));
        alice.pake_topic = Some(pake_topic(code));
        alice.set_accepting_pairing(true);
        alice
    }

    /// Bob's "init" for `code`, published on that code's topic.
    fn init_for(code: &str) -> (String, Vec<u8>) {
        let init = engine("bob", None).start_handshake(code).unwrap();
        (pake_topic(code), init)
    }

    /// Feed one published payload into `to` as if gossipsub delivered it.
    fn deliver(
        to: &mut SyncEngine,
        from: &str,
        (topic, data): (String, Vec<u8>),
        events: &mut Vec<SyncEvent>,
    ) -> Option<(String, Vec<u8>)> {
        let msg = serde_json::from_slice(&data).unwrap();
        to.handle_pake_msg(from.to_string(), topic, msg, events)
    }

    /// Bob types `typed` and his init lands on Alice's cached topic.
    /// Messages are pumped by hand until neither side replies. Returns both
    /// sides' events and every message that went over the wire.
    fn pair_with(
        alice: &mut SyncEngine,
        bob: &mut SyncEngine,
        typed: &str,
    ) -> (Vec<SyncEvent>, Vec<SyncEvent>, Vec<Vec<u8>>) {
        let (mut events_a, mut events_b, mut wire) = (Vec::new(), Vec::new(), Vec::new());
        let init = bob.start_handshake(typed).unwrap();
        let mut next = alice.pake_topic.clone().map(|topic| (topic, init));
        let mut to_alice = true;
        while let Some(msg) = next.take() {
            wire.push(msg.1.clone());
            next = if to_alice {
                deliver(alice, "bob", msg, &mut events_a)
            } else {
                deliver(bob, "alice", msg, &mut events_b)
            };
            to_alice = !to_alice;
        }
        (events_a, events_b, wire)
    }

    fn pair(alice: &mut SyncEngine, typed: &str) -> (Vec<SyncEvent>, Vec<SyncEvent>) {
        let (events_a, events_b, _) = pair_with(alice, &mut engine("bob", None), typed);
        (events_a, events_b)
    }

    fn paired_with(events: &[SyncEvent]) -> Option<&str> {
        events.iter().find_map(|e| match e {
            SyncEvent::HandshakeComplete { peer_name, .. } => Some(peer_name.as_str()),
            _ => None,
        })
    }

    /// True when `alice` neither answers `msg` from `from` nor reports
    /// anything.
    fn ignores(alice: &mut SyncEngine, msg: (String, Vec<u8>), from: &str) -> bool {
        let mut events = Vec::new();
        deliver(alice, from, msg, &mut events).is_none() && events.is_empty()
    }

    /// Run Bob's side with the right code up to his "confirm", which is
    /// returned undelivered while Alice holds his pending session.
    fn bob_confirm(alice: &mut SyncEngine) -> (String, Vec<u8>) {
        let mut bob = engine("bob", None);
        let mut events = Vec::new();
        let init = (
            alice.pake_topic.clone().unwrap(),
            bob.start_handshake(CODE).unwrap(),
        );
        let resp = deliver(alice, "bob", init, &mut events).unwrap();
        deliver(&mut bob, "alice", resp, &mut events).unwrap()
    }

    // REGRESSION: the side showing its code ignored every incoming init,
    // because only the side that typed a code had a pending code set.
    // Each side only reports HandshakeComplete after verifying the other's
    // key confirmation, so both completing proves equal session keys.
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
        assert!(ignores(&mut alice, init_for(CODE), "bob"));
    }

    /// The topic is compared against the cached string only: with nothing
    /// cached yet the init is dropped rather than running the KDF.
    #[test]
    fn an_init_before_the_topic_is_cached_is_ignored() {
        let mut alice = engine("alice", Some(CODE));
        assert!(ignores(&mut alice, init_for(CODE), "bob"));
    }

    #[test]
    fn an_init_for_a_code_we_do_not_show_is_ignored() {
        assert!(ignores(&mut showing(CODE), init_for(OTHER), "bob"));
    }

    #[test]
    fn a_wrong_code_never_pairs() {
        let (events_a, events_b) = pair(&mut showing(CODE), OTHER);
        assert_eq!(paired_with(&events_a), None);
        assert_eq!(paired_with(&events_b), None);
    }

    // REGRESSION: both sides sent the same kind of confirmation, so an
    // attacker without the code could send Alice's own confirmation from
    // "resp" straight back as "confirm" and complete the handshake.
    #[test]
    fn reflecting_the_responders_confirm_back_does_not_pair() {
        let mut alice = showing(CODE);
        let mut events = Vec::new();
        let init = (alice.pake_topic.clone().unwrap(), init_for(OTHER).1);
        let (topic, resp) = deliver(&mut alice, "mallory", init, &mut events).unwrap();
        let resp: p2p::PakeMsgPayload = serde_json::from_slice(&resp).unwrap();
        let reflected = p2p::PakeMsgPayload {
            kind: "confirm".to_string(),
            node_name: "mallory".to_string(),
            pake_bytes: Vec::new(),
            confirm: resp.confirm,
            group: None,
        };
        let reflected = serde_json::to_vec(&reflected).unwrap();
        let released = deliver(&mut alice, "mallory", (topic, reflected), &mut events);
        assert!(released.is_none(), "a reflected confirm released the group");
        assert_eq!(paired_with(&events), None);
    }

    // REGRESSION: any peer's "confirm" was checked against the pending
    // session, and a bad one cleared it for the real peer too.
    #[test]
    fn a_confirm_from_another_peer_is_ignored() {
        let mut alice = showing(CODE);
        let confirm = bob_confirm(&mut alice);
        assert!(ignores(&mut alice, confirm.clone(), "mallory"));
        let mut events = Vec::new();
        deliver(&mut alice, "bob", confirm, &mut events);
        assert_eq!(paired_with(&events), Some("bob"));
    }

    #[test]
    fn an_expired_sessions_confirm_is_rejected() {
        let mut alice = showing(CODE);
        let confirm = bob_confirm(&mut alice);
        for pending in alice.pending_responders.values_mut() {
            pending.since -= PENDING_PAIRING_TIMEOUT;
        }
        assert!(ignores(&mut alice, confirm, "bob"));
    }

    #[test]
    fn a_full_pending_table_drops_its_oldest_session() {
        let mut alice = showing(CODE);
        for peer in 0..=MAX_PENDING_PAIRINGS {
            let session = pake::PakeSession {
                shared_key: [0; 32],
            };
            alice.hold_pending(peer.to_string(), session);
        }
        assert_eq!(alice.pending_responders.len(), MAX_PENDING_PAIRINGS);
        assert!(!alice.pending_responders.contains_key("0"));
    }

    #[test]
    fn closing_the_window_clears_pending_sessions() {
        let mut alice = showing(CODE);
        let confirm = bob_confirm(&mut alice);
        alice.set_accepting_pairing(false);
        assert!(alice.pending_responders.is_empty());
        alice.set_accepting_pairing(true);
        assert!(ignores(&mut alice, confirm, "bob"));
    }

    #[test]
    fn a_code_pairs_only_once() {
        let mut alice = showing(CODE);
        let (events_a, _) = pair(&mut alice, CODE);
        assert_eq!(paired_with(&events_a), Some("bob"));
        assert_ne!(alice.config.pairing_code.as_deref(), Some(CODE));
        assert!(alice.pake_topic.is_none());
        assert!(ignores(&mut alice, init_for(CODE), "bob"));
    }

    // REGRESSION: one pending slot that ignored new inits let any peer
    // lock out pairing by sending an init every timeout period.
    #[test]
    fn an_attackers_pending_init_does_not_block_another_peer() {
        let mut alice = showing(CODE);
        let mut events = Vec::new();
        let attack = (alice.pake_topic.clone().unwrap(), init_for(OTHER).1);
        assert!(deliver(&mut alice, "mallory", attack, &mut events).is_some());
        let (events_a, _) = pair(&mut alice, CODE);
        assert_eq!(paired_with(&events_a), Some("bob"));
    }

    /// Gossipsub hands an entry only to nodes subscribed to its topic, so
    /// `to` merges it only when it listens on the sender's sync topic.
    fn gossip(from: &SyncEngine, to: &mut SyncEngine, entry: CrdtEntry) -> bool {
        from.sync_topic() == to.sync_topic()
            && matches!(to.store.merge_remote(entry), MergeResult::Accepted(_))
    }

    // REGRESSION: each node's sync topic came from its own pairing code,
    // so two peers that had just paired never heard each other's entries.
    #[test]
    fn paired_peers_share_a_sync_topic_and_entries() {
        let (mut alice, mut bob) = (showing(CODE), engine("bob", None));
        let alice_secret = alice.config.group_secret;
        let entry = alice.apply_local_change(DataType::Request, "{}".into());
        assert!(
            !gossip(&alice, &mut bob, entry.clone()),
            "unpaired peers share"
        );

        let (events_a, _, _) = pair_with(&mut alice, &mut bob, CODE);
        assert_eq!(paired_with(&events_a), Some("bob"));
        assert_eq!(bob.config.group_secret, alice_secret);
        assert_eq!(bob.sync_topic(), alice.sync_topic());
        assert!(gossip(&alice, &mut bob, entry));
    }

    #[test]
    fn the_group_secret_never_travels_in_the_clear() {
        let (mut alice, mut bob) = (showing(CODE), engine("bob", None));
        let secret = alice.config.group_secret.unwrap().0;
        let (_, _, wire) = pair_with(&mut alice, &mut bob, CODE);
        let msgs: Vec<p2p::PakeMsgPayload> = wire
            .iter()
            .map(|d| serde_json::from_slice(d).unwrap())
            .collect();
        let kinds: Vec<&str> = msgs.iter().map(|m| m.kind.as_str()).collect();
        assert_eq!(kinds, ["init", "resp", "confirm", "group"]);
        // Only the last message, sent after Bob's confirm verified, carries
        // the group, and only sealed.
        assert!(msgs[..3].iter().all(|m| m.group.is_none()));
        assert!(msgs[3].group.is_some());
        for msg in msgs {
            let fields = [Some(msg.pake_bytes), msg.confirm, msg.group];
            for bytes in fields.iter().flatten() {
                assert!(!bytes.windows(32).any(|w| w == secret));
            }
        }
    }

    #[test]
    fn a_group_from_another_peer_is_ignored() {
        let (mut alice, mut bob) = (showing(CODE), engine("bob", None));
        let mut events = Vec::new();
        let init = (
            alice.pake_topic.clone().unwrap(),
            bob.start_handshake(CODE).unwrap(),
        );
        let resp = deliver(&mut alice, "bob", init, &mut events).unwrap();
        let confirm = deliver(&mut bob, "alice", resp, &mut events).unwrap();
        let group = deliver(&mut alice, "bob", confirm, &mut events).unwrap();
        let mut events_b = Vec::new();
        assert!(deliver(&mut bob, "mallory", group.clone(), &mut events_b).is_none());
        assert_eq!(paired_with(&events_b), None);
        deliver(&mut bob, "alice", group, &mut events_b);
        assert_eq!(paired_with(&events_b), Some("alice"));
        assert_eq!(bob.sync_topic(), alice.sync_topic());
    }

    #[test]
    fn a_wrong_code_shares_no_group() {
        let (mut alice, mut bob) = (showing(CODE), engine("bob", None));
        let bob_secret = bob.config.group_secret;
        pair_with(&mut alice, &mut bob, OTHER);
        assert_eq!(bob.config.group_secret, bob_secret);
        assert_ne!(bob.sync_topic(), alice.sync_topic());
    }

    /// Send `count` wrong code inits to Alice's current topic from `peer`
    /// and return how many she answered.
    fn guesses(alice: &mut SyncEngine, peer: &str, count: usize) -> usize {
        let mut events = Vec::new();
        (0..count)
            .filter(|_| {
                let topic = alice.pake_topic.clone().unwrap_or_default();
                let init = (topic, engine(peer, None).start_handshake(OTHER).unwrap());
                deliver(alice, peer, init, &mut events).is_some()
            })
            .count()
    }

    // REGRESSION: every answered init counted toward one shared limit that
    // rotated the code, so one peer spamming inits kept replacing the code
    // under an honest user.
    #[test]
    fn one_spamming_peer_cannot_block_an_honest_peer() {
        let mut alice = showing(CODE);
        let answered = guesses(&mut alice, "mallory", 20);
        assert_eq!(answered, MAX_GUESSES_PER_PEER as usize);
        assert_eq!(alice.config.pairing_code.as_deref(), Some(CODE));
        let (events_a, _) = pair(&mut alice, CODE);
        assert_eq!(paired_with(&events_a), Some("bob"));
    }

    /// Many PeerIds together get MAX_GUESSES_PER_WINDOW answers per window.
    /// The last one replaces the code; the old code then no longer pairs,
    /// and the new one does once the swarm reports its topic and a new
    /// window starts.
    #[test]
    fn the_global_guess_rate_stays_bounded() {
        let mut alice = showing(CODE);
        let answered: usize = (0..30)
            .map(|i| guesses(&mut alice, &format!("p{i}"), 1))
            .sum();
        assert_eq!(answered, MAX_GUESSES_PER_WINDOW as usize);
        let new_code = alice.config.pairing_code.clone().unwrap();
        assert_ne!(new_code, CODE);

        alice.pake_topic = Some(pake_topic(&new_code));
        assert_eq!(guesses(&mut alice, "late", 1), 0, "window budget spent");
        alice.guesses.start -= GUESS_WINDOW;
        assert!(ignores(&mut alice, init_for(CODE), "bob"));
        let (events_a, _) = pair(&mut alice, &new_code);
        assert_eq!(paired_with(&events_a), Some("bob"));
    }

    #[test]
    fn an_honest_pending_session_survives_a_rotation() {
        let mut alice = showing(CODE);
        let confirm = bob_confirm(&mut alice);
        let others = (MAX_GUESSES_PER_WINDOW - 1) as usize;
        let answered: usize = (0..others)
            .map(|i| guesses(&mut alice, &format!("p{i}"), 1))
            .sum();
        assert_eq!(answered, others);
        assert_ne!(alice.config.pairing_code.as_deref(), Some(CODE), "rotated");
        let mut events = Vec::new();
        deliver(&mut alice, "bob", confirm, &mut events);
        assert_eq!(paired_with(&events), Some("bob"));
    }
}
