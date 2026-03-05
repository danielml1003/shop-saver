use sqlx::{PgPool, Executor, Row};
use crate::models::{PriceComparisonRequest, PriceComparisonResponse, StoreInfo, ItemPrice, StoreComparison, StoreRecord};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct DatabaseManager {
    pub pool: PgPool,
}

impl DatabaseManager {
    pub async fn new(database_url: &str) -> Result<Self> {
        // force_custom_plan: PostgreSQL evaluates the actual parameter value (e.g., '%חלב%')
        // each time, allowing the GIN trigram index to be used for LIKE '$1' queries.
        // Without this, after 5 executions PostgreSQL falls back to a generic plan that does
        // a sequential scan instead of using the trigram index.
        let pool = sqlx::postgres::PgPoolOptions::new()
            .after_connect(|conn, _| Box::pin(async move {
                conn.execute("SET plan_cache_mode = 'force_custom_plan'").await?;
                Ok(())
            }))
            .connect(database_url)
            .await?;

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

        // Add new columns if they don't exist yet (idempotent migrations)
        sqlx::query("ALTER TABLE stores ADD COLUMN IF NOT EXISTS store_name VARCHAR(200)")
            .execute(&pool)
            .await?;
        sqlx::query("ALTER TABLE stores ADD COLUMN IF NOT EXISTS zip_code VARCHAR(20)")
            .execute(&pool)
            .await?;

        // Enable trigram extension for fast LIKE '%...%' substring searches
        sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_trgm")
            .execute(&pool)
            .await?;
        // GIN trigram index on lowercase item_name for fast LIKE '%term%' queries
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_items_lower_name_trgm ON items USING gin (LOWER(item_name) gin_trgm_ops)"
        )
        .execute(&pool)
        .await?;

