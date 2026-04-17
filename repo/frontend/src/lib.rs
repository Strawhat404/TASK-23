//! Frontend library target — exposes pure-logic modules for integration tests.
//!
//! The Dioxus component and page modules depend on wasm-only crates and cannot
//! be compiled for native test targets.  This lib re-exports only the modules
//! that are framework-free and testable on any platform.

pub mod logic;
pub mod state;

/// Fallback API base URL used when `web_sys::window()` is unavailable
/// (tests, SSR, or non-browser contexts).
pub const API_BASE: &str = "http://localhost:8080/api";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_base_constant_is_valid_url() {
        assert!(API_BASE.starts_with("http"));
        assert!(API_BASE.ends_with("/api"));
    }

    #[test]
    fn api_base_points_to_default_dev_port() {
        assert!(API_BASE.contains("8080"));
    }

    #[test]
    fn logic_module_is_reexported() {
        // Verify the logic module is accessible through the lib crate.
        let price = logic::format_price(4.50, "en");
        assert_eq!(price, "$4.50");
    }

    #[test]
    fn state_module_is_reexported() {
        // Verify the state module is accessible through the lib crate.
        let s = state::AppState::default();
        assert!(!s.auth.is_authenticated);
        assert_eq!(s.current_locale(), "en");
    }

    #[test]
    fn state_and_logic_interop() {
        // Verify both modules work together (as they do in main.rs).
        let mut s = state::AppState::default();
        s.set_auth("cookie".into(), state::UserInfo {
            id: 1,
            username: "test".into(),
            display_name: None,
            roles: vec!["Customer".into()],
            preferred_locale: "zh".into(),
        });
        let locale = s.current_locale();
        let price = logic::format_price(9.99, locale);
        assert_eq!(price, "\u{00a5}9.99");
    }

    #[test]
    fn logic_localized_path_produces_valid_routes() {
        // These match the Route enum variants defined in main.rs.
        assert_eq!(logic::localized_path("en", "/menu"), "/en/menu");
        assert_eq!(logic::localized_path("zh", "/cart"), "/zh/cart");
        assert_eq!(logic::localized_path("en", "/staff"), "/en/staff");
        assert_eq!(logic::localized_path("en", "/training"), "/en/training");
        assert_eq!(logic::localized_path("en", "/admin"), "/en/admin");
    }
}
