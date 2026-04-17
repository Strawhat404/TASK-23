use sqlx::{MySqlPool, Row};
use shared::models::{ExamAttempt, AttemptAnswer, Question, WrongAnswerEntry};

pub async fn create_attempt(
    pool: &MySqlPool,
    user_id: i64,
    exam_version_id: i64,
    total_questions: i32,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO exam_attempts (user_id, exam_version_id, total_questions, correct_count, status)
         VALUES (?, ?, ?, 0, 'InProgress')"
    )
    .bind(user_id)
    .bind(exam_version_id)
    .bind(total_questions)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

pub async fn save_answer(
    pool: &MySqlPool,
    attempt_id: i64,
    question_id: i64,
    selected_ids_json: &str,
    is_correct: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO attempt_answers (attempt_id, question_id, selected_option_ids, is_correct)
         VALUES (?, ?, ?, ?)"
    )
    .bind(attempt_id)
    .bind(question_id)
    .bind(selected_ids_json)
    .bind(is_correct)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn finish_attempt(
    pool: &MySqlPool,
    attempt_id: i64,
    score: f64,
    correct_count: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE exam_attempts SET finished_at = NOW(), score = ?, correct_count = ?, status = 'Completed'
         WHERE id = ?"
    )
    .bind(score)
    .bind(correct_count)
    .bind(attempt_id)
    .execute(pool)
    .await?;

    Ok(())
}

fn row_to_attempt(r: sqlx::mysql::MySqlRow) -> ExamAttempt {
    ExamAttempt {
        id: r.get("id"),
        user_id: r.get("user_id"),
        exam_version_id: r.get("exam_version_id"),
        started_at: r.get("started_at"),
        finished_at: r.get("finished_at"),
        score: r.get("score"),
        total_questions: r.get("total_questions"),
        correct_count: r.get("correct_count"),
        status: r.get("status"),
    }
}