        // Table to track which XML files have been fully processed — prevents re-scanning on restart
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS processed_files (
                filename VARCHAR PRIMARY KEY,
                file_size BIGINT NOT NULL,
                processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
            "#
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub async fn get_nearby_stores(&self, lat: f64, lon: f64, radius_km: f64) -> Result<Vec<StoreInfo>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, chain_id, sub_chain_id, store_id, store_name,
                address, city, latitude, longitude,
                6371 * acos(cos(radians($1)) * cos(radians(latitude))
                    * cos(radians(longitude) - radians($2))
                    + sin(radians($1)) * sin(radians(latitude))) as distance_km
            FROM stores
            WHERE latitude IS NOT NULL AND longitude IS NOT NULL
              AND 6371 * acos(cos(radians($1)) * cos(radians(latitude))
                  * cos(radians(longitude) - radians($2))
                  + sin(radians($1)) * sin(radians(latitude))) <= $3
            ORDER BY distance_km
            "#
        )
        .bind(lat).bind(lon).bind(radius_km)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| StoreInfo {
            id: row.get("id"),
            chain_id: row.get("chain_id"),
            sub_chain_id: row.get("sub_chain_id"),
            store_id: row.get("store_id"),
            store_name: row.get("store_name"),
            address: row.get("address"),
            city: row.get("city"),
            latitude: row.get("latitude"),
            longitude: row.get("longitude"),
            distance_km: row.get("distance_km"),
        }).collect())
    }

    // Returns stores that carry at least one of the requested items (LIKE match), ordered by
    // how many distinct search terms are covered (desc).
    // Uses one indexed LIKE query per term, then combines in Rust — allows the GIN trigram
    // index to be used for each individual term lookup.
    pub async fn get_stores_with_items(
        &self,
        item_names: &[String],
        page: usize,
        page_size: usize,
    ) -> Result<(Vec<StoreInfo>, usize)> {
        if item_names.is_empty() {
            return Ok((vec![], 0));
        }

        // For each term, find all stores that have at least one matching item (uses trigram index)
        let mut term_store_sets: Vec<HashSet<i32>> = Vec::new();
        for term in item_names {
            let pattern = format!("%{}%", term.to_lowercase());
            let store_ids: Vec<i32> = sqlx::query_scalar(
                "SELECT DISTINCT store_pk FROM items WHERE LOWER(item_name) LIKE $1"
            )
            .bind(&pattern)
            .fetch_all(&self.pool)
            .await?;
            term_store_sets.push(store_ids.into_iter().collect());
        }

        // Union of all matching store IDs; count how many terms each store covers
        let all_ids: HashSet<i32> = term_store_sets.iter()
            .flat_map(|s| s.iter().cloned())
            .collect();

        let mut coverage: Vec<(i32, usize)> = all_ids.iter().map(|&sid| {
            let count = term_store_sets.iter().filter(|s| s.contains(&sid)).count();
            (sid, count)
        }).collect();
        // Sort: most terms covered first, then stable by store id
        coverage.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let total = all_ids.len();
        let offset = (page - 1) * page_size;
        let page_ids: Vec<i32> = coverage.into_iter()
            .skip(offset)
            .take(page_size)
            .map(|(id, _)| id)
            .collect();

        if page_ids.is_empty() {
            return Ok((vec![], total));
        }

        // Fetch store metadata for this page (in coverage-sorted order)
        let rows = sqlx::query(
            "SELECT id, chain_id, sub_chain_id, store_id, store_name, address, city \
             FROM stores WHERE id = ANY($1)"
        )
        .bind(&page_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut id_to_store: HashMap<i32, StoreInfo> = rows.into_iter().map(|row| {
            let id: i32 = row.get("id");
            (id, StoreInfo {
                id,
                chain_id: row.get("chain_id"),
                sub_chain_id: row.get("sub_chain_id"),
                store_id: row.get("store_id"),
                store_name: row.get("store_name"),
                address: row.get("address"),
                city: row.get("city"),
                latitude: None,
                longitude: None,
                distance_km: None,
            })
        }).collect();

        // Return stores in the coverage-sorted order
        let stores: Vec<StoreInfo> = page_ids.iter()
            .filter_map(|id| id_to_store.remove(id))
            .collect();

        Ok((stores, total))
    }

    pub async fn search_item_names(&self, query: &str, limit: i64) -> Result<Vec<String>> {
        let pattern = format!("%{}%", query.to_lowercase());
        let rows = sqlx::query(
            "SELECT DISTINCT LOWER(item_name) as name FROM items \
             WHERE LOWER(item_name) LIKE $1 ORDER BY name LIMIT $2"
        )
        .bind(&pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.get::<String, _>("name")).collect())
    }

    // For each search term, finds the single cheapest item in the store whose name LIKE '%term%'.
    // Returns at most one item per grocery list term.
    pub async fn find_items_in_store(&self, store_id: i32, item_names: &[String]) -> Result<Vec<ItemPrice>> {
        let store_items = self.find_items_for_stores(&[store_id], item_names).await?;
        Ok(store_items.into_values().next().unwrap_or_default())
    }

    // Batch version: fetches cheapest-per-term items for multiple stores in one query.
    // Uses dynamic OR conditions so each LIKE $N can use the GIN trigram index.
    pub async fn find_items_for_stores(
        &self,
        store_ids: &[i32],
        item_names: &[String],
    ) -> Result<HashMap<i32, Vec<ItemPrice>>> {
        if store_ids.is_empty() || item_names.is_empty() {
            return Ok(HashMap::new());
        }

        let like_patterns: Vec<String> = item_names.iter()
            .map(|n| format!("%{}%", n.to_lowercase()))
            .collect();

        // Build: LOWER(item_name) LIKE $2 OR LOWER(item_name) LIKE $3 ...
        let or_clause: String = (1..=like_patterns.len())
            .map(|i| format!("LOWER(item_name) LIKE ${}", i + 1))
            .collect::<Vec<_>>()
            .join(" OR ");

        let sql = format!(
            "SELECT store_pk, item_code, item_name, item_price::float8 as price, \
             unit_of_measure, manufacturer_name \
             FROM items \
             WHERE store_pk = ANY($1) AND ({}) \
             ORDER BY store_pk, item_price ASC",
            or_clause
        );

        let mut q = sqlx::query(&sql).bind(store_ids);
        for pat in &like_patterns {
            q = q.bind(pat);
        }
        let rows = q.fetch_all(&self.pool).await?;

        // Group rows by store, then pick cheapest item per grocery term
        let mut by_store: HashMap<i32, Vec<_>> = HashMap::new();
        for row in rows {
            let sid: i32 = row.get("store_pk");
            by_store.entry(sid).or_default().push(row);
        }

        let mut result: HashMap<i32, Vec<ItemPrice>> = HashMap::new();
        for (sid, rows) in by_store {
            let mut term_items: HashMap<usize, ItemPrice> = HashMap::new();
            // rows are sorted by price ASC — first match per term is cheapest
            for row in &rows {
                let name_lower: String = row.get::<String, _>("item_name").to_lowercase();
                for (idx, term) in item_names.iter().enumerate() {
                    if term_items.contains_key(&idx) { continue; }
                    if name_lower.contains(term.to_lowercase().as_str()) {
                        term_items.insert(idx, ItemPrice {
                            item_code: row.get("item_code"),
                            item_name: row.get("item_name"),
                            price: row.get::<f64, _>("price"),
                            unit_of_measure: row.get("unit_of_measure"),
                            manufacturer_name: row.get("manufacturer_name"),
                        });
                    }
                }
            }
            result.insert(sid, term_items.into_values().collect());
        }

        Ok(result)
    }

    pub async fn compare_prices(&self, request: PriceComparisonRequest) -> Result<PriceComparisonResponse> {
        let page = request.page.unwrap_or(1).max(1);
        let page_size = request.page_size.unwrap_or(10).max(1).min(50);

        let (stores, total_stores) = if let Some(ref loc) = request.user_location {
            let radius_km = loc.radius_km.unwrap_or(10.0);
            let all = self.get_nearby_stores(loc.latitude, loc.longitude, radius_km).await?;
            let total = all.len();
            let offset = (page - 1) * page_size;
            let paged: Vec<_> = all.into_iter().skip(offset).take(page_size).collect();
            (paged, total)
        } else {
            self.get_stores_with_items(&request.grocery_list, page, page_size).await?
        };
        // Batch-fetch items for all stores on this page in one query
        let page_store_ids: Vec<i32> = stores.iter().map(|s| s.id).collect();
        let mut items_by_store = self
            .find_items_for_stores(&page_store_ids, &request.grocery_list)
            .await?;

        let mut store_comparisons = Vec::new();

        for store in stores {
            let items = items_by_store.remove(&store.id).unwrap_or_default();
            let total_price: f64 = items.iter().map(|item| item.price).sum();

            // A grocery term is "found" if any returned item's name contains it (LIKE match).
            let missing_items: Vec<String> = request.grocery_list
                .iter()
                .filter(|term| {
                    !items.iter().any(|item| {
                        item.item_name.to_lowercase().contains(term.to_lowercase().as_str())
                    })
                })
                .cloned()
                .collect();

            let items_found = request.grocery_list.len() - missing_items.len();

            store_comparisons.push(StoreComparison {
                store,
                items,
                total_price,
                items_found,
                items_missing: missing_items,
            });
        }

        // Sort by items found desc, then total price asc
        store_comparisons.sort_by(|a, b| {
            b.items_found.cmp(&a.items_found)
                .then_with(|| a.total_price.partial_cmp(&b.total_price).unwrap_or(std::cmp::Ordering::Equal))
        });

        let delivered = (page - 1) * page_size + store_comparisons.len();
        let has_more = delivered < total_stores;
        // Only return best_store on the first page
        let best_store = if page == 1 { store_comparisons.first().cloned() } else { None };

        Ok(PriceComparisonResponse {
            stores: store_comparisons,
            best_store,
            requested_items: request.grocery_list,
            total_stores,
            has_more,
        })
    }

    pub async fn update_store_from_stores_full(
        &self,
        chain_id: &str,
        sub_chain_id: i32,
        store: &StoreRecord,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO stores (chain_id, sub_chain_id, store_id, store_name, address, city, zip_code)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (chain_id, sub_chain_id, store_id)
            DO UPDATE SET
                store_name = COALESCE(EXCLUDED.store_name, stores.store_name),
                address    = COALESCE(EXCLUDED.address,    stores.address),
                city       = COALESCE(EXCLUDED.city,       stores.city),
                zip_code   = COALESCE(EXCLUDED.zip_code,  stores.zip_code)
            "#,
        )
        .bind(chain_id)
        .bind(sub_chain_id)
        .bind(store.store_id)
        .bind(&store.store_name)
        .bind(&store.address)
        .bind(&store.city)
        .bind(&store.zip_code)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

impl DatabaseManager {
    /// Returns true if this file (identified by name + size) was already processed.
    pub async fn is_file_processed(&self, filename: &str, file_size: i64) -> Result<bool> {
        let exists: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM processed_files WHERE filename = $1 AND file_size = $2"
        )
        .bind(filename)
        .bind(file_size)
        .fetch_optional(&self.pool)
        .await?;
        Ok(exists.is_some())
    }

    /// Marks a file as processed so it won't be re-scanned on next startup.
    pub async fn mark_file_processed(&self, filename: &str, file_size: i64) -> Result<()> {
        sqlx::query(
            "INSERT INTO processed_files (filename, file_size) VALUES ($1, $2) \
             ON CONFLICT (filename) DO UPDATE SET file_size = $2, processed_at = NOW()"
        )
        .bind(filename)
        .bind(file_size)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) fn parse_datetime(&self, datetime_str: &str) -> Result<chrono::NaiveDateTime> {
        chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|e| anyhow::anyhow!("Failed to parse datetime '{}': {}", datetime_str, e))
    }
}
