use dioxus::prelude::*;
use crate::components::navbar::Navbar;
use crate::components::footer::Footer;
use crate::components::price_display::PriceDisplay;
use crate::components::status_badge::StatusBadge;
use crate::components::hold_timer::HoldTimer;
use crate::state::AppState;
use shared::dto::{ApiResponse, OrderDetail, OrderSummary};

#[component]
pub fn OrdersPage(locale: String) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.clone();
    let app_state = use_context::<Signal<AppState>>();

    let orders_resource = use_resource(move || {
        let session_cookie = app_state().auth.session_cookie.clone();
        async move {
            let mut req = reqwest::Client::new().get(&format!("{}/orders", &crate::api_base()));
            if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
            let resp = req.send().await.map_err(|e| e.to_string())?;
            let data: ApiResponse<Vec<OrderSummary>> = resp.json().await.map_err(|e| e.to_string())?;
            data.data.ok_or_else(|| "No data".to_string())
        }
    });

    let page_title = t.t(&loc, "page.orders");

    rsx! {
        div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
            Navbar { locale: locale.clone() }
            main { class: "flex-1 max-w-7xl mx-auto px-4 py-8 w-full",
                h2 { class: "text-2xl font-bold mb-5 text-gray-800", "{page_title}" }

                match &*orders_resource.read() {
                    Some(Ok(orders)) => {
                        if orders.is_empty() {
                            rsx! {
                                div { class: "text-center py-16",
                                    p { class: "text-gray-400 text-lg mb-4", if loc == "zh" { "\u{6682}\u{65e0}\u{8ba2}\u{5355}" } else { "No orders yet" } }
                                    Link { to: crate::Route::Menu { locale: locale.clone() }, class: "inline-flex items-center justify-center px-5 py-2.5 rounded-lg text-sm font-medium bg-primary text-white hover:bg-primary-dark transition-all no-underline",
                                        if loc == "zh" { "\u{53bb}\u{9009}\u{8d2d}" } else { "Browse Menu" }
                                    }
                                }
                            }
                        } else {
                            rsx! {
                                div { class: "space-y-4",
                                    for order in orders.iter() {
                                        { let oid = order.id; rsx! {
                                            Link { to: crate::Route::OrderDetail { locale: locale.clone(), id: oid },
                                                class: "block bg-white rounded-xl shadow-sm hover:shadow-md transition-shadow p-5 no-underline text-gray-800",
                                                div { class: "flex justify-between items-center mb-3",
                                                    span { class: "font-semibold text-lg", "#{order.order_number}" }
                                                    StatusBadge { status: order.status.clone(), locale: locale.clone() }
                                                }
                                                div { class: "flex justify-between items-center",
                                                    PriceDisplay { amount: order.total, locale: locale.clone() }
                                                    span { class: "text-sm text-gray-400", "{order.created_at}" }
                                                }
                                                if let Some(ref voucher) = order.voucher_code {
                                                    p { class: "text-sm text-gray-500 mt-2 font-mono", if loc == "zh" { "\u{53d6}\u{9910}\u{7801}: " } else { "Voucher: " } "{voucher}" }
                                                }
                                                if let Some(ref slot) = order.pickup_slot {
                                                    p { class: "text-sm text-gray-400 mt-1", if loc == "zh" { "\u{53d6}\u{9910}: " } else { "Pickup: " } "{slot}" }
                                                }
                                            }
                                        } }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! { div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm", "Error: {e}" } },
                    None => rsx! { div { class: "text-center py-12 text-gray-400", "Loading..." } },
                }
            }
            Footer {}
        }
    }
}

#[component]
pub fn OrderDetailPage(locale: String, id: i64) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.clone();
    let app_state = use_context::<Signal<AppState>>();

    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut action_loading = use_signal(|| false);
    let mut refresh_trigger = use_signal(|| 0u32);

    let detail_resource = use_resource(move || {
        let _trigger = refresh_trigger();
        let session_cookie = app_state().auth.session_cookie.clone();
        async move {
            let mut req = reqwest::Client::new().get(&format!("{}/orders/{}", &crate::api_base(), id));
            if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
            let resp = req.send().await.map_err(|e| e.to_string())?;
            let data: ApiResponse<OrderDetail> = resp.json().await.map_err(|e| e.to_string())?;
            data.data.ok_or_else(|| "Order not found".to_string())
        }
    });

    let page_title = t.t(&loc, "page.order_detail");
    let total_label = t.t(&loc, "label.total");
    let voucher_label = t.t(&loc, "label.voucher_code");
    let confirm_text = t.t(&loc, "btn.confirm");
    let cancel_text = t.t(&loc, "btn.cancel");

    rsx! {
        div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
            Navbar { locale: locale.clone() }
            main { class: "flex-1 max-w-3xl mx-auto px-4 py-8 w-full",
                h2 { class: "text-2xl font-bold mb-5 text-gray-800", "{page_title}" }

                if let Some(err) = error_msg() {
                    div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm mb-4", "{err}" }
                }

                match &*detail_resource.read() {
                    Some(Ok(detail)) => {
                        let order = &detail.order;
                        let status = &order.status;
                        let can_confirm = status == "Pending";
                        let can_cancel = status == "Pending" || status == "Accepted";

                        rsx! {
                            div { class: "bg-white rounded-2xl shadow p-6 space-y-6",
                                // Header
                                div { class: "flex justify-between items-center",
                                    h3 { class: "text-xl font-bold", "#{order.order_number}" }
                                    StatusBadge { status: order.status.clone(), locale: locale.clone() }
                                }

                                // Items
                                div {
                                    h4 { class: "font-semibold text-gray-700 mb-3", if loc == "zh" { "\u{8ba2}\u{5355}\u{9879}\u{76ee}" } else { "Items" } }
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
                                    }
                                    div { class: "flex justify-between font-bold mt-3 pt-3 border-t border-gray-100",
                                        span { "{total_label}" }
                                        PriceDisplay { amount: order.total, locale: locale.clone() }
                                    }
                                }

                                // Reservation
                                if let Some(ref reservation) = detail.reservation {
                                    div { class: "p-4 bg-gray-50 rounded-xl",
                                        h4 { class: "font-semibold text-gray-700 mb-3", if loc == "zh" { "\u{9884}\u{7ea6}\u{4fe1}\u{606f}" } else { "Reservation" } }
                                        div { class: "text-center py-4 px-4 border-2 border-dashed border-primary rounded-xl mb-3",
                                            p { class: "text-sm text-gray-500 mb-1", "{voucher_label}" }
                                            p { class: "text-2xl font-bold tracking-widest text-primary font-mono", "{reservation.voucher_code}" }
                                        }
                                        p { class: "text-sm text-gray-500 mb-2",
                                            if loc == "zh" { "\u{53d6}\u{9910}\u{65f6}\u{6bb5}: " } else { "Pickup: " }
                                            "{reservation.pickup_slot_start} - {reservation.pickup_slot_end}"
                                        }
                                        div { class: "flex items-center gap-3 mb-2",
                                            StatusBadge { status: reservation.status.clone(), locale: locale.clone() }
                                        }
                                        if reservation.status == "Held" {
                                            HoldTimer { expires_at: reservation.hold_expires_at.clone(), locale: locale.clone() }
                                        }
                                    }
                                }

                                // Timeline
                                if !detail.fulfillment_history.is_empty() {
                                    div {
                                        h4 { class: "font-semibold text-gray-700 mb-3", if loc == "zh" { "\u{5c65}\u{7ea6}\u{65f6}\u{95f4}\u{7ebf}" } else { "Fulfillment Timeline" } }
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

                                // Actions
                                if can_confirm || can_cancel {
                                    div { class: "flex gap-3",
                                        if can_confirm {
                                            { rsx! { button {
                                                class: "inline-flex items-center justify-center px-5 py-2.5 rounded-lg text-sm font-medium bg-primary text-white hover:bg-primary-dark transition-all disabled:opacity-50 disabled:cursor-not-allowed",
                                                disabled: action_loading(),
                                                onclick: move |_| {
                                                    let session_cookie = app_state().auth.session_cookie.clone();
                                                    spawn(async move {
                                                        action_loading.set(true); error_msg.set(None);
                                                        let body = serde_json::json!({"action":"confirm"});
                                                        let mut req = reqwest::Client::new().post(&format!("{}/orders/{}/confirm", &crate::api_base(), id)).json(&body);
                                                        if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
                                                        match req.send().await {
                                                            Ok(resp) if resp.status().is_success() => { refresh_trigger.set(refresh_trigger()+1); }
                                                            Ok(resp) => { let body = resp.text().await.unwrap_or_default(); error_msg.set(Some(format!("Failed: {}", body))); }
                                                            Err(e) => error_msg.set(Some(format!("Error: {}", e))),
                                                        }
                                                        action_loading.set(false);
                                                    });
                                                },
                                                "{confirm_text}"
                                            } } }
                                        }
                                        if can_cancel {
                                            { rsx! { button {
                                                class: "inline-flex items-center justify-center px-5 py-2.5 rounded-lg text-sm font-medium bg-red-500 text-white hover:bg-red-700 transition-all disabled:opacity-50 disabled:cursor-not-allowed",
                                                disabled: action_loading(),
                                                onclick: move |_| {
                                                    let session_cookie = app_state().auth.session_cookie.clone();
                                                    spawn(async move {
                                                        action_loading.set(true); error_msg.set(None);
                                                        let body = serde_json::json!({"action":"cancel"});
                                                        let mut req = reqwest::Client::new().post(&format!("{}/orders/{}/cancel", &crate::api_base(), id)).json(&body);
                                                        if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
                                                        match req.send().await {
                                                            Ok(resp) if resp.status().is_success() => { refresh_trigger.set(refresh_trigger()+1); }
                                                            Ok(resp) => { let body = resp.text().await.unwrap_or_default(); error_msg.set(Some(format!("Failed: {}", body))); }
                                                            Err(e) => error_msg.set(Some(format!("Error: {}", e))),
                                                        }
                                                        action_loading.set(false);
                                                    });
                                                },
                                                "{cancel_text}"
                                            } } }
                                        }
                                    }
                                }

                                p { class: "text-sm text-gray-400",
                                    if loc == "zh" { "\u{521b}\u{5efa}\u{65f6}\u{95f4}: " } else { "Created: " }
                                    "{order.created_at}"
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! { div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm", "Error: {e}" } },
                    None => rsx! { div { class: "text-center py-12 text-gray-400", "Loading..." } },
                }
            }
            Footer {}
        }
    }
}
