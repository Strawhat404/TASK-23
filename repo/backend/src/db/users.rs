use sqlx::{MySqlPool, Row};
use shared::models::User;

pub async fn find_by_username(pool: &MySqlPool, username: &str) -> Option<User> {
    let row = sqlx::query(
        "SELECT id, username, password_hash, display_name, email, preferred_locale, created_at, updated_at
         FROM users WHERE username = ?"
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(|r| User {
        id: r.get("id"),
        username: r.get("username"),
        password_hash: r.get("password_hash"),
        display_name: r.get("display_name"),
        email: r.get("email"),
        preferred_locale: r.get("preferred_locale"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}

pub async fn find_by_id(pool: &MySqlPool, id: i64) -> Option<User> {
    let row = sqlx::query(
        "SELECT id, username, password_hash, display_name, email, preferred_locale, created_at, updated_at
         FROM users WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(|r| User {
        id: r.get("id"),
        username: r.get("username"),
        password_hash: r.get("password_hash"),
        display_name: r.get("display_name"),
        email: r.get("email"),
        preferred_locale: r.get("preferred_locale"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}

pub async fn create_user(
    pool: &MySqlPool,
    username: &str,
    password_hash: &str,
    display_name: Option<&str>,
    email: Option<&str>,
    locale: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO users (username, password_hash, display_name, email, preferred_locale)
         VALUES (?, ?, ?, ?, ?)"
    )
    .bind(username)
    .bind(password_hash)
    .bind(display_name)
    .bind(email)
    .bind(locale)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

pub async fn get_user_roles(pool: &MySqlPool, user_id: i64) -> Vec<String> {
    let rows = sqlx::query(
        "SELECT r.name FROM user_roles ur
         JOIN roles r ON r.id = ur.role_id
         WHERE ur.user_id = ?"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.iter().map(|r| r.get("name")).collect()
}

pub async fn assign_role(pool: &MySqlPool, user_id: i64, name: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id)
         SELECT ?, id FROM roles WHERE name = ?
         ON DUPLICATE KEY UPDATE user_id = user_id"
    )
    .bind(user_id)
    .bind(name)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_locale(pool: &MySqlPool, user_id: i64, locale: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET preferred_locale = ? WHERE id = ?")
        .bind(locale)
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn list_all_users(pool: &MySqlPool) -> Vec<shared::models::User> {
    sqlx::query("SELECT id, username, password_hash, display_name, email, preferred_locale, created_at, updated_at FROM users ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| shared::models::User {
            id: r.get("id"),
            username: r.get("username"),
            password_hash: r.get("password_hash"),
            display_name: r.get("display_name"),
            email: r.get("email"),
            preferred_locale: r.get("preferred_locale"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect()
}

pub async fn remove_role(pool: &MySqlPool, user_id: i64, role_name: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE ur FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = ? AND r.name = ?")
        .bind(user_id)
        .bind(role_name)
        .execute(pool)
        .await?;
    Ok(())
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

    #[test]
    fn user_struct_construction() {
        let u = User {
            id: 1,
            username: "alice".to_string(),
            password_hash: "$2b$12$stub".to_string(),
            display_name: Some("Alice".to_string()),
            email: Some("alice@example.com".to_string()),
            preferred_locale: "en".to_string(),
            created_at: sample_dt(),
            updated_at: None,
        };
        assert_eq!(u.id, 1);
        assert_eq!(u.username, "alice");
        assert_eq!(u.preferred_locale, "en");
    }

    #[test]
    fn user_struct_optional_fields_none() {
        let u = User {
            id: 2,
            username: "bob".to_string(),
            password_hash: "hash".to_string(),
            display_name: None,
            email: None,
            preferred_locale: "zh".to_string(),
            created_at: sample_dt(),
            updated_at: None,
        };
        assert!(u.display_name.is_none());
        assert!(u.email.is_none());
        assert!(u.updated_at.is_none());
    }

    #[test]
    fn user_struct_serde_round_trip() {
        let u = User {
            id: 3,
            username: "carol".to_string(),
            password_hash: "hashed_pw".to_string(),
            display_name: Some("Carol C.".to_string()),
            email: Some("carol@test.com".to_string()),
            preferred_locale: "en".to_string(),
            created_at: sample_dt(),
            updated_at: Some(sample_dt()),
        };
        let json = serde_json::to_string(&u).unwrap();
        let back: User = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 3);
        assert_eq!(back.username, "carol");
        assert!(back.updated_at.is_some());
    }

    #[test]
    fn user_struct_clone() {
        let u = User {
            id: 4,
            username: "dave".to_string(),
            password_hash: "pw".to_string(),
            display_name: None,
            email: None,
            preferred_locale: "en".to_string(),
            created_at: sample_dt(),
            updated_at: None,
        };
        let cloned = u.clone();
        assert_eq!(cloned.id, u.id);
        assert_eq!(cloned.username, u.username);
    }

    #[test]
    fn user_import_from_shared_works() {
        // Verify that the `use shared::models::User` import compiles and
        // the type is indeed shared::models::User.
        let u = User {
            id: 0,
            username: String::new(),
            password_hash: String::new(),
            display_name: None,
            email: None,
            preferred_locale: "en".to_string(),
            created_at: sample_dt(),
            updated_at: None,
        };
        let _debug = format!("{:?}", u);
        assert!(true);
    }
}
