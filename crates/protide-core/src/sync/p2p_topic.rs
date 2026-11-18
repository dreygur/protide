#![cfg(feature = "p2p-sync")]

const GOSSIP_TOPIC: &str = "protide-crdt";
pub(super) const PAKE_TOPIC_PREFIX: &str = "protide-pake-";

/// Derive a one-way, non-reversible topic-name component from a plaintext
/// pairing code.
///
/// Gossipsub topic subscriptions are visible to every peer on the mesh (via
/// GRAFT / mesh announcements), so embedding the pairing code itself in a
/// topic name - as this code previously did for both the PAKE and CRDT
/// topics - lets any observer who never learned the code read it straight
/// off the wire. Peers who *do* know `pairing_code` can still compute the
/// same topic locally (this is a plain deterministic hash, not a secret
/// derivation), so pairing still works the same way; observers who only see
/// the topic name cannot invert the hash to recover the code.
///
/// `domain` provides domain separation so the CRDT topic and PAKE topic
/// derived from the same code don't collide with each other.
fn topic_hash(domain: &str, pairing_code: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain.as_bytes());
    hasher.update(b":");
    hasher.update(pairing_code.as_bytes());
    hasher.finalize().to_hex()[..32].to_string()
}

/// Full gossipsub topic string for CRDT sync, scoped to `pairing_code`.
pub(super) fn crdt_topic_string(pairing_code: &str) -> String {
    if pairing_code.is_empty() {
        GOSSIP_TOPIC.to_string()
    } else {
        format!("{}-{}", GOSSIP_TOPIC, topic_hash("crdt", pairing_code))
    }
}

/// Full gossipsub topic string for PAKE handshake messages, scoped to
/// `pairing_code`.
pub(super) fn pake_topic_string(pairing_code: &str) -> String {
    format!("{}{}", PAKE_TOPIC_PREFIX, topic_hash("pake", pairing_code))
}
