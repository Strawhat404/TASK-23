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
}
