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

/// Topic name component for `domain`, derived from a 32 byte key.
/// `domain` keeps topics derived from one key apart.
fn topic_hash(domain: &str, key: &[u8; 32]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain.as_bytes());
    hasher.update(b":");
    hasher.update(key);
    hasher.finalize().to_hex()[..32].to_string()
}

/// Gossipsub topic for PAKE handshakes on `pairing_code`. Runs Argon2id,
/// so call it off the UI thread.
pub(super) fn pake_topic(pairing_code: &str) -> String {
    let key = pairing_key(pairing_code);
    format!("{}{}", PAKE_TOPIC_PREFIX, topic_hash("pake", &key))
}

/// Gossipsub topic for CRDT sync within one paired group. The group secret
/// is 32 random bytes, so a fast key derivation is enough: the topic cannot
/// be enumerated and reveals nothing about the secret.
pub(super) fn sync_topic(group_secret: &[u8; 32]) -> String {
    let key = blake3::derive_key("protide sync topic v1", group_secret);
    format!("{}-{}", GOSSIP_TOPIC, topic_hash("crdt", &key))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODE: &str = "amber-otter-river-tiger-482";

    #[test]
    fn both_peers_derive_the_same_pake_topic_from_one_code() {
        let topic = pake_topic(CODE);
        assert_eq!(topic, pake_topic(CODE));
        assert!(topic.starts_with(PAKE_TOPIC_PREFIX));
        assert_ne!(topic, pake_topic("amber-otter-river-tiger-483"));
    }

    /// REGRESSION: the topic was blake3(domain ":" code), cheap enough to
    /// compute for every code. It must now go through the slow KDF.
    #[test]
    fn the_topic_is_not_the_old_plain_hash() {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"pake:");
        hasher.update(CODE.as_bytes());
        let plain = hasher.finalize().to_hex()[..32].to_string();
        assert!(!pake_topic(CODE).contains(&plain));
    }

    /// The sync topic comes from the group secret alone, so every member of
    /// a group shares it whatever pairing code each one shows.
    #[test]
    fn the_sync_topic_depends_only_on_the_group_secret() {
        let secret = [7u8; 32];
        let topic = sync_topic(&secret);
        assert_eq!(topic, sync_topic(&secret));
        assert_ne!(topic, sync_topic(&[8u8; 32]));
        assert!(topic.starts_with(GOSSIP_TOPIC));
        let hex: String = secret.iter().map(|b| format!("{b:02x}")).collect();
        assert!(!topic.contains(&hex[..16]), "topic leaks the secret");
    }
}
