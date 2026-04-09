use dioxus::prelude::*;
use crate::components::navbar::Navbar;
use crate::components::footer::Footer;
use crate::components::price_display::PriceDisplay;
use crate::components::slot_picker::SlotPicker;
use crate::components::hold_timer::HoldTimer;
use crate::state::AppState;
use shared::dto::{ApiResponse, CartResponse, CheckoutRequest, CheckoutResponse, PickupSlot};

#[component]
pub fn CheckoutPage(locale: String) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.clone();
    let mut app_state = use_context::<Signal<AppState>>();

    let mut selected_slot = use_signal(|| Option::<PickupSlot>::None);
    let mut checkout_result = use_signal(|| Option::<CheckoutResponse>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut placing = use_signal(|| false);

    let cart_resource = use_resource(move || {
        let session_cookie = app_state().auth.session_cookie.clone();
        async move {
            let mut req = reqwest::Client::new().get(&format!("{}/cart", &crate::api_base()));
            if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
            let resp = req.send().await.map_err(|e| e.to_string())?;
            let data: ApiResponse<CartResponse> = resp.json().await.map_err(|e| e.to_string())?;
            data.data.ok_or_else(|| "No cart data".to_string())
        }
    });

    let slots_resource = use_resource(move || {
        let session_cookie = app_state().auth.session_cookie.clone();
        async move {
            let mut req = reqwest::Client::new().get(&format!("{}/store/pickup-slots?date={}", &crate::api_base(), chrono::Utc::now().format("%Y-%m-%d")));
            if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
            let resp = req.send().await.map_err(|e| e.to_string())?;
            let data: ApiResponse<Vec<PickupSlot>> = resp.json().await.map_err(|e| e.to_string())?;
            data.data.ok_or_else(|| "No slots available".to_string())
        }
    });

    let page_title = t.t(&loc, "page.checkout");
    let subtotal_label = t.t(&loc, "label.subtotal");
    let tax_label = t.t(&loc, "label.tax");
    let total_label = t.t(&loc, "label.total");
    let voucher_label = t.t(&loc, "label.voucher_code");
    let hold_warning = t.t(&loc, "msg.hold_warning");
    let place_order_text = if loc == "zh" { "\u{4e0b}\u{5355}" } else { "Place Order" };

    // Success view
    if let Some(result) = checkout_result() {
        return rsx! {
            div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
                Navbar { locale: locale.clone() }
                main { class: "flex-1 max-w-2xl mx-auto px-4 py-8 w-full",
                    div { class: "bg-white rounded-2xl shadow-lg p-8 text-center",
                        div { class: "text-5xl mb-4", "\u{2705}" }
                        h2 { class: "text-2xl font-bold text-gray-800 mb-2",
                            if loc == "zh" { "\u{8ba2}\u{5355}\u{5df2}\u{521b}\u{5efa}\u{ff01}" } else { "Order Placed!" }
                        }
                        p { class: "text-gray-500 mb-6",
                            if loc == "zh" { "\u{8ba2}\u{5355}\u{53f7}: " } else { "Order #: " }
                            strong { class: "text-gray-800", "{result.order_number}" }
                        }

                        div { class: "text-center py-6 px-4 bg-gray-50 border-2 border-dashed border-primary rounded-xl mb-6",
                            p { class: "text-sm text-gray-500 mb-2", "{voucher_label}" }
                            p { class: "text-3xl font-bold tracking-widest text-primary font-mono", "{result.voucher_code}" }
                        }

                        div { class: "mb-6",
                            HoldTimer { expires_at: result.hold_expires_at.clone(), locale: locale.clone() }
                        }

                        div { class: "bg-amber-50 border border-amber-200 text-amber-700 px-4 py-3 rounded-lg text-sm mb-6", "{hold_warning}" }

                        p { class: "text-sm text-gray-500 mb-2",
                            if loc == "zh" { "\u{53d6}\u{9910}\u{65f6}\u{6bb5}: " } else { "Pickup: " }
                            strong { "{result.pickup_slot}" }
                        }

                        div { class: "text-lg font-bold mb-6",
                            span { "{total_label}: " }
                            PriceDisplay { amount: result.total, locale: locale.clone() }
                        }

                        div { class: "flex justify-center gap-4 flex-wrap",
                            Link { to: crate::Route::OrderDetail { locale: locale.clone(), id: result.order_id },
                                class: "inline-flex items-center justify-center px-5 py-2.5 rounded-lg text-sm font-medium bg-primary text-white hover:bg-primary-dark transition-all no-underline",
                                if loc == "zh" { "\u{67e5}\u{770b}\u{8ba2}\u{5355}\u{8be6}\u{60c5}" } else { "View Order Detail" }
                            }
                            Link { to: crate::Route::Home { locale: locale.clone() },
                                class: "inline-flex items-center justify-center px-5 py-2.5 rounded-lg text-sm font-medium bg-gray-100 text-gray-700 hover:bg-gray-200 transition-all no-underline",
                                if loc == "zh" { "\u{8fd4}\u{56de}\u{9996}\u{9875}" } else { "Back to Home" }
                            }
                        }
                    }
                }
                Footer {}
            }
        };
    }

    rsx! {
        div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
            Navbar { locale: locale.clone() }
            main { class: "flex-1 max-w-4xl mx-auto px-4 py-8 w-full",
                h2 { class: "text-2xl font-bold mb-5 text-gray-800", "{page_title}" }

                if let Some(err) = error_msg() {
                    div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm mb-4", "{err}" }
                }

                div { class: "grid grid-cols-1 md:grid-cols-2 gap-8",
                    // Cart summary
                    div { class: "bg-white rounded-xl shadow p-6",
                        h3 { class: "text-lg font-semibold mb-4", if loc == "zh" { "\u{8ba2}\u{5355}\u{6458}\u{8981}" } else { "Order Summary" } }
                        match &*cart_resource.read() {
                            Some(Ok(cart)) => rsx! {
                                div { class: "space-y-3 mb-4",
                                    for item in cart.items.iter() {
                                        {
                                            let item_name = if loc == "zh" { &item.spu_name_zh } else { &item.spu_name_en };
                                            let options_text = item.options.join(", ");
                                            rsx! {
                                                div { class: "flex justify-between text-sm",
                                                    div {
                                                        span { class: "text-gray-800", "{item_name}" }
                                                        if !options_text.is_empty() { span { class: "text-gray-400", " ({options_text})" } }
                                                        span { class: "text-gray-400", " x{item.quantity}" }
                                                    }
                                                    PriceDisplay { amount: item.line_total, locale: locale.clone() }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "pt-3 border-t border-gray-100 space-y-2",
                                    div { class: "flex justify-between text-sm",
                                        span { class: "text-gray-500", "{subtotal_label}" }
                                        PriceDisplay { amount: cart.subtotal, locale: locale.clone() }
                                    }
                                    div { class: "flex justify-between text-sm",
                                        span { class: "text-gray-500", "{tax_label} ({cart.tax_rate * 100.0:.0}%)" }
                                        PriceDisplay { amount: cart.tax_amount, locale: locale.clone() }
                                    }
                                    div { class: "flex justify-between text-base font-bold pt-2 border-t border-gray-200",
                                        span { "{total_label}" }
                                        PriceDisplay { amount: cart.total, locale: locale.clone() }
                                    }
                                }
                            },
                            Some(Err(e)) => rsx! { div { class: "text-red-500 text-sm", "Error: {e}" } },
                            None => rsx! { div { class: "text-center py-8 text-gray-400", "Loading..." } },
                        }
                    }

                    // Slot picker
                    div { class: "bg-white rounded-xl shadow p-6",
                        match &*slots_resource.read() {
                            Some(Ok(slots)) => rsx! {
                                SlotPicker { slots: slots.clone(), locale: locale.clone(), on_select: move |slot: PickupSlot| { selected_slot.set(Some(slot)); } }
                            },
                            Some(Err(e)) => rsx! { div { class: "text-red-500 text-sm", "Error: {e}" } },
                            None => rsx! { div { class: "text-center py-8 text-gray-400", "Loading slots..." } },
                        }
                        if selected_slot().is_some() {
                            div { class: "mt-4 p-3 bg-green-50 rounded-lg text-sm text-green-700",
                                { let slot = selected_slot().unwrap(); rsx! { p { if loc == "zh" { "\u{5df2}\u{9009}\u{62e9}: " } else { "Selected: " } strong { "{slot.start} - {slot.end}" } } } }
                            }
                        }
                    }
                }

                // Place order
                div { class: "mt-6",
                    { let locale_place = locale.clone(); rsx! {
                        button {
                            class: "w-full inline-flex items-center justify-center px-6 py-3 rounded-lg text-base font-medium bg-primary text-white hover:bg-primary-dark transition-all disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: selected_slot().is_none() || placing(),
                            onclick: move |_| {
                                let Some(slot) = selected_slot() else { return; };
                                let session_cookie = app_state().auth.session_cookie.clone();
                                let locale_inner = locale_place.clone();
                                spawn(async move {
                                    placing.set(true); error_msg.set(None);
                                    let body = CheckoutRequest { pickup_slot_start: slot.start.clone(), pickup_slot_end: slot.end.clone() };
                                    let mut req = reqwest::Client::new().post(&format!("{}/orders/checkout", &crate::api_base())).json(&body);
                                    if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
                                    match req.send().await {
                                        Ok(resp) => {
                                            if resp.status().is_success() {
                                                match resp.json::<ApiResponse<CheckoutResponse>>().await {
                                                    Ok(api) => { if let Some(data) = api.data { app_state.write().cart_count = 0; checkout_result.set(Some(data)); } else { error_msg.set(Some(api.error.unwrap_or_else(|| "Checkout failed".to_string()))); } }
                                                    Err(e) => error_msg.set(Some(format!("Parse error: {}", e))),
                                                }
                                            } else { let body = resp.text().await.unwrap_or_default(); error_msg.set(Some(format!("Checkout failed: {}", body))); }
                                        }
                                        Err(e) => error_msg.set(Some(format!("Network error: {}", e))),
                                    }
                                    placing.set(false);
                                });
                            },
                            if placing() { "..." } else { "{place_order_text}" }
                        }
                    } }
                }
            }
            Footer {}
        }
    }
}
