use sqlx::{PgPool, Executor, Row};
use crate::models::{PriceComparisonRequest, PriceComparisonResponse, StoreInfo, ItemPrice, StoreComparison, StoreRecord, ProductSearchResult, StoreItemRow};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// Returns true if `code` is a valid EAN-13 barcode (13 digits + correct check digit).
pub fn is_ean13(code: &str) -> bool {
    if code.len() != 13 {
        return false;
    }
    let digits: Vec<u32> = match code.chars().map(|c| c.to_digit(10)).collect::<Option<Vec<_>>>() {
        Some(d) => d,
        None => return false,
    };
    let sum: u32 = digits[..12]
        .iter()
        .enumerate()
        .map(|(i, &d)| if i % 2 == 0 { d } else { d * 3 })
        .sum();
    let check = (10 - (sum % 10)) % 10;
    check == digits[12]
}

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

        // Canonical product catalog — one row per unique EAN-13 barcode.
        // Populated automatically during XML ingest.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS products (
                barcode VARCHAR(13) PRIMARY KEY,
                canonical_name VARCHAR NOT NULL,
                manufacturer VARCHAR,
                quantity VARCHAR,
                unit_of_measure VARCHAR,
                first_seen_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
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
        // GIN trigram index on items for name search
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_items_lower_name_trgm ON items USING gin (LOWER(item_name) gin_trgm_ops)"
        )
        .execute(&pool)
        .await?;
        // GIN trigram index on products for autocomplete
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_products_lower_name_trgm ON products USING gin (LOWER(canonical_name) gin_trgm_ops)"
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

        // Known chain ID → Hebrew display name mapping.
        // Used as a fallback when store_name is NULL (before StoresFull XML is ingested).
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS chain_names (
                chain_id VARCHAR PRIMARY KEY,
                display_name VARCHAR NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await?;

        let chain_data: &[(&str, &str)] = &[
            ("7290027600007", "שופרסל"),
            ("7290058140886", "רמי לוי"),
            ("7290055700007", "קרפור"),
            ("7290058108879", "קינג סטור"),
            ("7290058159628", "מעיין 2000"),
            ("7290058197699", "גוד פארם"),
            ("7290492000005", "דור אלון"),
            ("7290873255550", "טיב טעם"),
            ("7290696200003", "ויקטורי"),
            ("7290058173198", "זול ובגדול"),
            ("7290803800003", "יוחננוף"),
            ("7290695900006", "אושר עד"),
            ("7290633800006", "AM:PM"),
            ("7290876100000", "חצי חינם"),
            ("7290011900477", "סופר-פארם"),
        ];
        for (chain_id, display_name) in chain_data {
            sqlx::query(
                "INSERT INTO chain_names (chain_id, display_name) VALUES ($1, $2) \
                 ON CONFLICT (chain_id) DO UPDATE SET display_name = EXCLUDED.display_name"
            )
            .bind(chain_id)
            .bind(display_name)
            .execute(&pool)
            .await?;
        }

        Ok(Self { pool })
    }

    pub async fn get_nearby_stores(&self, lat: f64, lon: f64, radius_km: f64) -> Result<Vec<StoreInfo>> {
        let rows = sqlx::query(
            r#"
            SELECT
                s.id, s.chain_id, s.sub_chain_id, s.store_id,
                COALESCE(s.store_name, cn.display_name) as store_name,
                s.address, s.city,
                s.latitude::float8 as latitude, s.longitude::float8 as longitude,
                6371 * acos(cos(radians($1)) * cos(radians(s.latitude))
                    * cos(radians(s.longitude) - radians($2))
                    + sin(radians($1)) * sin(radians(s.latitude))) as distance_km
            FROM stores s
            LEFT JOIN chain_names cn ON s.chain_id = cn.chain_id
            WHERE s.latitude IS NOT NULL AND s.longitude IS NOT NULL
              AND 6371 * acos(cos(radians($1)) * cos(radians(s.latitude))
                  * cos(radians(s.longitude) - radians($2))
                  + sin(radians($1)) * sin(radians(s.latitude))) <= $3
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

    /// Returns all stores with coordinates, name, and city — for the stores map page.
    pub async fn get_all_stores(&self) -> Result<Vec<StoreInfo>> {
        let rows = sqlx::query(
            "SELECT s.id, s.chain_id, s.sub_chain_id, s.store_id, \
                    COALESCE(s.store_name, cn.display_name) as store_name, \
                    s.address, s.city, \
                    s.latitude::float8, s.longitude::float8 \
             FROM stores s \
             LEFT JOIN chain_names cn ON s.chain_id = cn.chain_id \
             ORDER BY s.city NULLS LAST, store_name NULLS LAST"
        )
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
            distance_km: None,
        }).collect())
    }

    /// Returns paginated items for one store, optionally filtered by name query.
    /// Uses DISTINCT ON item_code to return the cheapest price per distinct item.
    pub async fn get_store_items(
        &self,
        store_id: i32,
        query: Option<&str>,
        page: usize,
        limit: usize,
    ) -> Result<(Vec<StoreItemRow>, usize)> {
        let offset = ((page.saturating_sub(1)) * limit) as i64;
        let pattern = match query {
            Some(q) if !q.is_empty() => format!("%{}%", q.to_lowercase()),
            _ => "%".to_string(),
        };

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT item_code) FROM items \
             WHERE store_pk = $1 AND LOWER(item_name) LIKE $2"
        )
        .bind(store_id)
        .bind(&pattern)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query(
            "SELECT DISTINCT ON (item_code) \
                    item_code, item_name, manufacturer_name, \
                    item_price::float8 as item_price, unit_of_measure, quantity \
             FROM items \
             WHERE store_pk = $1 AND LOWER(item_name) LIKE $2 \
             ORDER BY item_code, item_price ASC \
             LIMIT $3 OFFSET $4"
        )
        .bind(store_id)
        .bind(&pattern)
        .bind(limit as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let items = rows.into_iter().map(|row| StoreItemRow {
            item_code: row.get("item_code"),
            item_name: row.get("item_name"),
            manufacturer_name: row.get("manufacturer_name"),
            item_price: row.get("item_price"),
            unit_of_measure: row.get("unit_of_measure"),
            quantity: row.get("quantity"),
        }).collect();

        Ok((items, total as usize))
    }

    /// Returns paginated items across all stores, DISTINCT by item name (cheapest price per name).
    /// Supports optional query string and price range filters.
    pub async fn search_items_paginated(
        &self,
        query: &str,
        min_price: Option<f64>,
        max_price: Option<f64>,
        page: usize,
        limit: usize,
    ) -> Result<(Vec<StoreItemRow>, usize)> {
        let offset = ((page.saturating_sub(1)) * limit) as i64;
        let pattern = if query.is_empty() {
            "%".to_string()
        } else {
            format!("%{}%", query.to_lowercase())
        };

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT LOWER(item_name)) FROM items \
             WHERE LOWER(item_name) LIKE $1 \
               AND ($2::float8 IS NULL OR item_price::float8 >= $2) \
               AND ($3::float8 IS NULL OR item_price::float8 <= $3)"
        )
        .bind(&pattern)
        .bind(min_price)
        .bind(max_price)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query(
            "SELECT DISTINCT ON (LOWER(item_name)) \
                    item_code, item_name, manufacturer_name, \
                    item_price::float8 as item_price, unit_of_measure, quantity \
             FROM items \
             WHERE LOWER(item_name) LIKE $1 \
               AND ($2::float8 IS NULL OR item_price::float8 >= $2) \
               AND ($3::float8 IS NULL OR item_price::float8 <= $3) \
             ORDER BY LOWER(item_name), item_price ASC \
             LIMIT $4 OFFSET $5"
        )
        .bind(&pattern)
        .bind(min_price)
        .bind(max_price)
        .bind(limit as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let items = rows.into_iter().map(|row| StoreItemRow {
            item_code: row.get("item_code"),
            item_name: row.get("item_name"),
            manufacturer_name: row.get("manufacturer_name"),
            item_price: row.get("item_price"),
            unit_of_measure: row.get("unit_of_measure"),
            quantity: row.get("quantity"),
        }).collect();

        Ok((items, total as usize))
    }

    // Returns stores that carry at least one of the requested items, ordered by coverage.
    // Handles barcodes (exact match) and name terms (LIKE) separately so each can use its index.
    pub async fn get_stores_with_items(
        &self,
        grocery_list: &[String],
        page: usize,
        page_size: usize,
    ) -> Result<(Vec<StoreInfo>, usize)> {
        if grocery_list.is_empty() {
            return Ok((vec![], 0));
        }

        let (barcodes, name_terms): (Vec<&String>, Vec<&String>) = grocery_list
            .iter()
            .partition(|s| is_ean13(s));

        let mut term_store_sets: Vec<HashSet<i32>> = Vec::new();

        // Barcode terms — exact item_code lookup
        for barcode in &barcodes {
            let store_ids: Vec<i32> = sqlx::query_scalar(
                "SELECT DISTINCT store_pk FROM items WHERE item_code = $1"
            )
            .bind(barcode.as_str())
            .fetch_all(&self.pool)
            .await?;
            term_store_sets.push(store_ids.into_iter().collect());
        }

        // Name terms — LIKE via trigram index
        for term in &name_terms {
            let pattern = format!("%{}%", term.to_lowercase());
            let store_ids: Vec<i32> = sqlx::query_scalar(
                "SELECT DISTINCT store_pk FROM items WHERE LOWER(item_name) LIKE $1"
            )
            .bind(&pattern)
            .fetch_all(&self.pool)
            .await?;
            term_store_sets.push(store_ids.into_iter().collect());
        }

        let all_ids: HashSet<i32> = term_store_sets.iter()
            .flat_map(|s| s.iter().cloned())
            .collect();

        let mut coverage: Vec<(i32, usize)> = all_ids.iter().map(|&sid| {
            let count = term_store_sets.iter().filter(|s| s.contains(&sid)).count();
            (sid, count)
        }).collect();
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

        let rows = sqlx::query(
            "SELECT s.id, s.chain_id, s.sub_chain_id, s.store_id, \
                    COALESCE(s.store_name, cn.display_name) as store_name, \
                    s.address, s.city \
             FROM stores s \
             LEFT JOIN chain_names cn ON s.chain_id = cn.chain_id \
             WHERE s.id = ANY($1)"
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

        let stores: Vec<StoreInfo> = page_ids.iter()
            .filter_map(|id| id_to_store.remove(id))
            .collect();

        Ok((stores, total))
    }

    /// Search for items matching `query`. Returns results with a barcode when the item is a
    /// known EAN-13 product (so the frontend can send the barcode for exact comparison),
    /// or None for store-brand / non-standard items (fallback to name matching).
    pub async fn search_item_names(&self, query: &str, limit: i64) -> Result<Vec<ProductSearchResult>> {
        let pattern = format!("%{}%", query.to_lowercase());
        // Join items with products: barcode items get their barcode, others get NULL.
        // DISTINCT ON ensures each unique item name appears only once.
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT ON (LOWER(i.item_name))
                p.barcode,
                LOWER(i.item_name) as name
            FROM items i
            LEFT JOIN products p ON i.item_code = p.barcode
            WHERE LOWER(i.item_name) LIKE $1
            ORDER BY LOWER(i.item_name), p.barcode NULLS LAST
            LIMIT $2
            "#,
        )
        .bind(&pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| ProductSearchResult {
            barcode: r.get("barcode"),
            name: r.get("name"),
        }).collect())
    }

    /// Batch-fetch the cheapest matching item per grocery list term, per store.
    /// Returns map of store_pk -> (term_index -> ItemPrice).
    /// Barcodes use exact item_code lookup; name terms use LIKE.
    pub async fn find_items_for_stores(
        &self,
        store_ids: &[i32],
        grocery_list: &[String],
    ) -> Result<HashMap<i32, HashMap<usize, ItemPrice>>> {
        if store_ids.is_empty() || grocery_list.is_empty() {
            return Ok(HashMap::new());
        }

        let barcodes: Vec<(usize, &String)> = grocery_list
            .iter()
            .enumerate()
            .filter(|(_, s)| is_ean13(s))
            .collect();

        let name_terms: Vec<(usize, &String)> = grocery_list
            .iter()
            .enumerate()
            .filter(|(_, s)| !is_ean13(s))
            .collect();

        // by_store[store_pk][term_index] = ItemPrice
        let mut by_store: HashMap<i32, HashMap<usize, ItemPrice>> = HashMap::new();

        // --- Barcode lookup: exact match, cheapest price per barcode per store ---
        if !barcodes.is_empty() {
            let barcode_vals: Vec<String> = barcodes.iter().map(|(_, s)| s.to_string()).collect();
            let rows = sqlx::query(
                "SELECT store_pk, item_code, item_name, \
                 MIN(item_price)::float8 as price, unit_of_measure, manufacturer_name \
                 FROM items \
                 WHERE store_pk = ANY($1) AND item_code = ANY($2) \
                 GROUP BY store_pk, item_code, item_name, unit_of_measure, manufacturer_name"
            )
            .bind(store_ids)
            .bind(&barcode_vals)
            .fetch_all(&self.pool)
            .await?;

            for row in rows {
                let sid: i32 = row.get("store_pk");
                let code: String = row.get("item_code");
                if let Some(&(idx, _)) = barcodes.iter().find(|(_, b)| b.as_str() == code.as_str()) {
                    by_store.entry(sid).or_default().insert(idx, ItemPrice {
                        item_code: code,
                        item_name: row.get("item_name"),
                        price: row.get::<f64, _>("price"),
                        unit_of_measure: row.get("unit_of_measure"),
                        manufacturer_name: row.get("manufacturer_name"),
                    });
                }
            }
        }

        // --- Name (LIKE) lookup: trigram index, cheapest per matching term per store ---
        if !name_terms.is_empty() {
            let like_patterns: Vec<String> = name_terms.iter()
                .map(|(_, n)| format!("%{}%", n.to_lowercase()))
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

            for row in rows {
                let sid: i32 = row.get("store_pk");
                let name_lower: String = row.get::<String, _>("item_name").to_lowercase();
                let store_map = by_store.entry(sid).or_default();
                for &(idx, term) in &name_terms {
                    if store_map.contains_key(&idx) { continue; }
                    if name_lower.contains(term.to_lowercase().as_str()) {
                        store_map.insert(idx, ItemPrice {
                            item_code: row.get("item_code"),
                            item_name: row.get("item_name"),
                            price: row.get::<f64, _>("price"),
                            unit_of_measure: row.get("unit_of_measure"),
                            manufacturer_name: row.get("manufacturer_name"),
                        });
                    }
                }
            }
        }

        Ok(by_store)
    }

    /// Like get_stores_with_items but pre-filtered to a set of candidate store IDs.
    /// Used when a location or city pre-filter has already determined the candidate set.
    async fn get_stores_with_items_from_set(
        &self,
        grocery_list: &[String],
        candidate_ids: &[i32],
        page: usize,
        page_size: usize,
    ) -> Result<(Vec<StoreInfo>, usize)> {
        if grocery_list.is_empty() || candidate_ids.is_empty() {
            return Ok((vec![], 0));
        }

        let (barcodes, name_terms): (Vec<&String>, Vec<&String>) =
            grocery_list.iter().partition(|s| is_ean13(s));

        let mut term_store_sets: Vec<HashSet<i32>> = Vec::new();

        for barcode in &barcodes {
            let ids: Vec<i32> = sqlx::query_scalar(
                "SELECT DISTINCT store_pk FROM items \
                 WHERE item_code = $1 AND store_pk = ANY($2)"
            )
            .bind(barcode.as_str())
            .bind(candidate_ids)
            .fetch_all(&self.pool)
            .await?;
            term_store_sets.push(ids.into_iter().collect());
        }

        for term in &name_terms {
            let pattern = format!("%{}%", term.to_lowercase());
            let ids: Vec<i32> = sqlx::query_scalar(
                "SELECT DISTINCT store_pk FROM items \
                 WHERE LOWER(item_name) LIKE $1 AND store_pk = ANY($2)"
            )
            .bind(&pattern)
            .bind(candidate_ids)
            .fetch_all(&self.pool)
            .await?;
            term_store_sets.push(ids.into_iter().collect());
        }

        let all_ids: HashSet<i32> = term_store_sets.iter()
            .flat_map(|s| s.iter().cloned())
            .collect();

        let mut coverage: Vec<(i32, usize)> = all_ids.iter().map(|&sid| {
            let count = term_store_sets.iter().filter(|s| s.contains(&sid)).count();
            (sid, count)
        }).collect();
        coverage.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let total = all_ids.len();
        let offset = (page - 1) * page_size;
        let page_ids: Vec<i32> = coverage.into_iter()
            .skip(offset).take(page_size).map(|(id, _)| id).collect();

        if page_ids.is_empty() {
            return Ok((vec![], total));
        }

        let rows = sqlx::query(
            "SELECT s.id, s.chain_id, s.sub_chain_id, s.store_id, \
                    COALESCE(s.store_name, cn.display_name) as store_name, \
                    s.address, s.city, \
                    s.latitude::float8 as latitude, s.longitude::float8 as longitude \
             FROM stores s \
             LEFT JOIN chain_names cn ON s.chain_id = cn.chain_id \
             WHERE s.id = ANY($1)"
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
                latitude: row.get("latitude"),
                longitude: row.get("longitude"),
                distance_km: None,
            })
        }).collect();

        let stores: Vec<StoreInfo> = page_ids.iter()
            .filter_map(|id| id_to_store.remove(id))
            .collect();

        Ok((stores, total))
    }

    pub async fn compare_prices(&self, request: PriceComparisonRequest) -> Result<PriceComparisonResponse> {
        let page = request.page.unwrap_or(1).max(1);
        let page_size = request.page_size.unwrap_or(10).max(1).min(50);

        let (stores, total_stores) = if let Some(ref loc) = request.user_location {
            // Get IDs of all stores within radius, then intersect with item-carrying stores
            let radius_km = loc.radius_km.unwrap_or(10.0);
            let nearby = self.get_nearby_stores(loc.latitude, loc.longitude, radius_km).await?;
            let nearby_ids: Vec<i32> = nearby.iter().map(|s| s.id).collect();
            self.get_stores_with_items_from_set(&request.grocery_list, &nearby_ids, page, page_size).await?
        } else if let Some(ref city) = request.city {
            // Get IDs of all stores in that city, then intersect with item-carrying stores
            let city_ids: Vec<i32> = sqlx::query_scalar(
                "SELECT id FROM stores WHERE LOWER(city) LIKE $1"
            )
            .bind(format!("%{}%", city.to_lowercase()))
            .fetch_all(&self.pool)
            .await?;
            self.get_stores_with_items_from_set(&request.grocery_list, &city_ids, page, page_size).await?
        } else {
            self.get_stores_with_items(&request.grocery_list, page, page_size).await?
        };

        let page_store_ids: Vec<i32> = stores.iter().map(|s| s.id).collect();
        let mut items_by_store = self
            .find_items_for_stores(&page_store_ids, &request.grocery_list)
            .await?;

        let mut store_comparisons = Vec::new();

        for store in stores {
            // term_map: term_index -> ItemPrice (one entry per grocery list term, if found)
            let term_map = items_by_store.remove(&store.id).unwrap_or_default();
            let total_price: f64 = term_map.values().map(|item| item.price).sum();

            // A term is missing if its index has no entry in term_map
            let missing_items: Vec<String> = request.grocery_list
                .iter()
                .enumerate()
                .filter(|(idx, _)| !term_map.contains_key(idx))
                .map(|(_, term)| term.clone())
                .collect();

            let items_found = request.grocery_list.len() - missing_items.len();
            let items: Vec<ItemPrice> = term_map.into_values().collect();

            store_comparisons.push(StoreComparison {
                store,
                items,
                total_price,
                items_found,
                items_missing: missing_items,
            });
        }

        // Sort: most items found first, then cheapest total
        store_comparisons.sort_by(|a, b| {
            b.items_found.cmp(&a.items_found)
                .then_with(|| a.total_price.partial_cmp(&b.total_price).unwrap_or(std::cmp::Ordering::Equal))
        });

        let delivered = (page - 1) * page_size + store_comparisons.len();
        let has_more = delivered < total_stores;
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

    /// Upsert a product into the canonical products catalog.
    /// Called during XML ingest for every item with a valid EAN-13 barcode.
    /// ON CONFLICT DO NOTHING keeps the first-seen canonical name.
    pub async fn upsert_product(&self, item: &crate::models::Item) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO products (barcode, canonical_name, manufacturer, quantity, unit_of_measure)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (barcode) DO NOTHING
            "#,
        )
        .bind(&item.item_code)
        .bind(&item.item_name)
        .bind(&item.manufacturer_name)
        .bind(&item.quantity)
        .bind(&item.unit_of_measure)
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

#[cfg(test)]
mod tests {
    use super::is_ean13;

    #[test]
    fn valid_ean13_barcodes() {
        // EAN-13 codes with correct check digits
        assert!(is_ean13("7290000066769"));
        assert!(is_ean13("7290027600007"));
        assert!(is_ean13("4006381333931"));
    }

    #[test]
    fn wrong_check_digit_rejected() {
        assert!(!is_ean13("7290000066768"));
        assert!(!is_ean13("4006381333930"));
    }

    #[test]
    fn wrong_length_rejected() {
        assert!(!is_ean13(""));
        assert!(!is_ean13("729000006676"));    // 12 digits
        assert!(!is_ean13("72900000667680"));  // 14 digits
    }

    #[test]
    fn non_digits_rejected() {
        assert!(!is_ean13("729000006676a"));
        assert!(!is_ean13("חלב תנובה 1 ל"));
        assert!(!is_ean13("7290-00006676"));
    }
}
