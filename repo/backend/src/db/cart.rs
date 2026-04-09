use sqlx::{MySqlPool, Row};

/// Returns the cart id for the user, creating one if it doesn't exist.
pub async fn get_or_create_cart(pool: &MySqlPool, user_id: i64) -> Result<i64, sqlx::Error> {
    let existing = sqlx::query("SELECT id FROM carts WHERE user_id = ? LIMIT 1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = existing {
        Ok(row.get("id"))
    } else {
        let result = sqlx::query("INSERT INTO carts (user_id) VALUES (?)")
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(result.last_insert_id() as i64)
    }
}

pub async fn add_item(
    pool: &MySqlPool,
    cart_id: i64,
    sku_id: i64,
    quantity: i32,
    unit_price: f64,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO cart_items (cart_id, sku_id, quantity, unit_price) VALUES (?, ?, ?, ?)"
    )
    .bind(cart_id)
    .bind(sku_id)
    .bind(quantity)
    .bind(unit_price)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

pub async fn add_item_options(
    pool: &MySqlPool,
    cart_item_id: i64,
    option_value_ids: &[i64],
) -> Result<(), sqlx::Error> {
    for ov_id in option_value_ids {
        sqlx::query(
            "INSERT INTO cart_item_options (cart_item_id, option_value_id) VALUES (?, ?)"
        )
        .bind(cart_item_id)
        .bind(ov_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Represents a cart item row joined with product information.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CartItemRow {
    pub id: i64,
    pub cart_id: i64,
    pub sku_id: i64,
    pub quantity: i32,
    pub unit_price: f64,
    pub sku_code: String,
    pub spu_name_en: String,
    pub spu_name_zh: String,
    pub option_labels: Vec<String>,
}

pub async fn get_cart_items(pool: &MySqlPool, cart_id: i64) -> Vec<CartItemRow> {
    let rows = sqlx::query(
        "SELECT ci.id, ci.cart_id, ci.sku_id, ci.quantity, CAST(ci.unit_price AS DOUBLE) AS unit_price,
                sk.sku_code, sp.name_en AS spu_name_en, sp.name_zh AS spu_name_zh
         FROM cart_items ci
         JOIN sku sk ON sk.id = ci.sku_id
         JOIN spu sp ON sp.id = sk.spu_id
         WHERE ci.cart_id = ?
         ORDER BY ci.id"
    )
    .bind(cart_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut items = Vec::new();
    for r in rows {
        let item_id: i64 = r.get("id");

        // Fetch option labels for this cart item
        let opt_rows = sqlx::query(
            "SELECT ov.label_en FROM cart_item_options cio
             JOIN option_values ov ON ov.id = cio.option_value_id
             WHERE cio.cart_item_id = ?
             ORDER BY ov.sort_order"
        )
        .bind(item_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let option_labels: Vec<String> = opt_rows.iter().map(|or| or.get("label_en")).collect();

        items.push(CartItemRow {
            id: item_id,
            cart_id: r.get("cart_id"),
            sku_id: r.get("sku_id"),
            quantity: r.get("quantity"),
            unit_price: r.get("unit_price"),
            sku_code: r.get("sku_code"),
            spu_name_en: r.get("spu_name_en"),
            spu_name_zh: r.get("spu_name_zh"),
            option_labels,
        });
    }

    items
}

pub async fn update_item_quantity(
    pool: &MySqlPool,
    item_id: i64,
    quantity: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE cart_items SET quantity = ? WHERE id = ?")
        .bind(quantity)
        .bind(item_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn remove_item(pool: &MySqlPool, item_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM cart_item_options WHERE cart_item_id = ?")
        .bind(item_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM cart_items WHERE id = ?")
        .bind(item_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn clear_cart(pool: &MySqlPool, cart_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE cio FROM cart_item_options cio
         JOIN cart_items ci ON ci.id = cio.cart_item_id
         WHERE ci.cart_id = ?"
    )
    .bind(cart_id)
    .execute(pool)
    .await?;

    sqlx::query("DELETE FROM cart_items WHERE cart_id = ?")
        .bind(cart_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Return the maximum `prep_time_minutes` across all SPUs in the cart.
/// Returns `None` if the cart is empty.
pub async fn get_max_prep_time_for_cart(pool: &MySqlPool, cart_id: i64) -> Option<i32> {
    let row = sqlx::query(
        "SELECT MAX(sp.prep_time_minutes) AS max_prep
         FROM cart_items ci
         JOIN sku sk ON sk.id = ci.sku_id
         JOIN spu sp ON sp.id = sk.spu_id
         WHERE ci.cart_id = ?",
    )
    .bind(cart_id)
    .fetch_optional(pool)
    .await
    .ok()??;

    row.get("max_prep")
}
