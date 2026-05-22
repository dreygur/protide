use std::time::{Duration, Instant};
use gpui::SharedString;

/// A connected peer on the network
#[derive(Debug, Clone)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub last_seen: Instant,
    pub is_active: bool,
    /// What protocol the peer was discovered through
    pub source: PeerSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerSource {
    UDPBroadcast,
    P2P,
    FileSync,
}

impl Peer {
    pub fn new(id: String, name: String, source: PeerSource) -> Self {
        Self {
            id,
            name,
            last_seen: Instant::now(),
            is_active: true,
            source,
        }
    }

    /// Display initials (first letter of each word)
    pub fn initials(&self) -> String {
        self.name
            .split(|c: char| !c.is_alphanumeric())
            .filter_map(|w| w.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase()
    }
}

/// Connection state for the Join Peer flow
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Idle,
    Handshaking,
    Connected,
    /// Handshake timed out or PAKE verification failed
    Error(String),
}

/// Manages the list of known peers and their status.
/// UI component that integrates with the sync engine.
#[derive(Debug, Clone)]
pub struct PresenceManager {
    pub(super) peers: Vec<Peer>,
    /// Whether collaboration is currently enabled
    pub enabled: bool,
    /// Show the pairing flyout
    pub show_pairing: bool,
    /// Current pairing code (generated or entered)
    pub pairing_code: SharedString,
    /// Last generated pairing code
    pub generated_code: SharedString,
    /// Connection state for the join flow
    pub connection_status: ConnectionStatus,
    /// Flips every tick while Handshaking - drives pulsing border
    pub handshake_tick: bool,
}

impl Default for PresenceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PresenceManager {
    pub fn new() -> Self {
        Self {
            peers: Vec::new(),
            enabled: true,
            show_pairing: false,
            pairing_code: SharedString::default(),
            generated_code: SharedString::default(),
            connection_status: ConnectionStatus::Idle,
            handshake_tick: false,
        }
    }

    /// Add or update a peer
    pub fn upsert_peer(&mut self, id: String, name: String, source: PeerSource) {
        if let Some(existing) = self.peers.iter_mut().find(|p| p.id == id) {
            existing.last_seen = Instant::now();
            existing.is_active = true;
            existing.name = name;
        } else {
            self.peers.push(Peer::new(id, name, source));
        }
    }

    /// Remove a peer
    pub fn remove_peer(&mut self, id: &str) {
        self.peers.retain(|p| p.id != id);
    }

    /// Reap stale peers (not seen for >30s)
    pub fn reap_stale(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(30);
        self.peers.retain(|p| p.last_seen > cutoff);
    }

    /// Get active peers
    pub fn active_peers(&self) -> &[Peer] {
        &self.peers
    }

    /// Number of active peers
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Generate a new pairing code
    #[allow(unexpected_cfgs)]
    pub fn generate_code(&mut self) {
        #[cfg(feature = "pake-auth")]
        {
            self.generated_code = SharedString::from(protide_core::sync::pake::generate_pairing_code());
            self.pairing_code = self.generated_code.clone();
        }
        #[cfg(not(feature = "pake-auth"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
                % 10000;
            self.generated_code = SharedString::from(format!("protide-{:04}", suffix));
            self.pairing_code = self.generated_code.clone();
        }
    }

    /// Flip the pulse tick - call once per second while Handshaking
    pub fn tick_handshake(&mut self) {
        if self.connection_status == ConnectionStatus::Handshaking {
            self.handshake_tick = !self.handshake_tick;
        }
    }

    /// Transition to Connected, update peer display
    pub fn set_connected(&mut self, peer_id: String, peer_name: String) {
        self.connection_status = ConnectionStatus::Connected;
        self.upsert_peer(peer_id, peer_name, PeerSource::P2P);
        self.show_pairing = false;
    }

    /// Reset connection state back to Idle
    pub fn reset_connection(&mut self) {
        self.connection_status = ConnectionStatus::Idle;
        self.handshake_tick = false;
    }

    /// Nearby peers (mDNS / UDP-discovered, not yet PAKE-authenticated)
    pub fn nearby_peers(&self) -> Vec<&Peer> {
        self.peers
            .iter()
            .filter(|p| p.source == PeerSource::UDPBroadcast || p.source == PeerSource::P2P)
            .collect()
    }
}
