use dioxus::prelude::*;
use shared::dto::OptionGroupDetail;

#[component]
pub fn OptionSelector(
    groups: Vec<OptionGroupDetail>,
    locale: String,
    on_change: EventHandler<(Vec<i64>, f64)>,
) -> Element {
    let loc = locale.as_str();
    let is_zh = loc == "zh";

    let initial_selections: Vec<(i64, i64, f64)> = groups
        .iter()
        .filter_map(|g| {
            let default_opt = g.options.iter().find(|o| o.is_default).or(g.options.first());
            default_opt.map(|o| (g.id, o.id, o.price_delta))
        })
        .collect();

    let mut selections = use_signal(|| initial_selections);

    let total_delta: f64 = selections().iter().map(|(_, _, d)| d).sum();
    let delta_display = format!("+{:.2}", total_delta);

    rsx! {
        div { class: "space-y-5 my-4",
            for group in groups.iter() {
                {
                    let group_name = if is_zh { &group.name_zh } else { &group.name_en };
                    let required_marker = if group.is_required { " *" } else { "" };
                    let group_id = group.id;

                    rsx! {
                        div { class: "space-y-2",
                            label { class: "block font-semibold text-sm text-gray-700",
                                "{group_name}"
                                if group.is_required {
                                    span { class: "text-red-500 text-xs ml-1", "{required_marker}" }
                                }
                            }
                            div { class: "flex flex-wrap gap-2",
                                for option in group.options.iter() {
                                    {
                                        let opt_id = option.id;
                                        let opt_label = if is_zh { option.label_zh.clone() } else { option.label_en.clone() };
                                        let opt_delta = option.price_delta;
                                        let delta_text = if opt_delta > 0.0 {
                                            format!(" (+{:.2})", opt_delta)
                                        } else if opt_delta < 0.0 {
                                            format!(" ({:.2})", opt_delta)
                                        } else {
                                            String::new()
                                        };

                                        let is_selected = selections()
                                            .iter()
                                            .any(|(gid, oid, _)| *gid == group_id && *oid == opt_id);

                                        let btn_class = if is_selected {
                                            "px-4 py-2 rounded-lg text-sm font-medium border-2 border-primary bg-primary/10 text-primary cursor-pointer transition-all"
                                        } else {
                                            "px-4 py-2 rounded-lg text-sm font-medium border border-gray-200 bg-white text-gray-700 cursor-pointer hover:border-primary-light hover:bg-primary/5 transition-all"
                                        };

                                        rsx! {
                                            button {
                                                class: "{btn_class}",
                                                onclick: move |_| {
                                                    let mut sels = selections.write();
                                                    if let Some(entry) = sels.iter_mut().find(|(gid, _, _)| *gid == group_id) {
                                                        entry.1 = opt_id;
                                                        entry.2 = opt_delta;
                                                    } else {
                                                        sels.push((group_id, opt_id, opt_delta));
                                                    }
                                                    let ids: Vec<i64> = sels.iter().map(|(_, oid, _)| *oid).collect();
                                                    let delta: f64 = sels.iter().map(|(_, _, d)| d).sum();
                                                    drop(sels);
                                                    on_change.call((ids, delta));
                                                },
                                                "{opt_label}{delta_text}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "flex justify-between items-center pt-2 border-t border-gray-100",
                span { class: "text-sm text-gray-500", "Options: " }
                span { class: "font-semibold text-primary", "{delta_display}" }
            }
        }
    }
}
