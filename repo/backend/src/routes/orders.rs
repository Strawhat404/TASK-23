use rocket::{get, post, routes};
use rocket::serde::json::Json;
use rocket::http::Status;
use rocket::State;
use sqlx::MySqlPool;

use shared::dto::{
    ApiResponse, CheckoutRequest, CheckoutResponse, OrderDetail, OrderSummary,
    OrderItemDetail, FulfillmentEventDetail, ReservationDetail,
};
use crate::services::crypto::CryptoConfig;
use crate::middleware::auth_guard::AuthenticatedUser;
use crate::services::reservation_lock::{
    self, ReservationLockManager, LockError,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Fetch option_value_ids for a single cart item.
async fn get_cart_item_option_ids(pool: &MySqlPool, cart_item_id: i64) -> Vec<i64> {
    let rows = sqlx::query_scalar::<_, i64>(
        "SELECT option_value_id FROM cart_item_options WHERE cart_item_id = ? ORDER BY option_value_id",
    )
    .bind(cart_item_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

#[post("/checkout", data = "<body>")]
pub async fn checkout(
    pool: &State<MySqlPool>,
    lock_mgr: &State<ReservationLockManager>,
    crypto: &State<crate::services::crypto::CryptoConfig>,
    user: AuthenticatedUser,
    body: Json<CheckoutRequest>,
) -> Result<Json<ApiResponse<CheckoutResponse>>, (Status, Json<ApiResponse<()>>)> {
    let user_id = user.claims.sub;

    // Parse pickup slot times
    let slot_start = chrono::NaiveDateTime::parse_from_str(&body.pickup_slot_start, "%Y-%m-%dT%H:%M:%S")
        .map_err(|_| {
            (
                Status::BadRequest,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pickup_slot_start format".into()),
                }),
            )
        })?;

    let slot_end = chrono::NaiveDateTime::parse_from_str(&body.pickup_slot_end, "%Y-%m-%dT%H:%M:%S")
        .map_err(|_| {
            (
                Status::BadRequest,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pickup_slot_end format".into()),
                }),
            )
        })?;

    // Derive the required prep time from the items actually in the cart.
    let cart_id_for_prep = crate::db::cart::get_or_create_cart(pool.inner(), user_id)
        .await
        .ok();
    let prep_minutes = if let Some(cid) = cart_id_for_prep {
        crate::db::cart::get_max_prep_time_for_cart(pool.inner(), cid)
            .await
            .unwrap_or(15)
    } else {
        15
    };

    // Check slot availability
    if !crate::services::pickup::is_slot_available(slot_start, prep_minutes) {
        return Err((
            Status::Conflict,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Selected pickup slot is no longer available".into()),
            }),
        ));
    }

    // Generate voucher code
    let voucher_code = crate::services::pickup::generate_voucher_code();

    // Get cart items for computing order
    let cart_id = crate::db::cart::get_or_create_cart(pool.inner(), user_id)
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to get cart: {}", e)),
                }),
            )
        })?;

    let cart_items = crate::db::cart::get_cart_items(pool.inner(), cart_id).await;

    if cart_items.is_empty() {
        return Err((
            Status::BadRequest,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Cart is empty".into()),
            }),
        ));
    }

    // -----------------------------------------------------------------------
    // Acquire concurrency locks for each cart item's SKU + options
    // -----------------------------------------------------------------------
    let hold_secs: u64 = 600; // 10 minutes
    let mut acquired_keys: Vec<String> = Vec::new();

    for ci in &cart_items {
        let option_ids = get_cart_item_option_ids(pool.inner(), ci.id).await;
        let key = reservation_lock::lock_key_for_sku_options(ci.sku_id, &option_ids);

        match reservation_lock::try_acquire(
            pool.inner(),
            lock_mgr.inner(),
            &key,
            ci.sku_id,
            user_id,
            ci.quantity,
            hold_secs,
        )
        .await
        {
            Ok(_handle) => {
                acquired_keys.push(key);
            }
            Err(lock_err) => {
                // Roll back all previously acquired locks
                for prev_key in &acquired_keys {
                    let _ = reservation_lock::release(
                        pool.inner(),
                        lock_mgr.inner(),
                        prev_key,
                    )
                    .await;
                }

                let (status, msg) = match &lock_err {
                    LockError::AlreadyLocked { .. } => {
                        (Status::Conflict, format!("Item locked by another user: {}", lock_err))
                    }
                    LockError::StockExhausted => {
                        (Status::Conflict, format!("Out of stock: {}", lock_err))
                    }
                    LockError::DatabaseError(_) => {
                        (Status::InternalServerError, format!("Lock error: {}", lock_err))
                    }
                };

                return Err((
                    status,
                    Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(msg),
                    }),
                ));
            }
        }
    }

    // Compute pricing
    let tax_config = crate::db::store::get_tax_config(pool.inner()).await;
    let tax_rate = tax_config.map(|t| t.rate).unwrap_or(0.0);

    let line_items: Vec<(f64, i32)> = cart_items.iter().map(|ci| (ci.unit_price, ci.quantity)).collect();
    let breakdown = crate::services::pricing::compute_breakdown(&line_items, tax_rate);

    let order_number = format!("ORD-{}", chrono::Utc::now().timestamp_millis());
    let hold_expires_at = chrono::Utc::now().naive_utc() + chrono::Duration::minutes(10);
    let encrypted_code = crate::services::crypto::encrypt(crypto.inner(), &voucher_code);

    // Flatten cart items to (sku_id, quantity, unit_price) for the transactional write.
    let items_tuple: Vec<(i64, i32, f64)> = cart_items
        .iter()
        .map(|ci| (ci.sku_id, ci.quantity, ci.unit_price))
        .collect();

    // All writes (reservation, order, order_items, voucher, cart clear) execute atomically.
    let (reservation_id, order_id) = crate::db::orders::create_checkout(
        pool.inner(),
        user_id,
        slot_start,
        slot_end,
        &voucher_code,
        &encrypted_code,
        hold_expires_at,
        &order_number,
        breakdown.subtotal,
        breakdown.tax_amount,
        breakdown.total,
        &items_tuple,
        cart_id,
    )
    .await
    .map_err(|e| {
        // Transaction rolled back internally; release the concurrency locks.
        let keys = acquired_keys.clone();
        let pool_c = pool.inner().clone();
        let mgr_c = lock_mgr.inner().clone();
        tokio::spawn(async move {
            for k in &keys {
                let _ = reservation_lock::release(&pool_c, &mgr_c, k).await;
            }
        });
        (
            Status::InternalServerError,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Checkout failed: {}", e)),
            }),
        )
    })?;

    // Associate each acquired lock with this reservation so that confirm_order
    // can consume them without restoring stock.
    for key in &acquired_keys {
        let _ = reservation_lock::associate_reservation(pool.inner(), key, reservation_id).await;
    }

    Ok(Json(ApiResponse {
        success: true,
        data: Some(CheckoutResponse {
            order_id,
            order_number,
            voucher_code,
            hold_expires_at: hold_expires_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
            pickup_slot: format!("{} - {}", slot_start, slot_end),
            total: breakdown.total,
        }),
        error: None,
    }))
}

