#![cfg(feature = "p2p-sync")]

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use futures_util::StreamExt;
use libp2p::gossipsub;
use libp2p::identify;
use libp2p::kad::{self, store::MemoryStore};
use libp2p::mdns;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{identity, Multiaddr, PeerId, SwarmBuilder};
use serde::{Deserialize, Serialize};

use super::types::{CrdtEntry, NodeId};

const GOSSIP_TOPIC: &str = "protide-crdt";
const PAKE_TOPIC_PREFIX: &str = "protide-pake-";

// ── Wire protocol ─────────────────────────────────────────────────────────────

/// Serialised payload for PAKE handshake messages over gossipsub
#[derive(Serialize, Deserialize)]
pub struct PakeMsgPayload {
    /// "init" (Bob → Alice) or "resp" (Alice → Bob)
    pub kind: String,
    /// Display name of the sender
    pub node_name: String,
    /// Raw SPAKE2 public-key bytes
    pub pake_bytes: Vec<u8>,
}

// ── Command channel ───────────────────────────────────────────────────────────

/// Commands from the SyncEngine to the libp2p event loop.
pub enum SwarmCmd {
    /// Subscribe to a new gossipsub topic (e.g. the PAKE topic for a peer's code)
    Subscribe(String),
    /// Publish bytes on a given gossipsub topic
    Publish(String, Vec<u8>),
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
        /// The full topic string (e.g. "protide-pake-apple-banana-123")
        topic: String,
        node_name: String,
        kind: String,
        pake_bytes: Vec<u8>,
    },
}

// ── libp2p behaviour ──────────────────────────────────────────────────────────

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "ProtideBehaviourEvent")]
struct ProtideBehaviour {
    gossipsub: gossipsub::Behaviour,
    kademlia: kad::Behaviour<MemoryStore>,
    mdns: mdns::tokio::Behaviour,
    identify: identify::Behaviour,
}

#[derive(Debug)]
#[allow(dead_code)]
enum ProtideBehaviourEvent {
    Gossipsub(gossipsub::Event),
    Kademlia(kad::Event),
    Mdns(mdns::Event),
    Identify(identify::Event),
}

impl From<gossipsub::Event> for ProtideBehaviourEvent {
    fn from(e: gossipsub::Event) -> Self { ProtideBehaviourEvent::Gossipsub(e) }
}
impl From<kad::Event> for ProtideBehaviourEvent {
    fn from(e: kad::Event) -> Self { ProtideBehaviourEvent::Kademlia(e) }
}
impl From<mdns::Event> for ProtideBehaviourEvent {
    fn from(e: mdns::Event) -> Self { ProtideBehaviourEvent::Mdns(e) }
}
impl From<identify::Event> for ProtideBehaviourEvent {
    fn from(e: identify::Event) -> Self { ProtideBehaviourEvent::Identify(e) }
}

// ── P2PSync ───────────────────────────────────────────────────────────────────

/// Libp2p-based P2P sync backend.
pub struct P2PSync {
    peer_id: PeerId,
    _node_id: NodeId,
    _event_tx: Sender<P2PEvent>,
    event_rx: Receiver<P2PEvent>,
    known_peers: HashSet<PeerId>,
    _crdt_topic: gossipsub::IdentTopic,
    /// Outgoing CRDT broadcast channel
    broadcast_tx: Sender<CrdtEntry>,
    /// Command channel for subscribe/publish operations on dynamic topics
    cmd_tx: Sender<SwarmCmd>,
}

impl P2PSync {
    /// Create and start a new P2P sync node.
    /// `pairing_code` scopes gossip topics so only peers with the same code connect.
    pub fn start(node_id: NodeId, listen_port: Option<u16>, pairing_code: &str) -> Result<Self, String> {
        let (event_tx, event_rx) = mpsc::channel::<P2PEvent>();
        let (broadcast_tx, broadcast_rx) = mpsc::channel::<CrdtEntry>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<SwarmCmd>();

        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());

        let crdt_topic_str = if pairing_code.is_empty() {
            GOSSIP_TOPIC.to_string()
        } else {
            format!("{}-{}", GOSSIP_TOPIC, pairing_code)
        };

        let mut swarm = SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::new().nodelay(true),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .map_err(|e| format!("TCP transport error: {}", e))?
            .with_dns()
            .map_err(|e| format!("DNS error: {}", e))?
            .with_behaviour(|key: &identity::Keypair| {
                let peer_id = key.public().to_peer_id();

                let gs_config = gossipsub::ConfigBuilder::default()
                    .validation_mode(gossipsub::ValidationMode::Permissive)
                    .message_id_fn(|msg| {
                        let data = &msg.data[..msg.data.len().min(64)];
                        let mut hasher = blake3::Hasher::new();
                        hasher.update(data);
                        gossipsub::MessageId::from(&hasher.finalize().as_bytes()[..8])
                    })
                    .build()
                    .map_err(|e| format!("Gossipsub config error: {:?}", e))?;

                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gs_config,
                )
                .map_err(|e| format!("Gossipsub error: {:?}", e))?;

                let store = MemoryStore::new(peer_id);
                let kademlia = kad::Behaviour::new(peer_id, store);

                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )
                .map_err(|e| format!("mDNS error: {:?}", e))?;

                let identify = identify::Behaviour::new(
                    identify::Config::new("protide/0.1.0".into(), key.public())
                        .with_agent_version("protide/0.1.0".into()),
                );

