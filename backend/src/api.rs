use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tracing::error;

use crate::database::DatabaseManager;
use crate::models::{LocationQuery, PriceComparisonRequest, PriceComparisonResponse, StoreInfo};

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
    if request.grocery_list.is_empty() {
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

pub fn create_router(db_manager: Arc<DatabaseManager>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/stores/nearby", get(get_nearby_stores))
        .route("/api/compare-prices", post(compare_prices))
        .with_state(db_manager)
}
