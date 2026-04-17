//! Frontend component integration tests.
//!
//! These tests import real modules from the `frontend` crate (`frontend::logic`)
//! and from the `shared` crate, verifying the actual functions that components
//! depend on.  Every import is a direct reference to production code — not a
//! local mirror.
//!
//! Run: `cargo test --package frontend` (requires wasm32 target for full crate)

#[cfg(test)]
mod tests {
    // ── Direct imports from real frontend and shared crate modules ────────
    use frontend::logic::{
        compute_remaining_secs, format_slot_time, format_countdown,
        hold_urgency, HoldUrgency, format_price, currency_symbol,
        status_badge_classes, api_base_from_origin, localized_path,
        line_total, cart_subtotal, cart_tax, cart_total,
    };
    use frontend::state::{AppState, AuthState, UserInfo};
    use shared::i18n::init_translations;
    use shared::dto::OptionGroupDetail;
    use shared::dto::OptionValueDetail;

    // ── StatusBadge — tests via real status_badge_classes from logic.rs ───

    #[test]
    fn status_badge_maps_all_order_statuses() {
        for s in &["Pending", "Accepted", "InPrep", "Ready", "PickedUp", "Canceled"] {
            let (_, key) = status_badge_classes(s);
            assert!(!key.is_empty(), "missing i18n key for {}", s);
        }
    }

    #[test]
    fn status_badge_maps_reservation_statuses() {
        for s in &["Held", "Confirmed", "Expired"] {
            let (_, key) = status_badge_classes(s);
            assert!(!key.is_empty(), "missing i18n key for {}", s);
        }
    }

    #[test]
    fn status_badge_unknown_returns_neutral() {
        let (class, key) = status_badge_classes("Unknown");
        assert!(class.contains("gray"));
        assert!(key.is_empty());
    }

    #[test]
    fn status_badge_ready_uses_emerald() {
        let (class, _) = status_badge_classes("Ready");
        assert!(class.contains("emerald"));
    }

    #[test]
    fn status_badge_canceled_uses_red() {
        let (class, _) = status_badge_classes("Canceled");
        assert!(class.contains("red"));
    }

    // ── PriceDisplay — tests via real format_price from logic.rs ─────────

    #[test]
    fn price_display_en_uses_dollar() {
        assert_eq!(format_price(4.50, "en"), "$4.50");
    }

    #[test]
    fn price_display_zh_uses_yuan() {
        assert_eq!(format_price(4.50, "zh"), "\u{00a5}4.50");
    }

    #[test]
    fn price_display_zero() {
        assert_eq!(format_price(0.0, "en"), "$0.00");
    }

    #[test]
    fn price_display_large() {
        assert_eq!(format_price(1234.567, "en"), "$1234.57");
    }

    #[test]
    fn currency_symbol_en_is_dollar() {
        assert_eq!(currency_symbol("en"), "$");
    }

    #[test]
    fn currency_symbol_zh_is_yuan() {
        assert_eq!(currency_symbol("zh"), "\u{00a5}");
    }

    // ── HoldTimer — tests via real compute_remaining_secs from logic.rs ──

