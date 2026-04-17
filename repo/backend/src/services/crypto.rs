use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::fmt;

/// Errors that can occur during cryptographic operations.
#[derive(Debug)]
pub enum CryptoError {
    InvalidKey,
    DecryptionFailed,
    InvalidFormat,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::InvalidKey => write!(f, "Invalid encryption key"),
            CryptoError::DecryptionFailed => write!(f, "Decryption failed"),
            CryptoError::InvalidFormat => write!(f, "Invalid ciphertext format"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// Configuration for AES-256-GCM encryption.
pub struct CryptoConfig {
    pub encryption_key: [u8; 32],
}

impl CryptoConfig {
    /// Build a `CryptoConfig` from the `ENCRYPTION_KEY` env var (or a default
    /// dev key).  The raw env value is hashed with SHA-256 to produce a
    /// deterministic 32-byte key.
    pub fn from_env() -> Self {
        let raw = std::env::var("ENCRYPTION_KEY")
            .unwrap_or_else(|_| "brewflow-dev-encryption-key".into());

        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        let hash = hasher.finalize();

        let mut encryption_key = [0u8; 32];
        encryption_key.copy_from_slice(&hash);

        CryptoConfig { encryption_key }
    }
}

/// Encrypt `plaintext` with AES-256-GCM.  A random 12-byte nonce is prepended
/// to the ciphertext and the whole blob is base64-encoded.
pub fn encrypt(config: &CryptoConfig, plaintext: &str) -> String {
    let key = Key::<Aes256Gcm>::from_slice(&config.encryption_key);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("AES-GCM encryption should not fail");

    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    B64.encode(combined)
}

/// Decrypt a value previously produced by [`encrypt`].
pub fn decrypt(config: &CryptoConfig, ciphertext_b64: &str) -> Result<String, CryptoError> {
    let combined = B64.decode(ciphertext_b64).map_err(|_| CryptoError::InvalidFormat)?;

    if combined.len() < 13 {
        return Err(CryptoError::InvalidFormat);
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let key = Key::<Aes256Gcm>::from_slice(&config.encryption_key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed)?;

    String::from_utf8(plaintext).map_err(|_| CryptoError::DecryptionFailed)
}

/// Mask a value for logging, showing only the first `visible_chars` characters
/// followed by `***`.  E.g. `mask_for_log("BF-A3XYZ", 4)` -> `"BF-A***"`.
pub fn mask_for_log(value: &str, visible_chars: usize) -> String {
    if value.len() <= visible_chars {
        return format!("{}***", value);
    }
    let visible: String = value.chars().take(visible_chars).collect();
    format!("{}***", visible)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CryptoConfig {
        CryptoConfig {
            encryption_key: [0xABu8; 32],
        }
    }

    // ── encrypt / decrypt ──────────────────────────────────────────────────

    #[test]
    fn encrypt_decrypt_round_trip() {
        let config = test_config();
        let plaintext = "BF-ABC123";
        let ciphertext = encrypt(&config, plaintext);
        let recovered = decrypt(&config, &ciphertext).unwrap();
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn encrypt_produces_different_ciphertext_each_call() {
        let config = test_config();
        let c1 = encrypt(&config, "hello");
        let c2 = encrypt(&config, "hello");
        // Random nonce means different ciphertext every time.
        assert_ne!(c1, c2);
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let config1 = test_config();
        let config2 = CryptoConfig {
            encryption_key: [0x11u8; 32],
        };
        let ciphertext = encrypt(&config1, "secret");
        assert!(decrypt(&config2, &ciphertext).is_err());
    }

    #[test]
    fn decrypt_invalid_base64_fails() {
        let config = test_config();
        assert!(matches!(decrypt(&config, "!!!notbase64"), Err(CryptoError::InvalidFormat)));
    }

    #[test]
    fn decrypt_too_short_fails() {
        let config = test_config();
        // 12 bytes of nonce minimum + at least 1 byte ciphertext; fewer bytes → InvalidFormat.
        let short = base64::engine::general_purpose::STANDARD.encode([0u8; 5]);
        assert!(decrypt(&config, &short).is_err());
    }

    // ── mask_for_log ───────────────────────────────────────────────────────

    #[test]
    fn mask_hides_suffix() {
        assert_eq!(mask_for_log("BF-A3XYZ", 4), "BF-A***");
    }

    #[test]
    fn mask_short_value_shows_all_plus_stars() {
        assert_eq!(mask_for_log("AB", 4), "AB***");
    }

    #[test]
    fn mask_exact_length_shows_all_plus_stars() {
        assert_eq!(mask_for_log("ABCD", 4), "ABCD***");
    }

    #[test]
    fn mask_zero_visible() {
        assert_eq!(mask_for_log("hello", 0), "***");
    }

    // ── extra encryption coverage ──────────────────────────────────────────

    #[test]
    fn encrypt_empty_string_round_trips() {
        let config = test_config();
        let enc = encrypt(&config, "");
        let dec = decrypt(&config, &enc).unwrap();
        assert_eq!(dec, "");
    }

    #[test]
    fn encrypt_unicode_payload_round_trips() {
        let config = test_config();
        let plain = "\u{53d6}\u{9910}\u{7801}-CN\u{1f4b0}";
        let enc = encrypt(&config, plain);
        let dec = decrypt(&config, &enc).unwrap();
        assert_eq!(dec, plain);
    }

    #[test]
    fn encrypt_long_payload_round_trips() {
        let config = test_config();
        let plain: String = std::iter::repeat('a').take(10_000).collect();
        let enc = encrypt(&config, &plain);
        let dec = decrypt(&config, &enc).unwrap();
        assert_eq!(dec, plain);
    }

    #[test]
    fn ciphertext_is_base64_safe() {
        let config = test_config();
        let enc = encrypt(&config, "BF-ABCDEF");
        // Standard base64 alphabet plus padding.
        assert!(
            enc.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='),
            "ciphertext must be valid base64: {}",
            enc
        );
    }

    #[test]
    fn decrypt_corrupted_ciphertext_fails() {
        let config = test_config();
        let enc = encrypt(&config, "payload");
        // Flip a byte in the middle of the ciphertext.
        let mut bytes = enc.into_bytes();
        let mid = bytes.len() / 2;
        bytes[mid] = bytes[mid].wrapping_add(1);
        let tampered = String::from_utf8(bytes).unwrap();
        let result = decrypt(&config, &tampered);
        assert!(result.is_err());
    }

    // ── CryptoConfig::from_env ─────────────────────────────────────────────

    #[test]
    fn crypto_config_from_env_key_has_sha256_length() {
        // std::env is process-global so we avoid mutating it in parallel
        // tests. Whatever value is present (or absent), the SHA-256 output is
        // always 32 bytes — that's the safe invariant we can assert.
        let cfg = CryptoConfig::from_env();
        assert_eq!(cfg.encryption_key.len(), 32);
    }

    // ── mask_for_log edge cases ────────────────────────────────────────────

    #[test]
    fn mask_is_stable_for_unicode() {
        // mask takes character count, not bytes, so two chars of "\u{4e2d}\u{6587}BF" is "\u{4e2d}\u{6587}***"
        assert_eq!(mask_for_log("\u{4e2d}\u{6587}BF", 2), "\u{4e2d}\u{6587}***");
    }

    #[test]
    fn mask_handles_empty_input() {
        assert_eq!(mask_for_log("", 4), "***");
    }
}
