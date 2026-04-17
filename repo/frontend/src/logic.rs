//! Pure, framework-agnostic logic used by the Dioxus UI.
//!
//! These functions are factored out of component files so they can be unit-
//! tested on any target (including the host's native toolchain), independent
//! of Dioxus, wasm-bindgen, or the browser.
//!
//! When touching UI behavior that involves string formatting, time math, or
//! class lookups, add the code here first and test it, then call it from the
//! matching component.

use chrono::NaiveDateTime;

// ---------------------------------------------------------------------------
// Pickup-slot / hold-timer time helpers
// ---------------------------------------------------------------------------

/// Seconds remaining until `expires_at`.  Returns a non-negative value; callers
/// should treat 0 as "expired".  Accepts the ISO 8601 formats the backend
/// produces (with and without fractional seconds).  Unparseable input yields 0.
pub fn compute_remaining_secs(expires_at: &str, now: NaiveDateTime) -> i64 {
    let parsed = NaiveDateTime::parse_from_str(expires_at, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(expires_at, "%Y-%m-%dT%H:%M:%S%.f"));
    match parsed {
        Ok(t) => (t - now).num_seconds(),
        Err(_) => 0,
    }
}

/// Extract the `HH:MM` portion from an ISO-8601 datetime string.  Used by the
/// slot picker to render 15-minute-slot labels.
pub fn format_slot_time(datetime_str: &str) -> String {
    if let Some(t_pos) = datetime_str.find('T') {
        let time_part = &datetime_str[t_pos + 1..];
        if time_part.len() >= 5 {
            return time_part[..5].to_string();
        }
    }
    datetime_str.to_string()
}

/// Hold-timer urgency tier based on remaining seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldUrgency {
    Expired,
    Critical,
    Normal,
}

pub fn hold_urgency(remaining_secs: i64) -> HoldUrgency {
    if remaining_secs <= 0 {
        HoldUrgency::Expired
    } else if remaining_secs < 60 {
        HoldUrgency::Critical
    } else {
        HoldUrgency::Normal
    }
}

/// Pretty-print a remaining-seconds countdown as `MM:SS` (or `00:00` once
/// expired).
pub fn format_countdown(remaining_secs: i64) -> String {
    if remaining_secs <= 0 {
        return "00:00".to_string();
    }
    let minutes = remaining_secs / 60;
    let seconds = remaining_secs % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

// ---------------------------------------------------------------------------
// Price display
// ---------------------------------------------------------------------------

/// Format a price with locale-appropriate currency symbol and two decimals.
pub fn format_price(amount: f64, locale: &str) -> String {
    let symbol = currency_symbol(locale);
    format!("{}{:.2}", symbol, amount)
}

pub fn currency_symbol(locale: &str) -> &'static str {
    if locale == "zh" {
        "\u{00a5}"
    } else {
        "$"
    }
}

// ---------------------------------------------------------------------------
// Status-badge colour mapping
// ---------------------------------------------------------------------------

/// Returns `(tailwind_colour_classes, i18n_key)` for an order / reservation
/// status.  Mirrors the matcher used by the `StatusBadge` component so that it
/// can be tested without mounting a component.
pub fn status_badge_classes(status: &str) -> (&'static str, &'static str) {
    match status {
        "Pending" => ("bg-gray-200 text-gray-600", "status.pending"),
        "Accepted" => ("bg-blue-100 text-blue-700", "status.accepted"),
        "InPrep" => ("bg-amber-100 text-amber-800", "status.in_prep"),
        "Ready" => ("bg-emerald-100 text-emerald-800", "status.ready"),
        "PickedUp" => ("bg-teal-100 text-teal-700", "status.picked_up"),
        "Canceled" => ("bg-red-100 text-red-800", "status.canceled"),
        "Held" => ("bg-amber-100 text-amber-800", "status.held"),
        "Confirmed" => ("bg-emerald-100 text-emerald-800", "status.confirmed"),
        "Expired" => ("bg-gray-200 text-gray-500", "status.expired"),
        _ => ("bg-gray-100 text-gray-500", ""),
    }
}

// ---------------------------------------------------------------------------
// URL / API base helpers
// ---------------------------------------------------------------------------

