use dioxus::prelude::*;
use crate::components::navbar::Navbar;
use crate::components::footer::Footer;
use crate::state::AppState;
use shared::dto::{ApiResponse, LoginRequest, LoginResponse};

const INPUT: &str = "w-full px-3 py-2.5 border border-gray-300 rounded-lg text-sm transition-colors focus:outline-none focus:border-primary focus:ring-2 focus:ring-primary/15 bg-white";
const BTN_SUBMIT: &str = "w-full inline-flex items-center justify-center px-5 py-2.5 rounded-lg text-sm font-medium bg-primary text-white hover:bg-primary-dark transition-all disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer";
const SELECT: &str = "w-full px-3 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:border-primary bg-white";

// ── Login ────────────────────────────────────────────────────────────────────

#[component]
pub fn LoginPage(locale: String) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.clone();
    let nav = use_navigator();
    let mut app_state = use_context::<Signal<AppState>>();

    let mut username = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| false);

    let title = if loc == "zh" { "\u{767b}\u{5f55}" } else { "Login" };
    let username_label = if loc == "zh" { "\u{7528}\u{6237}\u{540d}" } else { "Username" };
    let password_label = if loc == "zh" { "\u{5bc6}\u{7801}" } else { "Password" };
    let submit_text = t.t(&loc, "btn.submit");
    let register_text = if loc == "zh" { "\u{6ca1}\u{6709}\u{8d26}\u{53f7}\u{ff1f}\u{6ce8}\u{518c}" } else { "No account? Register" };

    let locale_submit = locale.clone();
    let locale_nav = locale.clone();

    rsx! {
        div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
            Navbar { locale: locale.clone() }

            main { class: "flex-1 flex items-center justify-center px-4 py-12",
                div { class: "w-full max-w-md bg-white rounded-2xl shadow-lg p-8",
                    h2 { class: "text-2xl font-bold text-center text-gray-800 mb-6", "{title}" }

                    if let Some(err) = error_msg() {
                        div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm mb-4",
                            p { class: "font-medium mb-1", "Login Failed" }
                            p { "{err}" }
                        }
                    }

                    // No <form> — use div + button onclick to avoid WASM page reload
                    div {
                        div { class: "mb-4",
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "{username_label}" }
                            input {
                                r#type: "text",
                                class: INPUT,
                                placeholder: "{username_label}",
                                value: "{username}",
                                oninput: move |evt| { username.set(evt.value()); error_msg.set(None); },
                            }
                        }
                        div { class: "mb-6",
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "{password_label}" }
                            input {
                                r#type: "password",
                                class: INPUT,
                                placeholder: "{password_label}",
                                value: "{password}",
                                oninput: move |evt| { password.set(evt.value()); error_msg.set(None); },
                            }
                        }
                        button {
                            class: BTN_SUBMIT,
                            disabled: loading() || username().is_empty() || password().is_empty(),
                            onclick: move |_| {
                                let user = username().clone();
                                let pass = password().clone();
                                let locale_inner = locale_submit.clone();
                                spawn(async move {
                                    loading.set(true);
                                    error_msg.set(None);
                                    let body = LoginRequest { username: user, password: pass };
                                    let result = reqwest::Client::new()
                                        .post(&format!("{}/auth/login", &crate::api_base()))
                                        .json(&body)
                                        .send()
                                        .await;
                                    match result {
                                        Ok(resp) => {
                                            let status = resp.status();
                                            let body_text = resp.text().await.unwrap_or_default();
                                            if status.is_success() {
                                                match serde_json::from_str::<ApiResponse<LoginResponse>>(&body_text) {
                                                    Ok(api_resp) => {
                                                        if let Some(data) = api_resp.data {
                                                            let user_info = crate::state::UserInfo {
                                                                id: data.user.id,
                                                                username: data.user.username,
                                                                display_name: data.user.display_name,
                                                                roles: data.user.roles,
                                                                preferred_locale: data.user.preferred_locale,
                                                            };
                                                            app_state.write().set_auth(data.session_cookie, user_info);
                                                            nav.push(crate::Route::Home { locale: locale_inner });
                                                        } else {
                                                            error_msg.set(Some(api_resp.error.unwrap_or_else(|| "Login failed".to_string())));
                                                        }
                                                    }
                                                    Err(e) => error_msg.set(Some(format!("Parse error: {}", e))),
                                                }
                                            } else {
                                                let msg = serde_json::from_str::<serde_json::Value>(&body_text)
                                                    .ok()
                                                    .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(|s| s.to_string()))
                                                    .unwrap_or(format!("HTTP {}: {}", status, body_text));
                                                error_msg.set(Some(msg));
                                            }
                                        }
                                        Err(e) => error_msg.set(Some(format!("Could not reach server: {}", e))),
                                    }
                                    loading.set(false);
                                });
                            },
                            if loading() { "Logging in..." } else { "{submit_text}" }
                        }
                    }

                    div { class: "text-center mt-6",
                        Link { to: crate::Route::Register { locale: locale_nav.clone() }, class: "text-sm text-primary hover:text-primary-dark underline", "{register_text}" }
                    }
                }
            }

            Footer {}
        }
    }
}

