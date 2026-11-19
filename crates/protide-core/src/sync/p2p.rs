#![cfg(feature = "p2p-sync")]

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use futures_util::StreamExt;
use libp2p::gossipsub;
use libp2p::kad;
use libp2p::{Multiaddr, PeerId, identity};
use serde::{Deserialize, Serialize};

use log::{error, info, warn};

use super::p2p_swarm::{bootstrap_kademlia, build_swarm, handle_swarm_event};
use super::p2p_topic::{GOSSIP_TOPIC, PairingTopics};
use super::types::{CrdtEntry, NodeId};

// ── Wire protocol ─────────────────────────────────────────────────────────────

/// Serialised payload for PAKE handshake messages over gossipsub
#[derive(Serialize, Deserialize)]
pub struct PakeMsgPayload {
    /// "init" (Bob → Alice), "resp" (Alice → Bob), or "confirm" (Bob → Alice,
    /// final mutual key-confirmation step)
    pub kind: String,
    /// Display name of the sender
    pub node_name: String,
    /// Raw SPAKE2 public-key bytes (unused/empty for "confirm")
    pub pake_bytes: Vec<u8>,
    /// AEAD key-confirmation blob (see `pake::confirm_blob`), present on
    /// "resp" and "confirm" messages once each side has derived a session
    /// key. Proves the sender derived the same shared key as us - SPAKE2's
    /// `finish()` alone does not guarantee that.
    #[serde(default)]
    pub confirm: Option<Vec<u8>>,
}

// ── Command channel ───────────────────────────────────────────────────────────

/// Commands from the SyncEngine to the libp2p event loop.
pub enum SwarmCmd {
    /// Publish bytes on a given gossipsub topic
    Publish(String, Vec<u8>),
    /// Publish bytes on the PAKE topic of a pairing code. The topic is
    /// derived on the swarm thread because the KDF is slow.
    PublishPake(String, Vec<u8>),
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Events from the P2P backend
#[derive(Debug, Clone)]
pub enum P2PEvent {
    EntryReceived(CrdtEntry),
    PeerJoined(PeerId),
    PeerLeft(PeerId),
    Error(String),
    /// A PAKE handshake message arrived on a `protide-pake-*` topic
    PakeMsg {
        from: PeerId,
        /// The full topic string (a `protide-pake-<hash>` topic; the hash is
        /// an Argon2id derivation of the pairing code, see `PairingTopics`)
        topic: String,
        node_name: String,
        kind: String,
        pake_bytes: Vec<u8>,
        /// AEAD key-confirmation blob, see `PakeMsgPayload::confirm`
        confirm: Option<Vec<u8>>,
    },
    /// Our own listen multiaddress - emitted once the swarm binds a port
    LocalAddr(String),
}

// ── P2PSync ───────────────────────────────────────────────────────────────────

/// Libp2p-based P2P sync backend.
pub struct P2PSync {
    peer_id: PeerId,
    _node_id: NodeId,
    _event_tx: Sender<P2PEvent>,
    event_rx: Receiver<P2PEvent>,
    known_peers: HashSet<PeerId>,
    /// Outgoing CRDT broadcast channel
    broadcast_tx: Sender<CrdtEntry>,
    /// Command channel for subscribe/publish operations on dynamic topics
    cmd_tx: Sender<SwarmCmd>,
}

impl P2PSync {
    /// Create and start a new P2P sync node.
    /// `pairing_code` scopes gossip topics so only peers with the same code connect.
    pub fn start(
        node_id: NodeId,
        listen_port: Option<u16>,
        pairing_code: &str,
    ) -> Result<Self, String> {
        let (event_tx, event_rx) = mpsc::channel::<P2PEvent>();
        let (broadcast_tx, broadcast_rx) = mpsc::channel::<CrdtEntry>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<SwarmCmd>();

        // Generate keypair here so peer_id is known before the swarm thread starts.
        // Using with_existing_identity fixes the prior bug where self.peer_id came from
        // a different keypair than the one actually used by the swarm.
        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());

        info!("[P2P] Node starting. Local PeerID: {}", peer_id);
        info!(
            "[P2P] Requested listen addr: /ip4/0.0.0.0/tcp/{}",
            listen_port.unwrap_or(0)
        );

        let pairing_code_owned = pairing_code.to_string();
        let event_tx_thread = event_tx.clone();

        // Spawn a dedicated OS thread with its own current_thread Tokio runtime.
        // GPUI's main thread has no Tokio reactor, so tokio::spawn() panics there.
        // mdns::tokio::Behaviour and SwarmBuilder::with_tokio() both need a live reactor,
        // so all libp2p construction happens inside block_on.
        std::thread::Builder::new()
            .name("protide-p2p".into())
            .spawn(move || {
                // Topics come from a slow KDF, so derive them here rather
                // than on the caller's (UI) thread.
                let topics = (!pairing_code_owned.is_empty())
                    .then(|| PairingTopics::derive(&pairing_code_owned));
                let crdt_topic_thread = gossipsub::IdentTopic::new(
                    topics.as_ref().map_or(GOSSIP_TOPIC, |t| t.crdt.as_str()),
                );
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        error!("[P2P] Failed to build Tokio runtime: {}", e);
                        return;
                    }
                };

