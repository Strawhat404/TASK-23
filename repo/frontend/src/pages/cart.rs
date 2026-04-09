use dioxus::prelude::*;
use crate::components::navbar::Navbar;
use crate::components::footer::Footer;
use crate::components::price_display::PriceDisplay;
use crate::state::AppState;
use shared::dto::{ApiResponse, CartResponse};

const QTY_BTN: &str = "inline-flex items-center justify-center w-8 h-8 rounded-lg text-sm font-medium bg-gray-100 text-gray-700 hover:bg-gray-200 transition-all disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer";

#[component]
pub fn CartPage(locale: String) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.clone();
    let mut app_state = use_context::<Signal<AppState>>();
    let mut cart_data = use_signal(|| Option::<Result<CartResponse, String>>::None);
    let mut loading = use_signal(|| true);
    let mut update_trigger = use_signal(|| 0u32);

    use_resource(move || {
        let _trigger = update_trigger();
        let session_cookie = app_state().auth.session_cookie.clone();
        async move {
            loading.set(true);
            let mut req = reqwest::Client::new().get(&format!("{}/cart", &crate::api_base()));
            if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
            match req.send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<ApiResponse<CartResponse>>().await {
                            Ok(api) => { if let Some(data) = api.data { let count = data.items.iter().map(|i| i.quantity).sum::<i32>(); app_state.write().cart_count = count; cart_data.set(Some(Ok(data))); } else { cart_data.set(Some(Err(api.error.unwrap_or_else(|| "No cart data".to_string())))); } }
                            Err(e) => cart_data.set(Some(Err(format!("Parse error: {}", e)))),
                        }
                    } else { cart_data.set(Some(Err(format!("HTTP {}", resp.status())))); }
                }
                Err(e) => cart_data.set(Some(Err(format!("Network error: {}", e)))),
            }
            loading.set(false);
        }
    });

    let page_title = t.t(&loc, "page.cart");
    let subtotal_label = t.t(&loc, "label.subtotal");
    let tax_label = t.t(&loc, "label.tax");
    let total_label = t.t(&loc, "label.total");
    let checkout_text = t.t(&loc, "btn.checkout");
    let empty_text = if loc == "zh" { "\u{8d2d}\u{7269}\u{8f66}\u{4e3a}\u{7a7a}" } else { "Your cart is empty" };
    let remove_text = if loc == "zh" { "\u{5220}\u{9664}" } else { "Remove" };

    rsx! {
        div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
            Navbar { locale: locale.clone() }
            main { class: "flex-1 max-w-7xl mx-auto px-4 py-8 w-full",
                h2 { class: "text-2xl font-bold mb-5 text-gray-800", "{page_title}" }

                if loading() {
                    div { class: "text-center py-12 text-gray-400", "Loading..." }
                } else {
                    match cart_data() {
                        Some(Ok(cart)) => {
                            if cart.items.is_empty() {
                                rsx! {
                                    div { class: "text-center py-16",
                                        p { class: "text-gray-400 text-lg mb-4", "{empty_text}" }
                                        Link { to: crate::Route::Menu { locale: locale.clone() },
                                            class: "inline-flex items-center justify-center px-5 py-2.5 rounded-lg text-sm font-medium bg-primary text-white hover:bg-primary-dark transition-all no-underline",
                                            if loc == "zh" { "\u{53bb}\u{9009}\u{8d2d}" } else { "Browse Menu" }
                                        }
                                    }
                                }
                            } else {
                                rsx! {
                                    div { class: "space-y-4 mb-6",
                                        for item in cart.items.iter() {
                                            {
                                                let item_name = if loc == "zh" { &item.spu_name_zh } else { &item.spu_name_en };
                                                let item_id = item.id;
                                                let item_qty = item.quantity;
                                                let options_text = item.options.join(", ");
                                                rsx! {
                                                    div { class: "flex items-center justify-between p-4 bg-white rounded-xl shadow-sm",
                                                        div { class: "flex-1 min-w-0",
                                                            h3 { class: "font-semibold text-gray-800", "{item_name}" }
                                                            if !options_text.is_empty() { p { class: "text-sm text-gray-400 mt-0.5", "{options_text}" } }
                                                        }
                                                        div { class: "flex items-center gap-4 ml-4",
                                                            div { class: "flex items-center gap-2",
                                                                button { class: QTY_BTN, disabled: item_qty <= 1,
                                                                    onclick: move |_| { let sc = app_state().auth.session_cookie.clone(); let nq = item_qty - 1; spawn(async move { let mut req = reqwest::Client::new().put(&format!("{}/cart/{}", &crate::api_base(), item_id)).json(&serde_json::json!({"quantity":nq})); if let Some(ref s)=sc{req=req.header("Cookie",format!("brewflow_session={}",s));} let _=req.send().await; update_trigger.set(update_trigger()+1); }); },
                                                                    "-"
                                                                }
                                                                span { class: "text-sm font-semibold w-6 text-center tabular-nums", "{item_qty}" }
                                                                button { class: QTY_BTN,
                                                                    onclick: move |_| { let sc = app_state().auth.session_cookie.clone(); let nq = item_qty + 1; spawn(async move { let mut req = reqwest::Client::new().put(&format!("{}/cart/{}", &crate::api_base(), item_id)).json(&serde_json::json!({"quantity":nq})); if let Some(ref s)=sc{req=req.header("Cookie",format!("brewflow_session={}",s));} let _=req.send().await; update_trigger.set(update_trigger()+1); }); },
                                                                    "+"
                                                                }
                                                            }
                                                            PriceDisplay { amount: item.line_total, locale: locale.clone() }
                                                            button { class: "text-xs text-red-500 hover:text-red-700 cursor-pointer",
                                                                onclick: move |_| { let sc = app_state().auth.session_cookie.clone(); spawn(async move { let mut req = reqwest::Client::new().delete(&format!("{}/cart/{}", &crate::api_base(), item_id)); if let Some(ref s)=sc{req=req.header("Cookie",format!("brewflow_session={}",s));} let _=req.send().await; update_trigger.set(update_trigger()+1); }); },
                                                                "{remove_text}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Totals
                                    div { class: "p-4 bg-gray-50 rounded-xl space-y-2 mb-6",
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

                                    div { class: "flex justify-between gap-4",
                                        Link { to: crate::Route::Menu { locale: locale.clone() },
                                            class: "inline-flex items-center justify-center px-5 py-2.5 rounded-lg text-sm font-medium bg-gray-100 text-gray-700 hover:bg-gray-200 transition-all no-underline",
                                            if loc == "zh" { "\u{7ee7}\u{7eed}\u{9009}\u{8d2d}" } else { "Continue Shopping" }
                                        }
                                        Link { to: crate::Route::Checkout { locale: locale.clone() },
                                            class: "inline-flex items-center justify-center px-6 py-3 rounded-lg text-base font-medium bg-primary text-white hover:bg-primary-dark transition-all no-underline",
                                            "{checkout_text}"
                                        }
                                    }
                                }
                            }
                        },
                        Some(Err(e)) => rsx! { div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm", "Error: {e}" } },
                        None => rsx! { div { class: "text-center py-12 text-gray-400", "Loading..." } },
                    }
                }
            }
            Footer {}
        }
    }
}
