#![cfg(feature = "p2p-sync")]

use argon2::{Algorithm, Argon2, Params, Version};

/// Shared CRDT topic for nodes without a pairing code.
pub(super) const GOSSIP_TOPIC: &str = "protide-crdt";
pub(super) const PAKE_TOPIC_PREFIX: &str = "protide-pake-";

/// Fixed salt for the topic derivation. It is an application identifier,
/// not a secret: it only stops precomputed tables built for other apps
/// from applying here. Bump the version to change every topic at once.
const TOPIC_KDF_SALT: &[u8] = b"protide-pairing-topic-v1";

/// Argon2id memory cost in KiB (19 MiB, the OWASP Argon2id baseline).
/// Memory hardness is what makes GPU and ASIC guessing expensive, and
/// 19 MiB is trivial for a desktop app that derives once per pairing.
const TOPIC_KDF_MEMORY_KIB: u32 = 19 * 1024;

/// Argon2id passes over memory. With the memory cost above one derivation
/// measured about 67 ms in a release build on an Apple silicon laptop:
/// unnoticeable when pairing, yet hashing all 2^42 pairing codes costs an
/// attacker on the order of 10^4 CPU years.
const TOPIC_KDF_ITERATIONS: u32 = 2;

/// Argon2id lanes. One lane keeps the derivation single threaded so it
/// never competes with the UI for cores.
const TOPIC_KDF_LANES: u32 = 1;

/// Gossipsub topic names scoped to one pairing code.
pub(super) struct PairingTopics {
    pub(super) crdt: String,
    pub(super) pake: String,
}

/// Stretch `pairing_code` into a 32 byte key with Argon2id.
///
/// Gossipsub topic subscriptions are visible to every peer on the mesh, so
/// the topic must not reveal the code. A plain hash did not: with a small
/// code space an observer could hash every code and match the topic. The
/// slow, memory hard KDF makes that enumeration impractical. This is slow
/// on purpose, so call it off the UI thread.
fn pairing_key(pairing_code: &str) -> [u8; 32] {
    let params = Params::new(
        TOPIC_KDF_MEMORY_KIB,
        TOPIC_KDF_ITERATIONS,
        TOPIC_KDF_LANES,
        Some(32),
    )
    .expect("Argon2 parameters are valid constants");
    let mut key = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(pairing_code.as_bytes(), TOPIC_KDF_SALT, &mut key)
        .expect("salt and output lengths are valid constants");
    key
}

/// Topic name component for `domain`, derived from the stretched key.
/// `domain` keeps the CRDT and PAKE topics of one code apart.
fn topic_hash(domain: &str, key: &[u8; 32]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain.as_bytes());
    hasher.update(b":");
    hasher.update(key);
    hasher.finalize().to_hex()[..32].to_string()
}

impl PairingTopics {
    /// Derive both topics with a single Argon2id run.
    pub(super) fn derive(pairing_code: &str) -> Self {
        let key = pairing_key(pairing_code);
        Self {
            crdt: format!("{}-{}", GOSSIP_TOPIC, topic_hash("crdt", &key)),
            pake: format!("{}{}", PAKE_TOPIC_PREFIX, topic_hash("pake", &key)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODE: &str = "amber-otter-river-tiger-482";

    #[test]
    fn both_peers_derive_the_same_topics_from_one_code() {
        let alice = PairingTopics::derive(CODE);
        let bob = PairingTopics::derive(CODE);
        assert_eq!(alice.crdt, bob.crdt);
        assert_eq!(alice.pake, bob.pake);
        assert_ne!(alice.crdt, alice.pake);
        assert!(alice.pake.starts_with(PAKE_TOPIC_PREFIX));

        let other = PairingTopics::derive("amber-otter-river-tiger-483");
        assert_ne!(alice.pake, other.pake);
        assert_ne!(alice.crdt, other.crdt);
    }

    /// REGRESSION: the topic was blake3(domain ":" code), cheap enough to
    /// compute for every code. It must now go through the slow KDF.
    #[test]
    fn the_topic_is_not_the_old_plain_hash() {
        let topics = PairingTopics::derive(CODE);
        for (domain, topic) in [("pake", &topics.pake), ("crdt", &topics.crdt)] {
            let mut hasher = blake3::Hasher::new();
            hasher.update(domain.as_bytes());
            hasher.update(b":");
            hasher.update(CODE.as_bytes());
            let plain = hasher.finalize().to_hex()[..32].to_string();
            assert!(!topic.contains(&plain), "{domain} topic is the plain hash");
        }
    }
}
