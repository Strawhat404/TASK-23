#[cfg(target_arch = "wasm32")]
mod components;
mod logic;
#[cfg(target_arch = "wasm32")]
mod pages;
mod state;

/// Kept as a constant fallback for non-WASM contexts (tests, etc.)
pub const API_BASE: &str = "http://localhost:8080/api";

// ---------------------------------------------------------------------------
// Everything below requires the wasm32 target (Dioxus, web-sys, gloo, etc.)
// On native targets, only the `logic`, `state`, and `API_BASE` above compile.
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod app {
    use dioxus::prelude::*;
    use crate::state;

    /// Returns the API base URL using the browser's current origin.
    pub fn api_base() -> String {
        let origin = web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_else(|| "http://localhost:8080".to_string());
        format!("{}/api", origin)
    }

    #[derive(Debug, Clone, Routable, PartialEq)]
    #[rustfmt::skip]
    pub(crate) enum Route {
        #[redirect("/", || Route::Login { locale: "en".to_string() })]

        #[route("/:locale")]
        Home { locale: String },
        #[route("/:locale/menu")]
        Menu { locale: String },
        #[route("/:locale/menu/:id")]
        ProductDetail { locale: String, id: i64 },
        #[route("/:locale/cart")]
        Cart { locale: String },
        #[route("/:locale/checkout")]
        Checkout { locale: String },
        #[route("/:locale/orders")]
        Orders { locale: String },
        #[route("/:locale/orders/:id")]
        OrderDetail { locale: String, id: i64 },

        #[route("/:locale/login")]
        Login { locale: String },
        #[route("/:locale/register")]
        Register { locale: String },

        #[route("/:locale/staff")]
        StaffDashboard { locale: String },
        #[route("/:locale/staff/orders/:id")]
        StaffOrderDetail { locale: String, id: i64 },
        #[route("/:locale/staff/scan")]
        StaffScan { locale: String },

        #[route("/:locale/training")]
        Training { locale: String },
        #[route("/:locale/training/exams")]
        MockExams { locale: String },
        #[route("/:locale/training/exams/:id")]
        TakeExam { locale: String, id: i64 },
        #[route("/:locale/training/analytics")]
        Analytics { locale: String },
        #[route("/:locale/training/favorites")]
        Favorites { locale: String },
        #[route("/:locale/training/wrong-notebook")]
        WrongNotebook { locale: String },
        #[route("/:locale/training/review")]
        ReviewSession { locale: String },

        #[route("/:locale/admin")]
        Admin { locale: String },
        #[route("/:locale/admin/questions")]
        QuestionBank { locale: String },
        #[route("/:locale/admin/import")]
        ImportQuestions { locale: String },
        #[route("/:locale/admin/generate-exam")]
        GenerateExam { locale: String },
    }

    pub fn launch_app() {
        dioxus::launch(App);
    }

    #[component]
    fn App() -> Element {
        use_context_provider(|| Signal::new(state::AppState::default()));

        rsx! {
            Router::<Route> {}
        }
    }

    fn require_auth(locale: &str) -> Option<Element> {
        let state = use_context::<Signal<state::AppState>>();
        let nav = use_navigator();
        if !state().auth.is_authenticated {
            nav.replace(Route::Login { locale: locale.to_string() });
            return Some(rsx! { div { class: "text-center py-12 text-gray-400", "Redirecting to login..." } });
        }
        None
    }

    #[component]
    fn Home(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::home::HomePage { locale: locale } }
    }

    #[component]
    fn Menu(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::menu::MenuPage { locale: locale } }
    }

    #[component]
    fn ProductDetail(locale: String, id: i64) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::product::ProductDetailPage { locale: locale, id: id } }
    }

    #[component]
    fn Cart(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::cart::CartPage { locale: locale } }
    }

    #[component]
    fn Checkout(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::checkout::CheckoutPage { locale: locale } }
    }

    #[component]
    fn Orders(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::orders::OrdersPage { locale: locale } }
    }

    #[component]
    fn OrderDetail(locale: String, id: i64) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::orders::OrderDetailPage { locale: locale, id: id } }
    }

    #[component]
    fn Login(locale: String) -> Element {
        rsx! { crate::pages::auth::LoginPage { locale: locale } }
    }

    #[component]
    fn Register(locale: String) -> Element {
        rsx! { crate::pages::auth::RegisterPage { locale: locale } }
    }

    #[component]
    fn StaffDashboard(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::staff::StaffDashboardPage { locale: locale } }
    }

    #[component]
    fn StaffOrderDetail(locale: String, id: i64) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::staff::StaffOrderDetailPage { locale: locale, id: id } }
    }

    #[component]
    fn StaffScan(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::staff::StaffScanPage { locale: locale } }
    }

    #[component]
    fn Training(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::training::TrainingPage { locale: locale } }
    }

    #[component]
    fn MockExams(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::training::MockExamsPage { locale: locale } }
    }

    #[component]
    fn TakeExam(locale: String, id: i64) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::training::TakeExamPage { locale: locale, id: id } }
    }

    #[component]
    fn Analytics(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::training::AnalyticsPage { locale: locale } }
    }

    #[component]
    fn Favorites(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::training::FavoritesPage { locale: locale } }
    }

    #[component]
    fn WrongNotebook(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::training::WrongNotebookPage { locale: locale } }
    }

    #[component]
    fn ReviewSession(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::training::ReviewSessionPage { locale: locale } }
    }

    #[component]
    fn Admin(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::admin::AdminPage { locale: locale } }
    }

    #[component]
    fn QuestionBank(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::admin::QuestionBankPage { locale: locale } }
    }

    #[component]
    fn ImportQuestions(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::admin::ImportQuestionsPage { locale: locale } }
    }

    #[component]
    fn GenerateExam(locale: String) -> Element {
        if let Some(el) = require_auth(&locale) { return el; }
        rsx! { crate::pages::admin::GenerateExamPage { locale: locale } }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    app::launch_app();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Non-wasm stub — the binary target is only meaningful on wasm32.
    // Tests use the lib target (lib.rs) instead.
    eprintln!("BrewFlow frontend is a WASM application. Build with --target wasm32-unknown-unknown.");
}
