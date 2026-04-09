use dioxus::prelude::*;
use crate::components::navbar::Navbar;
use crate::components::footer::Footer;
use crate::components::price_display::PriceDisplay;
use crate::components::option_selector::OptionSelector;
use crate::state::AppState;
use shared::dto::{AddToCartRequest, ApiResponse, ProductDetail};
use shared::models::SalesTaxConfig;

const BTN_SM: &str = "inline-flex items-center justify-center w-8 h-8 rounded-lg text-sm font-medium bg-gray-100 text-gray-700 hover:bg-gray-200 transition-all disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer";

#[component]
pub fn ProductDetailPage(locale: String, id: i64) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.clone();
    let mut app_state = use_context::<Signal<AppState>>();
    let nav = use_navigator();

    let mut quantity = use_signal(|| 1i32);
    let mut selected_options = use_signal(|| Vec::<i64>::new());
    let mut options_delta = use_signal(|| 0.0f64);
    let mut add_error = use_signal(|| Option::<String>::None);
    let mut add_success = use_signal(|| false);
    let mut adding = use_signal(|| false);

    let product_resource = use_resource(move || async move {
        let url = format!("{}/products/{}", &crate::api_base(), id);
        let resp = reqwest::Client::new().get(&url).send().await.map_err(|e| e.to_string())?;
        let data: ApiResponse<ProductDetail> = resp.json().await.map_err(|e| e.to_string())?;
        data.data.ok_or_else(|| "Product not found".to_string())
    });

    let tax_resource = use_resource(move || async move {
        let url = format!("{}/store/tax", &crate::api_base());
        let resp = reqwest::Client::new().get(&url).send().await.ok()?;
        let data: ApiResponse<SalesTaxConfig> = resp.json().await.ok()?;
        data.data
    });

    let tax_rate = tax_resource.read().as_ref().and_then(|opt| opt.as_ref()).map(|cfg| cfg.rate).unwrap_or(0.0);
    let tax_pct = format!("{:.1}%", tax_rate * 100.0);

    rsx! {
        div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
            Navbar { locale: locale.clone() }

            main { class: "flex-1 max-w-7xl mx-auto px-4 py-8 w-full",
                match &*product_resource.read() {
                    Some(Ok(detail)) => {
                        let spu = &detail.spu;
                        let name = if loc == "zh" { &spu.name_zh } else { &spu.name_en };
                        let desc = if loc == "zh" { spu.description_zh.as_deref().unwrap_or("") } else { spu.description_en.as_deref().unwrap_or("") };
                        let cat = spu.category.as_deref().unwrap_or("");
                        let base_price = spu.base_price;
                        let delta = options_delta();
                        let unit_price = base_price + delta;
                        let qty = quantity();
                        let line_total = unit_price * qty as f64;
                        let tax_amount = line_total * tax_rate;
                        let total_with_tax = line_total + tax_amount;
                        let prep = spu.prep_time_minutes;
                        let groups = detail.option_groups.clone();
                        let quantity_label = t.t(&loc, "label.quantity");
                        let tax_label = t.t(&loc, "label.tax");
                        let total_label = t.t(&loc, "label.total");
                        let add_text = t.t(&loc, "btn.add_to_cart");

                        rsx! {
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-8",
                                // Image
                                div { class: "aspect-square bg-gray-100 rounded-2xl flex items-center justify-center text-6xl text-gray-300 overflow-hidden",
                                    if let Some(ref img) = spu.image_url {
                                        img { src: "{img}", alt: "{name}", class: "w-full h-full object-cover" }
                                    } else {
                                        "\u{2615}"
                                    }
                                }
                                // Info
                                div {
                                    span { class: "text-xs font-medium text-primary-light uppercase tracking-wide", "{cat}" }
                                    h1 { class: "text-2xl font-bold text-gray-800 mt-1", "{name}" }
                                    p { class: "text-gray-500 mt-2", "{desc}" }
                                    p { class: "text-sm text-gray-400 mt-1",
                                        if loc == "zh" { "\u{5236}\u{4f5c}\u{65f6}\u{95f4}: {prep}\u{5206}\u{949f}" } else { "Prep time: {prep} min" }
                                    }

                                    div { class: "flex justify-between items-center mt-4 py-3 border-t border-gray-100",
                                        span { class: "text-sm text-gray-500", if loc == "zh" { "\u{57fa}\u{7840}\u{4ef7}\u{683c}" } else { "Base price" } }
                                        PriceDisplay { amount: base_price, locale: locale.clone() }
                                    }

                                    if !groups.is_empty() {
                                        OptionSelector { groups: groups, locale: locale.clone(),
                                            on_change: move |(ids, delta): (Vec<i64>, f64)| { selected_options.set(ids); options_delta.set(delta); },
                                        }
                                    }

                                    // Quantity
                                    div { class: "flex items-center justify-between mt-4",
                                        span { class: "text-sm font-medium text-gray-700", "{quantity_label}" }
                                        div { class: "flex items-center gap-3",
                                            button { class: BTN_SM, disabled: qty <= 1, onclick: move |_| { if quantity() > 1 { quantity.set(quantity() - 1); } }, "-" }
                                            span { class: "text-lg font-semibold w-8 text-center tabular-nums", "{qty}" }
                                            button { class: BTN_SM, onclick: move |_| quantity.set(quantity() + 1), "+" }
                                        }
                                    }

                                    // Price summary
                                    div { class: "mt-4 p-4 bg-gray-50 rounded-xl space-y-2",
                                        div { class: "flex justify-between text-sm",
                                            span { class: "text-gray-500", if loc == "zh" { "\u{5355}\u{4ef7}" } else { "Unit price" } }
                                            PriceDisplay { amount: unit_price, locale: locale.clone() }
                                        }
                                        div { class: "flex justify-between text-sm",
                                            span { class: "text-gray-500", "{tax_label} ({tax_pct})" }
                                            PriceDisplay { amount: tax_amount, locale: locale.clone() }
                                        }
                                        div { class: "flex justify-between text-base font-bold pt-2 border-t border-gray-200",
                                            span { "{total_label}" }
                                            PriceDisplay { amount: total_with_tax, locale: locale.clone() }
                                        }
                                    }

                                    if let Some(err) = add_error() {
                                        div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm mt-4", "{err}" }
                                    }
                                    if add_success() {
                                        div { class: "bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg text-sm mt-4",
                                            if loc == "zh" { "\u{5df2}\u{52a0}\u{5165}\u{8d2d}\u{7269}\u{8f66}\u{ff01}" } else { "Added to cart!" }
                                        }
                                    }

                                    {
                                        let locale_cart = locale.clone();
                                        rsx! {
                                            button {
                                                class: "w-full mt-4 inline-flex items-center justify-center px-6 py-3 rounded-lg text-base font-medium bg-primary text-white hover:bg-primary-dark transition-all disabled:opacity-50 disabled:cursor-not-allowed",
                                                disabled: adding(),
                                                onclick: move |_| {
                                                    let session_cookie = app_state().auth.session_cookie.clone();
                                                    let opts = selected_options().clone();
                                                    let qty_val = quantity();
                                                    let locale_c = locale_cart.clone();
                                                    spawn(async move {
                                                        adding.set(true); add_error.set(None); add_success.set(false);
                                                        let body = AddToCartRequest { sku_id: None, spu_id: id, selected_options: opts, quantity: qty_val };
                                                        let mut req = reqwest::Client::new().post(&format!("{}/cart/add", &crate::api_base())).json(&body);
                                                        if let Some(ref sc) = session_cookie { req = req.header("Cookie", format!("brewflow_session={}", sc)); }
                                                        match req.send().await {
                                                            Ok(resp) => {
                                                                if resp.status().is_success() { add_success.set(true); app_state.write().cart_count += qty_val; }
                                                                else { let body = resp.text().await.unwrap_or_default(); add_error.set(Some(format!("Failed: {}", body))); }
                                                            }
                                                            Err(e) => add_error.set(Some(format!("Network error: {}", e))),
                                                        }
                                                        adding.set(false);
                                                    });
                                                },
                                                if adding() { "..." } else { "{add_text}" }
                                            }
                                        }
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
