use dioxus::prelude::*;

const BADGE: &str = "inline-flex items-center px-3 py-1 rounded-full text-xs font-semibold";

/// Returns (color_class, i18n_key) for a given order/reservation status string.
pub(crate) fn badge_classes(status: &str) -> (&'static str, &'static str) {
    match status {
        "Pending"   => ("bg-gray-200 text-gray-600", "status.pending"),
        "Accepted"  => ("bg-blue-100 text-blue-700", "status.accepted"),
        "InPrep"    => ("bg-amber-100 text-amber-800", "status.in_prep"),
        "Ready"     => ("bg-emerald-100 text-emerald-800", "status.ready"),
        "PickedUp"  => ("bg-teal-100 text-teal-700", "status.picked_up"),
        "Canceled"  => ("bg-red-100 text-red-800", "status.canceled"),
        "Held"      => ("bg-amber-100 text-amber-800", "status.held"),
        "Confirmed" => ("bg-emerald-100 text-emerald-800", "status.confirmed"),
        "Expired"   => ("bg-gray-200 text-gray-500", "status.expired"),
        _           => ("bg-gray-100 text-gray-500", ""),
    }
}

/// Returns the human-readable badge label for a status + locale.
pub(crate) fn badge_label(status: &str, locale: &str) -> String {
    let (_color, i18n_key) = badge_classes(status);
    if i18n_key.is_empty() {
        status.to_string()
    } else {
        let t = shared::i18n::init_translations();
        t.t(locale, i18n_key)
    }
}

#[component]
pub fn StatusBadge(status: String, locale: String) -> Element {
    let (color_class, _) = badge_classes(&status);
    let label = badge_label(&status, &locale);
    let class = format!("{} {}", BADGE, color_class);

    rsx! {
        span { class: "{class}", "{label}" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_classes_known_statuses() {
        let (color, key) = badge_classes("Pending");
        assert!(color.contains("gray"), "Pending should use gray colors");
        assert_eq!(key, "status.pending");

        let (color, key) = badge_classes("Accepted");
        assert!(color.contains("blue"));
        assert_eq!(key, "status.accepted");

        let (color, key) = badge_classes("Ready");
        assert!(color.contains("emerald"));
        assert_eq!(key, "status.ready");
    }

    #[test]
    fn badge_classes_unknown_status_returns_neutral() {
        let (color, key) = badge_classes("SomeUnknown");
        assert!(color.contains("gray-100"));
        assert_eq!(key, "", "Unknown status should have empty i18n key");
    }

    #[test]
    fn badge_label_resolves_both_locales() {
        let en = badge_label("Pending", "en");
        assert!(!en.is_empty(), "English label should not be empty");

        let zh = badge_label("Pending", "zh");
        assert!(!zh.is_empty(), "Chinese label should not be empty");

        // Unknown status returns the raw status string
        let unknown = badge_label("FooBar", "en");
        assert_eq!(unknown, "FooBar");
    }

    #[test]
    fn badge_classes_covers_all_known_variants() {
        let known = vec![
            "Pending", "Accepted", "InPrep", "Ready", "PickedUp",
            "Canceled", "Held", "Confirmed", "Expired",
        ];
        for status in known {
            let (_color, key) = badge_classes(status);
            assert!(!key.is_empty(), "Known status '{}' should have an i18n key", status);
        }
    }
}
