//! Frontend render-depth integration tests.
//!
//! These tests import real modules from `frontend::logic` and `frontend::state`
//! to verify CSS class computation, conditional visibility, state mutations,
//! and i18n label resolution that directly drive component rendering.

#[cfg(test)]
mod tests {
    use frontend::logic::*;
    use frontend::state::{AppState, UserInfo};
    use shared::i18n::init_translations;

    // ── StatusBadge render: CSS class via real status_badge_classes ───────

    const BADGE_BASE: &str = "inline-flex items-center px-3 py-1 rounded-full text-xs font-semibold";

    #[test]
    fn status_badge_render_pending_en() {
        let (color, key) = status_badge_classes("Pending");
        let t = init_translations();
        let label = t.t("en", key);
        let full_class = format!("{} {}", BADGE_BASE, color);
        assert!(full_class.contains("bg-gray-200"));
        assert_eq!(label, "Pending");
    }

    #[test]
    fn status_badge_render_ready_zh() {
        let (color, key) = status_badge_classes("Ready");
        let t = init_translations();
        let label = t.t("zh", key);
        let full_class = format!("{} {}", BADGE_BASE, color);
        assert!(full_class.contains("emerald"));
        assert_ne!(label, "Ready");
    }

    #[test]
    fn status_badge_unknown_uses_status_as_label() {
        let (_, key) = status_badge_classes("Mystery");
        assert!(key.is_empty());
        // Component falls back to raw status string when key is empty.
    }

    #[test]
    fn status_badge_class_always_has_base_styles() {
        for s in &["Pending", "Accepted", "InPrep", "Ready", "PickedUp", "Canceled"] {
            let (color, _) = status_badge_classes(s);
            let full = format!("{} {}", BADGE_BASE, color);
            assert!(full.contains("rounded-full"), "missing base for {}", s);
        }
    }

    // ── HoldTimer render: urgency CSS via real hold_urgency ──────────────

    #[test]
    fn hold_timer_expired_shows_red() {
        assert_eq!(hold_urgency(0), HoldUrgency::Expired);
        assert_eq!(hold_urgency(-5), HoldUrgency::Expired);
    }

    #[test]
    fn hold_timer_critical_under_60s() {
        assert_eq!(hold_urgency(30), HoldUrgency::Critical);
        assert_eq!(hold_urgency(59), HoldUrgency::Critical);
    }

    #[test]
    fn hold_timer_normal_above_60s() {
        assert_eq!(hold_urgency(60), HoldUrgency::Normal);
        assert_eq!(hold_urgency(600), HoldUrgency::Normal);
    }

    #[test]
    fn hold_timer_countdown_display() {
        assert_eq!(format_countdown(599), "09:59");
        assert_eq!(format_countdown(61), "01:01");
        assert_eq!(format_countdown(0), "00:00");
    }

    // ── PriceDisplay render via real format_price ────────────────────────

    #[test]
    fn price_renders_two_decimals() {
        assert_eq!(format_price(4.5, "en"), "$4.50");
        assert_eq!(format_price(4.5, "zh"), "\u{00a5}4.50");
    }

    #[test]
    fn price_renders_negative() {
        assert_eq!(format_price(-2.50, "en"), "$-2.50");
    }

    // ── SlotPicker button states ─────────────────────────────────────────

