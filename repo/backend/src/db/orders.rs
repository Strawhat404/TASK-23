use sqlx::{MySqlPool, Row};
use shared::models::{Order, FulfillmentEvent};

pub async fn create_order(
    pool: &MySqlPool,
    user_id: i64,
    reservation_id: Option<i64>,
    order_number: &str,
    subtotal: f64,
    tax_amount: f64,
    total: f64,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO orders (user_id, reservation_id, order_number, subtotal, tax_amount, total, status)
         VALUES (?, ?, ?, ?, ?, ?, 'Pending')"
    )
    .bind(user_id)
    .bind(reservation_id)
    .bind(order_number)
    .bind(subtotal)
    .bind(tax_amount)
    .bind(total)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

pub async fn create_order_item(
    pool: &MySqlPool,
    order_id: i64,
    sku_id: i64,
    quantity: i32,
    unit_price: f64,
    item_total: f64,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO order_items (order_id, sku_id, quantity, unit_price, item_total)
         VALUES (?, ?, ?, ?, ?)"
    )
    .bind(order_id)
    .bind(sku_id)
    .bind(quantity)
    .bind(unit_price)
    .bind(item_total)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

pub async fn create_order_item_option(
    pool: &MySqlPool,
    order_item_id: i64,
    option_value_id: i64,
    option_label: &str,
    price_delta: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO order_item_options (order_item_id, option_value_id, option_label, price_delta)
         VALUES (?, ?, ?, ?)"
    )
    .bind(order_item_id)
    .bind(option_value_id)
    .bind(option_label)
    .bind(price_delta)
    .execute(pool)
    .await?;

    Ok(())
}

