use dioxus::prelude::*;
use shared::dto::OptionGroupDetail;

/// Computes the initial (group_id, option_id, price_delta) selections
/// by picking the default option (or first) from each group.
pub(crate) fn compute_initial_selections(groups: &[OptionGroupDetail]) -> Vec<(i64, i64, f64)> {
    groups
        .iter()
        .filter_map(|g| {
            let default_opt = g.options.iter().find(|o| o.is_default).or(g.options.first());
            default_opt.map(|o| (g.id, o.id, o.price_delta))
        })
        .collect()
}

/// Sum of all price deltas in the current selections.
pub(crate) fn total_delta(selections: &[(i64, i64, f64)]) -> f64 {
    selections.iter().map(|(_, _, d)| d).sum()
}

/// Format the total delta for display (always prefixed with +).
pub(crate) fn delta_display(delta: f64) -> String {
    format!("+{:.2}", delta)
}

#[component]
pub fn OptionSelector(
    groups: Vec<OptionGroupDetail>,
    locale: String,
    on_change: EventHandler<(Vec<i64>, f64)>,
) -> Element {
    let loc = locale.as_str();
    let is_zh = loc == "zh";

    let initial_selections = compute_initial_selections(&groups);

    let mut selections = use_signal(|| initial_selections);

    let current_total_delta = total_delta(&selections());
    let current_delta_display = delta_display(current_total_delta);

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
                span { class: "font-semibold text-primary", "{current_delta_display}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::dto::{OptionGroupDetail, OptionValueDetail};

    fn make_group(id: i64, options: Vec<OptionValueDetail>) -> OptionGroupDetail {
        OptionGroupDetail {
            id,
            name_en: "Group".to_string(),
            name_zh: "Group".to_string(),
            is_required: true,
            options,
        }
    }

    fn make_option(id: i64, price_delta: f64, is_default: bool) -> OptionValueDetail {
        OptionValueDetail {
            id,
            label_en: "Opt".to_string(),
            label_zh: "Opt".to_string(),
            price_delta,
            is_default,
        }
    }

    #[test]
    fn default_selection_picks_default_option() {
        let groups = vec![make_group(
            1,
            vec![
                make_option(10, 0.0, false),
                make_option(11, 1.50, true),
            ],
        )];
        let sels = compute_initial_selections(&groups);
        assert_eq!(sels.len(), 1);
        assert_eq!(sels[0], (1, 11, 1.50));
    }

    #[test]
    fn default_selection_falls_back_to_first() {
        let groups = vec![make_group(
            2,
            vec![
                make_option(20, 0.50, false),
                make_option(21, 2.00, false),
            ],
        )];
        let sels = compute_initial_selections(&groups);
        assert_eq!(sels[0].1, 20, "Should pick first option when no default");
    }

    #[test]
    fn total_delta_sums_correctly() {
        let sels = vec![(1, 10, 1.50), (2, 20, 0.50), (3, 30, -0.25)];
        let total = total_delta(&sels);
        assert!((total - 1.75).abs() < f64::EPSILON);
    }

    #[test]
    fn delta_display_format() {
        assert_eq!(delta_display(1.75), "+1.75");
        assert_eq!(delta_display(0.0), "+0.00");
        assert_eq!(delta_display(-0.5), "+-0.50");
    }
}