pub async fn get_attempt(pool: &MySqlPool, id: i64) -> Option<ExamAttempt> {
    let row = sqlx::query(
        "SELECT id, user_id, exam_version_id, started_at, finished_at, score,
                total_questions, correct_count, status
         FROM exam_attempts WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(row_to_attempt)
}

pub async fn get_user_attempts(pool: &MySqlPool, user_id: i64) -> Vec<ExamAttempt> {
    let rows = sqlx::query(
        "SELECT id, user_id, exam_version_id, started_at, finished_at, score,
                total_questions, correct_count, status
         FROM exam_attempts WHERE user_id = ? ORDER BY started_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter().map(row_to_attempt).collect()
}

pub async fn get_attempt_answers(pool: &MySqlPool, attempt_id: i64) -> Vec<AttemptAnswer> {
    let rows = sqlx::query(
        "SELECT id, attempt_id, question_id, selected_option_ids, is_correct, answered_at
         FROM attempt_answers WHERE attempt_id = ? ORDER BY id"
    )
    .bind(attempt_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| AttemptAnswer {
            id: r.get("id"),
            attempt_id: r.get("attempt_id"),
            question_id: r.get("question_id"),
            selected_option_ids: r.get("selected_option_ids"),
            is_correct: r.get("is_correct"),
            answered_at: r.get("answered_at"),
        })
        .collect()
}

pub async fn add_favorite(
    pool: &MySqlPool,
    user_id: i64,
    question_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT IGNORE INTO favorites (user_id, question_id) VALUES (?, ?)"
    )
    .bind(user_id)
    .bind(question_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn remove_favorite(
    pool: &MySqlPool,
    user_id: i64,
    question_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM favorites WHERE user_id = ? AND question_id = ?")
        .bind(user_id)
        .bind(question_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_favorites(pool: &MySqlPool, user_id: i64) -> Vec<Question> {
    let rows = sqlx::query(
        "SELECT q.id, q.subject_id, q.chapter_id, q.difficulty, q.question_text_en,
                q.question_text_zh, q.explanation_en, q.explanation_zh, q.question_type,
                q.created_at, q.updated_at
         FROM favorites f
         JOIN questions q ON q.id = f.question_id
         WHERE f.user_id = ?
         ORDER BY f.created_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| Question {
            id: r.get("id"),
            subject_id: r.get("subject_id"),
            chapter_id: r.get("chapter_id"),
            difficulty: r.get("difficulty"),
            question_text_en: r.get("question_text_en"),
            question_text_zh: r.get("question_text_zh"),
            explanation_en: r.get("explanation_en"),
            explanation_zh: r.get("explanation_zh"),
            question_type: r.get("question_type"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect()
}

pub async fn upsert_wrong_answer(
    pool: &MySqlPool,
    user_id: i64,
    question_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO wrong_answer_notebook (user_id, question_id, wrong_count, last_wrong_at, review_interval_days)
         VALUES (?, ?, 1, NOW(), 1)
         ON DUPLICATE KEY UPDATE
           wrong_count = wrong_count + 1,
           last_wrong_at = NOW(),
           next_review_at = DATE_ADD(NOW(), INTERVAL review_interval_days DAY)"
    )
    .bind(user_id)
    .bind(question_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_wrong_answers_for_review(pool: &MySqlPool, user_id: i64) -> Vec<WrongAnswerEntry> {
    let rows = sqlx::query(
        "SELECT id, user_id, question_id, wrong_count, last_wrong_at, next_review_at, review_interval_days
         FROM wrong_answer_notebook
         WHERE user_id = ? AND (next_review_at IS NULL OR next_review_at <= NOW())
         ORDER BY wrong_count DESC, last_wrong_at ASC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter().map(row_to_wrong_entry).collect()
}

pub async fn get_wrong_notebook(pool: &MySqlPool, user_id: i64) -> Vec<WrongAnswerEntry> {
    let rows = sqlx::query(
        "SELECT id, user_id, question_id, wrong_count, last_wrong_at, next_review_at, review_interval_days
         FROM wrong_answer_notebook
         WHERE user_id = ?
         ORDER BY wrong_count DESC, last_wrong_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter().map(row_to_wrong_entry).collect()
}

fn row_to_wrong_entry(r: sqlx::mysql::MySqlRow) -> WrongAnswerEntry {
    WrongAnswerEntry {
        id: r.get("id"),
        user_id: r.get("user_id"),
        question_id: r.get("question_id"),
        wrong_count: r.get("wrong_count"),
        last_wrong_at: r.get("last_wrong_at"),
        next_review_at: r.get("next_review_at"),
        review_interval_days: r.get("review_interval_days"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn sample_dt() -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 4, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    // ── ExamAttempt ──────────────────────────────────────────────────────

    #[test]
    fn exam_attempt_construction() {
        let a = ExamAttempt {
            id: 1,
            user_id: 10,
            exam_version_id: 5,
            started_at: sample_dt(),
            finished_at: None,
            score: None,
            total_questions: 20,
            correct_count: 0,
            status: "InProgress".to_string(),
        };
        assert_eq!(a.id, 1);
        assert_eq!(a.user_id, 10);
        assert_eq!(a.status, "InProgress");
        assert!(a.finished_at.is_none());
        assert!(a.score.is_none());
    }

    #[test]
    fn exam_attempt_completed_round_trip() {
        let a = ExamAttempt {
            id: 7,
            user_id: 3,
            exam_version_id: 2,
            started_at: sample_dt(),
            finished_at: Some(sample_dt()),
            score: Some(85.0),
            total_questions: 20,
            correct_count: 17,
            status: "Completed".to_string(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let back: ExamAttempt = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 7);
        assert_eq!(back.correct_count, 17);
        assert!((back.score.unwrap() - 85.0).abs() < 1e-9);
        assert_eq!(back.status, "Completed");
    }

    #[test]
    fn exam_attempt_score_proportion() {
        let a = ExamAttempt {
            id: 1,
            user_id: 1,
            exam_version_id: 1,
            started_at: sample_dt(),
            finished_at: Some(sample_dt()),
            score: Some(80.0),
            total_questions: 10,
            correct_count: 8,
            status: "Completed".to_string(),
        };
        let expected = (a.correct_count as f64 / a.total_questions as f64) * 100.0;
        assert!((a.score.unwrap() - expected).abs() < 1e-9);
    }

    // ── AttemptAnswer ────────────────────────────────────────────────────

    #[test]
    fn attempt_answer_construction() {
        let ans = AttemptAnswer {
            id: 1,
            attempt_id: 5,
            question_id: 10,
            selected_option_ids: Some("[1,3]".to_string()),
            is_correct: Some(true),
            answered_at: Some(sample_dt()),
        };
        assert_eq!(ans.attempt_id, 5);
        assert_eq!(ans.question_id, 10);
        assert!(ans.is_correct.unwrap());
    }

    #[test]
    fn attempt_answer_serde_round_trip() {
        let ans = AttemptAnswer {
            id: 2,
            attempt_id: 3,
            question_id: 7,
            selected_option_ids: None,
            is_correct: None,
            answered_at: None,
        };
        let json = serde_json::to_string(&ans).unwrap();
        let back: AttemptAnswer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 2);
        assert!(back.selected_option_ids.is_none());
        assert!(back.is_correct.is_none());
    }

    // ── WrongAnswerEntry ─────────────────────────────────────────────────

    #[test]
    fn wrong_answer_entry_construction() {
        let w = WrongAnswerEntry {
            id: 1,
            user_id: 5,
            question_id: 20,
            wrong_count: 3,
            last_wrong_at: Some(sample_dt()),
            next_review_at: Some(sample_dt()),
            review_interval_days: 2,
        };
        assert_eq!(w.wrong_count, 3);
        assert_eq!(w.review_interval_days, 2);
        assert!(w.last_wrong_at.is_some());
    }

    #[test]
    fn wrong_answer_entry_serde_round_trip() {
        let w = WrongAnswerEntry {
            id: 10,
            user_id: 1,
            question_id: 50,
            wrong_count: 7,
            last_wrong_at: None,
            next_review_at: None,
            review_interval_days: 1,
        };
        let json = serde_json::to_string(&w).unwrap();
        let back: WrongAnswerEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 10);
        assert_eq!(back.wrong_count, 7);
        assert!(back.last_wrong_at.is_none());
    }

    #[test]
    fn wrong_answer_entry_clone() {
        let w = WrongAnswerEntry {
            id: 1,
            user_id: 1,
            question_id: 1,
            wrong_count: 1,
            last_wrong_at: None,
            next_review_at: None,
            review_interval_days: 1,
        };
        let cloned = w.clone();
        assert_eq!(cloned.id, w.id);
        assert_eq!(cloned.wrong_count, w.wrong_count);
    }
}
