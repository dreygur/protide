#![cfg(feature = "pake-auth")]

use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use spake2::{Ed25519Group, Identity, Password, Spake2};

use rand::{Rng, RngCore};

const PAKE_IDENTITY: &[u8] = b"protide-p2p-pairing";

/// Pairing code vocabulary, kept apart from `types::ADJECTIVES` and
/// `types::NOUNS` so existing display names never change. Exactly 256
/// distinct words of 3 to 5 letters: each word carries 8 bits and a whole
/// code still fits the pairing flyout.
const CODE_WORDS: [&str; 256] = [
    "acorn", "actor", "agent", "alarm", "album", "alpha", "amber", "angel", "apple", "arena",
    "arrow", "atlas", "bacon", "badge", "baker", "basil", "beach", "berry", "bison", "blade",
    "bloom", "board", "bread", "brick", "brook", "cabin", "cable", "camel", "candy", "canoe",
    "cargo", "cedar", "chalk", "chess", "cliff", "clock", "cloud", "comet", "coral", "crane",
    "crown", "daisy", "dance", "delta", "denim", "diary", "donut", "drama", "dream", "drift",
    "drum", "dune", "eagle", "earth", "easel", "elbow", "elder", "ember", "epoch", "essay",
    "fable", "fancy", "feast", "ferry", "fiber", "field", "flame", "flute", "focus", "forge",
    "fox", "frost", "gamma", "gecko", "ghost", "giant", "glass", "globe", "glove", "gold", "goose",
    "grape", "grass", "guide", "hazel", "heart", "hedge", "heron", "hill", "honey", "horse",
    "hotel", "husky", "icon", "igloo", "index", "inlet", "iris", "iron", "ivory", "ivy", "jazz",
    "jelly", "jewel", "joker", "judge", "juice", "jumbo", "kayak", "kebab", "kiosk", "kite",
    "kiwi", "knife", "koala", "label", "lake", "lamp", "laser", "latte", "lava", "lemon", "lime",
    "linen", "lion", "llama", "lotus", "lunar", "magic", "mango", "maple", "march", "medal",
    "melon", "metal", "mint", "moose", "motor", "mouse", "music", "nacho", "navy", "nest", "night",
    "ninja", "noble", "north", "novel", "nylon", "oasis", "ocean", "olive", "omega", "onion",
    "opera", "orbit", "otter", "owl", "panda", "paper", "pasta", "peach", "pearl", "penny",
    "piano", "pilot", "pine", "pixel", "pizza", "plaza", "polar", "prism", "puppy", "quail",
    "queen", "quest", "quiet", "quill", "quilt", "quiz", "radar", "radio", "rain", "ranch",
    "raven", "rhino", "ridge", "river", "robin", "robot", "rose", "ruby", "sail", "salad", "salsa",
    "scarf", "scout", "shark", "shell", "slate", "sloth", "snail", "snow", "solar", "spark",
    "spice", "squid", "stone", "storm", "sugar", "table", "tango", "tempo", "tent", "tiger",
    "toast", "token", "topaz", "torch", "tower", "trail", "train", "tulip", "ultra", "uncle",
    "union", "unity", "urban", "vapor", "vault", "venus", "verse", "video", "villa", "viola",
    "viper", "vocal", "wafer", "wagon", "waltz", "water", "whale", "wheat", "wolf", "world",
    "wren", "xenon", "yacht", "yeti", "yoga", "yolk", "zebra", "zero", "zesty", "zinc", "zone",
];

/// Words per pairing code.
const CODE_WORD_COUNT: usize = 4;

/// The numeric suffix is drawn from `0..CODE_NUMBER_RANGE` (three digits).
const CODE_NUMBER_RANGE: u32 = 1000;

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

/// Which side of the handshake a key confirmation comes from.
///
/// SPAKE2's `finish()` only checks that the peer's message is a well formed
/// curve point; it never proves both sides derived the same key. Each side
/// therefore sends a value only the holder of that key can compute. The
/// value is bound to the sender's role, so a side never accepts its own
/// confirmation reflected back at it: the initiator (who typed the code)
/// sends the initiator value and the responder (who shows it) the
/// responder value, and each checks the other role's value.
#[derive(Debug, Clone, Copy)]
pub enum Role {
    Initiator,
    Responder,
}

impl Role {
    fn confirm_context(self) -> &'static str {
        match self {
            Role::Initiator => "protide pairing confirm initiator",
            Role::Responder => "protide pairing confirm responder",
        }
    }
}

