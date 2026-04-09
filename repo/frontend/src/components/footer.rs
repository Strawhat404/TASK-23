use dioxus::prelude::*;

#[component]
pub fn Footer() -> Element {
    rsx! {
        footer { class: "bg-gray-800 text-gray-400 text-center py-6 px-4 text-sm",
            p { class: "font-medium text-gray-300", "BrewFlow Offline Retail & Training Suite" }
            p { class: "mt-1", "\u{00a9} 2026 BrewFlow. All rights reserved." }
        }
    }
}
