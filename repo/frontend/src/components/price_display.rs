use dioxus::prelude::*;

/// Formats a price amount with the correct currency symbol for the locale.
pub(crate) fn format_price(amount: f64, locale: &str) -> String {
    let symbol = if locale == "zh" { "\u{00a5}" } else { "$" };
    format!("{}{:.2}", symbol, amount)
}

#[component]
pub fn PriceDisplay(amount: f64, locale: String) -> Element {
    let formatted = format_price(amount, &locale);

    rsx! {
        span { class: "font-semibold text-primary-dark", "{formatted}" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_price_dollar() {
        assert_eq!(format_price(9.99, "en"), "$9.99");
    }

    #[test]
    fn format_price_yuan() {
        assert_eq!(format_price(9.99, "zh"), "\u{00a5}9.99");
    }

    #[test]
    fn format_price_zero() {
        assert_eq!(format_price(0.0, "en"), "$0.00");
    }

    #[test]
    fn format_price_negative() {
        assert_eq!(format_price(-5.5, "en"), "$-5.50");
    }

    #[test]
    fn format_price_rounding() {
        assert_eq!(format_price(1.999, "en"), "$2.00");
        assert_eq!(format_price(1.001, "en"), "$1.00");
    }
}
