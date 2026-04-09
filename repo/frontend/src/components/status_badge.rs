use dioxus::prelude::*;

const BADGE: &str = "inline-flex items-center px-3 py-1 rounded-full text-xs font-semibold";

#[component]
pub fn StatusBadge(status: String, locale: String) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.as_str();

    let (color_class, i18n_key) = match status.as_str() {
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
    };

    let label = if i18n_key.is_empty() {
        status.clone()
    } else {
        t.t(loc, i18n_key)
    };

    let class = format!("{} {}", BADGE, color_class);

    rsx! {
        span { class: "{class}", "{label}" }
    }
}
