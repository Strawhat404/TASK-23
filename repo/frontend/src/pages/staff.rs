use dioxus::prelude::*;
use crate::components::navbar::Navbar;
use crate::components::footer::Footer;
use crate::components::price_display::PriceDisplay;
use crate::components::status_badge::StatusBadge;
use crate::state::AppState;
use shared::dto::{
    ApiResponse, OrderDetail, OrderSummary, ScanVoucherRequest, ScanVoucherResponse,
    UpdateOrderStatusRequest,
};

const INPUT: &str = "w-full px-3 py-2.5 border border-gray-300 rounded-lg text-sm transition-colors focus:outline-none focus:border-primary focus:ring-2 focus:ring-primary/15 bg-white";
const BTN_PRIMARY: &str = "inline-flex items-center justify-center px-5 py-2.5 rounded-lg text-sm font-medium bg-primary text-white hover:bg-primary-dark transition-all disabled:opacity-50 disabled:cursor-not-allowed no-underline";
const BTN_DANGER: &str = "inline-flex items-center justify-center px-5 py-2.5 rounded-lg text-sm font-medium bg-red-500 text-white hover:bg-red-700 transition-all disabled:opacity-50 disabled:cursor-not-allowed";

#[derive(serde::Deserialize, Clone, Debug)]
struct DashboardCounts { pending_count: i64, in_prep_count: i64, ready_count: i64 }

