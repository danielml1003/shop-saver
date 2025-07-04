mod models;
mod database;
mod xml_processor;
mod api;

use anyhow::Result;
use axum::http::{header, Method};
use std::{env, sync::Arc, time::Duration};
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
    #[serde(rename = "Items")]
    items: Items,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Items {
    #[serde(rename = "Item")]
    items: Vec<Item>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Item {
    #[serde(rename = "PriceUpdateDate")]
    price_update_date: String,
    #[serde(rename = "ItemCode")]
    item_code: String,
    #[serde(rename = "ItemType")]
    item_type: i32,
    #[serde(rename = "ItemNm")]
    item_name: String,
    #[serde(rename = "ManufacturerName")]
    manufacturer_name: Option<String>,
    #[serde(rename = "ManufactureCountry")]
    manufacture_country: Option<String>,
    #[serde(rename = "ManufacturerItemDescription")]
    manufacturer_item_description: Option<String>,
    #[serde(rename = "UnitQty")]
    unit_qty: Option<String>,
    #[serde(rename = "Quantity")]
    quantity: Option<String>,
    #[serde(rename = "UnitOfMeasure")]
    unit_of_measure: Option<String>,
    #[serde(rename = "bIsWeighted")]
    is_weighted: Option<i32>,
    #[serde(rename = "QtyInPackage")]
    qty_in_package: Option<String>,
    #[serde(rename = "ItemPrice")]
    item_price: String,
    #[serde(rename = "UnitOfMeasurePrice")]
    unit_of_measure_price: Option<String>,
    #[serde(rename = "AllowDiscount")]
    allow_discount: Option<i32>,
    #[serde(rename = "ItemStatus")]
    item_status: Option<i32>,
}

// API Request/Response structures
#[derive(Debug, Deserialize)]
struct LocationQuery {
    latitude: f64,
    longitude: f64,
    radius_km: Option<f64>, // Default to 10km if not provided
}

#[derive(Debug, Deserialize)]
struct PriceComparisonRequest {
    user_location: LocationQuery,
    grocery_list: Vec<String>, // List of item names to search for
}

#[derive(Debug, Serialize)]
struct StoreInfo {
    id: i32,
    chain_id: String,
    sub_chain_id: i32,
    store_id: i32,
    address: Option<String>,
    city: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    distance_km: Option<f64>,
}

#[derive(Debug, Serialize)]
struct ItemPrice {
    item_code: String,
    item_name: String,
    price: f64,
    unit_of_measure: Option<String>,
    manufacturer_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct StoreComparison {
    store: StoreInfo,
    items: Vec<ItemPrice>,
    total_price: f64,
    items_found: usize,
    items_missing: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PriceComparisonResponse {
    stores: Vec<StoreComparison>,
    best_store: Option<StoreComparison>,
    requested_items: Vec<String>,
}

struct DatabaseManager {
    pool: PgPool,
}

impl DatabaseManager {
    async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        
        // Create tables if they don't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS stores (
                id SERIAL PRIMARY KEY,
                chain_id VARCHAR NOT NULL,
                sub_chain_id INTEGER NOT NULL,
                store_id INTEGER NOT NULL,
                bikoret_no INTEGER,
                latitude DECIMAL(10, 8),
                longitude DECIMAL(11, 8),
                address TEXT,
                city VARCHAR(100),
                country VARCHAR(100),
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                UNIQUE(chain_id, sub_chain_id, store_id)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS items (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                store_pk INTEGER REFERENCES stores(id),
                item_code VARCHAR NOT NULL,
                item_type INTEGER NOT NULL,
                item_name VARCHAR NOT NULL,
                manufacturer_name VARCHAR,
                manufacture_country VARCHAR,
                manufacturer_item_description VARCHAR,
                unit_qty VARCHAR,
                quantity VARCHAR,
                unit_of_measure VARCHAR,
                is_weighted INTEGER,
                qty_in_package VARCHAR,
                item_price DECIMAL(10,4) NOT NULL,
                unit_of_measure_price DECIMAL(10,4),
                allow_discount INTEGER,
                item_status INTEGER,
                price_update_date TIMESTAMP,
                processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                file_source VARCHAR,
                UNIQUE(store_pk, item_code, price_update_date)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Create index for location-based queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_stores_location ON stores(latitude, longitude)
            "#,
        )
        .execute(&pool)
        .await?;

        info!("Database tables created/verified successfully");
        Ok(Self { pool })
    }

    async fn process_xml_data(&self, xml_data: XmlRoot, file_path: &str) -> Result<()> {
        info!("Processing XML data from file: {}", file_path);
        
        // Insert or get store
        let store_id = self.insert_or_get_store(&xml_data).await?;
        
        // Process items
        let mut processed_count = 0;
        let mut skipped_count = 0;
        
        for item in xml_data.items.items {
            match self.insert_item(store_id, &item, file_path).await {
                Ok(_) => processed_count += 1,
                Err(e) => {
                    if e.to_string().contains("duplicate key") {
                        skipped_count += 1;
                    } else {
                        error!("Error inserting item {}: {}", item.item_code, e);
                    }
                }
            }
        }
        
        info!(
            "Processed {} items, skipped {} duplicates from {}",
            processed_count, skipped_count, file_path
        );
        
        Ok(())
    }

    async fn insert_or_get_store(&self, xml_data: &XmlRoot) -> Result<i32> {
        let result = sqlx::query(
            r#"
            INSERT INTO stores (chain_id, sub_chain_id, store_id, bikoret_no)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (chain_id, sub_chain_id, store_id)
            DO UPDATE SET bikoret_no = EXCLUDED.bikoret_no
            RETURNING id
            "#,
        )
        .bind(&xml_data.chain_id)
        .bind(xml_data.sub_chain_id)
        .bind(xml_data.store_id)
        .bind(xml_data.bikoret_no)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.get("id"))
    }

    async fn insert_item(&self, store_pk: i32, item: &Item, file_source: &str) -> Result<()> {
        // Parse price update date
        let price_update_date = self.parse_datetime(&item.price_update_date)?;
        
        // Parse price as decimal
        let item_price: f64 = item.item_price.parse()
            .map_err(|_| anyhow::anyhow!("Invalid item price: {}", item.item_price))?;
        
        let unit_of_measure_price: Option<f64> = item.unit_of_measure_price
            .as_ref()
            .and_then(|price| price.parse().ok());

        sqlx::query(
            r#"
            INSERT INTO items (
                store_pk, item_code, item_type, item_name, manufacturer_name,
                manufacture_country, manufacturer_item_description, unit_qty,
                quantity, unit_of_measure, is_weighted, qty_in_package,
                item_price, unit_of_measure_price, allow_discount, item_status,
                price_update_date, file_source
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            "#,
        )
        .bind(store_pk)
        .bind(&item.item_code)
        .bind(item.item_type)
        .bind(&item.item_name)
        .bind(&item.manufacturer_name)
        .bind(&item.manufacture_country)
        .bind(&item.manufacturer_item_description)
        .bind(&item.unit_qty)
        .bind(&item.quantity)
        .bind(&item.unit_of_measure)
        .bind(item.is_weighted)
        .bind(&item.qty_in_package)
        .bind(item_price)
        .bind(unit_of_measure_price)
        .bind(item.allow_discount)
        .bind(item.item_status)
        .bind(price_update_date)
        .bind(file_source)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    fn parse_datetime(&self, datetime_str: &str) -> Result<NaiveDateTime> {
        NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|e| anyhow::anyhow!("Failed to parse datetime '{}': {}", datetime_str, e))
    }

    // New methods for price comparison API
    async fn get_nearby_stores(&self, lat: f64, lon: f64, radius_km: f64) -> Result<Vec<StoreInfo>> {
        let stores = sqlx::query_as!(
            StoreInfo,
            r#"
            SELECT 
                id, chain_id, sub_chain_id, store_id, 
                address, city, latitude, longitude,
                CASE 
                    WHEN latitude IS NOT NULL AND longitude IS NOT NULL THEN
                        6371 * acos(cos(radians($1)) * cos(radians(latitude)) 
                        * cos(radians(longitude) - radians($2)) 
                        + sin(radians($1)) * sin(radians(latitude)))
                    ELSE NULL
                END as distance_km
            FROM stores 
            WHERE latitude IS NOT NULL 
              AND longitude IS NOT NULL
              AND 6371 * acos(cos(radians($1)) * cos(radians(latitude)) 
                  * cos(radians(longitude) - radians($2)) 
                  + sin(radians($1)) * sin(radians(latitude))) <= $3
            ORDER BY distance_km
            "#,
            lat, lon, radius_km
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(stores)
    }

    async fn find_items_in_store(&self, store_id: i32, item_names: &[String]) -> Result<Vec<ItemPrice>> {
        let placeholders: Vec<String> = (1..=item_names.len())
            .map(|i| format!("${}", i + 1))
            .collect();
        
        let query = format!(
            r#"
            SELECT DISTINCT ON (LOWER(item_name)) 
                item_code, item_name, item_price as price, 
                unit_of_measure, manufacturer_name
            FROM items 
            WHERE store_pk = $1 
              AND LOWER(item_name) = ANY(ARRAY[{}])
            ORDER BY LOWER(item_name), price_update_date DESC
            "#,
            placeholders.join(", ")
        );
        
        let mut query_builder = sqlx::query(&query);
        query_builder = query_builder.bind(store_id);
        
        for item_name in item_names {
            query_builder = query_builder.bind(item_name.to_lowercase());
        }
        
        let rows = query_builder.fetch_all(&self.pool).await?;
        
        let mut items = Vec::new();
        for row in rows {
            items.push(ItemPrice {
                item_code: row.get("item_code"),
                item_name: row.get("item_name"),
                price: row.get::<sqlx::types::BigDecimal, _>("price").to_string().parse().unwrap_or(0.0),
                unit_of_measure: row.get("unit_of_measure"),
                manufacturer_name: row.get("manufacturer_name"),
            });
        }
        
        Ok(items)
    }

    async fn compare_prices(&self, request: PriceComparisonRequest) -> Result<PriceComparisonResponse> {
        let radius_km = request.user_location.radius_km.unwrap_or(10.0);
        let stores = self.get_nearby_stores(
            request.user_location.latitude,
            request.user_location.longitude,
            radius_km,
        ).await?;
        
        let mut store_comparisons = Vec::new();
        
        for store in stores {
            let items = self.find_items_in_store(store.id, &request.grocery_list).await?;
            let total_price: f64 = items.iter().map(|item| item.price).sum();
            
            let found_items: std::collections::HashSet<String> = items
                .iter()
                .map(|item| item.item_name.to_lowercase())
                .collect();
            
            let missing_items: Vec<String> = request.grocery_list
                .iter()
                .filter(|item| !found_items.contains(&item.to_lowercase()))
                .cloned()
                .collect();
            
            store_comparisons.push(StoreComparison {
                store,
                items,
                total_price,
                items_found: found_items.len(),
                items_missing: missing_items,
            });
        }
        
        // Sort by total price (ascending) and then by number of items found (descending)
        store_comparisons.sort_by(|a, b| {
            a.total_price.partial_cmp(&b.total_price)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.items_found.cmp(&a.items_found))
        });
        
        let best_store = store_comparisons.first().cloned();
        
        Ok(PriceComparisonResponse {
            stores: store_comparisons,
            best_store,
            requested_items: request.grocery_list,
        })
    }
}

struct XmlFileProcessor {
    db_manager: DatabaseManager,
    watch_directory: String,
}

impl XmlFileProcessor {
    fn new(db_manager: DatabaseManager, watch_directory: String) -> Self {
        Self {
            db_manager,
            watch_directory,
        }
    }

    async fn process_xml_file(&self, file_path: &Path) -> Result<()> {
        let file_path_str = file_path.to_string_lossy();
        info!("Processing XML file: {}", file_path_str);

        let content = fs::read_to_string(file_path).await?;
        
        match serde_xml_rs::from_str::<XmlRoot>(&content) {
            Ok(xml_data) => {
                self.db_manager.process_xml_data(xml_data, &file_path_str).await?;
                info!("Successfully processed: {}", file_path_str);
            }
            Err(e) => {
                error!("Failed to parse XML file {}: {}", file_path_str, e);
                return Err(anyhow::anyhow!("XML parsing error: {}", e));
            }
        }

        Ok(())
    }

    async fn scan_existing_files(&self) -> Result<()> {
        info!("Scanning existing XML files in: {}", self.watch_directory);
        
        let mut dir = fs::read_dir(&self.watch_directory).await?;
        
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                if let Err(e) = self.process_xml_file(&path).await {
                    error!("Error processing existing file {:?}: {}", path, e);
                }
            }
        }
        
        Ok(())
    }

    fn start_file_watcher(&self) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        
        let mut watcher: RecommendedWatcher = Watcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    if let Err(e) = tx.send(event) {
                        error!("Error sending file event: {}", e);
                    }
                }
            },
            notify::Config::default(),
        )?;
        
        watcher.watch(Path::new(&self.watch_directory), RecursiveMode::NonRecursive)?;
        info!("Started watching directory: {}", self.watch_directory);
        
        let rt = tokio::runtime::Handle::current();
        
        thread::spawn(move || {
            loop {
                match rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(event) => {
                        if let EventKind::Create(_) | EventKind::Modify(_) = event.kind {
                            for path in event.paths {
                                if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                                    info!("New/modified XML file detected: {:?}", path);
                                    
                                    let path_clone = path.clone();
                                    rt.spawn(async move {
                                        // Wait a bit to ensure file is completely written
                                        tokio::time::sleep(Duration::from_secs(2)).await;
                                        
                                        // Note: This is a simplified approach. In a real application,
                                        // you'd want to pass the db_manager properly to the async context
                                        info!("File ready for processing: {:?}", path_clone);
                                    });
                                }
                            }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Normal timeout, continue loop
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        error!("File watcher channel disconnected");
                        break;
                    }
                }
            }
        });
        
        // Keep the watcher alive
        std::mem::forget(watcher);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("Starting Shop Saver XML Processor Server");
    
    // Get configuration from environment variables
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/shop_saver".to_string());
    
    let watch_directory = env::var("WATCH_DIRECTORY")
        .unwrap_or_else(|_| "../service/downloads".to_string());
    
    info!("Database URL: {}", database_url.replace(&database_url[database_url.find("://").unwrap_or(0)+3..database_url.find("@").unwrap_or(database_url.len())], "://***:***@"));
    info!("Watch Directory: {}", watch_directory);
    
    // Initialize database manager
    let db_manager = DatabaseManager::new(&database_url).await?;
    
    // Initialize XML file processor
    let processor = XmlFileProcessor::new(db_manager, watch_directory);
    
    // Process existing files
    if let Err(e) = processor.scan_existing_files().await {
        warn!("Error scanning existing files: {}", e);
    }
    
    // Start file watcher
    processor.start_file_watcher()?;
    
    info!("Server is running. Press Ctrl+C to stop.");
    
    // Keep the server running
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        info!("Server heartbeat - still monitoring for XML files...");
    }
}
