use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    /// The signed `brewflow_session` cookie value.  The frontend stores this
    /// and passes it as `Cookie: brewflow_session=<value>` on every request.
    pub session_cookie: String,
    pub user: UserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub roles: Vec<String>,
    pub preferred_locale: String,
}

// ---------------------------------------------------------------------------
// Products / Menu
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductListItem {
    pub spu_id: i64,
    pub name_en: String,
    pub name_zh: String,
    pub description_en: Option<String>,
    pub description_zh: Option<String>,
    pub category: Option<String>,
    pub image_url: Option<String>,
    pub base_price: f64,
    pub prep_time_minutes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductDetail {
    pub spu: ProductListItem,
    pub option_groups: Vec<OptionGroupDetail>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionGroupDetail {
    pub id: i64,
    pub name_en: String,
    pub name_zh: String,
    pub is_required: bool,
    pub options: Vec<OptionValueDetail>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionValueDetail {
    pub id: i64,
    pub label_en: String,
    pub label_zh: String,
    pub price_delta: f64,
    pub is_default: bool,
}

// ---------------------------------------------------------------------------
// Cart
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddToCartRequest {
    pub sku_id: Option<i64>,
    pub spu_id: i64,
    pub selected_options: Vec<i64>,
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartResponse {
    pub items: Vec<CartItemDetail>,
    pub subtotal: f64,
    pub tax_rate: f64,
    pub tax_amount: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItemDetail {
    pub id: i64,
    pub spu_name_en: String,
    pub spu_name_zh: String,
    pub sku_code: Option<String>,
    pub options: Vec<String>,
    pub quantity: i32,
    pub unit_price: f64,
    pub line_total: f64,
}

// ---------------------------------------------------------------------------
// Checkout / Reservation
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PickupSlot {
    pub start: String,
    pub end: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutRequest {
    pub pickup_slot_start: String,
    pub pickup_slot_end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutResponse {
    pub order_id: i64,
    pub order_number: String,
    pub voucher_code: String,
    pub hold_expires_at: String,
    pub pickup_slot: String,
    pub total: f64,
}

// ---------------------------------------------------------------------------
// Orders
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSummary {
    pub id: i64,
    pub order_number: String,
    pub status: String,
    pub total: f64,
    pub voucher_code: Option<String>,
    pub created_at: String,
    pub pickup_slot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDetail {
    pub order: OrderSummary,
    pub items: Vec<OrderItemDetail>,
    pub fulfillment_history: Vec<FulfillmentEventDetail>,
    pub reservation: Option<ReservationDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemDetail {
    pub sku_code: String,
    pub spu_name: String,
    pub options: Vec<String>,
    pub quantity: i32,
    pub unit_price: f64,
    pub item_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulfillmentEventDetail {
    pub from_status: Option<String>,
    pub to_status: String,
    pub changed_by: String,
    pub notes: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationDetail {
    pub voucher_code: String,
    pub pickup_slot_start: String,
    pub pickup_slot_end: String,
    pub hold_expires_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOrderStatusRequest {
    pub new_status: String,
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Voucher scanning
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanVoucherRequest {
    pub voucher_code: String,
    #[serde(default)]
    pub order_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanVoucherResponse {
    pub valid: bool,
    pub order: Option<OrderSummary>,
    pub mismatch: bool,
    pub mismatch_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Question bank (admin listing with pagination)
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionListItem {
    pub id: i64,
    pub question_text_en: String,
    pub question_text_zh: Option<String>,
    pub question_type: String,
    pub difficulty: String,
    pub subject_name: Option<String>,
    pub chapter_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Question bank import
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportQuestionsRequest {
    pub subject_id: i64,
    pub chapter_id: Option<i64>,
    pub csv_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportQuestionsResponse {
    pub imported_count: i32,
    pub skipped_count: i32,
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Exams
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateExamRequest {
    pub title_en: String,
    pub title_zh: Option<String>,
    pub subject_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub difficulty: Option<String>,
    pub question_count: i32,
    pub time_limit_minutes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamVersionResponse {
    pub id: i64,
    pub title_en: String,
    pub title_zh: Option<String>,
    pub subject_name: Option<String>,
    pub difficulty: String,
    pub question_count: i32,
    pub time_limit_minutes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartExamResponse {
    pub attempt_id: i64,
    pub questions: Vec<ExamQuestionDetail>,
    pub time_limit_minutes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamQuestionDetail {
    pub question_id: i64,
    pub question_text_en: String,
    pub question_text_zh: Option<String>,
    pub question_type: String,
    pub options: Vec<ExamOptionDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamOptionDetail {
    pub id: i64,
    pub label: String,
    pub content_en: String,
    pub content_zh: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitAnswerRequest {
    /// `None` when submitting in wrong-answer review mode (no formal exam attempt).
    #[serde(default)]
    pub attempt_id: Option<i64>,
    pub question_id: i64,
    pub selected_option_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitAnswerResponse {
    pub is_correct: bool,
    /// Populated only when the answer is wrong, so the client can highlight correct options.
    pub correct_option_ids: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishExamResponse {
    pub attempt_id: i64,
    pub score: f64,
    pub total_questions: i32,
    pub correct_count: i32,
    pub wrong_questions: Vec<WrongQuestionDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrongQuestionDetail {
    pub question_id: i64,
    pub question_text_en: String,
    pub correct_options: Vec<String>,
    pub your_options: Vec<String>,
    pub explanation_en: Option<String>,
}

// ---------------------------------------------------------------------------
// Analytics
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreAnalytics {
    pub overall_score: f64,
    pub by_subject: Vec<SubjectScore>,
    pub by_difficulty: Vec<DifficultyScore>,
    pub recent_attempts: Vec<AttemptSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectScore {
    pub subject_id: i64,
    pub subject_name: String,
    pub avg_score: f64,
    pub attempt_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyScore {
    pub difficulty: String,
    pub avg_score: f64,
    pub attempt_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptSummary {
    pub id: i64,
    pub exam_title: String,
    pub score: f64,
    pub date: String,
    pub duration_minutes: Option<i32>,
}

// ---------------------------------------------------------------------------
// Wrong-answer review
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrongAnswerReviewSession {
    pub questions: Vec<ReviewQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewQuestion {
    pub question_id: i64,
    pub question_text_en: String,
    pub question_text_zh: Option<String>,
    pub question_type: String,
    pub options: Vec<ExamOptionDetail>,
    pub wrong_count: i32,
    pub last_wrong_at: String,
}

// ---------------------------------------------------------------------------
// Generic wrappers
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_response_success_serialization() {
        let resp = ApiResponse {
            success: true,
            data: Some("hello"),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""success":true"#));
        assert!(json.contains(r#""data":"hello""#));
        assert!(json.contains(r#""error":null"#));
    }

    #[test]
    fn api_response_error_serialization() {
        let resp: ApiResponse<()> = ApiResponse {
            success: false,
            data: None,
            error: Some("something went wrong".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"], "something went wrong");
    }

    #[test]
    fn api_response_deserialization() {
        let json = r#"{"success":true,"data":42,"error":null}"#;
        let resp: ApiResponse<i32> = serde_json::from_str(json).unwrap();
        assert!(resp.success);
        assert_eq!(resp.data, Some(42));
        assert!(resp.error.is_none());
    }

    #[test]
    fn login_request_deserialization() {
        let json = r#"{"username":"admin","password":"secret123!"}"#;
        let req: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "admin");
        assert_eq!(req.password, "secret123!");
    }

    #[test]
    fn add_to_cart_request_with_options() {
        let req = AddToCartRequest {
            sku_id: None,
            spu_id: 1,
            selected_options: vec![10, 20, 30],
            quantity: 2,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: AddToCartRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.spu_id, 1);
        assert_eq!(back.selected_options, vec![10, 20, 30]);
        assert_eq!(back.quantity, 2);
        assert!(back.sku_id.is_none());
    }

    #[test]
    fn scan_voucher_request_order_id_defaults_to_none() {
        let json = r#"{"voucher_code":"BF-ABC123"}"#;
        let req: ScanVoucherRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.voucher_code, "BF-ABC123");
        assert!(req.order_id.is_none());
    }

    #[test]
    fn scan_voucher_request_with_order_id() {
        let json = r#"{"voucher_code":"BF-XYZ","order_id":42}"#;
        let req: ScanVoucherRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.order_id, Some(42));
    }

    #[test]
    fn submit_answer_request_attempt_id_defaults_to_none() {
        let json = r#"{"question_id":5,"selected_option_ids":[1,3]}"#;
        let req: SubmitAnswerRequest = serde_json::from_str(json).unwrap();
        assert!(req.attempt_id.is_none());
        assert_eq!(req.question_id, 5);
        assert_eq!(req.selected_option_ids, vec![1, 3]);
    }

    #[test]
    fn checkout_request_round_trip() {
        let req = CheckoutRequest {
            pickup_slot_start: "2026-04-13T10:00:00".to_string(),
            pickup_slot_end: "2026-04-13T10:15:00".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: CheckoutRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.pickup_slot_start, "2026-04-13T10:00:00");
        assert_eq!(back.pickup_slot_end, "2026-04-13T10:15:00");
    }

    #[test]
    fn pickup_slot_equality() {
        let a = PickupSlot {
            start: "2026-04-13T09:00:00".to_string(),
            end: "2026-04-13T09:15:00".to_string(),
            available: true,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn paginated_response_serialization() {
        let resp = PaginatedResponse {
            items: vec![1, 2, 3],
            total: 100,
            page: 1,
            per_page: 3,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["total"], 100);
        assert_eq!(parsed["items"].as_array().unwrap().len(), 3);
    }

    // ── extended DTO coverage ──────────────────────────────────────────────

    #[test]
    fn user_info_with_multiple_roles_round_trips() {
        let ui = UserInfo {
            id: 1,
            username: "alice".into(),
            display_name: Some("Alice".into()),
            roles: vec!["Admin".into(), "Teacher".into()],
            preferred_locale: "zh".into(),
        };
        let json = serde_json::to_string(&ui).unwrap();
        let back: UserInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.roles.len(), 2);
        assert_eq!(back.preferred_locale, "zh");
    }

    #[test]
    fn login_response_includes_session_cookie_and_user() {
        let lr = LoginResponse {
            session_cookie: "id.sig".into(),
            user: UserInfo {
                id: 1,
                username: "u".into(),
                display_name: None,
                roles: vec!["Customer".into()],
                preferred_locale: "en".into(),
            },
        };
        let v = serde_json::to_value(&lr).unwrap();
        assert_eq!(v["session_cookie"], "id.sig");
        assert_eq!(v["user"]["username"], "u");
    }

    #[test]
    fn product_detail_nests_option_groups() {
        let pd = ProductDetail {
            spu: ProductListItem {
                spu_id: 1,
                name_en: "Latte".into(),
                name_zh: "\u{62ff}\u{94c1}".into(),
                description_en: None,
                description_zh: None,
                category: None,
                image_url: None,
                base_price: 4.0,
                prep_time_minutes: 5,
            },
            option_groups: vec![OptionGroupDetail {
                id: 1,
                name_en: "Size".into(),
                name_zh: "\u{5c3a}\u{5bf8}".into(),
                is_required: true,
                options: vec![OptionValueDetail {
                    id: 10,
                    label_en: "Small".into(),
                    label_zh: "\u{5c0f}".into(),
                    price_delta: 0.0,
                    is_default: true,
                }],
            }],
        };
        let json = serde_json::to_string(&pd).unwrap();
        let back: ProductDetail = serde_json::from_str(&json).unwrap();
        assert_eq!(back.option_groups.len(), 1);
        assert_eq!(back.option_groups[0].options[0].label_en, "Small");
        assert!(back.option_groups[0].is_required);
    }

    #[test]
    fn option_value_detail_equality() {
        let a = OptionValueDetail {
            id: 1,
            label_en: "Oat".into(),
            label_zh: "\u{71d5}\u{9ea6}".into(),
            price_delta: 0.50,
            is_default: false,
        };
        assert_eq!(a.clone(), a);
    }

    #[test]
    fn checkout_response_contains_voucher_and_total() {
        let cr = CheckoutResponse {
            order_id: 99,
            order_number: "ORD-99".into(),
            voucher_code: "BF-ABCDEF".into(),
            hold_expires_at: "2026-04-15T12:30:00".into(),
            pickup_slot: "2026-04-15T12:15:00 - 12:30:00".into(),
            total: 10.85,
        };
        let json = serde_json::to_value(&cr).unwrap();
        assert_eq!(json["voucher_code"], "BF-ABCDEF");
        assert_eq!(json["total"], 10.85);
    }

    #[test]
    fn cart_response_with_zero_items() {
        let cr = CartResponse {
            items: vec![],
            subtotal: 0.0,
            tax_rate: 0.0875,
            tax_amount: 0.0,
            total: 0.0,
        };
        let v = serde_json::to_value(&cr).unwrap();
        assert!(v["items"].as_array().unwrap().is_empty());
        assert_eq!(v["tax_rate"], 0.0875);
    }

    #[test]
    fn exam_question_detail_round_trips() {
        let q = ExamQuestionDetail {
            question_id: 1,
            question_text_en: "What is espresso?".into(),
            question_text_zh: None,
            question_type: "single_choice".into(),
            options: vec![
                ExamOptionDetail {
                    id: 1,
                    label: "A".into(),
                    content_en: "concentrated coffee".into(),
                    content_zh: None,
                },
                ExamOptionDetail {
                    id: 2,
                    label: "B".into(),
                    content_en: "a cappuccino".into(),
                    content_zh: None,
                },
            ],
        };
        let back: ExamQuestionDetail =
            serde_json::from_str(&serde_json::to_string(&q).unwrap()).unwrap();
        assert_eq!(back.options.len(), 2);
    }

    #[test]
    fn finish_exam_response_serializes_wrong_questions() {
        let fer = FinishExamResponse {
            attempt_id: 7,
            score: 80.0,
            total_questions: 10,
            correct_count: 8,
            wrong_questions: vec![WrongQuestionDetail {
                question_id: 3,
                question_text_en: "Stub".into(),
                correct_options: vec!["A".into()],
                your_options: vec!["B".into()],
                explanation_en: Some("because A".into()),
            }],
        };
        let json = serde_json::to_value(&fer).unwrap();
        assert_eq!(json["score"], 80.0);
        assert_eq!(json["wrong_questions"][0]["question_id"], 3);
    }

    #[test]
    fn import_questions_response_zero_on_empty_csv() {
        let r = ImportQuestionsResponse {
            imported_count: 0,
            skipped_count: 0,
            errors: vec!["no rows".into()],
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["imported_count"], 0);
        assert_eq!(json["errors"][0], "no rows");
    }

    #[test]
    fn scan_voucher_response_valid_no_mismatch() {
        let sr = ScanVoucherResponse {
            valid: true,
            order: None,
            mismatch: false,
            mismatch_reason: None,
        };
        let v = serde_json::to_value(&sr).unwrap();
        assert_eq!(v["valid"], true);
        assert_eq!(v["mismatch"], false);
        assert!(v["order"].is_null());
    }

    #[test]
    fn score_analytics_includes_breakdowns() {
        let sa = ScoreAnalytics {
            overall_score: 88.5,
            by_subject: vec![SubjectScore {
                subject_id: 1,
                subject_name: "Espresso".into(),
                avg_score: 90.0,
                attempt_count: 5,
            }],
            by_difficulty: vec![DifficultyScore {
                difficulty: "easy".into(),
                avg_score: 95.0,
                attempt_count: 3,
            }],
            recent_attempts: vec![AttemptSummary {
                id: 1,
                exam_title: "Espresso Basics".into(),
                score: 100.0,
                date: "2026-04-15".into(),
                duration_minutes: Some(15),
            }],
        };
        let json = serde_json::to_value(&sa).unwrap();
        assert_eq!(json["overall_score"], 88.5);
        assert_eq!(json["by_subject"][0]["avg_score"], 90.0);
        assert_eq!(json["by_difficulty"][0]["difficulty"], "easy");
    }

    #[test]
    fn generate_exam_request_allows_missing_difficulty() {
        let json = r#"{
            "title_en":"Quiz",
            "question_count":10,
            "time_limit_minutes":20
        }"#;
        let parsed: Result<GenerateExamRequest, _> = serde_json::from_str(json);
        // Missing optional fields should not fail if they are Option<T> — but
        // this struct has `subject_id: Option<i64>` without serde(default)
        // wrapper, so Option still deserialises as None.
        assert!(parsed.is_ok());
        let req = parsed.unwrap();
        assert!(req.subject_id.is_none());
        assert!(req.difficulty.is_none());
    }

    #[test]
    fn submit_answer_request_with_attempt_id() {
        let json = r#"{"attempt_id":7,"question_id":99,"selected_option_ids":[101,102]}"#;
        let req: SubmitAnswerRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.attempt_id, Some(7));
        assert_eq!(req.question_id, 99);
        assert_eq!(req.selected_option_ids.len(), 2);
    }

    #[test]
    fn add_to_cart_request_negative_quantity_still_parses() {
        // The DTO alone does not enforce quantity bounds — route-level
        // validation should catch them. Deserialisation must still succeed.
        let json = r#"{"spu_id":1,"selected_options":[],"quantity":-1}"#;
        let req: AddToCartRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.quantity, -1);
    }

    #[test]
    fn paginated_response_zero_items() {
        let p: PaginatedResponse<i32> = PaginatedResponse {
            items: vec![],
            total: 0,
            page: 1,
            per_page: 20,
        };
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["total"], 0);
        assert_eq!(v["per_page"], 20);
    }
}
