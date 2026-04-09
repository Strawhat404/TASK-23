use rocket::{get, routes};
use rocket::serde::json::Json;
use rocket::http::Status;
use rocket::State;
use sqlx::MySqlPool;

use shared::dto::{ApiResponse, ProductListItem, ProductDetail, OptionGroupDetail, OptionValueDetail};

#[derive(Debug, serde::Deserialize, rocket::FromForm)]
pub struct ProductQuery {
    pub featured: Option<bool>,
    pub limit: Option<usize>,
}

#[get("/?<params..>")]
pub async fn list_products(
    pool: &State<MySqlPool>,
    params: ProductQuery,
) -> Json<ApiResponse<Vec<ProductListItem>>> {
    let spus = crate::db::products::list_spus(pool.inner(), true).await;

    let mut items: Vec<ProductListItem> = spus
        .into_iter()
        .map(|s| ProductListItem {
            spu_id: s.id,
            name_en: s.name_en,
            name_zh: s.name_zh,
            description_en: s.description_en,
            description_zh: s.description_zh,
            category: s.category,
            image_url: s.image_url,
            base_price: s.base_price,
            prep_time_minutes: s.prep_time_minutes,
        })
        .collect();

    if params.featured == Some(true) {
        // Featured products: return the first N items (by default ordering)
        // In a real system this could use a `is_featured` column.
    }

    if let Some(limit) = params.limit {
        items.truncate(limit);
    }

    Json(ApiResponse {
        success: true,
        data: Some(items),
        error: None,
    })
}

#[get("/<id>")]
pub async fn get_product(
    pool: &State<MySqlPool>,
    id: i64,
) -> Result<Json<ApiResponse<ProductDetail>>, (Status, Json<ApiResponse<()>>)> {
    let spu = crate::db::products::get_spu(pool.inner(), id)
        .await
        .ok_or_else(|| {
            (
                Status::NotFound,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Product not found".into()),
                }),
            )
        })?;

    let groups = crate::db::products::get_option_groups(pool.inner(), spu.id).await;
    let mut option_group_details = Vec::new();
    for g in groups {
        let values = crate::db::products::get_option_values(pool.inner(), g.id).await;
        option_group_details.push(OptionGroupDetail {
            id: g.id,
            name_en: g.name_en,
            name_zh: g.name_zh,
            is_required: g.is_required,
            options: values
                .into_iter()
                .map(|v| OptionValueDetail {
                    id: v.id,
                    label_en: v.label_en,
                    label_zh: v.label_zh,
                    price_delta: v.price_delta,
                    is_default: v.is_default,
                })
                .collect(),
        });
    }

    let detail = ProductDetail {
        spu: ProductListItem {
            spu_id: spu.id,
            name_en: spu.name_en,
            name_zh: spu.name_zh,
            description_en: spu.description_en,
            description_zh: spu.description_zh,
            category: spu.category,
            image_url: spu.image_url,
            base_price: spu.base_price,
            prep_time_minutes: spu.prep_time_minutes,
        },
        option_groups: option_group_details,
    };

    Ok(Json(ApiResponse {
        success: true,
        data: Some(detail),
        error: None,
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![list_products, get_product]
}
