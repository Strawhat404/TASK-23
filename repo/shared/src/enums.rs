use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Role
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "staff")]
    Staff,
    #[serde(rename = "customer")]
    Customer,
    #[serde(rename = "academic_affairs")]
    AcademicAffairs,
    #[serde(rename = "teacher")]
    Teacher,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Role::Admin => "admin",
            Role::Staff => "staff",
            Role::Customer => "customer",
            Role::AcademicAffairs => "academic_affairs",
            Role::Teacher => "teacher",
        };
        write!(f, "{}", s)
    }
}

// ---------------------------------------------------------------------------
// Locale
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Locale {
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh")]
    Zh,
}

impl Locale {
    pub fn to_str(&self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Zh => "zh",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "zh" | "ZH" | "zh-CN" | "zh-TW" => Locale::Zh,
            _ => Locale::En,
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

// ---------------------------------------------------------------------------
// OrderStatus
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "in_prep")]
    InPrep,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "picked_up")]
    PickedUp,
    #[serde(rename = "canceled")]
    Canceled,
}

impl OrderStatus {
    pub fn allowed_transitions(&self) -> Vec<OrderStatus> {
        match self {
            OrderStatus::Pending => vec![OrderStatus::Accepted, OrderStatus::Canceled],
            OrderStatus::Accepted => vec![OrderStatus::InPrep, OrderStatus::Canceled],
            OrderStatus::InPrep => vec![OrderStatus::Ready, OrderStatus::Canceled],
            OrderStatus::Ready => vec![OrderStatus::PickedUp, OrderStatus::Canceled],
            OrderStatus::PickedUp => vec![],
            OrderStatus::Canceled => vec![],
        }
    }
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OrderStatus::Pending => "pending",
            OrderStatus::Accepted => "accepted",
            OrderStatus::InPrep => "in_prep",
            OrderStatus::Ready => "ready",
            OrderStatus::PickedUp => "picked_up",
            OrderStatus::Canceled => "canceled",
        };
        write!(f, "{}", s)
    }
}

// ---------------------------------------------------------------------------
// ReservationStatus
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReservationStatus {
    #[serde(rename = "held")]
    Held,
    #[serde(rename = "confirmed")]
    Confirmed,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "canceled")]
    Canceled,
}

impl fmt::Display for ReservationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ReservationStatus::Held => "held",
            ReservationStatus::Confirmed => "confirmed",
            ReservationStatus::Expired => "expired",
            ReservationStatus::Canceled => "canceled",
        };
        write!(f, "{}", s)
    }
}

// ---------------------------------------------------------------------------
// QuestionType
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestionType {
    #[serde(rename = "single_choice")]
    SingleChoice,
    #[serde(rename = "multiple_choice")]
    MultipleChoice,
    #[serde(rename = "true_false")]
    TrueFalse,
}

impl fmt::Display for QuestionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            QuestionType::SingleChoice => "single_choice",
            QuestionType::MultipleChoice => "multiple_choice",
            QuestionType::TrueFalse => "true_false",
        };
        write!(f, "{}", s)
    }
}

// ---------------------------------------------------------------------------
// Difficulty
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    #[serde(rename = "easy")]
    Easy,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "hard")]
    Hard,
    #[serde(rename = "mixed")]
    Mixed,
}

impl fmt::Display for Difficulty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
            Difficulty::Mixed => "mixed",
        };
        write!(f, "{}", s)
    }
}

// ---------------------------------------------------------------------------
// ExamAttemptStatus
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExamAttemptStatus {
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "abandoned")]
    Abandoned,
}

impl fmt::Display for ExamAttemptStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ExamAttemptStatus::InProgress => "in_progress",
            ExamAttemptStatus::Completed => "completed",
            ExamAttemptStatus::Abandoned => "abandoned",
        };
        write!(f, "{}", s)
    }
}

// ---------------------------------------------------------------------------
// SnapshotType
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotType {
    #[serde(rename = "user_score")]
    UserScore,
    #[serde(rename = "subject_stats")]
    SubjectStats,
    #[serde(rename = "difficulty_breakdown")]
    DifficultyBreakdown,
    #[serde(rename = "daily_activity")]
    DailyActivity,
}