// ── Register ─────────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct RegisterRequest {
    username: String,
    password: String,
    display_name: Option<String>,
    email: Option<String>,
    role: Option<String>,
}

#[component]
pub fn RegisterPage(locale: String) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.clone();

    let mut username = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut display_name = use_signal(|| String::new());
    let mut email = use_signal(|| String::new());
    let mut role = use_signal(|| "Customer".to_string());
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut success_msg = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| false);

    let title = if loc == "zh" { "\u{6ce8}\u{518c}" } else { "Register" };
    let username_label = if loc == "zh" { "\u{7528}\u{6237}\u{540d}" } else { "Username" };
    let password_label = if loc == "zh" { "\u{5bc6}\u{7801}" } else { "Password" };
    let display_name_label = if loc == "zh" { "\u{663e}\u{793a}\u{540d}\u{79f0}" } else { "Display Name" };
    let email_label = if loc == "zh" { "\u{7535}\u{5b50}\u{90ae}\u{4ef6}" } else { "Email" };
    let role_label = if loc == "zh" { "\u{89d2}\u{8272}" } else { "Role" };
    let submit_text = t.t(&loc, "btn.submit");
    let login_text = if loc == "zh" { "\u{5df2}\u{6709}\u{8d26}\u{53f7}\u{ff1f}\u{767b}\u{5f55}" } else { "Already have an account? Login" };
    let password_hint = if loc == "zh" {
        "\u{81f3}\u{5c11}12\u{4e2a}\u{5b57}\u{7b26}\u{ff0c}\u{5305}\u{542b}\u{5927}\u{5c0f}\u{5199}\u{5b57}\u{6bcd}\u{3001}\u{6570}\u{5b57}\u{548c}\u{7279}\u{6b8a}\u{5b57}\u{7b26}"
    } else {
        "Min 12 chars, with uppercase, lowercase, digit, and special character"
    };

    let locale_nav = locale.clone();

    rsx! {
        div { class: "min-h-screen flex flex-col bg-[#fefcf9]",
            Navbar { locale: locale.clone() }

            main { class: "flex-1 flex items-center justify-center px-4 py-12",
                div { class: "w-full max-w-md bg-white rounded-2xl shadow-lg p-8",
                    h2 { class: "text-2xl font-bold text-center text-gray-800 mb-6", "{title}" }

                    if let Some(err) = error_msg() {
                        div { class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm mb-4",
                            p { class: "font-medium mb-1", "Registration Failed" }
                            p { "{err}" }
                        }
                    }
                    if let Some(msg) = success_msg() {
                        div { class: "bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg text-sm mb-4",
                            p { class: "font-medium mb-1", "Success!" }
                            p { "{msg}" }
                            Link {
                                to: crate::Route::Login { locale: locale_nav.clone() },
                                class: "inline-block mt-2 px-4 py-2 rounded-lg text-sm font-medium bg-green-600 text-white hover:bg-green-700 transition-all no-underline",
                                if loc == "zh" { "\u{53bb}\u{767b}\u{5f55}" } else { "Go to Login" }
                            }
                        }
                    }

                    // No <form> — use div + button onclick to avoid WASM page reload
                    div {
                        div { class: "mb-4",
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "{username_label} *" }
                            input {
                                r#type: "text",
                                class: INPUT,
                                placeholder: "{username_label}",
                                value: "{username}",
                                oninput: move |evt| { username.set(evt.value()); error_msg.set(None); },
                            }
                        }
                        div { class: "mb-4",
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "{password_label} *" }
                            input {
                                r#type: "password",
                                class: INPUT,
                                placeholder: "{password_label}",
                                value: "{password}",
                                oninput: move |evt| { password.set(evt.value()); error_msg.set(None); },
                            }
                            p { class: "text-xs text-gray-400 mt-1", "{password_hint}" }
                        }
                        div { class: "mb-4",
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "{role_label}" }
                            select {
                                class: SELECT,
                                value: "{role}",
                                onchange: move |evt| role.set(evt.value()),
                                option { value: "Customer", if loc == "zh" { "\u{987e}\u{5ba2}" } else { "Customer" } }
                                option { value: "Staff", if loc == "zh" { "\u{5458}\u{5de5}" } else { "Staff" } }
                                option { value: "Teacher", if loc == "zh" { "\u{6559}\u{5e08}" } else { "Teacher" } }
                                option { value: "Admin", if loc == "zh" { "\u{7ba1}\u{7406}\u{5458}" } else { "Admin" } }
                            }
                        }
                        div { class: "mb-4",
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "{display_name_label}" }
                            input {
                                r#type: "text",
                                class: INPUT,
                                placeholder: "{display_name_label}",
                                value: "{display_name}",
                                oninput: move |evt| display_name.set(evt.value()),
                            }
                        }
                        div { class: "mb-6",
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "{email_label}" }
                            input {
                                r#type: "email",
                                class: INPUT,
                                placeholder: "{email_label}",
                                value: "{email}",
                                oninput: move |evt| email.set(evt.value()),
                            }
                        }
                        button {
                            class: BTN_SUBMIT,
                            disabled: loading() || success_msg().is_some() || username().is_empty() || password().is_empty(),
                            onclick: move |_| {
                                let user = username().clone();
                                let pass = password().clone();
                                let dname = display_name().clone();
                                let em = email().clone();
                                let r = role().clone();
                                spawn(async move {
                                    loading.set(true);
                                    error_msg.set(None);
                                    success_msg.set(None);
                                    let body = RegisterRequest {
                                        username: user,
                                        password: pass,
                                        display_name: if dname.is_empty() { None } else { Some(dname) },
                                        email: if em.is_empty() { None } else { Some(em) },
                                        role: Some(r),
                                    };
                                    let result = reqwest::Client::new()
                                        .post(&format!("{}/auth/register", &crate::api_base()))
                                        .json(&body)
                                        .send()
                                        .await;
                                    match result {
                                        Ok(resp) => {
                                            let status = resp.status();
                                            let body_text = resp.text().await.unwrap_or_default();
                                            if status.is_success() {
                                                success_msg.set(Some(
                                                    "Account created successfully! You can now log in with your credentials.".to_string()
                                                ));
                                            } else {
                                                let msg = serde_json::from_str::<serde_json::Value>(&body_text)
                                                    .ok()
                                                    .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(|s| s.to_string()))
                                                    .unwrap_or(format!("HTTP {}: {}", status, body_text));
                                                error_msg.set(Some(msg));
                                            }
                                        }
                                        Err(e) => error_msg.set(Some(format!("Could not reach server: {}", e))),
                                    }
                                    loading.set(false);
                                });
                            },
                            if loading() { "Registering..." } else { "{submit_text}" }
                        }
                    }

                    div { class: "text-center mt-6",
                        Link { to: crate::Route::Login { locale: locale_nav.clone() }, class: "text-sm text-primary hover:text-primary-dark underline", "{login_text}" }
                    }
                }
            }

            Footer {}
        }
    }
}