#[get("/")]
pub async fn list_orders(
    pool: &State<MySqlPool>,
    crypto: &State<CryptoConfig>,
    user: AuthenticatedUser,
) -> Json<ApiResponse<Vec<OrderSummary>>> {
    let orders = crate::db::orders::get_user_orders(pool.inner(), user.claims.sub).await;

    let mut summaries: Vec<OrderSummary> = Vec::with_capacity(orders.len());
    for o in orders {
        let (voucher_code, pickup_slot) = if let Some(res_id) = o.reservation_id {
            match crate::db::store::get_reservation(pool.inner(), res_id).await {
                Some(r) => {
                    let slot = format!(
                        "{} - {}",
                        r.pickup_slot_start.format("%Y-%m-%dT%H:%M:%S"),
                        r.pickup_slot_end.format("%Y-%m-%dT%H:%M:%S"),
                    );
                    // reservations.voucher_code is now a SHA-256 hash; recover the
                    // display value by decrypting from the vouchers table.
                    let display_code = crate::db::store::get_encrypted_code_by_reservation_id(
                        pool.inner(), res_id,
                    )
                    .await
                    .and_then(|enc| crate::services::crypto::decrypt(crypto.inner(), &enc).ok());
                    if let Some(ref vc) = display_code {
                        tracing::info!(
                            order_id = o.id,
                            voucher = %crate::services::crypto::mask_for_log(vc, 4),
                            "Order listed with voucher"
                        );
                    }
                    (display_code, Some(slot))
                }
                None => (None, None),
            }
        } else {
            (None, None)
        };

        summaries.push(OrderSummary {
            id: o.id,
            order_number: o.order_number,
            status: o.status,
            total: o.total,
            voucher_code,
            created_at: o.created_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
            pickup_slot,
        });
    }

    Json(ApiResponse {
        success: true,
        data: Some(summaries),
        error: None,
    })
}

