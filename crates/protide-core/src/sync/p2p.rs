#![cfg(feature = "p2p-sync")]

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use futures_util::StreamExt;
use libp2p::gossipsub;
use libp2p::kad;
use libp2p::{Multiaddr, PeerId, identity};

use log::{error, info, warn};

pub use super::p2p_swarm::PakeMsgPayload;
use super::p2p_swarm::{
    bootstrap_kademlia, build_swarm, handle_swarm_event, switch_own_pake_topic,
};
use super::p2p_topic::pake_topic;
use super::types::NodeId;

// ── Command channel ───────────────────────────────────────────────────────────

/// Commands from the SyncEngine to the libp2p event loop.
pub enum SwarmCmd {
    /// Publish bytes on a given gossipsub topic
    Publish(String, Vec<u8>),
    /// Publish bytes on the PAKE topic of a pairing code. The topic is
    /// derived on the swarm thread because the KDF is slow.
    PublishPake(String, Vec<u8>),
    /// Replace our own pairing code: derive its PAKE topic, move our
    /// subscription there and report it back as `P2PEvent::PakeTopic`
    SetPairingCode(String),
    /// Move CRDT sync to a new topic after joining a paired group
    SetSyncTopic(String),
    /// Publish a sealed CRDT entry on the current sync topic. It shares
    /// the command queue so it is never reordered across `SetSyncTopic`.
    PublishSync(Vec<u8>),
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Events from the P2P backend
#[derive(Debug, Clone)]
pub enum P2PEvent {
    /// A sealed CRDT entry arrived on the sync topic, still unopened
    SyncMsg(Vec<u8>),
    PeerJoined(PeerId),
    PeerLeft(PeerId),
    Error(String),
    /// A PAKE handshake message arrived on a `protide-pake-*` topic
    PakeMsg {
        from: PeerId,
        /// The full topic string (a `protide-pake-<hash>` topic; the hash is
        /// an Argon2id derivation of the pairing code, see `p2p_topic::pake_topic`)
        topic: String,
        payload: PakeMsgPayload,
    },
    /// Our own listen multiaddress - emitted once the swarm binds a port
    LocalAddr(String),
    /// The PAKE topic of our own pairing code, sent once the swarm thread
    /// has derived it and subscribed
    PakeTopic(String),
}

// ── P2PSync ───────────────────────────────────────────────────────────────────

/// Libp2p-based P2P sync backend.
pub struct P2PSync {
    peer_id: PeerId,
    _node_id: NodeId,
    _event_tx: Sender<P2PEvent>,
    event_rx: Receiver<P2PEvent>,
    known_peers: HashSet<PeerId>,
    /// Command channel for subscribe/publish operations on dynamic topics
    cmd_tx: Sender<SwarmCmd>,
}

impl P2PSync {
    /// Create and start a new P2P sync node. CRDT entries travel on
    /// `sync_topic`; handshakes for `pairing_code` arrive on its PAKE topic.
    pub fn start(
        node_id: NodeId,
        listen_port: Option<u16>,
        pairing_code: &str,
        sync_topic: String,
    ) -> Result<Self, String> {
        let (event_tx, event_rx) = mpsc::channel::<P2PEvent>();
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
                // The PAKE topic comes from a slow KDF, so derive it here
                // rather than on the caller's (UI) thread.
                let own_pake_topic =
                    (!pairing_code_owned.is_empty()).then(|| pake_topic(&pairing_code_owned));
                let mut crdt_topic_thread = gossipsub::IdentTopic::new(sync_topic);
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
                    let mut own_pake = None;
                    if let Some(topic) = own_pake_topic {
                        switch_own_pake_topic(
                            &mut swarm.behaviour_mut().gossipsub,
                            &mut own_pake,
                            topic,
                            &event_tx_thread,
                        );
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
                                SwarmCmd::PublishPake(code, data) => (pake_topic(&code), data),
                                SwarmCmd::SetPairingCode(code) => {
                                    switch_own_pake_topic(
                                        &mut swarm.behaviour_mut().gossipsub,
                                        &mut own_pake,
                                        pake_topic(&code),
                                        &event_tx_thread,
                                    );
                                    continue;
                                }
                                SwarmCmd::SetSyncTopic(topic) => {
                                    let gossipsub = &mut swarm.behaviour_mut().gossipsub;
                                    let _ = gossipsub.unsubscribe(&crdt_topic_thread);
                                    crdt_topic_thread = gossipsub::IdentTopic::new(topic);
                                    let _ = gossipsub.subscribe(&crdt_topic_thread);
                                    continue;
                                }
                                SwarmCmd::PublishSync(data) => {
                                    if let Err(e) = swarm
                                        .behaviour_mut()
                                        .gossipsub
                                        .publish(crdt_topic_thread.clone(), data)
                                    {
                                        warn!("[P2P] Sync publish failed: {}", e);
                                    }
                                    continue;
                                }
                            };
                            // Publishing subscribes first so the peer's
                            // reply on the same topic reaches us.
                            if !pake_topics.contains_key(&topic_str) {
                                let t = gossipsub::IdentTopic::new(&topic_str);
                                let _ = swarm.behaviour_mut().gossipsub.subscribe(&t);
                                pake_topics.insert(topic_str.clone(), t);
                            }
                            if let Some(t) = pake_topics.get(&topic_str)
                                && let Err(e) =
                                    swarm.behaviour_mut().gossipsub.publish(t.clone(), data)
                            {
                                warn!("[P2P] Publish failed: {}", e);
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

                        handle_swarm_event(&mut swarm, event, &event_tx_thread);
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
            cmd_tx,
        })
    }

    /// Publish a sealed CRDT entry on the current sync topic. Callers seal
    /// first (see `p2p_seal`); this never sees an entry in the clear.
    pub fn publish_sync(&self, data: Vec<u8>) {
        let _ = self.cmd_tx.send(SwarmCmd::PublishSync(data));
    }

    /// Subscribe to the PAKE topic for `code` and publish raw bytes on it.
    /// Returns at once; the swarm thread derives the topic.
    pub fn publish_on_pake_topic(&self, code: &str, data: Vec<u8>) {
        let _ = self
            .cmd_tx
            .send(SwarmCmd::PublishPake(code.to_string(), data));
    }

    /// Listen for handshakes on the PAKE topic of a new pairing code.
    /// Returns at once; the swarm thread derives the topic.
    pub fn set_pairing_code(&self, code: &str) {
        let _ = self.cmd_tx.send(SwarmCmd::SetPairingCode(code.to_string()));
    }

    /// Send and receive CRDT entries on `topic` from now on.
    pub fn set_sync_topic(&self, topic: String) {
        let _ = self.cmd_tx.send(SwarmCmd::SetSyncTopic(topic));
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
