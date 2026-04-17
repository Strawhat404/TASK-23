use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub preferred_locale: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spu {
    pub id: i64,
    pub name_en: String,
    pub name_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub category: Option<String>,
    pub image_url: Option<String>,
    pub base_price: f64,
    pub prep_time_minutes: i32,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionGroup {
    pub id: i64,
    pub spu_id: i64,
    pub name_en: String,
    pub name_zh: String,
    pub is_required: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionValue {
    pub id: i64,
    pub group_id: i64,
    pub label_en: String,
    pub label_zh: String,
    pub price_delta: f64,
    pub is_default: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sku {
    pub id: i64,
    pub spu_id: i64,
    pub sku_code: String,
    pub price: f64,
    pub stock_quantity: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreHours {
    pub id: i64,
    pub day_of_week: u8,
    pub open_time: String,
    pub close_time: String,
    pub is_closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reservation {
    pub id: i64,
    pub user_id: i64,
    pub pickup_slot_start: NaiveDateTime,
    pub pickup_slot_end: NaiveDateTime,
    pub voucher_code: String,
    pub hold_expires_at: NaiveDateTime,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cart {
    pub id: i64,
    pub user_id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItem {
    pub id: i64,
    pub cart_id: i64,
    pub sku_id: i64,
    pub quantity: i32,
    pub unit_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: i64,
    pub user_id: i64,
    pub reservation_id: Option<i64>,
    pub order_number: String,
    pub subtotal: f64,
    pub tax_amount: f64,
    pub total: f64,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub id: i64,
    pub order_id: i64,
    pub sku_id: i64,
    pub quantity: i32,
    pub unit_price: f64,
    pub item_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulfillmentEvent {
    pub id: i64,
    pub order_id: i64,
    pub from_status: Option<String>,
    pub to_status: String,
    pub changed_by_user_id: i64,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voucher {
    pub id: i64,
    pub reservation_id: i64,
    pub order_id: Option<i64>,
    pub code: String,
    pub scanned_at: Option<NaiveDateTime>,
    pub scanned_by_user_id: Option<i64>,
    pub mismatch_flag: bool,
    pub mismatch_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    pub id: i64,
    pub name_en: String,
    pub name_zh: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: i64,
    pub subject_id: i64,
    pub name_en: String,
    pub name_zh: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: i64,
    pub subject_id: i64,
    pub chapter_id: Option<i64>,
    pub difficulty: String,
    pub question_text_en: String,
    pub question_text_zh: Option<String>,
    pub explanation_en: Option<String>,
    pub explanation_zh: Option<String>,
    pub question_type: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    pub id: i64,
    pub question_id: i64,
    pub label: String,
    pub content_en: String,
    pub content_zh: Option<String>,
    pub is_correct: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamVersion {
    pub id: i64,
    pub title_en: String,
    pub title_zh: Option<String>,
    pub subject_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub difficulty: String,
    pub question_count: i32,
    pub time_limit_minutes: i32,
    pub created_by: Option<i64>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamAttempt {
    pub id: i64,
    pub user_id: i64,
    pub exam_version_id: i64,
    pub started_at: NaiveDateTime,
    pub finished_at: Option<NaiveDateTime>,
    pub score: Option<f64>,
    pub total_questions: i32,
    pub correct_count: i32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptAnswer {
    pub id: i64,
    pub attempt_id: i64,
    pub question_id: i64,
    pub selected_option_ids: Option<String>,
    pub is_correct: Option<bool>,
    pub answered_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favorite {
    pub id: i64,
    pub user_id: i64,
    pub question_id: i64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrongAnswerEntry {
    pub id: i64,
    pub user_id: i64,
    pub question_id: i64,
    pub wrong_count: i32,
    pub last_wrong_at: Option<NaiveDateTime>,
    pub next_review_at: Option<NaiveDateTime>,
    pub review_interval_days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSnapshot {
    pub id: i64,
    pub user_id: Option<i64>,
    pub snapshot_type: String,
    pub snapshot_data: String,
    pub snapshot_date: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesTaxConfig {
    pub id: i64,
    pub tax_name: String,
    pub rate: f64,
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn sample_dt() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 4, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    #[test]
    fn user_serialises_password_hash_field() {
        let u = User {
            id: 1,
            username: "alice".into(),
            password_hash: "$2b$12$stub".into(),
            display_name: Some("Alice A.".into()),
            email: Some("a@example.com".into()),
            preferred_locale: "en".into(),
            created_at: sample_dt(),
            updated_at: None,
        };
        let v = serde_json::to_value(&u).unwrap();
        // password_hash is still present in the raw model; masking is handled
        // at the HTTP layer, not the model layer.
        assert_eq!(v["username"], "alice");
        assert_eq!(v["password_hash"], "$2b$12$stub");
        assert!(v["updated_at"].is_null());
    }

    #[test]
    fn spu_model_round_trips() {
        let s = Spu {
            id: 42,
            name_en: "Latte".into(),
            name_zh: "\u{62ff}\u{94c1}".into(),
            description_en: None,
            description_zh: None,
            category: Some("coffee".into()),
            image_url: None,
            base_price: 4.50,
            prep_time_minutes: 5,
            is_active: true,
            created_at: sample_dt(),
            updated_at: None,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Spu = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 42);
        assert_eq!(back.name_zh, "\u{62ff}\u{94c1}");
        assert_eq!(back.prep_time_minutes, 5);
        assert!(back.is_active);
    }

    #[test]
    fn reservation_model_round_trips() {
        let r = Reservation {
            id: 9,
            user_id: 2,
            pickup_slot_start: sample_dt(),
            pickup_slot_end: sample_dt(),
            voucher_code: "BF-ABC123".into(),
            hold_expires_at: sample_dt(),
            status: "Held".into(),
            created_at: sample_dt(),
            updated_at: None,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: Reservation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, r.id);
        assert_eq!(back.voucher_code, "BF-ABC123");
    }

    #[test]
    fn order_model_total_matches_subtotal_plus_tax_contract() {
        let o = Order {
            id: 1,
            user_id: 2,
            reservation_id: Some(5),
            order_number: "ORD-1".into(),
            subtotal: 10.0,
            tax_amount: 0.85,
            total: 10.85,
            status: "Pending".into(),
            created_at: sample_dt(),
            updated_at: None,
        };
        assert!((o.total - (o.subtotal + o.tax_amount)).abs() < 1e-6);
    }

    #[test]
    fn voucher_mismatch_flag_is_serialised() {
        let v = Voucher {
            id: 1,
            reservation_id: 1,
            order_id: Some(2),
            code: "hashed".into(),
            scanned_at: None,
            scanned_by_user_id: None,
            mismatch_flag: true,
            mismatch_reason: Some("different order".into()),
        };
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["mismatch_flag"], true);
        assert_eq!(json["mismatch_reason"], "different order");
    }

    #[test]
    fn question_option_is_correct_is_serialised() {
        let opt = QuestionOption {
            id: 1,
            question_id: 10,
            label: "A".into(),
            content_en: "42".into(),
            content_zh: Some("\u{56db}\u{5341}\u{4e8c}".into()),
            is_correct: true,
            sort_order: 0,
        };
        let json = serde_json::to_value(&opt).unwrap();
        assert_eq!(json["is_correct"], true);
        assert_eq!(json["label"], "A");
    }

    #[test]
    fn option_value_with_negative_price_delta_round_trips() {
        let v = OptionValue {
            id: 1,
            group_id: 1,
            label_en: "No sweetener".into(),
            label_zh: "\u{65e0}\u{7cd6}".into(),
            price_delta: -0.25,
            is_default: false,
            sort_order: 0,
        };
        let json = serde_json::to_string(&v).unwrap();
        let back: OptionValue = serde_json::from_str(&json).unwrap();
        assert!((back.price_delta - -0.25).abs() < 1e-9);
    }

    #[test]
    fn store_hours_closed_day_is_preserved() {
        let h = StoreHours {
            id: 1,
            day_of_week: 0,
            open_time: "00:00".into(),
            close_time: "00:00".into(),
            is_closed: true,
        };
        let json = serde_json::to_value(&h).unwrap();
        assert_eq!(json["is_closed"], true);
        assert_eq!(json["day_of_week"], 0);
    }

    #[test]
    fn wrong_answer_entry_review_interval_defaults_serialised() {
        let w = WrongAnswerEntry {
            id: 1,
            user_id: 1,
            question_id: 10,
            wrong_count: 3,
            last_wrong_at: None,
            next_review_at: None,
            review_interval_days: 1,
        };
        let json = serde_json::to_value(&w).unwrap();
        assert_eq!(json["wrong_count"], 3);
        assert_eq!(json["review_interval_days"], 1);
    }

    #[test]
    fn analytics_snapshot_stores_json_string() {
        let a = AnalyticsSnapshot {
            id: 1,
            user_id: Some(5),
            snapshot_type: "user_score".into(),
            snapshot_data: r#"{"avg_score":82.5}"#.into(),
            snapshot_date: "2026-04-15".into(),
            created_at: sample_dt(),
        };
        let back: AnalyticsSnapshot =
            serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        assert!(back.snapshot_data.contains("82.5"));
    }
}