#[get("/<id>")]
pub async fn get_order(
    pool: &State<MySqlPool>,
    crypto: &State<CryptoConfig>,
    user: AuthenticatedUser,
    id: i64,
) -> Result<Json<ApiResponse<OrderDetail>>, (Status, Json<ApiResponse<()>>)> {
    let order = crate::db::orders::get_order(pool.inner(), id)
        .await
        .ok_or_else(|| {
            (
                Status::NotFound,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Order not found".into()),
                }),
            )
        })?;

    // Verify the order belongs to this user
    if order.user_id != user.claims.sub {
        return Err((
            Status::Forbidden,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Order does not belong to you".into()),
            }),
        ));
    }

    let items_rows = crate::db::orders::get_order_items(pool.inner(), order.id).await;
    let items: Vec<OrderItemDetail> = items_rows
        .into_iter()
        .map(|i| OrderItemDetail {
            sku_code: i.sku_code,
            spu_name: i.spu_name,
            options: i.options,
            quantity: i.quantity,
            unit_price: i.unit_price,
            item_total: i.item_total,
        })
        .collect();

    let events = crate::db::orders::get_fulfillment_events(pool.inner(), order.id).await;
    let fulfillment_history: Vec<FulfillmentEventDetail> = events
        .into_iter()
        .map(|e| FulfillmentEventDetail {
            from_status: e.from_status,
            to_status: e.to_status,
            changed_by: e.changed_by_user_id.to_string(),
            notes: e.notes,
            timestamp: e.created_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
        })
        .collect();

    // Get reservation if present; decrypt the voucher code for display.
    let reservation = if let Some(res_id) = order.reservation_id {
        match crate::db::store::get_reservation(pool.inner(), res_id).await {
            Some(r) => {
                let display_code = crate::db::store::get_encrypted_code_by_reservation_id(
                    pool.inner(), res_id,
                )
                .await
                .and_then(|enc| crate::services::crypto::decrypt(crypto.inner(), &enc).ok())
                .unwrap_or_default();
                if !display_code.is_empty() {
                    tracing::info!(
                        order_id = order.id,
                        voucher = %crate::services::crypto::mask_for_log(&display_code, 4),
                        "Order detail viewed with voucher"
                    );
                }
                Some(ReservationDetail {
                    voucher_code: display_code,
                    pickup_slot_start: r.pickup_slot_start.format("%Y-%m-%dT%H:%M:%S").to_string(),
                    pickup_slot_end: r.pickup_slot_end.format("%Y-%m-%dT%H:%M:%S").to_string(),
                    hold_expires_at: r.hold_expires_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
                    status: r.status,
                })
            }
            None => None,
        }
    } else {
        None
    };

    let (vc, ps) = if let Some(ref res) = reservation {
        (
            Some(res.voucher_code.clone()),
            Some(format!("{} - {}", res.pickup_slot_start, res.pickup_slot_end)),
        )
    } else {
        (None, None)
    };

    let summary = OrderSummary {
        id: order.id,
        order_number: order.order_number,
        status: order.status,
        total: order.total,
        voucher_code: vc,
        created_at: order.created_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
        pickup_slot: ps,
    };

    Ok(Json(ApiResponse {
        success: true,
        data: Some(OrderDetail {
            order: summary,
            items,
            fulfillment_history,
            reservation,
        }),
        error: None,
    }))
}

