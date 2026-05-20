#![cfg(feature = "pake-auth")]

use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use spake2::{Ed25519Group, Identity, Password, Spake2};

use rand::RngCore;

const PAKE_IDENTITY: &[u8] = b"protide-p2p-pairing";

/// A SPAKE2 handshake result - two peers derive a shared key from a
/// low-entropy password/pairing code without revealing it over the wire.
#[derive(Debug, Clone)]
pub struct PakeSession {
    /// Shared symmetric key derived from SPAKE2 (32 bytes)
    pub shared_key: [u8; 32],
}

/// Initiate a SPAKE2 handshake (side A).
/// Returns the outbound message to send to the peer.
pub fn pake_initiate(password: &str) -> Result<(Vec<u8>, Spake2<Ed25519Group>), String> {
    let (state, msg) = Spake2::<Ed25519Group>::start_a(
        &Password::new(password.as_bytes()),
        &Identity::new(PAKE_IDENTITY),
        &Identity::new(PAKE_IDENTITY),
    );
    Ok((msg, state))
}

/// Respond to a SPAKE2 handshake (side B).
/// Returns the outbound message to send back to the initiator.
pub fn pake_respond(password: &str) -> Result<(Vec<u8>, Spake2<Ed25519Group>), String> {
    let (state, msg) = Spake2::<Ed25519Group>::start_b(
        &Password::new(password.as_bytes()),
        &Identity::new(PAKE_IDENTITY),
        &Identity::new(PAKE_IDENTITY),
    );
    Ok((msg, state))
}

/// Finish the SPAKE2 handshake by processing the peer's message.
/// Returns the shared session key on success.
pub fn pake_finish(state: Spake2<Ed25519Group>, peer_msg: &[u8]) -> Result<PakeSession, String> {
    let key_bytes = state
        .finish(peer_msg)
        .map_err(|e| format!("SPAKE2 finish error: {:?}", e))?;

    if key_bytes.len() != 32 {
        return Err("SPAKE2 produced unexpected key length".into());
    }

    let mut shared_key = [0u8; 32];
    shared_key.copy_from_slice(&key_bytes);

    Ok(PakeSession { shared_key })
}

/// Encrypt a message using the PAKE-derived shared key (ChaCha20Poly1305).
/// Format: nonce (12 bytes) || ciphertext
pub fn encrypt_message(session: &PakeSession, plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let key = Key::from_slice(&session.shared_key);
    let cipher = ChaCha20Poly1305::new(key);

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("Encryption error: {:?}", e))?;

    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypt a message using the PAKE-derived shared key.
/// Expects format: nonce (12 bytes) || ciphertext
pub fn decrypt_message(session: &PakeSession, encrypted: &[u8]) -> Result<Vec<u8>, String> {
    if encrypted.len() < 12 {
        return Err("Message too short".into());
    }

    let key = Key::from_slice(&session.shared_key);
    let cipher = ChaCha20Poly1305::new(key);

    let (nonce_bytes, ciphertext) = encrypted.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Decryption error: wrong key or tampered data".to_string())
}

/// Generate a human-readable pairing code (Magic Wormhole style).
/// Format: adjective-noun-###   e.g. "apple-banana-123"
pub fn generate_pairing_code() -> String {
    const ADJECTIVES: &[&str] = &[
        "apple", "brave", "calm", "dark", "eager", "fancy", "golden",
        "happy", "ivory", "jolly", "keen", "lucky", "merry", "noble",
        "orange", "proud", "quiet", "rapid", "sharp", "tidy",
    ];
    const NOUNS: &[&str] = &[
        "banana", "cherry", "dragon", "eagle", "falcon", "garden",
        "hammer", "island", "jaguar", "knight", "lemon", "mango",
        "ninja", "ocean", "pilot", "queen", "river", "silver",
        "tiger", "union",
    ];

    let mut rng = OsRng;
    let adj = ADJECTIVES[rng.next_u32() as usize % ADJECTIVES.len()];
    let noun = NOUNS[rng.next_u32() as usize % NOUNS.len()];
    let num = rng.next_u32() % 1000;

    format!("{}-{}-{:03}", adj, noun, num)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code() {
        let code = generate_pairing_code();
        assert!(code.contains('-'));
        assert_eq!(code.split('-').count(), 3);
    }

    #[test]
    fn test_full_handshake() {
        let password = "apple-banana-123";

        // Alice initiates
        let (msg_a, state_a) = pake_initiate(password).unwrap();
        // Bob responds
        let (msg_b, state_b) = pake_respond(password).unwrap();

        // Both sides finish
        let session_a = pake_finish(state_a, &msg_b).unwrap();
        let session_b = pake_finish(state_b, &msg_a).unwrap();

        // Keys should match
        assert_eq!(session_a.shared_key, session_b.shared_key);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let password = "test-pairing-code-123";

        let (_msg_a, state_a) = pake_initiate(password).unwrap();
        let (msg_b, _state_b) = pake_respond(password).unwrap();

        let session = pake_finish(state_a, &msg_b).unwrap();

        let plaintext = b"Hello, Protide P2P! This is a secret message.";
        let encrypted = encrypt_message(&session, plaintext).unwrap();
        let decrypted = decrypt_message(&session, &encrypted).unwrap();

        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn test_different_passwords_dont_match() {
        let (msg_a, state_a) = pake_initiate("password-a").unwrap();
        let (msg_b, state_b) = pake_respond("password-b").unwrap();

        let session_a = pake_finish(state_a, &msg_b).unwrap();
        let session_b = pake_finish(state_b, &msg_a).unwrap();

        // Different passwords should produce different keys
        assert_ne!(session_a.shared_key, session_b.shared_key);
    }
}