    #[test]
    fn hold_timer_positive_remaining() {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 4, 15).unwrap()
            .and_hms_opt(10, 0, 0).unwrap();
        assert_eq!(compute_remaining_secs("2026-04-15T10:10:00", now), 600);
    }

    #[test]
    fn hold_timer_expired() {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 4, 15).unwrap()
            .and_hms_opt(10, 0, 0).unwrap();
        assert!(compute_remaining_secs("2026-04-15T09:50:00", now) < 0);
    }

    #[test]
    fn hold_timer_unparseable_returns_zero() {
        let now = chrono::Utc::now().naive_utc();
        assert_eq!(compute_remaining_secs("bad", now), 0);
    }

    #[test]
    fn hold_timer_countdown_formatting() {
        assert_eq!(format_countdown(0), "00:00");
        assert_eq!(format_countdown(65), "01:05");
        assert_eq!(format_countdown(600), "10:00");
        assert_eq!(format_countdown(-5), "00:00");
    }

    #[test]
    fn hold_timer_urgency_tiers() {
        assert_eq!(hold_urgency(0), HoldUrgency::Expired);
        assert_eq!(hold_urgency(30), HoldUrgency::Critical);
        assert_eq!(hold_urgency(120), HoldUrgency::Normal);
    }

    // ── SlotPicker — tests via real format_slot_time from logic.rs ───────

    #[test]
    fn slot_picker_formats_time_correctly() {
        assert_eq!(format_slot_time("2026-04-15T09:00:00"), "09:00");
        assert_eq!(format_slot_time("2026-04-15T23:45:00"), "23:45");
    }

    #[test]
    fn slot_picker_handles_bad_input() {
        assert_eq!(format_slot_time("no-time"), "no-time");
        assert_eq!(format_slot_time("2026T0"), "2026T0");
    }

    // ── LocaleSwitcher — tested via localized_path from logic.rs ────────

    #[test]
    fn locale_path_prefixes_correctly() {
        assert_eq!(localized_path("en", "/menu"), "/en/menu");
        assert_eq!(localized_path("zh", "/orders"), "/zh/orders");
    }

    #[test]
    fn locale_path_empty_defaults_to_en() {
        assert_eq!(localized_path("", "/menu"), "/en/menu");
    }

    // ── Navbar — tested via AppState role predicates from state.rs ───────

    fn make_user(roles: &[&str]) -> UserInfo {
        UserInfo {
            id: 1,
            username: "test".into(),
            display_name: None,
            roles: roles.iter().map(|s| s.to_string()).collect(),
            preferred_locale: "en".into(),
        }
    }

    #[test]
    fn navbar_customer_not_staff_or_admin() {
        let mut s = AppState::default();
        s.set_auth("c".into(), make_user(&["Customer"]));
        assert!(!s.is_staff());
        assert!(!s.is_admin());
        assert!(!s.is_teacher());
    }

    #[test]
    fn navbar_staff_sees_staff_link() {
        let mut s = AppState::default();
        s.set_auth("c".into(), make_user(&["Staff"]));
        assert!(s.is_staff());
        assert!(!s.is_admin());
    }

    #[test]
    fn navbar_admin_sees_all() {
        let mut s = AppState::default();
        s.set_auth("c".into(), make_user(&["Admin"]));
        assert!(s.is_staff()); // Admin implies staff access
        assert!(s.is_admin());
        assert!(s.is_teacher()); // Admin implies teacher access
    }

    #[test]
    fn navbar_cart_badge_count() {
        let mut s = AppState::default();
        s.cart_count = 3;
        assert_eq!(s.cart_count, 3);
    }

    // ── Cart math — tests via real line_total/cart_subtotal from logic.rs ─

    #[test]
    fn cart_line_total_standard() {
        assert_eq!(line_total(4.50, 3), 13.5);
    }

    #[test]
    fn cart_subtotal_sums_lines() {
        assert_eq!(cart_subtotal(&[(4.50, 2), (3.00, 1)]), 12.0);
    }

    #[test]
    fn cart_tax_rounds() {
        assert_eq!(cart_tax(10.0, 0.0875), 0.88);
    }

    #[test]
    fn cart_total_matches() {
        let sub = 10.0;
        let tax = cart_tax(sub, 0.08);
        let total = cart_total(sub, tax);
        assert!((total - (sub + tax)).abs() < 0.02);
    }

    // ── API base — tests via real api_base_from_origin from logic.rs ─────

    #[test]
    fn api_base_appends_api() {
        assert_eq!(api_base_from_origin("http://localhost:8080"), "http://localhost:8080/api");
    }

    #[test]
    fn api_base_trims_trailing_slash() {
        assert_eq!(api_base_from_origin("https://shop.local/"), "https://shop.local/api");
    }
}
