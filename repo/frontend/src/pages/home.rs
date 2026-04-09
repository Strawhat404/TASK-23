use dioxus::prelude::*;
use crate::components::navbar::Navbar;
use crate::components::footer::Footer;
use crate::components::price_display::PriceDisplay;
use shared::dto::{ApiResponse, ProductListItem};
use shared::models::StoreHours;

#[component]
pub fn HomePage(locale: String) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.clone();

    let featured = use_resource(move || {
        let locale = loc.clone();
        async move {
            let url = format!("{}/products?featured=true&limit=3", &crate::api_base());
            let resp = reqwest::Client::new().get(&url).send().await.map_err(|e| e.to_string())?;
            let data: ApiResponse<Vec<ProductListItem>> = resp.json().await.map_err(|e| e.to_string())?;
            data.data.ok_or_else(|| "No data returned".to_string())
        }
    });

    let store_hours = use_resource(move || async move {
        let url = format!("{}/store/hours", &crate::api_base());
        let resp = reqwest::Client::new().get(&url).send().await.map_err(|e| e.to_string())?;
        let data: ApiResponse<Vec<StoreHours>> = resp.json().await.map_err(|e| e.to_string())?;
        data.data.ok_or_else(|| "No hours data".to_string())
    });

    let loc = locale.clone();
    let hero_title = if loc == "zh" { "BrewFlow - \u{60a8}\u{7684}\u{667a}\u{80fd}\u{5496}\u{5561}\u{4f34}\u{4fa3}" } else { "BrewFlow - Your Smart Coffee Companion" };
    let hero_subtitle = if loc == "zh" { "\u{7ebf}\u{4e0a}\u{70b9}\u{5355}\u{ff0c}\u{5230}\u{5e97}\u{53d6}\u{9910}\u{ff0c}\u{667a}\u{80fd}\u{57f9}\u{8bad}" } else { "Order online, pick up in-store, train smarter" };
    let featured_title = t.t(&loc, "nav.menu");
    let hours_title = if loc == "zh" { "\u{8425}\u{4e1a}\u{65f6}\u{95f4}" } else { "Store Hours" };
    let menu_link_text = t.t(&loc, "nav.menu");
    let training_link_text = t.t(&loc, "nav.training");

    rsx! {
        div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
            Navbar { locale: locale.clone() }

            main { class: "flex-1 max-w-7xl mx-auto px-4 py-8 w-full",
                // Hero
                section { class: "text-center py-12 px-4 bg-gradient-to-br from-primary to-primary-dark text-white rounded-2xl mb-8",
                    h1 { class: "text-3xl md:text-4xl font-bold mb-3", "{hero_title}" }
                    p { class: "text-lg opacity-90 max-w-xl mx-auto mb-6", "{hero_subtitle}" }
                    div { class: "flex justify-center gap-4 flex-wrap",
                        Link {
                            to: crate::Route::Menu { locale: locale.clone() },
                            class: "inline-flex items-center justify-center px-6 py-3 rounded-lg text-base font-medium bg-white text-primary hover:bg-gray-100 transition-all no-underline",
                            "{menu_link_text}"
                        }
                        Link {
                            to: crate::Route::Training { locale: locale.clone() },
                            class: "inline-flex items-center justify-center px-6 py-3 rounded-lg text-base font-medium bg-white/20 text-white border border-white/30 hover:bg-white/30 transition-all no-underline",
                            "{training_link_text}"
                        }
                    }
                }

                // Featured products
                section { class: "mb-10",
                    h2 { class: "text-2xl font-bold mb-5 text-gray-800", "{featured_title}" }
                    div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6",
                        match &*featured.read() {
                            Some(Ok(products)) => rsx! {
                                for product in products.iter() {
                                    {
                                        let name = if loc == "zh" { &product.name_zh } else { &product.name_en };
                                        let desc = if loc == "zh" { product.description_zh.as_deref().unwrap_or("") } else { product.description_en.as_deref().unwrap_or("") };
                                        let cat = product.category.as_deref().unwrap_or("");
                                        let pid = product.spu_id;
                                        let price = product.base_price;
                                        rsx! {
                                            div { class: "bg-white rounded-xl shadow hover:shadow-md transition-shadow overflow-hidden",
                                                div { class: "h-44 bg-gray-100 flex items-center justify-center text-4xl text-gray-300",
                                                    if let Some(ref img) = product.image_url {
                                                        img { src: "{img}", alt: "{name}", class: "w-full h-full object-cover" }
                                                    } else {
                                                        "\u{2615}"
                                                    }
                                                }
                                                div { class: "p-5",
                                                    span { class: "text-xs font-medium text-primary-light uppercase tracking-wide", "{cat}" }
                                                    h3 { class: "text-lg font-semibold mt-1 text-gray-800", "{name}" }
                                                    p { class: "text-gray-500 text-sm mt-1 line-clamp-2", "{desc}" }
                                                    div { class: "flex justify-between items-center mt-4",
                                                        PriceDisplay { amount: price, locale: locale.clone() }
                                                        Link {
                                                            to: crate::Route::ProductDetail { locale: locale.clone(), id: pid },
                                                            class: "inline-flex items-center justify-center px-3 py-1.5 text-xs rounded-lg font-medium bg-primary text-white hover:bg-primary-dark transition-all no-underline",
                                                            if loc == "zh" { "\u{67e5}\u{770b}" } else { "View" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Err(e)) => rsx! {
                                p { class: "text-red-500 text-sm", "Failed to load featured products: {e}" }
                            },
                            None => rsx! {
                                div { class: "text-center py-12 text-gray-400", "Loading..." }
                            },
                        }
                    }
                }

                // Store hours
                section { class: "mb-10",
                    h2 { class: "text-2xl font-bold mb-5 text-gray-800", "{hours_title}" }
                    div { class: "grid grid-cols-2 sm:grid-cols-4 md:grid-cols-7 gap-3",
                        match &*store_hours.read() {
                            Some(Ok(hours_list)) => rsx! {
                                for h in hours_list.iter() {
                                    {
                                        let day_name = match h.day_of_week {
                                            0 => if loc == "zh" { "\u{5468}\u{65e5}" } else { "Sun" },
                                            1 => if loc == "zh" { "\u{5468}\u{4e00}" } else { "Mon" },
                                            2 => if loc == "zh" { "\u{5468}\u{4e8c}" } else { "Tue" },
                                            3 => if loc == "zh" { "\u{5468}\u{4e09}" } else { "Wed" },
                                            4 => if loc == "zh" { "\u{5468}\u{56db}" } else { "Thu" },
                                            5 => if loc == "zh" { "\u{5468}\u{4e94}" } else { "Fri" },
                                            6 => if loc == "zh" { "\u{5468}\u{516d}" } else { "Sat" },
                                            _ => "?",
                                        };
                                        let open = h.open_time.clone();
                                        let close = h.close_time.clone();
                                        rsx! {
                                            div { class: "text-center p-3 rounded-lg border border-gray-200 bg-white",
                                                p { class: "font-semibold text-sm text-gray-800", "{day_name}" }
                                                if h.is_closed {
                                                    p { class: "text-red-500 text-xs mt-1",
                                                        if loc == "zh" { "\u{4f11}\u{606f}" } else { "Closed" }
                                                    }
                                                } else {
                                                    p { class: "text-gray-500 text-xs mt-1", "{open} - {close}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Some(Err(_)) | None => rsx! {
                                p { class: "text-gray-400 text-center py-4",
                                    if loc == "zh" { "\u{52a0}\u{8f7d}\u{4e2d}..." } else { "Loading hours..." }
                                }
                            },
                        }
                    }
                }

                // Quick links
                section { class: "mb-8",
                    div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                        Link {
                            to: crate::Route::Menu { locale: locale.clone() },
                            class: "flex flex-col items-center p-6 bg-white rounded-xl shadow hover:shadow-md transition-shadow no-underline text-gray-800",
                            span { class: "text-3xl mb-2", "\u{2615}" }
                            h3 { class: "font-semibold", "{menu_link_text}" }
                        }
                        Link {
                            to: crate::Route::Training { locale: locale.clone() },
                            class: "flex flex-col items-center p-6 bg-white rounded-xl shadow hover:shadow-md transition-shadow no-underline text-gray-800",
                            span { class: "text-3xl mb-2", "\u{1f4da}" }
                            h3 { class: "font-semibold", "{training_link_text}" }
                        }
                        Link {
                            to: crate::Route::Orders { locale: locale.clone() },
                            class: "flex flex-col items-center p-6 bg-white rounded-xl shadow hover:shadow-md transition-shadow no-underline text-gray-800",
                            span { class: "text-3xl mb-2", "\u{1f4cb}" }
                            h3 { class: "font-semibold", "{t.t(&loc, \"nav.orders\")}" }
                        }
                    }
                }
            }

            Footer {}
        }
    }
}
