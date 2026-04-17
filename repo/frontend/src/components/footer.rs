use dioxus::prelude::*;

pub(crate) const COPYRIGHT_YEAR: &str = "2026";
pub(crate) const APP_NAME: &str = "BrewFlow Offline Retail & Training Suite";

#[component]
pub fn Footer() -> Element {
    rsx! {
        footer { class: "bg-gray-800 text-gray-400 text-center py-6 px-4 text-sm",
            p { class: "font-medium text-gray-300", "{APP_NAME}" }
            p { class: "mt-1", "\u{00a9} {COPYRIGHT_YEAR} BrewFlow. All rights reserved." }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copyright_year_matches() {
        assert_eq!(COPYRIGHT_YEAR, "2026");
    }

    #[test]
    fn app_name_matches() {
        assert_eq!(APP_NAME, "BrewFlow Offline Retail & Training Suite");
    }

    #[test]
    fn constants_are_non_empty() {
        assert!(!COPYRIGHT_YEAR.is_empty());
        assert!(!APP_NAME.is_empty());
    }
}
