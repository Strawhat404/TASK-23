use sqlx::{MySqlPool, Row};
use shared::models::AnalyticsSnapshot;
use shared::dto::{SubjectScore, DifficultyScore};

pub async fn save_snapshot(
    pool: &MySqlPool,
    user_id: Option<i64>,
    snapshot_type: &str,
    data_json: &str,
    date: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO analytics_snapshots (user_id, snapshot_type, snapshot_data, snapshot_date)
         VALUES (?, ?, ?, ?)"
    )
    .bind(user_id)
    .bind(snapshot_type)
    .bind(data_json)
    .bind(date)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

/// Returns (average_score, total_attempts, total_correct, total_questions) for a user.
pub async fn get_user_score_analytics(
    pool: &MySqlPool,
    user_id: i64,
) -> (f64, i64, i64, i64) {
    let row = sqlx::query(
        "SELECT COALESCE(AVG(score), 0) AS avg_score,
                COUNT(*) AS total_attempts,
                COALESCE(SUM(correct_count), 0) AS total_correct,
                COALESCE(SUM(total_questions), 0) AS total_questions
         FROM exam_attempts
         WHERE user_id = ? AND status = 'Completed'"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match row {
        Some(r) => {
            let avg_score: f64 = r.get("avg_score");
            let total_attempts: i64 = r.get::<i64, _>("total_attempts");
            let total_correct: i64 = r.get::<i64, _>("total_correct");
            let total_questions: i64 = r.get::<i64, _>("total_questions");
            (avg_score, total_attempts, total_correct, total_questions)
        }
        None => (0.0, 0, 0, 0),
    }
}

pub async fn get_subject_stats(pool: &MySqlPool, user_id: i64) -> Vec<SubjectScore> {
    let rows = sqlx::query(
        "SELECT s.id AS subject_id, s.name_en AS subject_name,
                COALESCE(AVG(ea.score), 0) AS avg_score,
                COUNT(ea.id) AS attempt_count
         FROM exam_attempts ea
         JOIN exam_versions ev ON ev.id = ea.exam_version_id
         JOIN subjects s ON s.id = ev.subject_id
         WHERE ea.user_id = ? AND ea.status = 'Completed' AND ev.subject_id IS NOT NULL
         GROUP BY s.id, s.name_en
         ORDER BY s.name_en"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| SubjectScore {
            subject_id: r.get("subject_id"),
            subject_name: r.get("subject_name"),
            avg_score: r.get("avg_score"),
            attempt_count: r.get::<i64, _>("attempt_count") as i32,
        })
        .collect()
}

pub async fn get_difficulty_breakdown(pool: &MySqlPool, user_id: i64) -> Vec<DifficultyScore> {
    let rows = sqlx::query(
        "SELECT ev.difficulty,
                COALESCE(AVG(ea.score), 0) AS avg_score,
                COUNT(ea.id) AS attempt_count
         FROM exam_attempts ea
         JOIN exam_versions ev ON ev.id = ea.exam_version_id
         WHERE ea.user_id = ? AND ea.status = 'Completed'
         GROUP BY ev.difficulty
         ORDER BY ev.difficulty"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| DifficultyScore {
            difficulty: r.get("difficulty"),
            avg_score: r.get("avg_score"),
            attempt_count: r.get::<i64, _>("attempt_count") as i32,
        })
        .collect()
}
