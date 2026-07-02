use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use std::{collections::HashMap, sync::Arc};
use tracing::error;

use crate::database::DatabaseManager;
use crate::models::{LocationQuery, PaginatedItems, PriceComparisonRequest, PriceComparisonResponse, ProductSearchResult, StoreInfo};

/// Hard limits on user-supplied input (see ARCHITECTURE.md §5.2 — input bounds).
const MAX_GROCERY_LIST_LEN: usize = 100;
const MAX_TERM_LEN: usize = 200;
const MAX_RADIUS_KM: f64 = 200.0;

pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "shop-saver-api"
    }))
}

pub async fn get_nearby_stores(
    State(db): State<Arc<DatabaseManager>>,
    Query(location): Query<LocationQuery>,
) -> Result<Json<Vec<StoreInfo>>, StatusCode> {
    let radius_km = location.radius_km.unwrap_or(10.0);
    if !(0.0..=MAX_RADIUS_KM).contains(&radius_km)
        || !(-90.0..=90.0).contains(&location.latitude)
        || !(-180.0..=180.0).contains(&location.longitude)
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    match db.get_nearby_stores(location.latitude, location.longitude, radius_km).await {
        Ok(stores) => Ok(Json(stores)),
        Err(e) => {
            error!("Error getting nearby stores: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn compare_prices(
    State(db): State<Arc<DatabaseManager>>,
    Json(request): Json<PriceComparisonRequest>,
) -> Result<Json<PriceComparisonResponse>, StatusCode> {
    if request.grocery_list.is_empty()
        || request.grocery_list.len() > MAX_GROCERY_LIST_LEN
        || request.grocery_list.iter().any(|t| t.is_empty() || t.len() > MAX_TERM_LEN)
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    match db.compare_prices(request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("Error comparing prices: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Autocomplete: short list of {barcode, name} suggestions for a query.
/// Distinct from GET /api/items (paginated browse with filters) below.
pub async fn search_items(
    State(db): State<Arc<DatabaseManager>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<ProductSearchResult>>, StatusCode> {
    let q = params.get("q").map(|s| s.as_str()).unwrap_or("");
    if q.len() < 2 || q.len() > MAX_TERM_LEN {
        return Ok(Json(vec![]));
    }
    match db.search_item_names(q, 20).await {
        Ok(results) => Ok(Json(results)),
        Err(e) => {
            error!("Error searching item names: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_all_stores_handler(
    State(db): State<Arc<DatabaseManager>>,
) -> Result<Json<Vec<StoreInfo>>, StatusCode> {
    match db.get_all_stores().await {
        Ok(stores) => Ok(Json(stores)),
        Err(e) => {
            error!("Error fetching all stores: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_store_items_handler(
    State(db): State<Arc<DatabaseManager>>,
    Path(store_id): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<PaginatedItems>, StatusCode> {
    let q = params.get("q").map(|s| s.as_str());
    let page: usize = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1).max(1);
    let limit: usize = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(20).clamp(1, 100);

    match db.get_store_items(store_id, q, page, limit).await {
        Ok((items, total)) => {
            let has_more = (page - 1) * limit + items.len() < total;
            Ok(Json(PaginatedItems { items, total: total as i64, page, page_size: limit, has_more }))
        }
        Err(e) => {
            error!("Error fetching items for store {}: {}", store_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Paginated browse across all stores with optional name + price filters.
pub async fn search_items_handler(
    State(db): State<Arc<DatabaseManager>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<PaginatedItems>, StatusCode> {
    let q = params.get("q").map(|s| s.as_str()).unwrap_or("");
    if q.len() > MAX_TERM_LEN {
        return Err(StatusCode::BAD_REQUEST);
    }
    let min_price: Option<f64> = params.get("min_price").and_then(|p| p.parse().ok());
    let max_price: Option<f64> = params.get("max_price").and_then(|p| p.parse().ok());
    let page: usize = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1).max(1);
    let limit: usize = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(20).clamp(1, 100);

    match db.search_items_paginated(q, min_price, max_price, page, limit).await {
        Ok((items, total)) => {
            let has_more = (page - 1) * limit + items.len() < total;
            Ok(Json(PaginatedItems { items, total: total as i64, page, page_size: limit, has_more }))
        }
        Err(e) => {
            error!("Error searching items: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub fn create_router(db_manager: Arc<DatabaseManager>) -> Router {
    // /api/stores/nearby is registered before /api/stores/:id/items so Axum
    // never tries to parse "nearby" as a store ID.
    Router::new()
        .route("/health", get(health_check))
        .route("/api/stores/nearby", get(get_nearby_stores))
        .route("/api/stores", get(get_all_stores_handler))
        .route("/api/stores/:id/items", get(get_store_items_handler))
        .route("/api/compare-prices", post(compare_prices))
        .route("/api/items/search", get(search_items))
        .route("/api/items", get(search_items_handler))
        .with_state(db_manager)
}
