use dioxus::prelude::*;
use crate::state::AppState;

/// Returns the CSS class string for a locale-switcher button.
pub(crate) fn button_class(is_active: bool) -> &'static str {
    if is_active {
        "px-2 py-1 rounded text-xs bg-white/30 text-white border border-white/50 cursor-default"
    } else {
        "px-2 py-1 rounded text-xs bg-white/15 text-white/70 border border-transparent cursor-pointer hover:bg-white/25 hover:text-white transition-all"
    }
}

#[component]
pub fn LocaleSwitcher(current_locale: String) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let nav = use_navigator();

    let is_en = current_locale == "en";
    let is_zh = current_locale == "zh";

    let en_class = button_class(is_en);
    let zh_class = button_class(is_zh);

    rsx! {
        div { class: "flex gap-1",
            button {
                class: "{en_class}",
                disabled: is_en,
                onclick: move |_| {
                    let mut s = state.write();
                    s.locale = "en".to_string();
                    nav.replace(crate::Route::Home { locale: "en".to_string() });
                },
                "EN"
            }
            button {
                class: "{zh_class}",
                disabled: is_zh,
                onclick: move |_| {
                    let mut s = state.write();
                    s.locale = "zh".to_string();
                    nav.replace(crate::Route::Home { locale: "zh".to_string() });
                },
                "\u{4e2d}\u{6587}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_button_class() {
        let cls = button_class(true);
        assert!(cls.contains("bg-white/30"), "Active class should contain bg-white/30");
        assert!(cls.contains("cursor-default"));
    }

    #[test]
    fn inactive_button_class() {
        let cls = button_class(false);
        assert!(cls.contains("bg-white/15"), "Inactive class should contain bg-white/15");
        assert!(cls.contains("cursor-pointer"));
    }

    #[test]
    fn active_and_inactive_differ() {
        assert_ne!(button_class(true), button_class(false));
    }
}