fn row_to_order(r: sqlx::mysql::MySqlRow) -> Order {
    Order {
        id: r.get("id"),
        user_id: r.get("user_id"),
        reservation_id: r.get("reservation_id"),
        order_number: r.get("order_number"),
        subtotal: r.get("subtotal"),
        tax_amount: r.get("tax_amount"),
        total: r.get("total"),
        status: r.get("status"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

pub async fn get_order(pool: &MySqlPool, id: i64) -> Option<Order> {
    let row = sqlx::query(
        "SELECT id, user_id, reservation_id, order_number, CAST(subtotal AS DOUBLE) AS subtotal, CAST(tax_amount AS DOUBLE) AS tax_amount, CAST(total AS DOUBLE) AS total, status, created_at, updated_at
         FROM orders WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(row_to_order)
}

pub async fn get_order_by_number(pool: &MySqlPool, order_number: &str) -> Option<Order> {
    let row = sqlx::query(
        "SELECT id, user_id, reservation_id, order_number, CAST(subtotal AS DOUBLE) AS subtotal, CAST(tax_amount AS DOUBLE) AS tax_amount, CAST(total AS DOUBLE) AS total, status, created_at, updated_at
         FROM orders WHERE order_number = ?"
    )
    .bind(order_number)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(row_to_order)
}

pub async fn get_user_orders(pool: &MySqlPool, user_id: i64) -> Vec<Order> {
    let rows = sqlx::query(
        "SELECT id, user_id, reservation_id, order_number, CAST(subtotal AS DOUBLE) AS subtotal, CAST(tax_amount AS DOUBLE) AS tax_amount, CAST(total AS DOUBLE) AS total, status, created_at, updated_at
         FROM orders WHERE user_id = ? ORDER BY created_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter().map(row_to_order).collect()
}

/// Represents an order item joined with product info.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OrderItemRow {
    pub id: i64,
    pub order_id: i64,
    pub sku_id: i64,
    pub sku_code: String,
    pub spu_name: String,
    pub quantity: i32,
    pub unit_price: f64,
    pub item_total: f64,
    pub options: Vec<String>,
}

pub async fn get_order_items(pool: &MySqlPool, order_id: i64) -> Vec<OrderItemRow> {
    let rows = sqlx::query(
        "SELECT oi.id, oi.order_id, oi.sku_id, oi.quantity, CAST(oi.unit_price AS DOUBLE) AS unit_price, CAST(oi.item_total AS DOUBLE) AS item_total,
                sk.sku_code, sp.name_en AS spu_name
         FROM order_items oi
         JOIN sku sk ON sk.id = oi.sku_id
         JOIN spu sp ON sp.id = sk.spu_id
         WHERE oi.order_id = ?
         ORDER BY oi.id"
    )
    .bind(order_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut items = Vec::new();
    for r in rows {
        let item_id: i64 = r.get("id");

        let opt_rows = sqlx::query(
            "SELECT option_label FROM order_item_options WHERE order_item_id = ?"
        )
        .bind(item_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let options: Vec<String> = opt_rows.iter().map(|or| or.get("option_label")).collect();

        items.push(OrderItemRow {
            id: item_id,
            order_id: r.get("order_id"),
            sku_id: r.get("sku_id"),
            sku_code: r.get("sku_code"),
            spu_name: r.get("spu_name"),
            quantity: r.get("quantity"),
            unit_price: r.get("unit_price"),
            item_total: r.get("item_total"),
            options,
        });
    }

    items
}

pub async fn update_order_status(
    pool: &MySqlPool,
    order_id: i64,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE orders SET status = ?, updated_at = NOW() WHERE id = ?")
        .bind(status)
        .bind(order_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn create_fulfillment_event(
    pool: &MySqlPool,
    order_id: i64,
    from_status: &str,
    to_status: &str,
    changed_by: i64,
    notes: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO fulfillment_events (order_id, from_status, to_status, changed_by_user_id, notes)
         VALUES (?, ?, ?, ?, ?)"
    )
    .bind(order_id)
    .bind(from_status)
    .bind(to_status)
    .bind(changed_by)
    .bind(notes)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_all_orders(pool: &MySqlPool, status_filter: Option<&str>) -> Vec<Order> {
    let (query, needs_bind) = if status_filter.is_some() {
        ("SELECT id, user_id, reservation_id, order_number, CAST(subtotal AS DOUBLE) AS subtotal, CAST(tax_amount AS DOUBLE) AS tax_amount, CAST(total AS DOUBLE) AS total, status, created_at, updated_at FROM orders WHERE status = ? ORDER BY created_at DESC", true)
    } else {
        ("SELECT id, user_id, reservation_id, order_number, CAST(subtotal AS DOUBLE) AS subtotal, CAST(tax_amount AS DOUBLE) AS tax_amount, CAST(total AS DOUBLE) AS total, status, created_at, updated_at FROM orders ORDER BY created_at DESC", false)
    };
    let mut q = sqlx::query(query);
    if needs_bind {
        q = q.bind(status_filter.unwrap());
    }
    q.fetch_all(pool).await.unwrap_or_default().into_iter().map(row_to_order).collect()
}

/// Execute the entire checkout write path inside a single MySQL transaction.
///
/// Writes: reservation → order → order_items → voucher → cart clear.
/// Either all succeed and are committed, or the transaction rolls back on any error.
///
/// `voucher_code` is the plaintext code (hashed internally before storing in
/// reservations); `encrypted_code` is the AES-GCM ciphertext for the vouchers table.
/// Returns `(reservation_id, order_id)`.
pub async fn create_checkout(
    pool: &MySqlPool,
    user_id: i64,
    slot_start: chrono::NaiveDateTime,
    slot_end: chrono::NaiveDateTime,
    voucher_code: &str,
    encrypted_code: &str,
    hold_expires_at: chrono::NaiveDateTime,
    order_number: &str,
    subtotal: f64,
    tax_amount: f64,
    total: f64,
    cart_items: &[(i64, i32, f64)],  // (sku_id, quantity, unit_price)
    cart_id: i64,
) -> Result<(i64, i64), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // 1. Reservation — store SHA-256 hash of the voucher code at rest.
    let hashed = crate::db::store::sha256_hex(voucher_code);
    let res = sqlx::query(
        "INSERT INTO reservations (user_id, pickup_slot_start, pickup_slot_end, voucher_code, hold_expires_at, status)
         VALUES (?, ?, ?, ?, ?, 'Held')"
    )
    .bind(user_id).bind(slot_start).bind(slot_end).bind(&hashed).bind(hold_expires_at)
    .execute(&mut *tx).await?;
    let reservation_id = res.last_insert_id() as i64;

    // 2. Order
    let res = sqlx::query(
        "INSERT INTO orders (user_id, reservation_id, order_number, subtotal, tax_amount, total, status)
         VALUES (?, ?, ?, ?, ?, ?, 'Pending')"
    )
    .bind(user_id).bind(reservation_id).bind(order_number)
    .bind(subtotal).bind(tax_amount).bind(total)
    .execute(&mut *tx).await?;
    let order_id = res.last_insert_id() as i64;

    // 3. Order items
    for &(sku_id, quantity, unit_price) in cart_items {
        let item_total = unit_price * quantity as f64;
        sqlx::query(
            "INSERT INTO order_items (order_id, sku_id, quantity, unit_price, item_total)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(order_id).bind(sku_id).bind(quantity).bind(unit_price).bind(item_total)
        .execute(&mut *tx).await?;
    }

    // 4. Voucher — hash for lookup, encrypted for display recovery
    let voucher_hash = crate::db::store::sha256_hex(voucher_code);
    sqlx::query(
        "INSERT INTO vouchers (reservation_id, order_id, code, encrypted_code) VALUES (?, ?, ?, ?)"
    )
    .bind(reservation_id).bind(order_id).bind(&voucher_hash).bind(encrypted_code)
    .execute(&mut *tx).await?;

    // 5. Clear cart
    sqlx::query(
        "DELETE cio FROM cart_item_options cio
         JOIN cart_items ci ON ci.id = cio.cart_item_id
         WHERE ci.cart_id = ?"
    )
    .bind(cart_id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM cart_items WHERE cart_id = ?")
        .bind(cart_id).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok((reservation_id, order_id))
}

pub async fn count_orders_by_status(pool: &MySqlPool) -> Vec<(String, i64)> {
    sqlx::query("SELECT status, COUNT(*) as cnt FROM orders GROUP BY status")
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| (r.get::<String, _>("status"), r.get::<i64, _>("cnt")))
        .collect()
}

pub async fn get_fulfillment_events(pool: &MySqlPool, order_id: i64) -> Vec<FulfillmentEvent> {
    let rows = sqlx::query(
        "SELECT id, order_id, from_status, to_status, changed_by_user_id, notes, created_at
         FROM fulfillment_events WHERE order_id = ? ORDER BY created_at"
    )
    .bind(order_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| FulfillmentEvent {
            id: r.get("id"),
            order_id: r.get("order_id"),
            from_status: r.get("from_status"),
            to_status: r.get("to_status"),
            changed_by_user_id: r.get("changed_by_user_id"),
            notes: r.get("notes"),
            created_at: r.get("created_at"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_item_row_construction() {
        let item = OrderItemRow {
            id: 1,
            order_id: 10,
            sku_id: 100,
            sku_code: "SKU-LATTE-SM".to_string(),
            spu_name: "Latte".to_string(),
            quantity: 2,
            unit_price: 4.50,
            item_total: 9.00,
            options: vec!["Small".to_string(), "Oat Milk".to_string()],
        };
        assert_eq!(item.id, 1);
        assert_eq!(item.order_id, 10);
        assert_eq!(item.sku_id, 100);
        assert_eq!(item.quantity, 2);
    }

    #[test]
    fn order_item_row_total_matches_unit_price_times_quantity() {
        let quantity = 3;
        let unit_price = 5.25;
        let item_total = unit_price * quantity as f64;
        let item = OrderItemRow {
            id: 1,
            order_id: 1,
            sku_id: 1,
            sku_code: "SKU-1".to_string(),
            spu_name: "Espresso".to_string(),
            quantity,
            unit_price,
            item_total,
            options: vec![],
        };
        assert!((item.item_total - (item.unit_price * item.quantity as f64)).abs() < 1e-9);
    }

    #[test]
    fn order_item_row_single_quantity() {
        let item = OrderItemRow {
            id: 2,
            order_id: 5,
            sku_id: 20,
            sku_code: "SKU-20".to_string(),
            spu_name: "Mocha".to_string(),
            quantity: 1,
            unit_price: 6.00,
            item_total: 6.00,
            options: vec!["Large".to_string()],
        };
        assert!((item.item_total - item.unit_price).abs() < 1e-9);
    }

    #[test]
    fn order_item_row_serializes_to_json() {
        let item = OrderItemRow {
            id: 1,
            order_id: 2,
            sku_id: 3,
            sku_code: "SKU-3".to_string(),
            spu_name: "Americano".to_string(),
            quantity: 1,
            unit_price: 3.50,
            item_total: 3.50,
            options: vec!["Hot".to_string()],
        };
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["sku_code"], "SKU-3");
        assert_eq!(json["spu_name"], "Americano");
        assert_eq!(json["item_total"], 3.50);
        assert_eq!(json["options"][0], "Hot");
    }

    #[test]
    fn order_item_row_empty_options() {
        let item = OrderItemRow {
            id: 1,
            order_id: 1,
            sku_id: 1,
            sku_code: "S".to_string(),
            spu_name: "T".to_string(),
            quantity: 1,
            unit_price: 1.0,
            item_total: 1.0,
            options: vec![],
        };
        assert!(item.options.is_empty());
    }

    #[test]
    fn order_item_row_clone_preserves_values() {
        let item = OrderItemRow {
            id: 7,
            order_id: 3,
            sku_id: 15,
            sku_code: "SKU-15".to_string(),
            spu_name: "Tea".to_string(),
            quantity: 4,
            unit_price: 2.00,
            item_total: 8.00,
            options: vec!["Iced".to_string()],
        };
        let cloned = item.clone();
        assert_eq!(cloned.id, item.id);
        assert_eq!(cloned.item_total, item.item_total);
        assert_eq!(cloned.options, item.options);
    }
}
