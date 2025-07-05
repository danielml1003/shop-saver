use sqlx::{PgPool, Row};
use crate::models::{LocationQuery, PriceComparisonRequest, PriceComparisonResponse, StoreInfo, ItemPrice, StoreComparison};
use anyhow::Result;
use std::collections::HashSet;

pub struct DatabaseManager {
    pub pool: PgPool,
}

impl DatabaseManager {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;

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

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_stores_location ON stores(latitude, longitude)
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub async fn get_nearby_stores(&self, lat: f64, lon: f64, radius_km: f64) -> Result<Vec<StoreInfo>> {
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

    pub async fn find_items_in_store(&self, store_id: i32, item_names: &[String]) -> Result<Vec<ItemPrice>> {
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

    pub async fn compare_prices(&self, request: PriceComparisonRequest) -> Result<PriceComparisonResponse> {
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
            
            let found_items: HashSet<String> = items
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

impl DatabaseManager {
    fn parse_datetime(&self, datetime_str: &str) -> Result<chrono::NaiveDateTime> {
        chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|e| anyhow::anyhow!("Failed to parse datetime '{}': {}", datetime_str, e))
    }
}
