use sqlx::{MySqlPool, Row};
use shared::models::{Spu, OptionGroup, OptionValue, Sku};

pub async fn list_spus(pool: &MySqlPool, active_only: bool) -> Vec<Spu> {
    let sql = if active_only {
        "SELECT id, name_en, name_zh, description_en, description_zh, category, image_url,
                CAST(base_price AS DOUBLE) AS base_price, prep_time_minutes, is_active, created_at, updated_at
         FROM spu WHERE is_active = 1 ORDER BY id"
    } else {
        "SELECT id, name_en, name_zh, description_en, description_zh, category, image_url,
                CAST(base_price AS DOUBLE) AS base_price, prep_time_minutes, is_active, created_at, updated_at
         FROM spu ORDER BY id"
    };

    let rows = sqlx::query(sql)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    rows.into_iter()
        .map(|r| Spu {
            id: r.get("id"),
            name_en: r.get("name_en"),
            name_zh: r.get("name_zh"),
            description_en: r.get("description_en"),
            description_zh: r.get("description_zh"),
            category: r.get("category"),
            image_url: r.get("image_url"),
            base_price: r.get("base_price"),
            prep_time_minutes: r.get("prep_time_minutes"),
            is_active: r.get("is_active"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect()
}

pub async fn get_spu(pool: &MySqlPool, id: i64) -> Option<Spu> {
    let row = sqlx::query(
        "SELECT id, name_en, name_zh, description_en, description_zh, category, image_url,
                CAST(base_price AS DOUBLE) AS base_price, prep_time_minutes, is_active, created_at, updated_at
         FROM spu WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(|r| Spu {
        id: r.get("id"),
        name_en: r.get("name_en"),
        name_zh: r.get("name_zh"),
        description_en: r.get("description_en"),
        description_zh: r.get("description_zh"),
        category: r.get("category"),
        image_url: r.get("image_url"),
        base_price: r.get("base_price"),
        prep_time_minutes: r.get("prep_time_minutes"),
        is_active: r.get("is_active"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}

pub async fn get_option_groups(pool: &MySqlPool, spu_id: i64) -> Vec<OptionGroup> {
    let rows = sqlx::query(
        "SELECT id, spu_id, name_en, name_zh, is_required, sort_order
         FROM option_groups WHERE spu_id = ? ORDER BY sort_order"
    )
    .bind(spu_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| OptionGroup {
            id: r.get("id"),
            spu_id: r.get("spu_id"),
            name_en: r.get("name_en"),
            name_zh: r.get("name_zh"),
            is_required: r.get("is_required"),
            sort_order: r.get("sort_order"),
        })
        .collect()
}

pub async fn get_option_values(pool: &MySqlPool, group_id: i64) -> Vec<OptionValue> {
    let rows = sqlx::query(
        "SELECT id, group_id, label_en, label_zh, CAST(price_delta AS DOUBLE) AS price_delta, is_default, sort_order
         FROM option_values WHERE group_id = ? ORDER BY sort_order"
    )
    .bind(group_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| OptionValue {
            id: r.get("id"),
            group_id: r.get("group_id"),
            label_en: r.get("label_en"),
            label_zh: r.get("label_zh"),
            price_delta: r.get("price_delta"),
            is_default: r.get("is_default"),
            sort_order: r.get("sort_order"),
        })
        .collect()
}

pub async fn get_sku_by_options(pool: &MySqlPool, spu_id: i64, option_value_ids: &[i64]) -> Option<Sku> {
    // Find an existing SKU that matches exactly the given option values.
    // Strategy: join sku_option_values, group by sku_id, verify the count matches.
    if option_value_ids.is_empty() {
        // No options selected -- look for a default SKU for this SPU
        let row = sqlx::query(
            "SELECT s.id, s.spu_id, s.sku_code, CAST(s.price AS DOUBLE) AS price, s.stock_quantity, s.is_active
             FROM sku s
             LEFT JOIN sku_option_values sov ON sov.sku_id = s.id
             WHERE s.spu_id = ? AND sov.id IS NULL AND s.is_active = 1
             LIMIT 1"
        )
        .bind(spu_id)
        .fetch_optional(pool)
        .await
        .ok()?;

        return row.map(|r| Sku {
            id: r.get("id"),
            spu_id: r.get("spu_id"),
            sku_code: r.get("sku_code"),
            price: r.get("price"),
            stock_quantity: r.get("stock_quantity"),
            is_active: r.get("is_active"),
        });
    }

    let placeholders: Vec<String> = option_value_ids.iter().map(|_| "?".to_string()).collect();
    let placeholders_str = placeholders.join(",");
    let option_count = option_value_ids.len() as i64;

    let sql = format!(
        "SELECT s.id, s.spu_id, s.sku_code, CAST(s.price AS DOUBLE) AS price, s.stock_quantity, s.is_active
         FROM sku s
         JOIN sku_option_values sov ON sov.sku_id = s.id
         WHERE s.spu_id = ? AND s.is_active = 1
           AND sov.option_value_id IN ({})
         GROUP BY s.id
         HAVING COUNT(DISTINCT sov.option_value_id) = ?
         LIMIT 1",
        placeholders_str
    );

    let mut query = sqlx::query(&sql).bind(spu_id);
    for ov_id in option_value_ids {
        query = query.bind(ov_id);
    }
    query = query.bind(option_count);

    let row = query.fetch_optional(pool).await.ok()?;

    row.map(|r| Sku {
        id: r.get("id"),
        spu_id: r.get("spu_id"),
        sku_code: r.get("sku_code"),
        price: r.get("price"),
        stock_quantity: r.get("stock_quantity"),
        is_active: r.get("is_active"),
    })
}

pub async fn get_sku(pool: &MySqlPool, id: i64) -> Option<Sku> {
    let row = sqlx::query(
        "SELECT id, spu_id, sku_code, CAST(price AS DOUBLE) AS price, stock_quantity, is_active
         FROM sku WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(|r| Sku {
        id: r.get("id"),
        spu_id: r.get("spu_id"),
        sku_code: r.get("sku_code"),
        price: r.get("price"),
        stock_quantity: r.get("stock_quantity"),
        is_active: r.get("is_active"),
    })
}

pub async fn get_option_value_by_id(pool: &MySqlPool, id: i64) -> Option<OptionValue> {
    sqlx::query("SELECT id, group_id, label_en, label_zh, CAST(price_delta AS DOUBLE) AS price_delta, is_default, sort_order FROM option_values WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()?
        .map(|r| OptionValue {
            id: r.get("id"),
            group_id: r.get("group_id"),
            label_en: r.get("label_en"),
            label_zh: r.get("label_zh"),
            price_delta: r.get("price_delta"),
            is_default: r.get("is_default"),
            sort_order: r.get("sort_order"),
        })
}

pub async fn create_spu(pool: &MySqlPool, name_en: &str, name_zh: &str, desc_en: Option<&str>, desc_zh: Option<&str>, category: Option<&str>, base_price: f64, prep_time: i32) -> Result<i64, sqlx::Error> {
    let r = sqlx::query("INSERT INTO spu (name_en, name_zh, description_en, description_zh, category, base_price, prep_time_minutes) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(name_en).bind(name_zh).bind(desc_en).bind(desc_zh).bind(category).bind(base_price).bind(prep_time)
        .execute(pool).await?;
    Ok(r.last_insert_id() as i64)
}

pub async fn update_spu(pool: &MySqlPool, id: i64, name_en: &str, name_zh: &str, desc_en: Option<&str>, desc_zh: Option<&str>, category: Option<&str>, base_price: f64, prep_time: i32) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE spu SET name_en=?, name_zh=?, description_en=?, description_zh=?, category=?, base_price=?, prep_time_minutes=?, updated_at=NOW() WHERE id=?")
        .bind(name_en).bind(name_zh).bind(desc_en).bind(desc_zh).bind(category).bind(base_price).bind(prep_time).bind(id)
        .execute(pool).await?;
    Ok(())
}

pub async fn create_option_group(pool: &MySqlPool, spu_id: i64, name_en: &str, name_zh: &str, is_required: bool, sort_order: i32) -> Result<i64, sqlx::Error> {
    let r = sqlx::query("INSERT INTO option_groups (spu_id, name_en, name_zh, is_required, sort_order) VALUES (?, ?, ?, ?, ?)")
        .bind(spu_id).bind(name_en).bind(name_zh).bind(is_required).bind(sort_order)
        .execute(pool).await?;
    Ok(r.last_insert_id() as i64)
}

pub async fn create_option_value(pool: &MySqlPool, group_id: i64, label_en: &str, label_zh: &str, price_delta: f64, is_default: bool, sort_order: i32) -> Result<i64, sqlx::Error> {
    let r = sqlx::query("INSERT INTO option_values (group_id, label_en, label_zh, price_delta, is_default, sort_order) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(group_id).bind(label_en).bind(label_zh).bind(price_delta).bind(is_default).bind(sort_order)
        .execute(pool).await?;
    Ok(r.last_insert_id() as i64)
}


/// Return `true` if the given `option_value_id` belongs to an option_group
/// that is linked to `spu_id`.  Used to reject cross-product option injection.
pub async fn option_value_belongs_to_spu(pool: &MySqlPool, option_value_id: i64, spu_id: i64) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM option_values ov
         JOIN option_groups og ON og.id = ov.group_id
         WHERE ov.id = ? AND og.spu_id = ?",
    )
    .bind(option_value_id)
    .bind(spu_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0)
        > 0
}
