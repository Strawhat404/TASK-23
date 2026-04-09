use dioxus::prelude::*;

#[component]
pub fn PriceDisplay(amount: f64, locale: String) -> Element {
    let symbol = if locale == "zh" { "\u{00a5}" } else { "$" };
    let formatted = format!("{}{:.2}", symbol, amount);

    rsx! {
        span { class: "font-semibold text-primary-dark", "{formatted}" }
    }
}
