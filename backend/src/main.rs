mod models;
mod database;
mod xml_processor;
mod api;

use anyhow::Result;
use axum::http::Method;
use std::{env, sync::Arc};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{error, info, warn};

use database::DatabaseManager;
use xml_processor::XmlFileProcessor;
use api::create_router;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Shop Saver API Server");

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/shop_saver".to_string());
    let watch_directory = env::var("WATCH_DIRECTORY")
        .unwrap_or_else(|_| "../service/downloads".to_string());
    let server_port = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);

    // Mask credentials (the part between "://" and "@", when both exist and are ordered)
    let safe_db_url = match (database_url.find("://"), database_url.rfind('@')) {
        (Some(scheme_end), Some(at)) if at > scheme_end + 3 => {
            format!("{}***:***{}", &database_url[..scheme_end + 3], &database_url[at..])
        }
        _ => database_url.clone(),
    };
    info!("📊 Database URL: {}", safe_db_url);
    info!("📁 Watch Directory: {}", watch_directory);
    info!("🌐 Server will run on port: {}", server_port);

    let db_manager = match DatabaseManager::new(&database_url).await {
        Ok(db) => {
            info!("✅ Database connection established");
            Arc::new(db)
        }
        Err(e) => {
            error!("❌ Failed to connect to database: {}", e);
            return Err(e);
        }
    };

    // Start XML file processor in background
    let xml_db_manager = DatabaseManager::new(&database_url).await?;
    let processor = XmlFileProcessor::new(xml_db_manager, watch_directory.clone());
    if let Err(e) = processor.start_file_watcher() {
        warn!("⚠️ Error starting file watcher: {}", e);
    } else {
        info!("👀 Started XML file watcher");
    }
    // Scan existing files in the background so API starts immediately
    tokio::spawn(async move {
        if let Err(e) = processor.scan_existing_files().await {
            warn!("⚠️ Error scanning existing XML files: {}", e);
        } else {
            info!("✅ Background scan of existing XML files completed");
        }
    });

    // CORS: restrict to CORS_ALLOWED_ORIGINS (comma-separated) when set;
    // wide open otherwise (dev / same-origin nginx deployments).
    let cors = match env::var("CORS_ALLOWED_ORIGINS") {
        Ok(origins) if !origins.trim().is_empty() => {
            let parsed: Vec<_> = origins
                .split(',')
                .filter_map(|o| o.trim().parse::<axum::http::HeaderValue>().ok())
                .collect();
            info!("🔒 CORS restricted to: {:?}", parsed);
            CorsLayer::new()
                .allow_origin(parsed)
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers(Any)
        }
        _ => CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(Any),
    };

    let app = create_router(db_manager)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
        );

    let addr = format!("0.0.0.0:{}", server_port);
    let listener = TcpListener::bind(&addr).await?;

    info!("🌐 API Server running on http://{}", addr);
    info!("📋 Available endpoints:");
    info!("   GET  /health                    - Health check");
    info!("   GET  /api/stores/nearby         - Get nearby stores");
    info!("   POST /api/compare-prices        - Compare prices across stores");
    info!("");
    info!("🛒 Shop Saver is ready to help you find the best prices!");

    match axum::serve(listener, app).await {
        Ok(_) => info!("✅ Server shut down gracefully"),
        Err(e) => {
            error!("❌ Server error: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
