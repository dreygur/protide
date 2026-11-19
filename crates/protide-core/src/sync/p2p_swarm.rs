#![cfg(feature = "p2p-sync")]

use std::num::NonZeroUsize;
use std::sync::mpsc::Sender;
use std::time::Duration;

use libp2p::gossipsub;
use libp2p::identify;
use libp2p::kad::{self, store::MemoryStore};
use libp2p::mdns;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{Multiaddr, PeerId, Swarm, SwarmBuilder, identity};

use log::{debug, info, warn};

use serde::{Deserialize, Serialize};

use super::p2p::P2PEvent;
use super::p2p_topic::PAKE_TOPIC_PREFIX;
use super::types::CrdtEntry;

/// Serialised payload for PAKE handshake messages over gossipsub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PakeMsgPayload {
    /// "init" (Bob → Alice), "resp" (Alice → Bob), "confirm" (Bob → Alice,
    /// mutual key confirmation) or "group" (Alice → Bob, sent only after
    /// Bob's confirm verified)
    pub kind: String,
    /// Display name of the sender
    pub node_name: String,
    /// Raw SPAKE2 public-key bytes (unused/empty for "confirm")
    pub pake_bytes: Vec<u8>,
    /// Role bound key-confirmation blob (see `pake::confirm_blob`), present on
    /// "resp" and "confirm" messages once each side has derived a session
    /// key. Proves the sender derived the same shared key as us - SPAKE2's
    /// `finish()` alone does not guarantee that.
    #[serde(default)]
    pub confirm: Option<Vec<u8>>,
    /// The responder's group secret on "group", sealed under a key derived
    /// from the PAKE session (see `pake::seal_group_secret`), so only a peer
    /// that derived the same key can open it
    #[serde(default)]
    pub group: Option<Vec<u8>>,
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "ProtideBehaviourEvent")]
pub(super) struct ProtideBehaviour {
    pub(super) gossipsub: gossipsub::Behaviour,
    pub(super) kademlia: kad::Behaviour<MemoryStore>,
    mdns: mdns::tokio::Behaviour,
    identify: identify::Behaviour,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(super) enum ProtideBehaviourEvent {
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

pub(super) fn build_swarm(keypair: identity::Keypair) -> Result<Swarm<ProtideBehaviour>, String> {
    Ok(SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::new().nodelay(true),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .map_err(|e| format!("TCP transport error: {}", e))?
        .with_quic()
        .with_behaviour(|key: &identity::Keypair| {
            let pid = key.public().to_peer_id();

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

            let store = MemoryStore::new(pid);
            // Throttle DHT to minimise background network noise:
            // - replication_factor 3 (down from 20): fewer peers contacted per query
            // - periodic_bootstrap 30 min (down from 5 min): less crawl churn
            // - disable record/provider replication: we use Kademlia only for
            //   peer discovery, not distributed storage
            let mut kad_config =
                kad::Config::new(libp2p::StreamProtocol::new("/protide/kad/1.0.0"));
            kad_config
                .set_replication_factor(NonZeroUsize::new(3).unwrap())
                .set_periodic_bootstrap_interval(Some(Duration::from_secs(30 * 60)))
                .set_replication_interval(None)
                .set_publication_interval(None)
                .set_provider_publication_interval(None);
            let kademlia = kad::Behaviour::with_config(pid, store, kad_config);

            let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), pid)
                .map_err(|e| format!("mDNS error: {:?}", e))?;

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
        .build())
}

pub(super) fn bootstrap_kademlia(swarm: &mut Swarm<ProtideBehaviour>) {
    // Bootstrap into the global libp2p DHT so remote peers can find us.
    // Note: libp2p does NOT use STUN. NAT traversal is handled by the DHT
    // (peer discovery) + DCUtR hole-punch protocol. For strict CGNAT you
    // would additionally need a circuit-relay server; that's a future step.
    const BOOTSTRAP_PEERS: &[(&str, &str)] = &[
        (
            "/dnsaddr/bootstrap.libp2p.io",
            "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
        ),
        (
            "/dnsaddr/bootstrap.libp2p.io",
            "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
        ),
        (
            "/dnsaddr/bootstrap.libp2p.io",
            "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
        ),
        (
            "/dnsaddr/bootstrap.libp2p.io",
            "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
        ),
    ];
    let mut added = 0usize;
    for (addr_str, peer_str) in BOOTSTRAP_PEERS {
        match (addr_str.parse::<Multiaddr>(), peer_str.parse::<PeerId>()) {
            (Ok(addr), Ok(pid)) => {
                swarm.behaviour_mut().kademlia.add_address(&pid, addr);
                added += 1;
            }
            _ => warn!(
                "[Kad] Failed to parse bootstrap entry: {} {}",
                addr_str, peer_str
            ),
        }
    }
    debug!("[Kad] Added {} bootstrap peers", added);
    match swarm.behaviour_mut().kademlia.bootstrap() {
        Ok(qid) => debug!("[Kad] Bootstrap query started: {:?}", qid),
        Err(e) => warn!("[Kad] Bootstrap failed (no known peers): {:?}", e),
    }
}

/// Move our own PAKE subscription to `pake` and report it to the engine,
/// which caches it so incoming messages never run the KDF.
pub(super) fn switch_own_pake_topic(
    gossipsub: &mut gossipsub::Behaviour,
    own: &mut Option<gossipsub::IdentTopic>,
    pake: String,
    events: &Sender<P2PEvent>,
) {
    let topic = gossipsub::IdentTopic::new(&pake);
    if let Some(old) = own.replace(topic.clone()) {
        let _ = gossipsub.unsubscribe(&old);
    }
    let _ = gossipsub.subscribe(&topic);
    let _ = events.send(P2PEvent::PakeTopic(pake));
}

pub(super) fn handle_swarm_event(
    event: SwarmEvent<ProtideBehaviourEvent>,
    event_tx_thread: &Sender<P2PEvent>,
) {
    match event {
        SwarmEvent::Behaviour(ProtideBehaviourEvent::Gossipsub(gossipsub::Event::Message {
            message,
            ..
        })) => {
            let topic_str = message.topic.to_string();
            let from = message.source.unwrap_or_else(PeerId::random);

            if topic_str.starts_with(PAKE_TOPIC_PREFIX) {
                debug!(
                    "[PAKE] Received packet on topic '{}' from {}",
                    topic_str, from
                );
                if let Ok(payload) = serde_json::from_slice::<PakeMsgPayload>(&message.data) {
                    debug!(
                        "[PAKE] Message kind='{}' node='{}'",
                        payload.kind, payload.node_name
                    );
                    let _ = event_tx_thread.send(P2PEvent::PakeMsg {
                        from,
                        topic: topic_str,
                        payload,
                    });
                }
            } else if let Ok(entry) = serde_json::from_slice::<CrdtEntry>(&message.data) {
                let _ = event_tx_thread.send(P2PEvent::EntryReceived(entry));
            }
        }
        SwarmEvent::Behaviour(ProtideBehaviourEvent::Mdns(mdns_event)) => match mdns_event {
            mdns::Event::Discovered(peers) => {
                for (peer, addr) in peers {
                    info!("[mDNS] Discovered peer {} at {}", peer, addr);
                    let _ = event_tx_thread.send(P2PEvent::PeerJoined(peer));
                }
            }
            mdns::Event::Expired(peers) => {
                for (peer, _addr) in peers {
                    let _ = event_tx_thread.send(P2PEvent::PeerLeft(peer));
                }
            }
        },
        SwarmEvent::Behaviour(ProtideBehaviourEvent::Identify(_)) => {}
        SwarmEvent::Behaviour(ProtideBehaviourEvent::Kademlia(kad_event)) => match kad_event {
            kad::Event::OutboundQueryProgressed {
                result: kad::QueryResult::Bootstrap(res),
                ..
            } => match res {
                Ok(kad::BootstrapOk {
                    peer,
                    num_remaining,
                }) => {
                    debug!(
                        "[Kad] Bootstrap progress: peer={} remaining={}",
                        peer, num_remaining
                    );
                }
                Err(e) => warn!("[Kad] Bootstrap error: {:?}", e),
            },
            kad::Event::RoutingUpdated { peer, .. } => {
                debug!("[Kad] Routing table updated: new peer {}", peer);
            }
            _ => {}
        },
        SwarmEvent::NewListenAddr { address, .. } => {
            info!("[P2P] Bound listen address: {}", address);
            let _ = event_tx_thread.send(P2PEvent::LocalAddr(address.to_string()));
        }
        _ => {}
    }
}