#[component]
pub fn StaffDashboardPage(locale: String) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.clone();
    let app_state = use_context::<Signal<AppState>>();
    let mut status_filter = use_signal(|| String::new());

    let counts_resource = use_resource(move || {
        let session_cookie = app_state().auth.session_cookie.clone();
        async move {
            let mut req = reqwest::Client::new().get(&format!("{}/staff/dashboard/counts", &crate::api_base()));
            if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
            let resp = req.send().await.map_err(|e| e.to_string())?;
            let data: ApiResponse<DashboardCounts> = resp.json().await.map_err(|e| e.to_string())?;
            data.data.ok_or_else(|| "No data".to_string())
        }
    });

    let orders_resource = use_resource(move || {
        let session_cookie = app_state().auth.session_cookie.clone();
        let filter = status_filter();
        async move {
            let url = if filter.is_empty() { format!("{}/staff/orders", &crate::api_base()) } else { format!("{}/staff/orders?status={}", &crate::api_base(), filter) };
            let mut req = reqwest::Client::new().get(&url);
            if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
            let resp = req.send().await.map_err(|e| e.to_string())?;
            let data: ApiResponse<Vec<OrderSummary>> = resp.json().await.map_err(|e| e.to_string())?;
            data.data.ok_or_else(|| "No data".to_string())
        }
    });

    let page_title = t.t(&loc, "page.staff_dashboard");
    let pending_label = t.t(&loc, "status.pending");
    let in_prep_label = t.t(&loc, "status.in_prep");
    let ready_label = t.t(&loc, "status.ready");
    let all_label = if loc == "zh" { "\u{5168}\u{90e8}" } else { "All" };
    let scan_label = t.t(&loc, "btn.scan");

    rsx! {
        div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
            Navbar { locale: locale.clone() }
            main { class: "flex-1 max-w-7xl mx-auto px-4 py-8 w-full",
                div { class: "flex justify-between items-center mb-6",
                    h2 { class: "text-2xl font-bold text-gray-800", "{page_title}" }
                    Link { to: crate::Route::StaffScan { locale: locale.clone() }, class: BTN_PRIMARY, "{scan_label}" }
                }

                // Dashboard cards
                div { class: "grid grid-cols-1 md:grid-cols-3 gap-4 mb-6",
                    match &*counts_resource.read() {
                        Some(Ok(counts)) => rsx! {
                            div { class: "text-center p-6 rounded-xl bg-gray-100 cursor-pointer hover:shadow-md transition-shadow", onclick: move |_| status_filter.set("Pending".to_string()),
                                h3 { class: "text-4xl font-bold text-gray-700", "{counts.pending_count}" }
                                p { class: "text-sm text-gray-500 mt-1", "{pending_label}" }
                            }
                            div { class: "text-center p-6 rounded-xl bg-amber-50 cursor-pointer hover:shadow-md transition-shadow", onclick: move |_| status_filter.set("InPrep".to_string()),
                                h3 { class: "text-4xl font-bold text-amber-700", "{counts.in_prep_count}" }
                                p { class: "text-sm text-gray-500 mt-1", "{in_prep_label}" }
                            }
                            div { class: "text-center p-6 rounded-xl bg-emerald-50 cursor-pointer hover:shadow-md transition-shadow", onclick: move |_| status_filter.set("Ready".to_string()),
                                h3 { class: "text-4xl font-bold text-emerald-700", "{counts.ready_count}" }
                                p { class: "text-sm text-gray-500 mt-1", "{ready_label}" }
                            }
                        },
                        Some(Err(e)) => rsx! { div { class: "text-red-500 text-sm", "Error: {e}" } },
                        None => rsx! { div { class: "text-center py-8 text-gray-400", "Loading..." } },
                    }
                }

                // Filter bar
                div { class: "flex flex-wrap gap-2 mb-6",
                    { let current = status_filter();
                      let statuses: Vec<(&str, &str)> = vec![("", all_label), ("Pending", &pending_label), ("Accepted", "Accepted"), ("InPrep", &in_prep_label), ("Ready", &ready_label)];
                      rsx! { for (val, label) in statuses.into_iter() {
                          { let is_active = current == val; let val_owned = val.to_string(); rsx! {
                              button { class: if is_active { "px-4 py-2 rounded-full text-sm font-medium bg-primary text-white" } else { "px-4 py-2 rounded-full text-sm font-medium bg-white text-gray-600 border border-gray-200 hover:bg-gray-50 transition-all cursor-pointer" },
                                  onclick: move |_| status_filter.set(val_owned.clone()), "{label}"
                              }
                          } }
                      } }
                    }
                }

                // Orders list
                match &*orders_resource.read() {
                    Some(Ok(orders)) => rsx! {
                        div { class: "space-y-3",
                            if orders.is_empty() { p { class: "text-center py-8 text-gray-400", if loc == "zh" { "\u{6ca1}\u{6709}\u{8ba2}\u{5355}" } else { "No orders" } } }
                            for order in orders.iter() {
                                { let oid = order.id; rsx! {
                                    Link { to: crate::Route::StaffOrderDetail { locale: locale.clone(), id: oid },
                                        class: "block bg-white rounded-xl shadow-sm hover:shadow-md transition-shadow p-4 no-underline text-gray-800",
                                        div { class: "flex justify-between items-center mb-2",
                                            span { class: "font-semibold", "#{order.order_number}" }
                                            StatusBadge { status: order.status.clone(), locale: locale.clone() }
                                        }
                                        div { class: "flex justify-between items-center",
                                            PriceDisplay { amount: order.total, locale: locale.clone() }
                                            span { class: "text-xs text-gray-400", "{order.created_at}" }
                                        }
                                    }
                                } }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! { div { class: "text-red-500 text-sm", "Error: {e}" } },
                    None => rsx! { div { class: "text-center py-8 text-gray-400", "Loading..." } },
                }
            }
            Footer {}
        }
    }
}

#[component]
pub fn StaffOrderDetailPage(locale: String, id: i64) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.clone();
    let app_state = use_context::<Signal<AppState>>();
    let mut status_notes = use_signal(|| String::new());
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut success_msg = use_signal(|| Option::<String>::None);
    let mut action_loading = use_signal(|| false);
    let mut refresh_trigger = use_signal(|| 0u32);

    let detail_resource = use_resource(move || {
        let _trigger = refresh_trigger();
        let session_cookie = app_state().auth.session_cookie.clone();
        async move {
            let mut req = reqwest::Client::new().get(&format!("{}/staff/orders/{}", &crate::api_base(), id));
            if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
            let resp = req.send().await.map_err(|e| e.to_string())?;
            let data: ApiResponse<OrderDetail> = resp.json().await.map_err(|e| e.to_string())?;
            data.data.ok_or_else(|| "Order not found".to_string())
        }
    });

    let page_title = if loc == "zh" { "\u{8ba2}\u{5355}\u{7ba1}\u{7406}" } else { "Manage Order" };
    let notes_label = if loc == "zh" { "\u{5907}\u{6ce8}" } else { "Notes" };

    rsx! {
        div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
            Navbar { locale: locale.clone() }
            main { class: "flex-1 max-w-3xl mx-auto px-4 py-8 w-full",
                h2 { class: "text-2xl font-bold mb-5 text-gray-800", "{page_title}" }

                if let Some(err) = error_msg() { div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm mb-4", "{err}" } }
                if let Some(msg) = success_msg() { div { class: "bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg text-sm mb-4", "{msg}" } }

                match &*detail_resource.read() {
                    Some(Ok(detail)) => {
                        let order = &detail.order;
                        let current_status = &order.status;
                        let allowed = match current_status.as_str() {
                            "Pending" => vec![("Accepted", "Accepted"), ("Canceled", "Canceled")],
                            "Accepted" => vec![("InPrep", "In Preparation"), ("Canceled", "Canceled")],
                            "InPrep" => vec![("Ready", "Ready"), ("Canceled", "Canceled")],
                            "Ready" => vec![("PickedUp", "Picked Up"), ("Canceled", "Canceled")],
                            _ => vec![],
                        };

                        rsx! {
                            div { class: "bg-white rounded-2xl shadow p-6 space-y-6",
                                div { class: "flex justify-between items-center",
                                    h3 { class: "text-xl font-bold", "#{order.order_number}" }
                                    StatusBadge { status: order.status.clone(), locale: locale.clone() }
                                }

                                div { class: "space-y-2",
                                    for item in detail.items.iter() {
                                        div { class: "flex justify-between text-sm",
                                            div {
                                                span { class: "text-gray-800", "{item.spu_name}" }
                                                if !item.options.is_empty() { span { class: "text-gray-400", " ({item.options.join(\", \")})" } }
                                                span { class: "text-gray-400", " x{item.quantity}" }
                                            }
                                            PriceDisplay { amount: item.item_total, locale: locale.clone() }
                                        }
                                    }
                                    div { class: "flex justify-between font-bold pt-3 border-t border-gray-100",
                                        span { if loc == "zh" { "\u{603b}\u{8ba1}" } else { "Total" } }
                                        PriceDisplay { amount: order.total, locale: locale.clone() }
                                    }
                                }

                                if !allowed.is_empty() {
                                    div { class: "pt-4 border-t border-gray-100",
                                        h4 { class: "font-semibold text-gray-700 mb-3", if loc == "zh" { "\u{66f4}\u{65b0}\u{72b6}\u{6001}" } else { "Update Status" } }
                                        div { class: "mb-3",
                                            label { class: "block text-sm font-medium text-gray-700 mb-1", "{notes_label}" }
                                            textarea { class: "{INPUT} min-h-[80px] resize-y", placeholder: "{notes_label}", value: "{status_notes}", oninput: move |evt| status_notes.set(evt.value()) }
                                        }
                                        div { class: "flex flex-wrap gap-2",
                                            for (status_val, status_label) in allowed.iter() {
                                                { let new_status = status_val.to_string(); let btn_class = if *status_val == "Canceled" { BTN_DANGER } else { BTN_PRIMARY }; rsx! {
                                                    button { class: "{btn_class}", disabled: action_loading(),
                                                        onclick: move |_| {
                                                            let session_cookie = app_state().auth.session_cookie.clone();
                                                            let ns = new_status.clone(); let notes = status_notes().clone();
                                                            spawn(async move {
                                                                action_loading.set(true); error_msg.set(None); success_msg.set(None);
                                                                let body = UpdateOrderStatusRequest { new_status: ns.clone(), notes: if notes.is_empty() { None } else { Some(notes) } };
                                                                let mut req = reqwest::Client::new().put(&format!("{}/staff/orders/{}/status", &crate::api_base(), id)).json(&body);
                                                                if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
                                                                match req.send().await {
                                                                    Ok(resp) if resp.status().is_success() => { success_msg.set(Some(format!("Status updated to {}", ns))); status_notes.set(String::new()); refresh_trigger.set(refresh_trigger()+1); }
                                                                    Ok(resp) => { let body = resp.text().await.unwrap_or_default(); error_msg.set(Some(format!("Failed: {}", body))); }
                                                                    Err(e) => error_msg.set(Some(format!("Error: {}", e))),
                                                                }
                                                                action_loading.set(false);
                                                            });
                                                        },
                                                        "{status_label}"
                                                    }
                                                } }
                                            }
                                        }
                                    }
                                }

                                if !detail.fulfillment_history.is_empty() {
                                    div {
                                        h4 { class: "font-semibold text-gray-700 mb-3", if loc == "zh" { "\u{5c65}\u{7ea6}\u{5386}\u{53f2}" } else { "Fulfillment History" } }
                                        div { class: "pl-6 border-l-2 border-gray-200 space-y-4",
                                            for event in detail.fulfillment_history.iter() {
                                                div { class: "relative pl-5",
                                                    div { class: "timeline-dot" }
                                                    div { class: "flex items-center gap-2 mb-1",
                                                        StatusBadge { status: event.from_status.clone().unwrap_or_default(), locale: locale.clone() }
                                                        span { class: "text-gray-400 text-xs", "\u{2192}" }
                                                        StatusBadge { status: event.to_status.clone(), locale: locale.clone() }
                                                    }
                                                    p { class: "text-xs text-gray-400", "{event.changed_by} - {event.timestamp}" }
                                                    if let Some(ref notes) = event.notes { p { class: "text-sm text-gray-500 mt-1", "{notes}" } }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! { div { class: "text-red-500 text-sm", "Error: {e}" } },
                    None => rsx! { div { class: "text-center py-8 text-gray-400", "Loading..." } },
                }
            }
            Footer {}
        }
    }
}

#[component]
pub fn StaffScanPage(locale: String) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.clone();
    let app_state = use_context::<Signal<AppState>>();
    let mut voucher_input = use_signal(|| String::new());
    let mut order_id_input = use_signal(|| String::new());
    let mut scan_result = use_signal(|| Option::<ScanVoucherResponse>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut scanning = use_signal(|| false);

    let page_title = if loc == "zh" { "\u{626b}\u{7801}\u{53d6}\u{9910}" } else { "Scan Voucher" };
    let input_label = t.t(&loc, "label.voucher_code");
    let scan_text = t.t(&loc, "btn.scan");
    let mismatch_text = t.t(&loc, "msg.mismatch_warning");

    rsx! {
        div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
            Navbar { locale: locale.clone() }
            main { class: "flex-1 max-w-2xl mx-auto px-4 py-8 w-full",
                h2 { class: "text-2xl font-bold mb-5 text-gray-800", "{page_title}" }

                div { class: "bg-white rounded-2xl shadow p-6 mb-6",
                    form { onsubmit: move |evt| {
                        evt.prevent_default();
                        let code = voucher_input().clone(); let oid_str = order_id_input().clone(); let session_cookie = app_state().auth.session_cookie.clone();
                        spawn(async move {
                            scanning.set(true); error_msg.set(None); scan_result.set(None);
                            let parsed_order_id = oid_str.trim().parse::<i64>().ok();
                            let body = ScanVoucherRequest { voucher_code: code, order_id: parsed_order_id };
                            let mut req = reqwest::Client::new().post(&format!("{}/staff/scan", &crate::api_base())).json(&body);
                            if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
                            match req.send().await {
                                Ok(resp) => {
                                    if resp.status().is_success() {
                                        match resp.json::<ApiResponse<ScanVoucherResponse>>().await {
                                            Ok(api) => { if let Some(data) = api.data { scan_result.set(Some(data)); } else { error_msg.set(Some(api.error.unwrap_or_else(|| "Scan failed".to_string()))); } }
                                            Err(e) => error_msg.set(Some(format!("Parse error: {}", e))),
                                        }
                                    } else { let body = resp.text().await.unwrap_or_default(); error_msg.set(Some(format!("Scan failed: {}", body))); }
                                }
                                Err(e) => error_msg.set(Some(format!("Network error: {}", e))),
                            }
                            scanning.set(false);
                        });
                    },
                        div { class: "mb-4",
                            label { r#for: "voucher-code", class: "block text-sm font-medium text-gray-700 mb-1", "{input_label}" }
                            input { r#type: "text", id: "voucher-code", class: INPUT, autofocus: true,
                                placeholder: if loc == "zh" { "\u{8f93}\u{5165}\u{6216}\u{626b}\u{63cf}\u{53d6}\u{9910}\u{7801}" } else { "Enter or scan voucher code" },
                                value: "{voucher_input}", oninput: move |evt| voucher_input.set(evt.value()),
                            }
                        }
                        div { class: "mb-4",
                            label { r#for: "order-id", class: "block text-sm font-medium text-gray-700 mb-1",
                                if loc == "zh" { "\u{8ba2}\u{5355}ID (\u{53ef}\u{9009})" } else { "Order ID (optional)" }
                            }
                            input { r#type: "text", id: "order-id", class: INPUT,
                                placeholder: if loc == "zh" { "\u{8f93}\u{5165}\u{8ba2}\u{5355}ID\u{4ee5}\u{9a8c}\u{8bc1}\u{5339}\u{914d}" } else { "Enter order ID to verify match" },
                                value: "{order_id_input}", oninput: move |evt| order_id_input.set(evt.value()),
                            }
                        }
                        button { r#type: "submit", class: "w-full inline-flex items-center justify-center px-6 py-3 rounded-lg text-base font-medium bg-primary text-white hover:bg-primary-dark transition-all disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: scanning() || voucher_input().is_empty(),
                            if scanning() { "..." } else { "{scan_text}" }
                        }
                    }
                }

                if let Some(err) = error_msg() { div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm mb-4", "{err}" } }

                if let Some(result) = scan_result() {
                    div { class: "space-y-4",
                        if result.mismatch {
                            div { class: "bg-red-50 border-2 border-red-300 rounded-xl p-5 text-red-800",
                                h3 { class: "font-bold mb-1", "{mismatch_text}" }
                                if let Some(ref reason) = result.mismatch_reason { p { class: "text-sm", "{reason}" } }
                            }
                        }

                        div { class: if result.valid { "text-center p-4 bg-green-50 rounded-xl" } else { "text-center p-4 bg-red-50 rounded-xl" },
                            span { class: if result.valid { "text-2xl font-bold text-green-600" } else { "text-2xl font-bold text-red-600" },
                                if result.valid {
                                    if loc == "zh" { "\u{2713} \u{6709}\u{6548}" } else { "\u{2713} Valid" }
                                } else {
                                    if loc == "zh" { "\u{2717} \u{65e0}\u{6548}" } else { "\u{2717} Invalid" }
                                }
                            }
                        }

                        if let Some(ref order) = result.order {
                            div { class: "bg-white rounded-xl shadow p-5",
                                h3 { class: "font-semibold mb-3", if loc == "zh" { "\u{8ba2}\u{5355}\u{8be6}\u{60c5}" } else { "Order Details" } }
                                div { class: "flex justify-between items-center mb-2",
                                    span { class: "font-semibold", "#{order.order_number}" }
                                    StatusBadge { status: order.status.clone(), locale: locale.clone() }
                                }
                                div { class: "flex justify-between items-center mb-2",
                                    PriceDisplay { amount: order.total, locale: locale.clone() }
                                    span { class: "text-xs text-gray-400", "{order.created_at}" }
                                }
                                if let Some(ref slot) = order.pickup_slot {
                                    p { class: "text-sm text-gray-500 mb-3", if loc == "zh" { "\u{53d6}\u{9910}: " } else { "Pickup: " } "{slot}" }
                                }
                                Link { to: crate::Route::StaffOrderDetail { locale: locale.clone(), id: order.id }, class: BTN_PRIMARY,
                                    if loc == "zh" { "\u{7ba1}\u{7406}\u{8ba2}\u{5355}" } else { "Manage Order" }
                                }
                            }
                        }
                    }
                }
            }
            Footer {}
        }
    }
}
