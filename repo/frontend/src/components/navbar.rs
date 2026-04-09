use dioxus::prelude::*;
use crate::state::AppState;
use super::locale_switcher::LocaleSwitcher;

const NAV_LINK: &str = "text-white/85 no-underline px-3 py-2 rounded-lg text-sm transition-colors hover:bg-white/15 hover:text-white";

#[component]
pub fn Navbar(locale: String) -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let t = shared::i18n::init_translations();
    let loc = locale.as_str();

    let display_name = state()
        .auth
        .user
        .as_ref()
        .map(|u| u.display_name.clone().unwrap_or(u.username.clone()))
        .unwrap_or_default();

    let is_authenticated = state().auth.is_authenticated;
    let cart_count = state().cart_count;
    let is_staff = state().is_staff();
    let is_teacher = state().is_teacher();
    let is_admin = state().is_admin();

    let nav_menu = t.t(loc, "nav.menu");
    let nav_cart = t.t(loc, "nav.cart");
    let nav_orders = t.t(loc, "nav.orders");
    let nav_staff = t.t(loc, "nav.staff");
    let nav_training = t.t(loc, "nav.training");
    let nav_admin = t.t(loc, "nav.admin");

    let locale_login = locale.clone();

    rsx! {
        nav { class: "bg-primary text-white px-6 flex items-center justify-between h-[60px] shadow-md sticky top-0 z-50",
            // Brand
            div {
                Link { to: crate::Route::Home { locale: locale.clone() },
                    class: "text-white no-underline",
                    h1 { class: "text-2xl font-bold tracking-tight", "BrewFlow" }
                }
            }
            // Nav links — only shown when authenticated
            if is_authenticated {
                div { class: "hidden md:flex gap-1",
                    Link { to: crate::Route::Menu { locale: locale.clone() }, class: NAV_LINK, "{nav_menu}" }
                    Link { to: crate::Route::Cart { locale: locale.clone() }, class: NAV_LINK,
                        "{nav_cart}"
                        if cart_count > 0 {
                            span { class: "bg-red-500 text-white rounded-full px-2 py-0.5 text-[0.7rem] font-semibold ml-1", "{cart_count}" }
                        }
                    }
                    Link { to: crate::Route::Orders { locale: locale.clone() }, class: NAV_LINK, "{nav_orders}" }
                    if is_staff {
                        Link { to: crate::Route::StaffDashboard { locale: locale.clone() }, class: NAV_LINK, "{nav_staff}" }
                    }
                    if is_teacher {
                        Link { to: crate::Route::Training { locale: locale.clone() }, class: NAV_LINK, "{nav_training}" }
                    }
                    if is_admin {
                        Link { to: crate::Route::Admin { locale: locale.clone() }, class: NAV_LINK, "{nav_admin}" }
                    }
                }
            }
            // Right side
            div { class: "flex items-center gap-3",
                LocaleSwitcher { current_locale: locale.clone() }
                if is_authenticated {
                    span { class: "text-sm opacity-90", "{display_name}" }
                    button {
                        class: "px-3 py-1.5 text-xs rounded-lg bg-white/15 text-white hover:bg-white/25 transition-all cursor-pointer",
                        onclick: move |_| { state.write().logout(); },
                        "Logout"
                    }
                } else {
                    Link { to: crate::Route::Login { locale: locale_login.clone() },
                        class: "inline-flex items-center justify-center px-3 py-1.5 text-xs rounded-lg font-medium bg-white text-primary hover:bg-gray-100 transition-all no-underline",
                        "Login"
                    }
                }
            }
        }
    }
}
