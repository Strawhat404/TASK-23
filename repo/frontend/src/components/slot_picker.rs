use dioxus::prelude::*;
use shared::dto::PickupSlot;

#[component]
pub fn SlotPicker(
    slots: Vec<PickupSlot>,
    locale: String,
    on_select: EventHandler<PickupSlot>,
) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.as_str();

    let mut selected_start = use_signal(|| Option::<String>::None);

    let label = t.t(loc, "label.pickup_time");

    rsx! {
        div {
            h3 { class: "text-lg font-semibold mb-3 text-gray-800", "{label}" }
            div { class: "grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-2",
                for slot in slots.iter() {
                    {
                        let is_available = slot.available;
                        let is_selected = selected_start()
                            .as_ref()
                            .map(|s| s == &slot.start)
                            .unwrap_or(false);

                        let slot_class = if !is_available {
                            "py-2.5 px-2 text-center border border-gray-200 rounded-lg text-sm bg-gray-100 text-gray-400 cursor-not-allowed line-through"
                        } else if is_selected {
                            "py-2.5 px-2 text-center border-2 border-primary rounded-lg text-sm bg-primary text-white font-medium cursor-pointer"
                        } else {
                            "py-2.5 px-2 text-center border border-gray-200 rounded-lg text-sm bg-white text-gray-700 cursor-pointer hover:border-primary hover:bg-primary/5 transition-all"
                        };

                        let display_time = format_slot_time(&slot.start);
                        let slot_clone = slot.clone();

                        rsx! {
                            button {
                                class: "{slot_class}",
                                disabled: !is_available,
                                onclick: move |_| {
                                    selected_start.set(Some(slot_clone.start.clone()));
                                    on_select.call(slot_clone.clone());
                                },
                                "{display_time}"
                            }
                        }
                    }
                }
            }
            if slots.is_empty() {
                p { class: "text-center py-8 text-gray-400",
                    "{t.t(loc, \"error.slot_unavailable\")}"
                }
            }
        }
    }
}

pub(crate) fn format_slot_time(datetime_str: &str) -> String {
    if let Some(t_pos) = datetime_str.find('T') {
        let time_part = &datetime_str[t_pos + 1..];
        if time_part.len() >= 5 {
            return time_part[..5].to_string();
        }
    }
    datetime_str.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_iso_datetime() {
        assert_eq!(format_slot_time("2026-04-16T14:30:00"), "14:30");
    }

    #[test]
    fn no_t_separator_returns_original() {
        let input = "2026-04-16 14:30:00";
        assert_eq!(format_slot_time(input), input);
    }

    #[test]
    fn short_time_part_returns_original() {
        // Time part shorter than 5 chars
        let input = "2026-04-16T14";
        assert_eq!(format_slot_time(input), input);
    }

    #[test]
    fn time_with_seconds_still_returns_hh_mm() {
        assert_eq!(format_slot_time("2026-01-01T09:05:59"), "09:05");
    }
}
