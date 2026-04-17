use sqlx::{MySqlPool, Row};
use shared::models::{Subject, Chapter, Question, QuestionOption, ExamVersion};

pub async fn list_subjects(pool: &MySqlPool) -> Vec<Subject> {
    let rows = sqlx::query(
        "SELECT id, name_en, name_zh, created_at FROM subjects ORDER BY id"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| Subject {
            id: r.get("id"),
            name_en: r.get("name_en"),
            name_zh: r.get("name_zh"),
            created_at: r.get("created_at"),
        })
        .collect()
}

pub async fn list_chapters(pool: &MySqlPool, subject_id: i64) -> Vec<Chapter> {
    let rows = sqlx::query(
        "SELECT id, subject_id, name_en, name_zh, sort_order
         FROM chapters WHERE subject_id = ? ORDER BY sort_order"
    )
    .bind(subject_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| Chapter {
            id: r.get("id"),
            subject_id: r.get("subject_id"),
            name_en: r.get("name_en"),
            name_zh: r.get("name_zh"),
            sort_order: r.get("sort_order"),
        })
        .collect()
}

pub async fn create_question(
    pool: &MySqlPool,
    subject_id: i64,
    chapter_id: Option<i64>,
    difficulty: &str,
    question_text_en: &str,
    question_text_zh: Option<&str>,
    explanation_en: Option<&str>,
    explanation_zh: Option<&str>,
    question_type: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO questions (subject_id, chapter_id, difficulty, question_text_en, question_text_zh,
         explanation_en, explanation_zh, question_type)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(subject_id)
    .bind(chapter_id)
    .bind(difficulty)
    .bind(question_text_en)
    .bind(question_text_zh)
    .bind(explanation_en)
    .bind(explanation_zh)
    .bind(question_type)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

pub async fn create_question_option(
    pool: &MySqlPool,
    question_id: i64,
    label: &str,
    content_en: &str,
    content_zh: Option<&str>,
    is_correct: bool,
    sort_order: i32,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO question_options (question_id, label, content_en, content_zh, is_correct, sort_order)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(question_id)
    .bind(label)
    .bind(content_en)
    .bind(content_zh)
    .bind(is_correct)
    .bind(sort_order)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

fn row_to_question(r: sqlx::mysql::MySqlRow) -> Question {
    Question {
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
    }
}

pub async fn get_questions_filtered(
    pool: &MySqlPool,
    subject_id: Option<i64>,
    chapter_id: Option<i64>,
    difficulty: Option<&str>,
    limit: i32,
) -> Vec<Question> {
    let mut sql = String::from(
        "SELECT id, subject_id, chapter_id, difficulty, question_text_en, question_text_zh,
                explanation_en, explanation_zh, question_type, created_at, updated_at
         FROM questions WHERE 1=1"
    );

    if subject_id.is_some() {
        sql.push_str(" AND subject_id = ?");
    }
    if chapter_id.is_some() {
        sql.push_str(" AND chapter_id = ?");
    }
    if difficulty.is_some() {
        sql.push_str(" AND difficulty = ?");
    }
    sql.push_str(" ORDER BY RAND() LIMIT ?");

    let mut query = sqlx::query(&sql);

    if let Some(sid) = subject_id {
        query = query.bind(sid);
    }
    if let Some(cid) = chapter_id {
        query = query.bind(cid);
    }
    if let Some(d) = difficulty {
        query = query.bind(d.to_string());
    }
    query = query.bind(limit);

    let rows = query.fetch_all(pool).await.unwrap_or_default();
    rows.into_iter().map(row_to_question).collect()
}

pub async fn get_question(pool: &MySqlPool, id: i64) -> Option<Question> {
    let row = sqlx::query(
        "SELECT id, subject_id, chapter_id, difficulty, question_text_en, question_text_zh,
                explanation_en, explanation_zh, question_type, created_at, updated_at
         FROM questions WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(row_to_question)
}

pub async fn get_question_options(pool: &MySqlPool, question_id: i64) -> Vec<QuestionOption> {
    let rows = sqlx::query(
        "SELECT id, question_id, label, content_en, content_zh, is_correct, sort_order
         FROM question_options WHERE question_id = ? ORDER BY sort_order"
    )
    .bind(question_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| QuestionOption {
            id: r.get("id"),
            question_id: r.get("question_id"),
            label: r.get("label"),
            content_en: r.get("content_en"),
            content_zh: r.get("content_zh"),
            is_correct: r.get("is_correct"),
            sort_order: r.get("sort_order"),
        })
        .collect()
}

pub async fn create_exam_version(
    pool: &MySqlPool,
    title_en: &str,
    title_zh: Option<&str>,
    subject_id: Option<i64>,
    chapter_id: Option<i64>,
    difficulty: &str,
    question_count: i32,
    time_limit_minutes: i32,
    created_by: i64,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO exam_versions (title_en, title_zh, subject_id, chapter_id, difficulty,
         question_count, time_limit_minutes, created_by)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(title_en)
    .bind(title_zh)
    .bind(subject_id)
    .bind(chapter_id)
    .bind(difficulty)
    .bind(question_count)
    .bind(time_limit_minutes)
    .bind(created_by)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

pub async fn add_exam_question(
    pool: &MySqlPool,
    exam_version_id: i64,
    question_id: i64,
    sort_order: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO exam_version_questions (exam_version_id, question_id, sort_order)
         VALUES (?, ?, ?)"
    )
    .bind(exam_version_id)
    .bind(question_id)
    .bind(sort_order)
    .execute(pool)
    .await?;

    Ok(())
}

fn row_to_exam_version(r: sqlx::mysql::MySqlRow) -> ExamVersion {
    ExamVersion {
        id: r.get("id"),
        title_en: r.get("title_en"),
        title_zh: r.get("title_zh"),
        subject_id: r.get("subject_id"),
        chapter_id: r.get("chapter_id"),
        difficulty: r.get("difficulty"),
        question_count: r.get("question_count"),
        time_limit_minutes: r.get("time_limit_minutes"),
        created_by: r.get("created_by"),
        created_at: r.get("created_at"),
        updated_at: None,
    }
}

pub async fn get_exam_version(pool: &MySqlPool, id: i64) -> Option<ExamVersion> {
    let row = sqlx::query(
        "SELECT id, title_en, title_zh, subject_id, chapter_id, difficulty,
                question_count, time_limit_minutes, created_by, created_at
         FROM exam_versions WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(row_to_exam_version)
}

pub async fn list_exam_versions(pool: &MySqlPool) -> Vec<ExamVersion> {
    let rows = sqlx::query(
        "SELECT id, title_en, title_zh, subject_id, chapter_id, difficulty,
                question_count, time_limit_minutes, created_by, created_at
         FROM exam_versions ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter().map(row_to_exam_version).collect()
}

/// Returns a page of questions joined with subject/chapter names for the admin question bank.
/// Also returns the total matching count for pagination metadata.
pub async fn get_questions_paginated(
    pool: &MySqlPool,
    subject_id: Option<i64>,
    chapter_id: Option<i64>,
    difficulty: Option<&str>,
    search: Option<&str>,
    page: i32,
    per_page: i32,
) -> (Vec<shared::dto::QuestionListItem>, i64) {
    let mut where_clauses = String::from(" WHERE 1=1");
    if subject_id.is_some() { where_clauses.push_str(" AND q.subject_id = ?"); }
    if chapter_id.is_some() { where_clauses.push_str(" AND q.chapter_id = ?"); }
    if difficulty.is_some() { where_clauses.push_str(" AND q.difficulty = ?"); }
    if search.is_some()     { where_clauses.push_str(" AND q.question_text_en LIKE ?"); }

    // Count query
    let count_sql = format!(
        "SELECT COUNT(*) AS cnt FROM questions q{}",
        where_clauses
    );
    let mut count_q = sqlx::query(&count_sql);
    if let Some(v) = subject_id  { count_q = count_q.bind(v); }
    if let Some(v) = chapter_id  { count_q = count_q.bind(v); }
    if let Some(v) = difficulty  { count_q = count_q.bind(v.to_string()); }
    if let Some(v) = search      { count_q = count_q.bind(format!("%{}%", v)); }
    let total: i64 = count_q.fetch_one(pool).await
        .map(|r| r.get::<i64, _>("cnt"))
        .unwrap_or(0);

    // Data query
    let offset = ((page - 1).max(0) as i64) * (per_page as i64);
    let data_sql = format!(
        "SELECT q.id, q.question_text_en, q.question_text_zh, q.question_type, q.difficulty,
                s.name_en AS subject_name, c.name_en AS chapter_name
         FROM questions q
         LEFT JOIN subjects s ON s.id = q.subject_id
         LEFT JOIN chapters c ON c.id = q.chapter_id{}
         ORDER BY q.id DESC LIMIT ? OFFSET ?",
        where_clauses
    );
    let mut data_q = sqlx::query(&data_sql);
    if let Some(v) = subject_id  { data_q = data_q.bind(v); }
    if let Some(v) = chapter_id  { data_q = data_q.bind(v); }
    if let Some(v) = difficulty  { data_q = data_q.bind(v.to_string()); }
    if let Some(v) = search      { data_q = data_q.bind(format!("%{}%", v)); }
    data_q = data_q.bind(per_page).bind(offset);

    let rows = data_q.fetch_all(pool).await.unwrap_or_default();
    let items = rows.into_iter().map(|r| shared::dto::QuestionListItem {
        id: r.get("id"),
        question_text_en: r.get("question_text_en"),
        question_text_zh: r.get("question_text_zh"),
        question_type: r.get("question_type"),
        difficulty: r.get("difficulty"),
        subject_name: r.get("subject_name"),
        chapter_name: r.get("chapter_name"),
    }).collect();

    (items, total)
}

/// Returns questions with their options for a given exam version, ordered by sort_order.
pub async fn get_exam_questions(
    pool: &MySqlPool,
    exam_version_id: i64,
) -> Vec<(Question, Vec<QuestionOption>)> {
    let question_rows = sqlx::query(
        "SELECT q.id, q.subject_id, q.chapter_id, q.difficulty, q.question_text_en,
                q.question_text_zh, q.explanation_en, q.explanation_zh, q.question_type,
                q.created_at, q.updated_at
         FROM exam_version_questions evq
         JOIN questions q ON q.id = evq.question_id
         WHERE evq.exam_version_id = ?
         ORDER BY evq.sort_order"
    )
    .bind(exam_version_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut result = Vec::new();
    for r in question_rows {
        let q = row_to_question(r);
        let opts = get_question_options(pool, q.id).await;
        result.push((q, opts));
    }
    result
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

    // ── Subject ──────────────────────────────────────────────────────────

    #[test]
    fn subject_construction() {
        let s = Subject {
            id: 1,
            name_en: "Espresso Basics".to_string(),
            name_zh: "\u{6d53}\u{7f29}\u{5496}\u{5561}\u{57fa}\u{7840}".to_string(),
            created_at: sample_dt(),
        };
        assert_eq!(s.id, 1);
        assert_eq!(s.name_en, "Espresso Basics");
    }

    #[test]
    fn subject_serde_round_trip() {
        let s = Subject {
            id: 5,
            name_en: "Latte Art".to_string(),
            name_zh: "\u{62c9}\u{82b1}".to_string(),
            created_at: sample_dt(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Subject = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 5);
        assert_eq!(back.name_en, "Latte Art");
    }

    // ── Chapter ──────────────────────────────────────────────────────────

    #[test]
    fn chapter_construction() {
        let c = Chapter {
            id: 1,
            subject_id: 3,
            name_en: "Grind Size".to_string(),
            name_zh: "\u{7814}\u{78e8}\u{5ea6}".to_string(),
            sort_order: 1,
        };
        assert_eq!(c.subject_id, 3);
        assert_eq!(c.sort_order, 1);
    }

    #[test]
    fn chapter_serde_round_trip() {
        let c = Chapter {
            id: 10,
            subject_id: 2,
            name_en: "Extraction".to_string(),
            name_zh: "\u{8403}\u{53d6}".to_string(),
            sort_order: 2,
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: Chapter = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 10);
        assert_eq!(back.sort_order, 2);
    }

    // ── Question ─────────────────────────────────────────────────────────

    #[test]
    fn question_construction() {
        let q = Question {
            id: 1,
            subject_id: 1,
            chapter_id: Some(2),
            difficulty: "medium".to_string(),
            question_text_en: "What is espresso?".to_string(),
            question_text_zh: None,
            explanation_en: Some("Espresso is...".to_string()),
            explanation_zh: None,
            question_type: "single_choice".to_string(),
            created_at: sample_dt(),
            updated_at: None,
        };
        assert_eq!(q.id, 1);
        assert_eq!(q.difficulty, "medium");
        assert_eq!(q.question_type, "single_choice");
        assert_eq!(q.chapter_id, Some(2));
    }

    #[test]
    fn question_serde_round_trip() {
        let q = Question {
            id: 7,
            subject_id: 3,
            chapter_id: None,
            difficulty: "hard".to_string(),
            question_text_en: "Describe extraction theory.".to_string(),
            question_text_zh: Some("\u{63cf}\u{8ff0}\u{8403}\u{53d6}\u{7406}\u{8bba}".to_string()),
            explanation_en: None,
            explanation_zh: None,
            question_type: "multiple_choice".to_string(),
            created_at: sample_dt(),
            updated_at: Some(sample_dt()),
        };
        let json = serde_json::to_string(&q).unwrap();
        let back: Question = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 7);
        assert_eq!(back.difficulty, "hard");
        assert!(back.chapter_id.is_none());
        assert!(back.question_text_zh.is_some());
    }

    // ── ExamVersion ──────────────────────────────────────────────────────

    #[test]
    fn exam_version_construction() {
        let ev = ExamVersion {
            id: 1,
            title_en: "Midterm Exam".to_string(),
            title_zh: Some("\u{671f}\u{4e2d}\u{8003}\u{8bd5}".to_string()),
            subject_id: Some(1),
            chapter_id: None,
            difficulty: "medium".to_string(),
            question_count: 20,
            time_limit_minutes: 30,
            created_by: Some(5),
            created_at: sample_dt(),
            updated_at: None,
        };
        assert_eq!(ev.id, 1);
        assert_eq!(ev.question_count, 20);
        assert_eq!(ev.time_limit_minutes, 30);
        assert_eq!(ev.created_by, Some(5));
    }

    #[test]
    fn exam_version_serde_round_trip() {
        let ev = ExamVersion {
            id: 10,
            title_en: "Final".to_string(),
            title_zh: None,
            subject_id: None,
            chapter_id: None,
            difficulty: "hard".to_string(),
            question_count: 50,
            time_limit_minutes: 60,
            created_by: None,
            created_at: sample_dt(),
            updated_at: None,
        };
        let json = serde_json::to_string(&ev).unwrap();
        let back: ExamVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 10);
        assert_eq!(back.difficulty, "hard");
        assert!(back.subject_id.is_none());
        assert!(back.created_by.is_none());
    }

    // ── QuestionOption ───────────────────────────────────────────────────

    #[test]
    fn question_option_construction_and_round_trip() {
        let opt = QuestionOption {
            id: 1,
            question_id: 5,
            label: "A".to_string(),
            content_en: "42".to_string(),
            content_zh: Some("\u{56db}\u{5341}\u{4e8c}".to_string()),
            is_correct: true,
            sort_order: 0,
        };
        let json = serde_json::to_string(&opt).unwrap();
        let back: QuestionOption = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 1);
        assert_eq!(back.label, "A");
        assert!(back.is_correct);
    }
}