                Ok(ProtideBehaviour { gossipsub, kademlia, mdns, identify })
            })
            .map_err(|e| format!("Behaviour error: {:?}", e))?
            .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        // Subscribe to the CRDT topic
        let crdt_topic = gossipsub::IdentTopic::new(&crdt_topic_str);
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&crdt_topic)
            .map_err(|e| format!("Failed to subscribe: {:?}", e))?;
        let crdt_topic_task = crdt_topic.clone();

        // Also subscribe to the PAKE topic so this node hears handshake requests
        // from peers who typed our code.
        if !pairing_code.is_empty() {
            let pake_topic = gossipsub::IdentTopic::new(
                format!("{}{}", PAKE_TOPIC_PREFIX, pairing_code)
            );
            let _ = swarm.behaviour_mut().gossipsub.subscribe(&pake_topic);
        }

        let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", listen_port.unwrap_or(0))
            .parse()
            .map_err(|_| "Invalid listen address".to_string())?;
        swarm
            .listen_on(listen_addr)
            .map_err(|e| format!("Failed to listen: {}", e))?;

        let event_tx_clone = event_tx.clone();

        tokio::spawn(async move {
            // Track dynamically-subscribed pake topics so we can publish on them
            let mut pake_topics: HashMap<String, gossipsub::IdentTopic> = HashMap::new();

            loop {
                // Drain outgoing CRDT broadcasts
                while let Ok(entry) = broadcast_rx.try_recv() {
                    if let Ok(data) = serde_json::to_vec(&entry) {
                        let _ = swarm.behaviour_mut().gossipsub.publish(crdt_topic_task.clone(), data);
                    }
                }

                // Drain commands (subscribe / publish on dynamic topics)
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        SwarmCmd::Subscribe(topic_str) => {
                            let t = gossipsub::IdentTopic::new(&topic_str);
                            let _ = swarm.behaviour_mut().gossipsub.subscribe(&t);
                            pake_topics.insert(topic_str, t);
                        }
                        SwarmCmd::Publish(topic_str, data) => {
                            // Auto-subscribe if we haven't yet (e.g. Bob's first publish)
                            if !pake_topics.contains_key(&topic_str) {
                                let t = gossipsub::IdentTopic::new(&topic_str);
                                let _ = swarm.behaviour_mut().gossipsub.subscribe(&t);
                                pake_topics.insert(topic_str.clone(), t);
                            }
                            if let Some(t) = pake_topics.get(&topic_str) {
                                let _ = swarm.behaviour_mut().gossipsub.publish(t.clone(), data);
                            }
                        }
                    }
                }

                // Process ONE swarm event, with a 50ms timeout so queues are drained regularly
                let result = tokio::time::timeout(
                    Duration::from_millis(50),
                    swarm.next(),
                ).await;

                let event = match result {
                    Ok(Some(ev)) => ev,
                    Ok(None) => break,      // swarm stream ended
                    Err(_) => continue,     // timeout — loop back to drain queues
                };

                match event {
                    SwarmEvent::Behaviour(ProtideBehaviourEvent::Gossipsub(gs_event)) => {
                        if let gossipsub::Event::Message { message, .. } = gs_event {
                            let topic_str = message.topic.to_string();
                            let from = message.source.unwrap_or_else(PeerId::random);

                            if topic_str.starts_with(PAKE_TOPIC_PREFIX) {
                                // PAKE handshake message
                                if let Ok(payload) = serde_json::from_slice::<PakeMsgPayload>(&message.data) {
                                    let _ = event_tx_clone.send(P2PEvent::PakeMsg {
                                        from,
                                        topic: topic_str,
                                        node_name: payload.node_name,
                                        kind: payload.kind,
                                        pake_bytes: payload.pake_bytes,
                                    });
                                }
                            } else {
                                // CRDT sync entry
                                if let Ok(entry) = serde_json::from_slice::<CrdtEntry>(&message.data) {
                                    let _ = event_tx_clone.send(P2PEvent::EntryReceived(entry));
                                }
                            }
                        }
                    }
                    SwarmEvent::Behaviour(ProtideBehaviourEvent::Mdns(mdns_event)) => {
                        match mdns_event {
                            mdns::Event::Discovered(peers) => {
                                for (peer, _addr) in peers {
                                    let _ = event_tx_clone.send(P2PEvent::PeerJoined(peer));
                                }
                            }
                            mdns::Event::Expired(peers) => {
                                for (peer, _addr) in peers {
                                    let _ = event_tx_clone.send(P2PEvent::PeerLeft(peer));
                                }
                            }
                        }
                    }
                    SwarmEvent::Behaviour(ProtideBehaviourEvent::Identify(_)) => {}
                    SwarmEvent::Behaviour(ProtideBehaviourEvent::Kademlia(_)) => {}
                    SwarmEvent::NewListenAddr { .. } => {}
                    _ => {}
                }
            }
        });

        Ok(Self {
            peer_id,
            _node_id: node_id,
            _event_tx: event_tx,
            event_rx,
            known_peers: HashSet::new(),
            _crdt_topic: crdt_topic,
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

    /// Subscribe to the PAKE topic for `code` so we can receive handshake messages.
    pub fn subscribe_pake_topic(&self, code: &str) {
        let topic = format!("{}{}", PAKE_TOPIC_PREFIX, code);
        let _ = self.cmd_tx.send(SwarmCmd::Subscribe(topic));
    }

    /// Publish raw bytes on the PAKE topic for `code`.
    pub fn publish_on_pake_topic(&self, code: &str, data: Vec<u8>) {
        let topic = format!("{}{}", PAKE_TOPIC_PREFIX, code);
        let _ = self.cmd_tx.send(SwarmCmd::Publish(topic, data));
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

    pub fn peer_id(&self) -> &PeerId { &self.peer_id }

    pub fn known_peers(&self) -> &HashSet<PeerId> { &self.known_peers }
}