#[post("/<id>/confirm")]
pub async fn confirm_order(
    pool: &State<MySqlPool>,
    lock_mgr: &State<ReservationLockManager>,
    user: AuthenticatedUser,
    id: i64,
) -> Result<Json<ApiResponse<()>>, (Status, Json<ApiResponse<()>>)> {
    let order = crate::db::orders::get_order(pool.inner(), id)
        .await
        .ok_or_else(|| {
            (
                Status::NotFound,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Order not found".into()),
                }),
            )
        })?;

    if order.user_id != user.claims.sub {
        return Err((
            Status::Forbidden,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Order does not belong to you".into()),
            }),
        ));
    }

    // Confirm the reservation — enforce hold-status and expiry before updating.
    if let Some(res_id) = order.reservation_id {
        let reservation = crate::db::store::get_reservation(pool.inner(), res_id)
            .await
            .ok_or_else(|| {
                (
                    Status::NotFound,
                    Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Reservation not found".into()),
                    }),
                )
            })?;

        if reservation.status != "Held" {
            return Err((
                Status::Conflict,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!(
                        "Reservation cannot be confirmed: status is '{}' (expected 'Held')",
                        reservation.status
                    )),
                }),
            ));
        }

        let now = chrono::Utc::now().naive_utc();
        if reservation.hold_expires_at < now {
            let _ = crate::db::store::update_reservation_status(pool.inner(), res_id, "Expired").await;
            return Err((
                Status::Conflict,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Reservation hold has expired; please place a new order".into()),
                }),
            ));
        }

        crate::db::store::update_reservation_status(pool.inner(), res_id, "Confirmed")
            .await
            .map_err(|e| {
                (
                    Status::BadRequest,
                    Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to confirm: {}", e)),
                    }),
                )
            })?;

        // Consume the reservation locks so the background expiry job does NOT
        // restore inventory for items that have now been sold.
        let _ = reservation_lock::consume_by_reservation_id(
            pool.inner(),
            lock_mgr.inner(),
            res_id,
        )
        .await;
    }

    Ok(Json(ApiResponse {
        success: true,
        data: None,
        error: None,
    }))
}

#[post("/<id>/cancel")]
pub async fn cancel_order(
    pool: &State<MySqlPool>,
    user: AuthenticatedUser,
    id: i64,
) -> Result<Json<ApiResponse<()>>, (Status, Json<ApiResponse<()>>)> {
    let order = crate::db::orders::get_order(pool.inner(), id)
        .await
        .ok_or_else(|| {
            (
                Status::NotFound,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Order not found".into()),
                }),
            )
        })?;

    if order.user_id != user.claims.sub {
        return Err((
            Status::Forbidden,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Order does not belong to you".into()),
            }),
        ));
    }

    // Check cancel permission using roles from JWT claims
    let roles = &user.claims.roles;
    if !crate::services::fulfillment::validate_transition(&order.status, "Canceled", roles) {
        let msg = crate::services::fulfillment::cancel_error_message(&order.status, roles);
        let status_code = if order.status == "Ready" {
            Status::Forbidden
        } else {
            Status::BadRequest
        };
        return Err((
            status_code,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(msg),
            }),
        ));
    }

    crate::db::orders::update_order_status(pool.inner(), id, "Canceled")
        .await
        .map_err(|e| {
            (
                Status::BadRequest,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to cancel order: {}", e)),
                }),
            )
        })?;

    Ok(Json(ApiResponse {
        success: true,
        data: None,
        error: None,
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![checkout, list_orders, get_order, confirm_order, cancel_order]
}