impl fmt::Display for SnapshotType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SnapshotType::UserScore => "user_score",
            SnapshotType::SubjectStats => "subject_stats",
            SnapshotType::DifficultyBreakdown => "difficulty_breakdown",
            SnapshotType::DailyActivity => "daily_activity",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Role ──────────────────────────────────────────────────────────────

    #[test]
    fn role_serde_round_trip() {
        let json = serde_json::to_string(&Role::Admin).unwrap();
        assert_eq!(json, r#""admin""#);
        let back: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Role::Admin);
    }

    #[test]
    fn role_display() {
        assert_eq!(Role::Customer.to_string(), "customer");
        assert_eq!(Role::AcademicAffairs.to_string(), "academic_affairs");
    }

    // ── Locale ────────────────────────────────────────────────────────────

    #[test]
    fn locale_from_str_defaults_to_english() {
        assert_eq!(Locale::from_str("en"), Locale::En);
        assert_eq!(Locale::from_str("unknown"), Locale::En);
        assert_eq!(Locale::from_str(""), Locale::En);
    }

    #[test]
    fn locale_from_str_recognizes_chinese_variants() {
        assert_eq!(Locale::from_str("zh"), Locale::Zh);
        assert_eq!(Locale::from_str("zh-CN"), Locale::Zh);
        assert_eq!(Locale::from_str("zh-TW"), Locale::Zh);
        assert_eq!(Locale::from_str("ZH"), Locale::Zh);
    }

    #[test]
    fn locale_display_and_to_str() {
        assert_eq!(Locale::En.to_str(), "en");
        assert_eq!(Locale::Zh.to_str(), "zh");
        assert_eq!(Locale::En.to_string(), "en");
    }

    // ── OrderStatus ───────────────────────────────────────────────────────

    #[test]
    fn order_status_serde_round_trip() {
        let json = serde_json::to_string(&OrderStatus::InPrep).unwrap();
        assert_eq!(json, r#""in_prep""#);
        let back: OrderStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, OrderStatus::InPrep);
    }

    #[test]
    fn terminal_statuses_have_no_transitions() {
        assert!(OrderStatus::PickedUp.allowed_transitions().is_empty());
        assert!(OrderStatus::Canceled.allowed_transitions().is_empty());
    }

    #[test]
    fn pending_can_transition_to_accepted_or_canceled() {
        let transitions = OrderStatus::Pending.allowed_transitions();
        assert!(transitions.contains(&OrderStatus::Accepted));
        assert!(transitions.contains(&OrderStatus::Canceled));
        assert_eq!(transitions.len(), 2);
    }

    #[test]
    fn ready_can_transition_to_picked_up_or_canceled() {
        let transitions = OrderStatus::Ready.allowed_transitions();
        assert!(transitions.contains(&OrderStatus::PickedUp));
        assert!(transitions.contains(&OrderStatus::Canceled));
    }

    // ── ReservationStatus ─────────────────────────────────────────────────

    #[test]
    fn reservation_status_serde() {
        let json = serde_json::to_string(&ReservationStatus::Held).unwrap();
        assert_eq!(json, r#""held""#);
    }

    // ── QuestionType ──────────────────────────────────────────────────────

    #[test]
    fn question_type_display() {
        assert_eq!(QuestionType::SingleChoice.to_string(), "single_choice");
        assert_eq!(QuestionType::MultipleChoice.to_string(), "multiple_choice");
        assert_eq!(QuestionType::TrueFalse.to_string(), "true_false");
    }

    // ── Difficulty ────────────────────────────────────────────────────────

    #[test]
    fn difficulty_serde_round_trip() {
        for d in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard, Difficulty::Mixed] {
            let json = serde_json::to_string(&d).unwrap();
            let back: Difficulty = serde_json::from_str(&json).unwrap();
            assert_eq!(back, d);
        }
    }

    // ── ExamAttemptStatus ─────────────────────────────────────────────────

    #[test]
    fn exam_attempt_status_display() {
        assert_eq!(ExamAttemptStatus::InProgress.to_string(), "in_progress");
        assert_eq!(ExamAttemptStatus::Completed.to_string(), "completed");
        assert_eq!(ExamAttemptStatus::Abandoned.to_string(), "abandoned");
    }

    // ── Role: full matrix ──────────────────────────────────────────────────

    #[test]
    fn all_roles_round_trip_through_json() {
        for r in [
            Role::Admin,
            Role::Staff,
            Role::Customer,
            Role::AcademicAffairs,
            Role::Teacher,
        ] {
            let json = serde_json::to_string(&r).unwrap();
            let back: Role = serde_json::from_str(&json).unwrap();
            assert_eq!(back, r);
        }
    }

    #[test]
    fn roles_display_strings_match_wire_format() {
        assert_eq!(Role::Admin.to_string(), "admin");
        assert_eq!(Role::Staff.to_string(), "staff");
        assert_eq!(Role::Teacher.to_string(), "teacher");
    }

    #[test]
    fn role_deserialization_is_strict() {
        // "Admin" capitalized is not a valid wire value.
        assert!(serde_json::from_str::<Role>(r#""Admin""#).is_err());
        assert!(serde_json::from_str::<Role>(r#""UNKNOWN""#).is_err());
    }

    // ── OrderStatus: full transition matrix ────────────────────────────────

    #[test]
    fn accepted_can_only_transition_to_in_prep_or_canceled() {
        let t = OrderStatus::Accepted.allowed_transitions();
        assert_eq!(t.len(), 2);
        assert!(t.contains(&OrderStatus::InPrep));
        assert!(t.contains(&OrderStatus::Canceled));
    }

    #[test]
    fn in_prep_can_only_transition_to_ready_or_canceled() {
        let t = OrderStatus::InPrep.allowed_transitions();
        assert_eq!(t.len(), 2);
        assert!(t.contains(&OrderStatus::Ready));
        assert!(t.contains(&OrderStatus::Canceled));
    }

    #[test]
    fn no_order_status_transitions_to_itself() {
        let all = [
            OrderStatus::Pending,
            OrderStatus::Accepted,
            OrderStatus::InPrep,
            OrderStatus::Ready,
            OrderStatus::PickedUp,
            OrderStatus::Canceled,
        ];
        for s in &all {
            assert!(
                !s.allowed_transitions().contains(s),
                "self-transition not allowed for {}",
                s
            );
        }
    }

    #[test]
    fn order_status_wire_values_are_snake_case() {
        assert_eq!(serde_json::to_string(&OrderStatus::PickedUp).unwrap(), r#""picked_up""#);
        assert_eq!(serde_json::to_string(&OrderStatus::Canceled).unwrap(), r#""canceled""#);
        assert_eq!(serde_json::to_string(&OrderStatus::Pending).unwrap(), r#""pending""#);
    }

    // ── ReservationStatus: full round-trip ─────────────────────────────────

    #[test]
    fn reservation_status_all_variants_round_trip() {
        for s in [
            ReservationStatus::Held,
            ReservationStatus::Confirmed,
            ReservationStatus::Expired,
            ReservationStatus::Canceled,
        ] {
            let json = serde_json::to_string(&s).unwrap();
            let back: ReservationStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, s);
        }
    }

    #[test]
    fn reservation_status_display_matches_serde() {
        assert_eq!(ReservationStatus::Expired.to_string(), "expired");
        assert_eq!(ReservationStatus::Confirmed.to_string(), "confirmed");
    }

    // ── Locale: symmetric conversion ───────────────────────────────────────

    #[test]
    fn locale_round_trips_via_to_str_and_from_str() {
        for loc in [Locale::En, Locale::Zh] {
            let back = Locale::from_str(loc.to_str());
            assert_eq!(back, loc);
        }
    }

    // ── SnapshotType ───────────────────────────────────────────────────────

    #[test]
    fn snapshot_type_display_matches_wire_form() {
        assert_eq!(SnapshotType::UserScore.to_string(), "user_score");
        assert_eq!(SnapshotType::DailyActivity.to_string(), "daily_activity");
    }

    #[test]
    fn snapshot_type_round_trips() {
        for s in [
            SnapshotType::UserScore,
            SnapshotType::SubjectStats,
            SnapshotType::DifficultyBreakdown,
            SnapshotType::DailyActivity,
        ] {
            let json = serde_json::to_string(&s).unwrap();
            let back: SnapshotType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, s);
        }
    }

    // ── QuestionType round-trip ────────────────────────────────────────────

    #[test]
    fn question_type_round_trips() {
        for q in [
            QuestionType::SingleChoice,
            QuestionType::MultipleChoice,
            QuestionType::TrueFalse,
        ] {
            let json = serde_json::to_string(&q).unwrap();
            let back: QuestionType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, q);
        }
    }
}
