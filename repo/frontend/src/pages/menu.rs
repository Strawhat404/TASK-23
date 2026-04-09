use dioxus::prelude::*;
use crate::components::navbar::Navbar;
use crate::components::footer::Footer;
use crate::components::price_display::PriceDisplay;
use shared::dto::{ApiResponse, ProductListItem};

#[component]
pub fn MenuPage(locale: String) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.clone();

    let mut category_filter = use_signal(|| String::new());

    let products_resource = use_resource(move || {
        async move {
            let url = format!("{}/products", &crate::api_base());
            let resp = reqwest::Client::new().get(&url).send().await.map_err(|e| e.to_string())?;
            let data: ApiResponse<Vec<ProductListItem>> = resp.json().await.map_err(|e| e.to_string())?;
            data.data.ok_or_else(|| "No data returned".to_string())
        }
    });

    let page_title = t.t(&loc, "page.menu");

    rsx! {
        div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
            Navbar { locale: locale.clone() }

            main { class: "flex-1 max-w-7xl mx-auto px-4 py-8 w-full",
                h2 { class: "text-2xl font-bold mb-5 text-gray-800", "{page_title}" }

                match &*products_resource.read() {
                    Some(Ok(products)) => {
                        let mut categories: Vec<String> = products
                            .iter()
                            .filter_map(|p| p.category.clone())
                            .collect::<std::collections::HashSet<_>>()
                            .into_iter()
                            .collect();
                        categories.sort();

                        let current_filter = category_filter();
                        let filtered: Vec<&ProductListItem> = products
                            .iter()
                            .filter(|p| {
                                if current_filter.is_empty() { true }
                                else { p.category.as_deref().unwrap_or("") == current_filter.as_str() }
                            })
                            .collect();

                        rsx! {
                            // Filter bar
                            div { class: "flex flex-wrap gap-2 mb-6",
                                button {
                                    class: if current_filter.is_empty() { "px-4 py-2 rounded-full text-sm font-medium bg-primary text-white" } else { "px-4 py-2 rounded-full text-sm font-medium bg-white text-gray-600 border border-gray-200 hover:bg-gray-50 transition-all cursor-pointer" },
                                    onclick: move |_| category_filter.set(String::new()),
                                    if loc == "zh" { "\u{5168}\u{90e8}" } else { "All" }
                                }
                                for cat in categories.iter() {
                                    {
                                        let cat_clone = cat.clone();
                                        let is_active = current_filter == *cat;
                                        rsx! {
                                            button {
                                                class: if is_active { "px-4 py-2 rounded-full text-sm font-medium bg-primary text-white" } else { "px-4 py-2 rounded-full text-sm font-medium bg-white text-gray-600 border border-gray-200 hover:bg-gray-50 transition-all cursor-pointer" },
                                                onclick: move |_| category_filter.set(cat_clone.clone()),
                                                "{cat}"
                                            }
                                        }
                                    }
                                }
                            }

                            if filtered.is_empty() {
                                p { class: "text-center py-12 text-gray-400",
                                    if loc == "zh" { "\u{6ca1}\u{6709}\u{627e}\u{5230}\u{4ea7}\u{54c1}" } else { "No products found" }
                                }
                            }

                            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6",
                                for product in filtered.iter() {
                                    {
                                        let name = if loc == "zh" { &product.name_zh } else { &product.name_en };
                                        let desc = if loc == "zh" { product.description_zh.as_deref().unwrap_or("") } else { product.description_en.as_deref().unwrap_or("") };
                                        let cat = product.category.as_deref().unwrap_or("");
                                        let pid = product.spu_id;
                                        let price = product.base_price;
                                        let prep = product.prep_time_minutes;
                                        rsx! {
                                            Link {
                                                to: crate::Route::ProductDetail { locale: locale.clone(), id: pid },
                                                class: "bg-white rounded-xl shadow hover:shadow-md transition-shadow overflow-hidden no-underline text-gray-800 block",
                                                div { class: "h-44 bg-gray-100 flex items-center justify-center text-4xl text-gray-300",
                                                    if let Some(ref img) = product.image_url {
                                                        img { src: "{img}", alt: "{name}", class: "w-full h-full object-cover" }
                                                    } else {
                                                        "\u{2615}"
                                                    }
                                                }
                                                div { class: "p-5",
                                                    span { class: "text-xs font-medium text-primary-light uppercase tracking-wide", "{cat}" }
                                                    h3 { class: "text-lg font-semibold mt-1", "{name}" }
                                                    p { class: "text-gray-500 text-sm mt-1 line-clamp-2", "{desc}" }
                                                    div { class: "flex justify-between items-center mt-4",
                                                        PriceDisplay { amount: price, locale: locale.clone() }
                                                        span { class: "text-xs text-gray-400",
                                                            if loc == "zh" { "{prep}\u{5206}\u{949f}" } else { "{prep} min" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm", "Failed to load menu: {e}" }
                    },
                    None => rsx! {
                        div { class: "text-center py-12 text-gray-400", "Loading..." }
                    },
                }
            }

            Footer {}
        }
    }
}
