use chrono::NaiveDateTime;
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Configuration for cookie-based session management.
pub struct SessionConfig {
    pub cookie_secret: [u8; 32],
    /// Idle timeout in seconds (default 1800 = 30 min).
    pub idle_timeout_secs: u64,
    /// How often session IDs are rotated (default 300 = 5 min).
    pub rotation_interval_secs: u64,
}

impl SessionConfig {
    /// Build from the `COOKIE_SECRET` env var (or a default dev secret).
    /// The raw value is hashed with SHA-256 to produce a 32-byte key.
    pub fn from_env() -> Self {
        let raw = std::env::var("COOKIE_SECRET")
            .unwrap_or_else(|_| "brewflow-dev-cookie-secret".into());

        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(raw.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();

        SessionConfig {
            cookie_secret: hash,
            idle_timeout_secs: 1800,
            rotation_interval_secs: 300,
        }
    }
}

/// Generate a cryptographically-random 32-byte hex session ID.
pub fn create_session_id() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    hex::encode(bytes)
}

/// Sign a session ID with HMAC-SHA256 and return `session_id.signature`.
pub fn sign_cookie(config: &SessionConfig, session_id: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(&config.cookie_secret).expect("HMAC accepts any key length");
    mac.update(session_id.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());
    format!("{}.{}", session_id, signature)
}

/// Verify a signed cookie value (`session_id.signature`).
/// Returns the session_id if the HMAC is valid, `None` otherwise.
pub fn verify_cookie(config: &SessionConfig, cookie_value: &str) -> Option<String> {
    let (session_id, signature) = cookie_value.rsplit_once('.')?;
    let sig_bytes = hex::decode(signature).ok()?;

    let mut mac =
        HmacSha256::new_from_slice(&config.cookie_secret).expect("HMAC accepts any key length");
    mac.update(session_id.as_bytes());
    mac.verify_slice(&sig_bytes).ok()?;

    Some(session_id.to_string())
}

/// Returns `true` if the session should be rotated (i.e. `last_rotated` is
/// older than `rotation_interval_secs`).
pub fn should_rotate(last_rotated: NaiveDateTime, config: &SessionConfig) -> bool {
    let now = chrono::Utc::now().naive_utc();
    let elapsed = now.signed_duration_since(last_rotated);
    elapsed.num_seconds() as u64 >= config.rotation_interval_secs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> SessionConfig {
        SessionConfig {
            cookie_secret: [42u8; 32],
            idle_timeout_secs: 1800,
            rotation_interval_secs: 300,
        }
    }

    #[test]
    fn sign_then_verify_round_trip() {
        let config = test_config();
        let session_id = "deadbeef1234567890abcdef";
        let signed = sign_cookie(&config, session_id);
        let recovered = verify_cookie(&config, &signed);
        assert_eq!(recovered, Some(session_id.to_string()));
    }

    #[test]
    fn verify_tampered_signature_fails() {
        let config = test_config();
        let signed = sign_cookie(&config, "some-session-id");
        // Flip the last character of the signature.
        let mut tampered = signed.clone();
        let last = tampered.pop().unwrap();
        tampered.push(if last == 'a' { 'b' } else { 'a' });
        assert_eq!(verify_cookie(&config, &tampered), None);
    }

    #[test]
    fn verify_wrong_key_fails() {
        let config1 = test_config();
        let config2 = SessionConfig {
            cookie_secret: [7u8; 32],
            idle_timeout_secs: 1800,
            rotation_interval_secs: 300,
        };
        let signed = sign_cookie(&config1, "abc");
        assert_eq!(verify_cookie(&config2, &signed), None);
    }

    #[test]
    fn verify_missing_dot_separator_fails() {
        let config = test_config();
        assert_eq!(verify_cookie(&config, "nodothere"), None);
    }

    #[test]
    fn should_rotate_old_session() {
        let config = test_config();
        // A timestamp well in the past should require rotation.
        let old = chrono::Utc::now().naive_utc() - chrono::Duration::seconds(600);
        assert!(should_rotate(old, &config));
    }

    #[test]
    fn should_not_rotate_recent_session() {
        let config = test_config();
        // A timestamp from 10 seconds ago should NOT require rotation.
        let recent = chrono::Utc::now().naive_utc() - chrono::Duration::seconds(10);
        assert!(!should_rotate(recent, &config));
    }

    // ── session ID generation ───────────────────────────────────────────────

    #[test]
    fn session_id_is_64_hex_chars() {
        let id = create_session_id();
        assert_eq!(id.len(), 64, "32 bytes → 64 hex chars, got: {}", id);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn session_ids_are_unique() {
        let ids: std::collections::HashSet<_> =
            (0..50).map(|_| create_session_id()).collect();
        assert_eq!(ids.len(), 50);
    }

    #[test]
    fn signed_cookie_format_has_single_dot_at_end() {
        let config = test_config();
        let signed = sign_cookie(&config, "id-abc");
        assert!(
            signed.matches('.').count() >= 1,
            "cookie must contain at least one dot separator: {}",
            signed
        );
        // With rsplit_once('.') the verifier handles session IDs that contain dots.
        let id_with_dots = "a.b.c.d";
        let signed2 = sign_cookie(&config, id_with_dots);
        assert_eq!(verify_cookie(&config, &signed2), Some(id_with_dots.to_string()));
    }

    #[test]
    fn verify_cookie_rejects_non_hex_signature() {
        let config = test_config();
        let bogus = format!("some-id.NOT-HEX-!!");
        assert_eq!(verify_cookie(&config, &bogus), None);
    }

    #[test]
    fn sign_is_deterministic_for_same_key_and_id() {
        let config = test_config();
        assert_eq!(sign_cookie(&config, "same"), sign_cookie(&config, "same"));
    }

    #[test]
    fn sign_differs_for_different_session_ids() {
        let config = test_config();
        assert_ne!(sign_cookie(&config, "a"), sign_cookie(&config, "b"));
    }

    // ── SessionConfig::from_env ────────────────────────────────────────────

    #[test]
    fn session_config_has_30_minute_idle_and_32_byte_key() {
        // The constants are not env-driven, so this assertion holds
        // regardless of any COOKIE_SECRET value another test may have set.
        let cfg = SessionConfig::from_env();
        assert_eq!(cfg.idle_timeout_secs, 1800);
        assert_eq!(cfg.rotation_interval_secs, 300);
        assert_eq!(cfg.cookie_secret.len(), 32);
    }
}
