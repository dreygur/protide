#![cfg(feature = "p2p-sync")]

use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use libp2p::gossipsub;
use libp2p::identify;
use libp2p::kad::{self, store::MemoryStore};
use libp2p::mdns;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{identity, Multiaddr, PeerId, SwarmBuilder};

use super::types::{CrdtEntry, NodeId};

const GOSSIP_TOPIC: &str = "protide-crdt";

/// Events from the P2P backend
#[derive(Debug, Clone)]
pub enum P2PEvent {
    EntryReceived(CrdtEntry),
    PeerJoined(PeerId),
    PeerLeft(PeerId),
    Error(String),
}

/// Combined behaviour: Gossipsub for CRDT broadcast, Kademlia for DHT discovery,
/// mDNS for local network discovery, Identify for address exchange.
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "ProtideBehaviourEvent")]
struct ProtideBehaviour {
    gossipsub: gossipsub::Behaviour,
    kademlia: kad::Behaviour<MemoryStore>,
    mdns: mdns::tokio::Behaviour,
    identify: identify::Behaviour,
}

/// Unified event enum for ProtideBehaviour
#[derive(Debug)]
#[allow(dead_code)]
enum ProtideBehaviourEvent {
    Gossipsub(gossipsub::Event),
    Kademlia(kad::Event),
    Mdns(mdns::Event),
    Identify(identify::Event),
}

impl From<gossipsub::Event> for ProtideBehaviourEvent {
    fn from(e: gossipsub::Event) -> Self {
        ProtideBehaviourEvent::Gossipsub(e)
    }
}

impl From<kad::Event> for ProtideBehaviourEvent {
    fn from(e: kad::Event) -> Self {
        ProtideBehaviourEvent::Kademlia(e)
    }
}

impl From<mdns::Event> for ProtideBehaviourEvent {
    fn from(e: mdns::Event) -> Self {
        ProtideBehaviourEvent::Mdns(e)
    }
}

impl From<identify::Event> for ProtideBehaviourEvent {
    fn from(e: identify::Event) -> Self {
        ProtideBehaviourEvent::Identify(e)
    }
}

/// Libp2p-based P2P sync backend.
pub struct P2PSync {
    peer_id: PeerId,
    _node_id: NodeId,
    _event_tx: Sender<P2PEvent>,
    event_rx: Receiver<P2PEvent>,
    known_peers: HashSet<PeerId>,
    _topic: gossipsub::IdentTopic,
    /// Channel sender to push broadcast entries to the event loop
    broadcast_tx: Sender<CrdtEntry>,
}

impl P2PSync {
    /// Create and start a new P2P sync node.
    /// `pairing_code` scopes the gossip topic so only peers with the same code discover each other.
    pub fn start(node_id: NodeId, listen_port: Option<u16>, pairing_code: &str) -> Result<Self, String> {
        let (event_tx, event_rx) = mpsc::channel::<P2PEvent>();
        let (broadcast_tx, broadcast_rx) = mpsc::channel::<CrdtEntry>();

        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());

        // Derive gossip topic from pairing code for namespace scoping
        let topic_str = if pairing_code.is_empty() {
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

                // Gossipsub for broadcasting CRDT updates
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

                // Kademlia DHT for peer discovery
                let store = MemoryStore::new(peer_id);
                let kademlia = kad::Behaviour::new(peer_id, store);

                // mDNS for local network discovery (Dhaka office LAN)
                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )
                .map_err(|e| format!("mDNS error: {:?}", e))?;

                // Identify protocol to exchange listening addresses
                let identify = identify::Behaviour::new(
                    identify::Config::new("protide/0.1.0".into(), key.public())
                        .with_agent_version("protide/0.1.0".into()),
                );

                Ok(ProtideBehaviour {
                    gossipsub,
                    kademlia,
                    mdns,
                    identify,
                })
            })
            .map_err(|e| format!("Behaviour error: {:?}", e))?
            .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        // Subscribe to pairing-code-scoped CRDT topic
        let topic = gossipsub::IdentTopic::new(&topic_str);
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic)
            .map_err(|e| format!("Failed to subscribe: {:?}", e))?;

        // Start listening
        let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", listen_port.unwrap_or(0))
            .parse()
            .map_err(|_| "Invalid listen address".to_string())?;

        swarm
            .listen_on(listen_addr)
            .map_err(|e| format!("Failed to listen: {}", e))?;

        // Spawn the event loop
        let event_tx_clone = event_tx.clone();
        let _peer_id = peer_id;
        tokio::spawn(async move {
            use libp2p::swarm::ConnectionId;
            use libp2p::Multiaddr;

            loop {
                tokio::time::sleep(Duration::from_millis(50)).await;

                // Drain broadcast queue and publish via gossipsub
                while let Ok(entry) = broadcast_rx.try_recv() {
                    if let Ok(data) = serde_json::to_vec(&entry) {
                        let _ = swarm
                            .behaviour_mut()
                            .gossipsub
                            .publish(topic.clone(), data);
                    }
                }

                while let Some(event) = swarm.next().await {
                    match event {
                        SwarmEvent::Behaviour(ProtideBehaviourEvent::Gossipsub(gs_event)) => {
                            if let gossipsub::Event::Message {
                                propagation_source: _,
                                message_id: _,
                                message,
                            } = gs_event
                            {
                                if let Ok(entry) =
                                    serde_json::from_slice::<CrdtEntry>(&message.data)
                                {
                                    let _ =
                                        event_tx_clone.send(P2PEvent::EntryReceived(entry));
                                }
                            }
                        }
                        SwarmEvent::Behaviour(ProtideBehaviourEvent::Mdns(mdns_event)) => {
                            match mdns_event {
                                mdns::Event::Discovered(peers) => {
                                    for (peer, _addrs) in peers {
                                        let _ = event_tx_clone
                                            .send(P2PEvent::PeerJoined(peer));
                                    }
                                }
                                mdns::Event::Expired(peers) => {
                                    for (peer, _addrs) in peers {
                                        let _ = event_tx_clone
                                            .send(P2PEvent::PeerLeft(peer));
                                    }
                                }
                            }
                        }
                        SwarmEvent::Behaviour(ProtideBehaviourEvent::Identify(
                            identify::Event::Received {
                                peer_id: _,
                                info: _,
                                ..
                            },
                        )) => {}
                        SwarmEvent::Behaviour(ProtideBehaviourEvent::Kademlia(_)) => {}
                        SwarmEvent::NewListenAddr { .. } => {}
                        _ => {}
                    }
                }
            }
        });

        Ok(Self {
            peer_id,
            _node_id: node_id,
            _event_tx: event_tx,
            event_rx,
            known_peers: HashSet::new(),
            _topic: topic,
            broadcast_tx,
        })
    }

    /// Broadcast a CRDT entry via Gossipsub.
    /// Sends the entry to the event loop which publishes via gossipsub.
    pub fn broadcast_entry(&mut self, entry: &CrdtEntry) -> Result<(), String> {
        self.broadcast_tx
            .send(entry.clone())
            .map_err(|e| format!("Broadcast channel error: {}", e))
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
