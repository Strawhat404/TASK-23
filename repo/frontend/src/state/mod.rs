use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    pub auth: AuthState,
    pub locale: String,
    pub cart_count: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthState {
    /// Signed `brewflow_session` cookie value returned by the login endpoint.
    /// Attached as `Cookie: brewflow_session=<value>` on every API request.
    pub session_cookie: Option<String>,
    pub user: Option<UserInfo>,
    pub is_authenticated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub roles: Vec<String>,
    pub preferred_locale: String,
}

impl AppState {
    pub fn current_locale(&self) -> &str {
        if self.locale.is_empty() {
            "en"
        } else {
            &self.locale
        }
    }

    pub fn is_staff(&self) -> bool {
        self.auth
            .user
            .as_ref()
            .map(|u| u.roles.iter().any(|r| r == "Staff" || r == "Admin"))
            .unwrap_or(false)
    }

    pub fn is_admin(&self) -> bool {
        self.auth
            .user
            .as_ref()
            .map(|u| u.roles.iter().any(|r| r == "Admin"))
            .unwrap_or(false)
    }

    pub fn is_teacher(&self) -> bool {
        self.auth
            .user
            .as_ref()
            .map(|u| {
                u.roles
                    .iter()
                    .any(|r| r == "Teacher" || r == "AcademicAffairs" || r == "Admin")
            })
            .unwrap_or(false)
    }

    pub fn set_auth(&mut self, session_cookie: String, user: UserInfo) {
        self.locale = user.preferred_locale.clone();
        self.auth = AuthState {
            session_cookie: Some(session_cookie),
            user: Some(user),
            is_authenticated: true,
        };
    }

    pub fn logout(&mut self) {
        self.auth = AuthState::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_user(roles: &[&str]) -> UserInfo {
        UserInfo {
            id: 1,
            username: "alice".into(),
            display_name: Some("Alice".into()),
            roles: roles.iter().map(|s| s.to_string()).collect(),
            preferred_locale: "en".into(),
        }
    }

    // ── defaults ───────────────────────────────────────────────────────────

    #[test]
    fn default_state_is_unauthenticated_and_english_fallback() {
        let s = AppState::default();
        assert!(!s.auth.is_authenticated);
        assert!(s.auth.session_cookie.is_none());
        assert!(s.auth.user.is_none());
        assert_eq!(s.cart_count, 0);
        // Empty locale falls back to "en".
        assert_eq!(s.current_locale(), "en");
    }

    #[test]
    fn current_locale_returns_explicit_value_when_set() {
        let mut s = AppState::default();
        s.locale = "zh".into();
        assert_eq!(s.current_locale(), "zh");
    }

    // ── role helpers ───────────────────────────────────────────────────────

    #[test]
    fn is_staff_true_for_staff_role() {
        let mut s = AppState::default();
        s.auth.user = Some(sample_user(&["Staff"]));
        assert!(s.is_staff());
    }

    #[test]
    fn is_staff_true_for_admin_role() {
        let mut s = AppState::default();
        s.auth.user = Some(sample_user(&["Admin"]));
        assert!(s.is_staff());
    }

    #[test]
    fn is_staff_false_for_customer_role() {
        let mut s = AppState::default();
        s.auth.user = Some(sample_user(&["Customer"]));
        assert!(!s.is_staff());
    }

    #[test]
    fn is_staff_false_when_no_user() {
        let s = AppState::default();
        assert!(!s.is_staff());
    }

    #[test]
    fn is_admin_true_only_for_admin() {
        let mut s = AppState::default();
        s.auth.user = Some(sample_user(&["Staff"]));
        assert!(!s.is_admin());
        s.auth.user = Some(sample_user(&["Admin"]));
        assert!(s.is_admin());
    }

    #[test]
    fn is_teacher_recognises_three_privileged_roles() {
        for role in &["Teacher", "AcademicAffairs", "Admin"] {
            let mut s = AppState::default();
            s.auth.user = Some(sample_user(&[role]));
            assert!(s.is_teacher(), "{} should be a teacher", role);
        }
    }

    #[test]
    fn is_teacher_false_for_customer_and_staff() {
        for role in &["Customer", "Staff"] {
            let mut s = AppState::default();
            s.auth.user = Some(sample_user(&[role]));
            assert!(!s.is_teacher(), "{} should not be a teacher", role);
        }
    }

    #[test]
    fn multi_role_user_matches_any_matching_predicate() {
        let mut s = AppState::default();
        s.auth.user = Some(sample_user(&["Customer", "Teacher"]));
        assert!(s.is_teacher());
        assert!(!s.is_admin());
        assert!(!s.is_staff());
    }

    // ── set_auth / logout ──────────────────────────────────────────────────

    #[test]
    fn set_auth_populates_session_and_user_and_propagates_locale() {
        let mut s = AppState::default();
        let user = UserInfo {
            preferred_locale: "zh".into(),
            ..sample_user(&["Customer"])
        };
        s.set_auth("signed-cookie".into(), user);
        assert!(s.auth.is_authenticated);
        assert_eq!(s.auth.session_cookie.as_deref(), Some("signed-cookie"));
        assert_eq!(s.locale, "zh", "preferred_locale must apply on login");
        assert_eq!(s.current_locale(), "zh");
    }

    #[test]
    fn logout_clears_authentication_state() {
        let mut s = AppState::default();
        s.set_auth("c".into(), sample_user(&["Admin"]));
        s.logout();
        assert!(!s.auth.is_authenticated);
        assert!(s.auth.session_cookie.is_none());
        assert!(s.auth.user.is_none());
    }

    #[test]
    fn logout_preserves_locale_for_next_login() {
        // Logout only wipes auth — the user's preferred locale for the UI
        // should persist so the login page is still in the right language.
        let mut s = AppState::default();
        s.set_auth("c".into(), UserInfo {
            preferred_locale: "zh".into(),
            ..sample_user(&["Customer"])
        });
        s.logout();
        assert_eq!(s.locale, "zh");
    }

    // ── serde ──────────────────────────────────────────────────────────────

    #[test]
    fn app_state_round_trips_through_json() {
        let mut s = AppState::default();
        s.cart_count = 3;
        s.set_auth("cookie".into(), sample_user(&["Customer"]));
        let json = serde_json::to_string(&s).unwrap();
        let back: AppState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cart_count, 3);
        assert!(back.auth.is_authenticated);
        assert_eq!(back.auth.user.as_ref().unwrap().username, "alice");
    }
}
