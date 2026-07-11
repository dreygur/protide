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

/// Fixed plaintext used for post-SPAKE2 key confirmation.
///
/// SPAKE2's `finish()` only checks that the peer's message is a well-formed
/// curve point - it never proves the two sides actually derived the *same*
/// key (that only happens if both used the same password/pairing code). To
/// get a real proof, each side encrypts this constant under its own derived
/// key (AEAD via ChaCha20Poly1305, see `encrypt_message`) and sends the
/// ciphertext to the peer. The peer can only decrypt it successfully -
/// ChaCha20Poly1305's Poly1305 tag check is constant-time - if it derived
/// the identical key, which only happens when both sides used the same
/// password. This is the standard "encrypt a known value, verify decryption"
/// key-confirmation pattern (cf. TLS Finished, Noise handshake payloads).
const CONFIRM_TAG: &[u8] = b"protide-pake-confirm";

/// Build this side's key-confirmation blob to send to the peer.
pub fn confirm_blob(session: &PakeSession) -> Result<Vec<u8>, String> {
    encrypt_message(session, CONFIRM_TAG)
}

/// Verify a confirmation blob received from the peer against our own
/// derived key. Returns `true` only if it decrypts (AEAD-authenticated) to
/// the expected constant - i.e. the peer derived the same key we did. The
/// authentication check inside `decrypt_message` is constant-time; the final
/// equality against the public constant compares already-authenticated,
/// non-secret data, so no additional constant-time handling is needed there.
pub fn verify_confirm(session: &PakeSession, blob: &[u8]) -> bool {
    match decrypt_message(session, blob) {
        Ok(pt) => pt == CONFIRM_TAG,
        Err(_) => false,
    }
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

    /// `engine.rs`'s PAKE handling (`sync/engine.rs::poll`) used to treat
    /// `pake_finish(..).is_ok()` as proof the peer knew the correct pairing
    /// code and fired `SyncEvent::HandshakeComplete`. But SPAKE2's `finish()`
    /// only validates that the peer's message is a well-formed curve point
    /// of the right length/side byte (see spake2 0.4.0 `finish()`) - it never
    /// checks the two sides actually agree on the password. `finish()` alone
    /// still reports `Ok(_)` (with different derived keys) even when two
    /// peers used completely different pairing codes - this test documents
    /// that raw-SPAKE2 fact - but the engine no longer gates
    /// `HandshakeComplete` on `finish()` alone: it now requires an explicit
    /// `confirm_blob`/`verify_confirm` round-trip (see below), which
    /// correctly rejects mismatched codes.
    #[test]
    fn test_key_confirmation_rejects_mismatched_codes() {
        let (msg_a, state_a) = pake_initiate("correct-horse-battery").unwrap();
        // Attacker/mismatched peer uses a totally different code, unrelated
        // to the real pairing code.
        let (msg_b, state_b) = pake_respond("totally-different-code").unwrap();

        let session_a = pake_finish(state_a, &msg_b).unwrap();
        let session_b = pake_finish(state_b, &msg_a).unwrap();

        // Raw SPAKE2 finish() succeeds even though the codes differ, and the
        // resulting keys differ - this is the vulnerability: `.is_ok()` is
        // not proof of a shared secret.
        assert_ne!(session_a.shared_key, session_b.shared_key);

        // Explicit key confirmation must catch the mismatch: each side's
        // confirm blob only decrypts correctly under the SAME derived key.
        let confirm_a = confirm_blob(&session_a).unwrap();
        let confirm_b = confirm_blob(&session_b).unwrap();

        assert!(
            !verify_confirm(&session_b, &confirm_a),
            "B must reject A's confirmation blob - different codes were used"
        );
        assert!(
            !verify_confirm(&session_a, &confirm_b),
            "A must reject B's confirmation blob - different codes were used"
        );
    }

    /// Sanity check for the happy path: two sides that used the SAME pairing
    /// code produce confirm blobs that verify successfully against each
    /// other's session.
    #[test]
    fn test_key_confirmation_accepts_matching_codes() {
        let password = "apple-banana-123";
        let (msg_a, state_a) = pake_initiate(password).unwrap();
        let (msg_b, state_b) = pake_respond(password).unwrap();

        let session_a = pake_finish(state_a, &msg_b).unwrap();
        let session_b = pake_finish(state_b, &msg_a).unwrap();

        let confirm_a = confirm_blob(&session_a).unwrap();
        let confirm_b = confirm_blob(&session_b).unwrap();

        assert!(verify_confirm(&session_b, &confirm_a));
        assert!(verify_confirm(&session_a, &confirm_b));
    }
}