/// Build the API base URL from a page origin (e.g. `"http://localhost:8080"`).
/// Callers on the web use `window.location.origin`; native tests can pass any
/// origin string directly.
pub fn api_base_from_origin(origin: &str) -> String {
    let trimmed = origin.trim_end_matches('/');
    format!("{}/api", trimmed)
}

/// Build a locale-prefixed frontend URL.  Used for locale-switching redirects
/// and SEO-friendly links.
pub fn localized_path(locale: &str, path: &str) -> String {
    let path = if path.starts_with('/') { path } else { &path[..] };
    let locale = if locale.is_empty() { "en" } else { locale };
    if path.is_empty() {
        format!("/{}", locale)
    } else if path.starts_with('/') {
        format!("/{}{}", locale, path)
    } else {
        format!("/{}/{}", locale, path)
    }
}

// ---------------------------------------------------------------------------
// Cart-line math (kept in sync with the backend pricing service)
// ---------------------------------------------------------------------------

pub fn line_total(unit_price: f64, quantity: i32) -> f64 {
    unit_price * (quantity.max(0) as f64)
}

pub fn cart_subtotal(lines: &[(f64, i32)]) -> f64 {
    lines.iter().map(|(p, q)| line_total(*p, *q)).sum()
}

pub fn cart_tax(subtotal: f64, rate: f64) -> f64 {
    (subtotal * rate * 100.0).round() / 100.0
}

