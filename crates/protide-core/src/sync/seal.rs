//! Sealing of CRDT entries under the key of a sync group.
//!
//! Every entry that leaves this node, on the gossip topic or into a shared
//! sync folder, is encrypted under a key derived from the group secret.
//! Anything that does not open under the current key is dropped, so only
//! members of the group read or inject entries. Resetting the group changes
//! the key, which locks out every member paired before.

use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

use super::types::{CrdtEntry, GroupSecret};

/// Length of the random ChaCha20Poly1305 nonce that prefixes a seal.
pub(super) const NONCE_LEN: usize = 12;

/// Cipher for the sync data of the group of `secret`, keyed under its own
/// context so the key is independent of the topic derived from the secret.
fn data_cipher(secret: &GroupSecret) -> ChaCha20Poly1305 {
    let key = blake3::derive_key("protide sync data v1", &secret.0);
    ChaCha20Poly1305::new(Key::from_slice(&key))
}

/// Seal `entry` as nonce || ciphertext, binding `aad` as associated data so
/// a seal never opens in any other context.
pub(super) fn seal_entry(secret: &GroupSecret, aad: &str, entry: &CrdtEntry) -> Option<Vec<u8>> {
    let msg = serde_json::to_vec(entry).ok()?;
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let aad = aad.as_bytes();
    let sealed = data_cipher(secret)
        .encrypt(&nonce, Payload { msg: &msg, aad })
        .ok()?;
    let mut data = nonce.to_vec();
    data.extend_from_slice(&sealed);
    Some(data)
}

/// Open an entry sealed for `aad` under the key of `secret`, or `None`
/// when it was sealed under another key or context, altered, or malformed.
pub(super) fn open_entry(secret: &GroupSecret, aad: &str, data: &[u8]) -> Option<CrdtEntry> {
    let (nonce, sealed) = data.split_at_checked(NONCE_LEN)?;
    let aad = aad.as_bytes();
    let msg = data_cipher(secret)
        .decrypt(Nonce::from_slice(nonce), Payload { msg: sealed, aad })
        .ok()?;
    serde_json::from_slice(&msg).ok()
}
