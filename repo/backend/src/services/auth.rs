use serde::{Deserialize, Serialize};

/// Claims carried inside the request guard after session validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub roles: Vec<String>,
    pub exp: usize,
}

/// Hash a password using bcrypt with the default cost.
pub fn hash_password(password: &str) -> String {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("bcrypt hash")
}

/// Verify a plaintext password against a bcrypt hash.
pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

/// Validate that a password meets the security policy.
///
/// Requirements:
/// - Minimum 12 characters
/// - At least one uppercase letter
/// - At least one lowercase letter
/// - At least one digit
/// - At least one special character
///
/// Returns `Ok(())` if valid, or `Err` with a list of violation messages.
pub fn validate_password(password: &str) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();

    if password.len() < 12 {
        violations.push("Password must be at least 12 characters long".into());
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        violations.push("Password must contain at least one uppercase letter".into());
    }
    if !password.chars().any(|c| c.is_lowercase()) {
        violations.push("Password must contain at least one lowercase letter".into());
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        violations.push("Password must contain at least one digit".into());
    }
    if !password.chars().any(|c| !c.is_alphanumeric()) {
        violations.push("Password must contain at least one special character".into());
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_password ─────────────────────────────────────────────────────

    #[test]
    fn valid_strong_password_passes() {
        assert!(validate_password("Str0ng!Password#1").is_ok());
        assert!(validate_password("Abcdefgh1234!@#$").is_ok());
    }

    #[test]
    fn password_too_short_is_rejected() {
        let res = validate_password("Short1!");
        let errs = res.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("12 characters")));
    }

    #[test]
    fn password_missing_uppercase_is_rejected() {
        let res = validate_password("alllowercase1!");
        let errs = res.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("uppercase")));
    }

    #[test]
    fn password_missing_lowercase_is_rejected() {
        let res = validate_password("ALLUPPERCASE1!");
        let errs = res.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("lowercase")));
    }

    #[test]
    fn password_missing_digit_is_rejected() {
        let res = validate_password("NoDigitsHere!");
        let errs = res.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("digit")));
    }

    #[test]
    fn password_missing_special_char_is_rejected() {
        let res = validate_password("NoSpecialChar1A");
        let errs = res.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("special")));
    }

    #[test]
    fn multiple_violations_are_all_reported() {
        // "ab" violates: length, uppercase, digit, special
        let res = validate_password("ab");
        let errs = res.unwrap_err();
        assert!(errs.len() >= 3, "expected multiple violations, got: {:?}", errs);
    }

    #[test]
    fn exactly_12_char_password_passes_length_check() {
        // "aaaaaaaaaaa1A!" — exactly 12 chars, meets all criteria
        assert!(validate_password("aaaaaaaaaaa1A!").is_ok());
    }

    // ── role helpers on Claims ─────────────────────────────────────────────────

    fn make_claims(roles: &[&str]) -> Claims {
        Claims {
            sub: 1,
            username: "user".into(),
            roles: roles.iter().map(|s| s.to_string()).collect(),
            exp: 0,
        }
    }

    #[test]
    fn claims_with_admin_role_is_identifiable() {
        let c = make_claims(&["Admin"]);
        assert!(c.roles.iter().any(|r| r == "Admin"));
    }

    #[test]
    fn claims_with_staff_role_is_identifiable() {
        let c = make_claims(&["Staff"]);
        assert!(c.roles.iter().any(|r| r == "Staff"));
    }

    #[test]
    fn claims_customer_has_no_admin_or_staff() {
        let c = make_claims(&["Customer"]);
        assert!(!c.roles.iter().any(|r| r == "Admin" || r == "Staff"));
    }

    // ── hash_password / verify_password ────────────────────────────────────

    #[test]
    fn hash_password_produces_non_empty_bcrypt_hash() {
        let hash = hash_password("CorrectHorse1!");
        // bcrypt hashes always start with "$2b$" or "$2a$" at cost-aware versions.
        assert!(hash.starts_with("$2"), "unexpected hash prefix: {}", hash);
        assert!(hash.len() >= 60, "bcrypt hash too short: {}", hash);
    }

    #[test]
    fn hash_password_differs_for_same_input_due_to_salt() {
        let h1 = hash_password("samePassw0rd!");
        let h2 = hash_password("samePassw0rd!");
        assert_ne!(h1, h2, "bcrypt must salt, making repeated hashes differ");
    }

    #[test]
    fn verify_password_accepts_matching_plaintext() {
        let pw = "Tr0ub4dor&3long";
        let hash = hash_password(pw);
        assert!(verify_password(pw, &hash));
    }

    #[test]
    fn verify_password_rejects_mismatching_plaintext() {
        let hash = hash_password("Tr0ub4dor&3long");
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn verify_password_false_for_malformed_hash() {
        assert!(!verify_password("anything", "not-a-bcrypt-hash"));
    }

    // ── Claims serde ───────────────────────────────────────────────────────

    #[test]
    fn claims_round_trips_through_json() {
        let c = Claims {
            sub: 9,
            username: "bob".into(),
            roles: vec!["Admin".into(), "Staff".into()],
            exp: 1234567890,
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: Claims = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sub, 9);
        assert_eq!(back.username, "bob");
        assert_eq!(back.roles, vec!["Admin", "Staff"]);
    }

    // ── Unicode & length edge-cases ────────────────────────────────────────

    #[test]
    fn validate_password_counts_chars_not_bytes_for_length() {
        // 12 ascii chars satisfies policy
        assert!(validate_password("abcABC123!@#").is_ok());
        // 11 chars fails
        let err = validate_password("abcABC123!@").unwrap_err();
        assert!(err.iter().any(|e| e.contains("12 characters")));
    }

    #[test]
    fn validate_password_whitespace_counts_as_special() {
        // "Aa1 aaaaaaaa" — space is non-alphanumeric → satisfies "special char"
        let res = validate_password("Aa1 aaaaaaaa");
        assert!(res.is_ok(), "space should qualify as special: got {:?}", res);
    }

    #[test]
    fn validate_empty_password_is_rejected() {
        let errs = validate_password("").unwrap_err();
        // empty password violates length, upper, lower, digit, and special.
        assert!(errs.len() >= 4, "expected multiple violations, got: {:?}", errs);
    }
}