pub fn cart_total(subtotal: f64, tax: f64) -> f64 {
    ((subtotal + tax) * 100.0).round() / 100.0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, mi, s).unwrap()
    }

    // ── compute_remaining_secs ─────────────────────────────────────────────

    #[test]
    fn compute_remaining_secs_future_positive() {
        let now = dt(2026, 4, 15, 10, 0, 0);
        let secs = compute_remaining_secs("2026-04-15T10:10:00", now);
        assert_eq!(secs, 600);
    }

    #[test]
    fn compute_remaining_secs_past_is_negative() {
        let now = dt(2026, 4, 15, 10, 0, 0);
        let secs = compute_remaining_secs("2026-04-15T09:55:00", now);
        assert_eq!(secs, -300);
    }

    #[test]
    fn compute_remaining_secs_accepts_fractional_seconds() {
        let now = dt(2026, 4, 15, 10, 0, 0);
        let secs = compute_remaining_secs("2026-04-15T10:00:10.123", now);
        assert_eq!(secs, 10);
    }

    #[test]
    fn compute_remaining_secs_unparseable_is_zero() {
        let now = dt(2026, 4, 15, 10, 0, 0);
        assert_eq!(compute_remaining_secs("not-a-date", now), 0);
        assert_eq!(compute_remaining_secs("", now), 0);
    }

    // ── format_slot_time ───────────────────────────────────────────────────

    #[test]
    fn format_slot_time_returns_hhmm_from_iso_datetime() {
        assert_eq!(format_slot_time("2026-04-15T09:00:00"), "09:00");
        assert_eq!(format_slot_time("2099-12-31T23:45:00"), "23:45");
    }

    #[test]
    fn format_slot_time_returns_original_when_no_t_separator() {
        assert_eq!(format_slot_time("bogus"), "bogus");
    }

    #[test]
    fn format_slot_time_handles_short_time_part() {
        // < 5 chars after 'T' → return original, don't panic.
        assert_eq!(format_slot_time("2026-04-15T09"), "2026-04-15T09");
    }

    // ── hold urgency / countdown ───────────────────────────────────────────

    #[test]
    fn hold_urgency_tiers() {
        assert_eq!(hold_urgency(0), HoldUrgency::Expired);
        assert_eq!(hold_urgency(-10), HoldUrgency::Expired);
        assert_eq!(hold_urgency(59), HoldUrgency::Critical);
        assert_eq!(hold_urgency(60), HoldUrgency::Normal);
        assert_eq!(hold_urgency(600), HoldUrgency::Normal);
    }

    #[test]
    fn format_countdown_padding() {
        assert_eq!(format_countdown(0), "00:00");
        assert_eq!(format_countdown(9), "00:09");
        assert_eq!(format_countdown(65), "01:05");
        assert_eq!(format_countdown(600), "10:00");
        // Negative → expired → zeros
        assert_eq!(format_countdown(-1), "00:00");
    }

    // ── price formatting ───────────────────────────────────────────────────

    #[test]
    fn format_price_english_uses_dollar_sign() {
        assert_eq!(format_price(4.5, "en"), "$4.50");
        assert_eq!(format_price(0.0, "en"), "$0.00");
        assert_eq!(format_price(1234.567, "en"), "$1234.57");
    }

    #[test]
    fn format_price_chinese_uses_yuan_sign() {
        assert_eq!(format_price(4.5, "zh"), "\u{00a5}4.50");
    }

    #[test]
    fn format_price_unknown_locale_falls_back_to_english() {
        // `currency_symbol` defaults to "$" for any non-zh locale — this is
        // the intended behaviour until more locales are added.
        assert_eq!(format_price(4.5, "fr"), "$4.50");
    }

    // ── status badge classes ───────────────────────────────────────────────

    #[test]
    fn status_classes_match_known_statuses() {
        let (class, key) = status_badge_classes("Ready");
        assert!(class.contains("emerald"));
        assert_eq!(key, "status.ready");

        let (class, key) = status_badge_classes("Canceled");
        assert!(class.contains("red"));
        assert_eq!(key, "status.canceled");
    }

    #[test]
    fn unknown_status_returns_neutral_classes_and_empty_key() {
        let (class, key) = status_badge_classes("Quantum");
        assert!(class.contains("gray"));
        assert!(key.is_empty(), "unknown status should not map to an i18n key");
    }

    #[test]
    fn every_order_status_has_a_badge_mapping() {
        for s in &[
            "Pending", "Accepted", "InPrep", "Ready", "PickedUp", "Canceled",
        ] {
            let (_class, key) = status_badge_classes(s);
            assert!(!key.is_empty(), "missing i18n key mapping for {}", s);
        }
    }

    // ── URL helpers ────────────────────────────────────────────────────────

    #[test]
    fn api_base_from_origin_appends_api_path() {
        assert_eq!(
            api_base_from_origin("http://localhost:8080"),
            "http://localhost:8080/api"
        );
    }

    #[test]
    fn api_base_from_origin_trims_trailing_slash() {
        assert_eq!(
            api_base_from_origin("https://shop.local/"),
            "https://shop.local/api"
        );
    }

    #[test]
    fn localized_path_prefixes_locale_once() {
        assert_eq!(localized_path("en", "/menu"), "/en/menu");
        assert_eq!(localized_path("zh", "/orders"), "/zh/orders");
    }

    #[test]
    fn localized_path_empty_path_is_locale_root() {
        assert_eq!(localized_path("en", ""), "/en");
    }

    #[test]
    fn localized_path_empty_locale_defaults_to_en() {
        assert_eq!(localized_path("", "/menu"), "/en/menu");
    }

    // ── cart math ──────────────────────────────────────────────────────────

    #[test]
    fn line_total_zero_or_negative_qty_yields_zero() {
        assert_eq!(line_total(5.0, 0), 0.0);
        assert_eq!(line_total(5.0, -3), 0.0);
    }

    #[test]
    fn line_total_standard() {
        assert_eq!(line_total(4.50, 3), 13.5);
    }

    #[test]
    fn cart_subtotal_sums_lines() {
        let lines = [(4.50, 2), (3.00, 1)];
        assert_eq!(cart_subtotal(&lines), 12.0);
    }

    #[test]
    fn cart_tax_rounds_to_cents() {
        assert_eq!(cart_tax(10.0, 0.0875), 0.88);
        assert_eq!(cart_tax(100.0, 0.10), 10.0);
    }

    #[test]
    fn cart_total_respects_rounding() {
        let sub = 9.99;
        let tax = cart_tax(sub, 0.08);
        let total = cart_total(sub, tax);
        assert!((total - (sub + tax)).abs() < 0.02);
    }

    #[test]
    fn cart_subtotal_empty_is_zero() {
        assert_eq!(cart_subtotal(&[]), 0.0);
    }
}