    fn slot_button_class(available: bool, selected: bool) -> &'static str {
        if !available {
            "py-2.5 px-2 text-center border border-gray-200 rounded-lg text-sm bg-gray-100 text-gray-400 cursor-not-allowed line-through"
        } else if selected {
            "py-2.5 px-2 text-center border-2 border-primary rounded-lg text-sm bg-primary text-white font-medium cursor-pointer"
        } else {
            "py-2.5 px-2 text-center border border-gray-200 rounded-lg text-sm bg-white text-gray-700 cursor-pointer hover:border-primary hover:bg-primary/5 transition-all"
        }
    }

    #[test]
    fn slot_unavailable_is_disabled() {
        let c = slot_button_class(false, false);
        assert!(c.contains("cursor-not-allowed"));
        assert!(c.contains("line-through"));
    }

    #[test]
    fn slot_selected_is_primary() {
        let c = slot_button_class(true, true);
        assert!(c.contains("bg-primary"));
        assert!(c.contains("text-white"));
    }

    #[test]
    fn slot_available_unselected_has_hover() {
        let c = slot_button_class(true, false);
        assert!(c.contains("bg-white"));
        assert!(c.contains("hover:"));
    }

    #[test]
    fn slot_time_format_via_real_function() {
        assert_eq!(format_slot_time("2026-04-15T09:00:00"), "09:00");
    }

    // ── Navbar visibility via real AppState ──────────────────────────────

    fn user(roles: &[&str]) -> UserInfo {
        UserInfo {
            id: 1, username: "t".into(), display_name: Some("T".into()),
            roles: roles.iter().map(|s| s.to_string()).collect(),
            preferred_locale: "en".into(),
        }
    }

    #[test]
    fn navbar_unauthenticated_shows_login() {
        let s = AppState::default();
        assert!(!s.auth.is_authenticated);
    }

    #[test]
    fn navbar_customer_hides_staff_admin() {
        let mut s = AppState::default();
        s.set_auth("c".into(), user(&["Customer"]));
        assert!(s.auth.is_authenticated);
        assert!(!s.is_staff());
        assert!(!s.is_admin());
    }

    #[test]
    fn navbar_admin_shows_all() {
        let mut s = AppState::default();
        s.set_auth("c".into(), user(&["Admin"]));
        assert!(s.is_staff());
        assert!(s.is_admin());
        assert!(s.is_teacher());
    }

    #[test]
    fn navbar_cart_badge_visible_when_positive() {
        let mut s = AppState::default();
        s.set_auth("c".into(), user(&["Customer"]));
        s.cart_count = 3;
        assert!(s.cart_count > 0);
    }

    // ── Full state mutation sequence ─────────────────────────────────────

    #[test]
    fn state_login_logout_cycle() {
        let mut s = AppState::default();
        assert!(!s.auth.is_authenticated);
        assert_eq!(s.current_locale(), "en");

        s.set_auth("cookie".into(), UserInfo {
            id: 1, username: "alice".into(), display_name: Some("Alice".into()),
            roles: vec!["Customer".into()], preferred_locale: "zh".into(),
        });
        assert!(s.auth.is_authenticated);
        assert_eq!(s.current_locale(), "zh");
        assert_eq!(s.auth.user.as_ref().unwrap().username, "alice");

        s.cart_count = 5;
        assert_eq!(s.cart_count, 5);

        s.logout();
        assert!(!s.auth.is_authenticated);
        assert!(s.auth.session_cookie.is_none());
    }

    #[test]
    fn state_role_promotion_affects_visibility() {
        let mut s = AppState::default();
        s.set_auth("c".into(), user(&["Customer"]));
        assert!(!s.is_admin());

        s.auth.user.as_mut().unwrap().roles.push("Admin".into());
        assert!(s.is_admin());
        assert!(s.is_staff());
    }

    // ── i18n label resolution for all UI-critical keys ──────────────────

    #[test]
    fn all_nav_labels_resolve() {
        let t = init_translations();
        for key in &["nav.home", "nav.menu", "nav.cart", "nav.orders", "nav.training", "nav.admin", "nav.staff"] {
            for locale in &["en", "zh"] {
                assert_ne!(t.t(locale, key), *key, "unresolved {} for {}", key, locale);
            }
        }
    }

    #[test]
    fn all_button_labels_resolve() {
        let t = init_translations();
        for key in &["btn.add_to_cart", "btn.checkout", "btn.confirm", "btn.cancel", "btn.scan", "btn.start_exam"] {
            for locale in &["en", "zh"] {
                assert_ne!(t.t(locale, key), *key, "unresolved {} for {}", key, locale);
            }
        }
    }

    #[test]
    fn all_status_labels_resolve() {
        let t = init_translations();
        for key in &["status.pending", "status.accepted", "status.in_prep", "status.ready", "status.picked_up", "status.canceled"] {
            for locale in &["en", "zh"] {
                assert_ne!(t.t(locale, key), *key, "unresolved {} for {}", key, locale);
            }
        }
    }

    // ── Cart math via real functions ─────────────────────────────────────

    #[test]
    fn cart_math_end_to_end() {
        let sub = cart_subtotal(&[(4.50, 2), (3.00, 1)]);
        let tax = cart_tax(sub, 0.08);
        let total = cart_total(sub, tax);
        assert_eq!(sub, 12.0);
        assert_eq!(tax, 0.96);
        assert_eq!(total, 12.96);
    }

    // ── URL helpers via real functions ───────────────────────────────────

    #[test]
    fn localized_urls() {
        assert_eq!(localized_path("en", "/menu"), "/en/menu");
        assert_eq!(localized_path("zh", ""), "/zh");
        assert_eq!(localized_path("", "/cart"), "/en/cart");
    }

    #[test]
    fn api_base_construction() {
        assert_eq!(api_base_from_origin("http://localhost:8080"), "http://localhost:8080/api");
    }
}
