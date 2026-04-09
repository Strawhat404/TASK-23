use dioxus::prelude::*;
use crate::state::AppState;

#[component]
pub fn LocaleSwitcher(current_locale: String) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let nav = use_navigator();

    let is_en = current_locale == "en";
    let is_zh = current_locale == "zh";

    let en_class = if is_en {
        "px-2 py-1 rounded text-xs bg-white/30 text-white border border-white/50 cursor-default"
    } else {
        "px-2 py-1 rounded text-xs bg-white/15 text-white/70 border border-transparent cursor-pointer hover:bg-white/25 hover:text-white transition-all"
    };

    let zh_class = if is_zh {
        "px-2 py-1 rounded text-xs bg-white/30 text-white border border-white/50 cursor-default"
    } else {
        "px-2 py-1 rounded text-xs bg-white/15 text-white/70 border border-transparent cursor-pointer hover:bg-white/25 hover:text-white transition-all"
    };

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
