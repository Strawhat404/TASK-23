use dioxus::prelude::*;

#[component]
pub fn HoldTimer(expires_at: String, locale: String) -> Element {
    let t = shared::i18n::init_translations();
    let loc = locale.as_str();

    let mut remaining_secs = use_signal(|| compute_remaining(&expires_at));
    let expires_clone = expires_at.clone();

    use_future(move || {
        let expires = expires_clone.clone();
        async move {
            loop {
                gloo_timers::future::TimeoutFuture::new(1_000).await;
                let secs = compute_remaining(&expires);
                remaining_secs.set(secs);
                if secs <= 0 {
                    break;
                }
            }
        }
    });

    let secs = remaining_secs();

    if secs <= 0 {
        let msg = t.t(loc, "msg.item_released");
        rsx! {
            div { class: "text-center p-4 rounded-xl bg-red-50 border border-red-300",
                span { class: "text-red-600 font-medium", "{msg}" }
            }
        }
    } else {
        let minutes = secs / 60;
        let seconds = secs % 60;
        let display = format!("{:02}:{:02}", minutes, seconds);
        let label = t.t(loc, "label.hold_timer");
        let wrapper_class = if secs < 60 {
            "text-center p-4 rounded-xl bg-red-50 border border-red-300"
        } else {
            "text-center p-4 rounded-xl bg-amber-50 border border-amber-300"
        };
        let timer_class = if secs < 60 {
            "text-3xl font-bold tabular-nums text-red-600"
        } else {
            "text-3xl font-bold tabular-nums text-amber-700"
        };

        rsx! {
            div { class: "{wrapper_class}",
                span { class: "text-sm text-gray-600", "{label}: " }
                span { class: "{timer_class}", "{display}" }
            }
        }
    }
}

pub(crate) fn compute_remaining(expires_at: &str) -> i64 {
    let Ok(expiry) = chrono::NaiveDateTime::parse_from_str(expires_at, "%Y-%m-%dT%H:%M:%S") else {
        let Ok(expiry) = chrono::NaiveDateTime::parse_from_str(expires_at, "%Y-%m-%dT%H:%M:%S%.f") else {
            return 0;
        };
        let now = chrono::Utc::now().naive_utc();
        return (expiry - now).num_seconds();
    };
    let now = chrono::Utc::now().naive_utc();
    (expiry - now).num_seconds()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn future_time_returns_positive() {
        let future = (chrono::Utc::now().naive_utc() + chrono::Duration::seconds(120))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        let remaining = compute_remaining(&future);
        assert!(remaining > 0, "Future time should return positive seconds, got {}", remaining);
    }

    #[test]
    fn past_time_returns_negative() {
        let past = (chrono::Utc::now().naive_utc() - chrono::Duration::seconds(60))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        let remaining = compute_remaining(&past);
        assert!(remaining < 0, "Past time should return negative seconds, got {}", remaining);
    }

    #[test]
    fn unparseable_returns_zero() {
        assert_eq!(compute_remaining("not-a-date"), 0);
        assert_eq!(compute_remaining(""), 0);
    }

    #[test]
    fn fractional_seconds_parsed() {
        let future = (chrono::Utc::now().naive_utc() + chrono::Duration::seconds(300))
            .format("%Y-%m-%dT%H:%M:%S%.3f")
            .to_string();
        let remaining = compute_remaining(&future);
        assert!(remaining > 0, "Fractional-second timestamps should parse correctly, got {}", remaining);
    }
}