/// Key confirmation value for `role`, derived from the shared key.
fn confirm_value(session: &PakeSession, role: Role) -> blake3::Hash {
    blake3::Hash::from(blake3::derive_key(
        role.confirm_context(),
        &session.shared_key,
    ))
}

/// This side's key confirmation blob, sent to the peer as `role`.
pub fn confirm_blob(session: &PakeSession, role: Role) -> Vec<u8> {
    confirm_value(session, role).as_bytes().to_vec()
}

/// Whether `blob` is the peer's confirmation for `role` under our derived
/// key, i.e. the peer derived the same key and sent it as that role.
/// `blake3::Hash` compares in constant time.
pub fn verify_confirm(session: &PakeSession, role: Role, blob: &[u8]) -> bool {
    <[u8; 32]>::try_from(blob)
        .is_ok_and(|bytes| blake3::Hash::from(bytes) == confirm_value(session, role))
}

/// Generate a pairing code that reads aloud easily, such as
/// "amber-otter-river-tiger-482" (Magic Wormhole style).
///
/// The code is both the PAKE password and the seed of the gossip topic, so
/// it must resist enumeration: four words of 8 bits plus three digits give
/// 256^4 * 1000 codes, about 2^41.97.
pub fn generate_pairing_code() -> String {
    let mut rng = OsRng;
    let mut parts: Vec<String> = (0..CODE_WORD_COUNT)
        .map(|_| CODE_WORDS[rng.gen_range(0..CODE_WORDS.len())].to_string())
        .collect();
    parts.push(format!("{:03}", rng.gen_range(0..CODE_NUMBER_RANGE)));
    parts.join("-")
}

/// Canonical form of a typed pairing code: lowercase words joined by single
/// hyphens, so "Amber Otter  river TIGER 482" pairs with the generated code.
pub fn normalize_pairing_code(input: &str) -> String {
    input
        .split(|c: char| c.is_whitespace() || c == '-')
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_code_is_four_known_words_and_three_digits() {
        for _ in 0..50 {
            let code = generate_pairing_code();
            let parts: Vec<&str> = code.split('-').collect();
            assert_eq!(parts.len(), CODE_WORD_COUNT + 1, "{code}");
            assert!(
                parts[..CODE_WORD_COUNT]
                    .iter()
                    .all(|w| CODE_WORDS.contains(w))
            );
            let number = parts[CODE_WORD_COUNT];
            assert!(number.len() == 3 && number.chars().all(|c| c.is_ascii_digit()));
            assert_eq!(normalize_pairing_code(&code), code, "already canonical");
        }
    }

    /// The code space must stay out of enumeration reach: at least 2^40.
    #[test]
    fn the_code_space_holds_at_least_forty_bits() {
        let unique: std::collections::HashSet<_> = CODE_WORDS.iter().collect();
        assert_eq!(unique.len(), CODE_WORDS.len(), "duplicate code word");
        assert!(
            CODE_WORDS.iter().all(|w| {
                (3..=5).contains(&w.len()) && w.chars().all(|c| c.is_ascii_lowercase())
            })
        );
        let bits = CODE_WORD_COUNT as f64 * (CODE_WORDS.len() as f64).log2()
            + f64::from(CODE_NUMBER_RANGE).log2();
        assert!(bits >= 40.0, "{bits} bits");
    }

    #[test]
    fn typed_codes_ignore_case_and_spaces_versus_hyphens() {
        let canonical = "amber-otter-river-tiger-482";
        for typed in [
            canonical,
            "Amber Otter River Tiger 482",
            "  AMBER-otter  river - tiger 482\n",
        ] {
            assert_eq!(normalize_pairing_code(typed), canonical, "{typed:?}");
        }
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
        // confirm blob only verifies under the SAME derived key.
        let confirm_a = confirm_blob(&session_a, Role::Responder);
        let confirm_b = confirm_blob(&session_b, Role::Initiator);

        assert!(
            !verify_confirm(&session_b, Role::Responder, &confirm_a),
            "B must reject A's confirmation blob - different codes were used"
        );
        assert!(
            !verify_confirm(&session_a, Role::Initiator, &confirm_b),
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

        let confirm_a = confirm_blob(&session_a, Role::Responder);
        let confirm_b = confirm_blob(&session_b, Role::Initiator);

        assert!(verify_confirm(&session_b, Role::Responder, &confirm_a));
        assert!(verify_confirm(&session_a, Role::Initiator, &confirm_b));

        // A side's own confirmation never passes as the peer's role.
        assert_ne!(confirm_a, confirm_b);
        assert!(!verify_confirm(&session_a, Role::Initiator, &confirm_a));
        assert!(!verify_confirm(&session_b, Role::Responder, &confirm_b));
    }
}