                rt.block_on(async move {
                    let build = build_swarm(keypair);

                    let mut swarm = match build {
                        Ok(s) => s,
                        Err(e) => {
                            error!("[P2P] Swarm setup failed: {}", e);
                            return;
                        }
                    };

                    if let Err(e) = swarm
                        .behaviour_mut()
                        .gossipsub
                        .subscribe(&crdt_topic_thread)
                    {
                        error!("[P2P] Failed to subscribe to CRDT topic: {:?}", e);
                        return;
                    }

                    // Also subscribe to our own PAKE topic so we hear handshake requests
                    if let Some(ref topics) = topics {
                        let pake_topic = gossipsub::IdentTopic::new(&topics.pake);
                        let _ = swarm.behaviour_mut().gossipsub.subscribe(&pake_topic);
                    }

                    // Listen on TCP
                    let tcp_addr: Multiaddr =
                        format!("/ip4/0.0.0.0/tcp/{}", listen_port.unwrap_or(0))
                            .parse()
                            .expect("valid tcp addr");
                    if let Err(e) = swarm.listen_on(tcp_addr) {
                        error!("[P2P] TCP listen_on failed: {}", e);
                        return;
                    }

                    // Listen on QUIC/UDP (port 0 = OS-assigned; better NAT traversal than TCP)
                    let quic_addr: Multiaddr = "/ip4/0.0.0.0/udp/0/quic-v1"
                        .parse()
                        .expect("valid quic addr");
                    if let Err(e) = swarm.listen_on(quic_addr) {
                        warn!("[P2P] QUIC listen_on failed (TCP still active): {}", e);
                    }

                    // Kademlia: server mode makes us discoverable to remote peers
                    swarm
                        .behaviour_mut()
                        .kademlia
                        .set_mode(Some(kad::Mode::Server));

                    bootstrap_kademlia(&mut swarm);

                    let mut pake_topics: HashMap<String, gossipsub::IdentTopic> = HashMap::new();

                    'run: loop {
                        // Drain outgoing CRDT broadcasts
                        while let Ok(entry) = broadcast_rx.try_recv() {
                            if let Ok(data) = serde_json::to_vec(&entry) {
                                let _ = swarm
                                    .behaviour_mut()
                                    .gossipsub
                                    .publish(crdt_topic_thread.clone(), data);
                            }
                        }

                        // Drain commands (subscribe / publish on dynamic topics)
                        loop {
                            let cmd = match cmd_rx.try_recv() {
                                Ok(cmd) => cmd,
                                Err(mpsc::TryRecvError::Empty) => break,
                                // P2PSync was dropped: drop the swarm so it
                                // stops listening.
                                Err(mpsc::TryRecvError::Disconnected) => break 'run,
                            };
                            let (topic_str, data) = match cmd {
                                SwarmCmd::Publish(topic_str, data) => (topic_str, data),
                                SwarmCmd::PublishPake(code, data) => {
                                    (PairingTopics::derive(&code).pake, data)
                                }
                            };
                            // Publishing subscribes first so the peer's
                            // reply on the same topic reaches us.
                            if !pake_topics.contains_key(&topic_str) {
                                let t = gossipsub::IdentTopic::new(&topic_str);
                                let _ = swarm.behaviour_mut().gossipsub.subscribe(&t);
                                pake_topics.insert(topic_str.clone(), t);
                            }
                            if let Some(t) = pake_topics.get(&topic_str) {
                                let _ = swarm.behaviour_mut().gossipsub.publish(t.clone(), data);
                            }
                        }

                        // Wait up to 200ms for a swarm event before re-checking the queues.
                        // 200ms balances responsiveness vs CPU spin; the OS wakes us immediately
                        // when a real network packet arrives.
                        let result =
                            tokio::time::timeout(Duration::from_millis(200), swarm.next()).await;

                        let event = match result {
                            Ok(Some(ev)) => ev,
                            Ok(None) => break,
                            Err(_) => continue,
                        };

                        handle_swarm_event(event, &event_tx_thread);
                    }
                });
            })
            .map_err(|e| format!("Failed to spawn P2P thread: {}", e))?;

        Ok(Self {
            peer_id,
            _node_id: node_id,
            _event_tx: event_tx,
            event_rx,
            known_peers: HashSet::new(),
            broadcast_tx,
            cmd_tx,
        })
    }

    /// Broadcast a CRDT entry via Gossipsub.
    pub fn broadcast_entry(&mut self, entry: &CrdtEntry) -> Result<(), String> {
        self.broadcast_tx
            .send(entry.clone())
            .map_err(|e| format!("Broadcast channel error: {}", e))
    }

    /// Subscribe to the PAKE topic for `code` and publish raw bytes on it.
    /// Returns at once; the swarm thread derives the topic.
    pub fn publish_on_pake_topic(&self, code: &str, data: Vec<u8>) {
        let _ = self
            .cmd_tx
            .send(SwarmCmd::PublishPake(code.to_string(), data));
    }

    /// Publish raw bytes on an arbitrary topic (used for PAKE response).
    pub fn publish_on_topic(&self, topic: &str, data: Vec<u8>) {
        let _ = self.cmd_tx.send(SwarmCmd::Publish(topic.to_string(), data));
    }

    /// Poll for P2P events (non-blocking).
    pub fn poll_events(&self) -> Vec<P2PEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.event_rx.try_recv() {
            events.push(evt);
        }
        events
    }

    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    pub fn known_peers(&self) -> &HashSet<PeerId> {
        &self.known_peers
    }
}
