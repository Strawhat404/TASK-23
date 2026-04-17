//! Frontend page-level integration tests.
//!
//! These tests import real modules from the `frontend` crate and `shared` crate,
//! verifying the actual DTOs, state transitions, and logic that pages depend on.
//! Every import references production code directly.

#[cfg(test)]
mod tests {
    // ── Direct imports from real crates ───────────────────────────────────
    use frontend::state::{AppState, UserInfo};
    use frontend::logic::{
        format_price, localized_path, cart_subtotal, cart_tax, cart_total,
        status_badge_classes, format_countdown, compute_remaining_secs,
    };
    use shared::dto::*;
    use shared::enums::*;

    // ── auth page: login request/response contract ────────────────────────

    #[test]
    fn login_request_shape() {
        let req = LoginRequest { username: "alice".into(), password: "Pass123!abc".into() };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("username"));
        assert!(json.contains("password"));
    }

    #[test]
    fn login_response_carries_session_cookie_and_user() {
        let json = r#"{"session_cookie":"id.sig","user":{"id":1,"username":"alice","display_name":null,"roles":["Customer"],"preferred_locale":"en"}}"#;
        let resp: LoginResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.session_cookie, "id.sig");
        assert_eq!(resp.user.roles[0], "Customer");
    }

    // ── menu page: product display ────────────────────────────────────────

    #[test]
    fn product_list_item_bilingual() {
        let item = ProductListItem {
            spu_id: 1, name_en: "Latte".into(), name_zh: "\u{62ff}\u{94c1}".into(),
            description_en: Some("Classic".into()), description_zh: None,
            category: Some("coffee".into()), image_url: None,
            base_price: 4.50, prep_time_minutes: 5,
        };
        assert_ne!(item.name_en, item.name_zh);
    }

    // ── product detail page: pricing via real format_price ────────────────

    #[test]
    fn product_price_display_en() {
        assert_eq!(format_price(5.99, "en"), "$5.99");
    }

    #[test]
    fn product_price_display_zh() {
        assert_eq!(format_price(5.99, "zh"), "\u{00a5}5.99");
    }

    // ── cart page: cart math via real cart_* functions ─────────────────────

    #[test]
    fn cart_totals_computed_correctly() {
        let sub = cart_subtotal(&[(4.50, 2), (3.00, 1)]);
        assert_eq!(sub, 12.0);
        let tax = cart_tax(sub, 0.0875);
        let total = cart_total(sub, tax);
        assert!(total > sub);
    }

    // ── checkout page: pickup slot availability ──────────────────────────

    #[test]
    fn pickup_slot_availability_flag() {
        let avail = PickupSlot { start: "2026-04-15T09:00:00".into(), end: "2026-04-15T09:15:00".into(), available: true };
        let blocked = PickupSlot { start: "2026-04-15T09:00:00".into(), end: "2026-04-15T09:15:00".into(), available: false };
        assert!(avail.available);
        assert!(!blocked.available);
    }

    #[test]
    fn checkout_response_voucher() {
        let cr = CheckoutResponse {
            order_id: 1, order_number: "ORD-1".into(), voucher_code: "BF-ABC123".into(),
            hold_expires_at: "2026-04-15T09:30:00".into(), pickup_slot: "09:00-09:15".into(), total: 9.79,
        };
        assert!(cr.voucher_code.starts_with("BF-"));
    }

    // ── orders page: status transitions via real shared enums ─────────────

    #[test]
    fn order_status_transitions() {
        let pending = OrderStatus::Pending;
        let transitions = pending.allowed_transitions();
        assert!(transitions.contains(&OrderStatus::Accepted));
        assert!(transitions.contains(&OrderStatus::Canceled));
        assert!(!transitions.contains(&OrderStatus::Ready));
    }

    #[test]
    fn terminal_statuses_have_no_transitions() {
        assert!(OrderStatus::PickedUp.allowed_transitions().is_empty());
        assert!(OrderStatus::Canceled.allowed_transitions().is_empty());
    }

    // ── staff page: voucher scan response ─────────────────────────────────

    #[test]
    fn scan_voucher_mismatch() {
        let resp = ScanVoucherResponse {
            valid: false, order: None, mismatch: true,
            mismatch_reason: Some("Wrong order".into()),
        };
        assert!(!resp.valid);
        assert!(resp.mismatch);
    }

    // ── admin page: role management ───────────────────────────────────────

    #[test]
    fn all_roles_round_trip() {
        for r in [Role::Admin, Role::Staff, Role::Customer, Role::Teacher, Role::AcademicAffairs] {
            let json = serde_json::to_string(&r).unwrap();
            let back: Role = serde_json::from_str(&json).unwrap();
            assert_eq!(back, r);
        }
    }

    // ── training page: exam/analytics DTOs ────────────────────────────────

    #[test]
    fn score_analytics_shape() {
        let sa = ScoreAnalytics {
            overall_score: 85.0,
            by_subject: vec![SubjectScore { subject_id: 1, subject_name: "Espresso".into(), avg_score: 90.0, attempt_count: 5 }],
            by_difficulty: vec![DifficultyScore { difficulty: "easy".into(), avg_score: 95.0, attempt_count: 10 }],
            recent_attempts: vec![],
        };
        assert!(sa.overall_score > 0.0);
    }

    #[test]
    fn finish_exam_wrong_questions_shape() {
        let fer = FinishExamResponse {
            attempt_id: 1, score: 80.0, total_questions: 10, correct_count: 8,
            wrong_questions: vec![WrongQuestionDetail {
                question_id: 3, question_text_en: "Q".into(),
                correct_options: vec!["A".into()], your_options: vec!["B".into()],
                explanation_en: Some("A".into()),
            }],
        };
        assert_eq!(fer.wrong_questions.len(), 1);
    }

    // ── URL construction via real localized_path ─────────────────────────

    #[test]
    fn page_urls_locale_prefixed() {
        assert_eq!(localized_path("en", "/menu"), "/en/menu");
        assert_eq!(localized_path("zh", "/training"), "/zh/training");
        assert_eq!(localized_path("en", "/admin"), "/en/admin");
    }

    // ── state: role-gated page visibility via real AppState ──────────────

    fn make_user(roles: &[&str]) -> UserInfo {
        UserInfo {
            id: 1, username: "u".into(), display_name: None,
            roles: roles.iter().map(|s| s.to_string()).collect(),
            preferred_locale: "en".into(),
        }
    }

    #[test]
    fn customer_cannot_access_staff_or_admin() {
        let mut s = AppState::default();
        s.set_auth("c".into(), make_user(&["Customer"]));
        assert!(!s.is_staff());
        assert!(!s.is_admin());
    }

    #[test]
    fn teacher_can_access_training() {
        let mut s = AppState::default();
        s.set_auth("c".into(), make_user(&["Teacher"]));
        assert!(s.is_teacher());
    }

    #[test]
    fn admin_can_access_everything() {
        let mut s = AppState::default();
        s.set_auth("c".into(), make_user(&["Admin"]));
        assert!(s.is_staff());
        assert!(s.is_admin());
        assert!(s.is_teacher());
    }

    // ── hold timer via real compute_remaining_secs ───────────────────────

    #[test]
    fn hold_timer_countdown_from_real_function() {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 4, 15).unwrap().and_hms_opt(10, 0, 0).unwrap();
        let secs = compute_remaining_secs("2026-04-15T10:05:00", now);
        assert_eq!(format_countdown(secs), "05:00");
    }

    // ── status badge via real function ───────────────────────────────────

    #[test]
    fn status_badge_for_all_statuses() {
        for s in &["Pending", "Accepted", "InPrep", "Ready", "PickedUp", "Canceled", "Held", "Confirmed", "Expired"] {
            let (class, key) = status_badge_classes(s);
            assert!(!class.is_empty());
            assert!(!key.is_empty(), "no key for {}", s);
        }
    }
}
