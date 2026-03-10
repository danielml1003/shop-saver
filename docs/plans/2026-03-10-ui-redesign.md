# UI Redesign + Location Filtering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Redesign the frontend UI (clean/modern, Hebrew RTL), unify cart with grocery list, add location-aware store filtering, and add a stores map + store detail page backed by real API endpoints.

**Architecture:** Three backend endpoints are added (`GET /api/stores`, `GET /api/stores/:id/items`, `GET /api/items`) and `compare-prices` gains a `city` filter. The frontend replaces CartPage with a unified grocery list in CartContext, rewrites StoresPage with a react-leaflet map, adds StoreDetailPage, and wires ItemsPage to real data.

**Tech Stack:** Rust/Axum backend, PostgreSQL/sqlx, React 19/TypeScript, MUI v7, react-leaflet + leaflet (new dependency).

---

## Phase 1 — Backend

### Task 1: Add `city` to request model + new response structs

**Files:**
- Modify: `backend/src/models.rs`

**Step 1: Add `city` field to `PriceComparisonRequest` and two new structs**

In `backend/src/models.rs`, make these changes:

```rust
// Change PriceComparisonRequest (line ~119) to add city field:
#[derive(Debug, Deserialize)]
pub struct PriceComparisonRequest {
    pub user_location: Option<LocationQuery>,
    pub grocery_list: Vec<String>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub city: Option<String>,  // ← ADD THIS LINE
}

// Add these two new structs AFTER the existing StoreComparison struct:

/// A single item row returned by the store-items and item-search endpoints.
#[derive(Debug, Serialize, Clone)]
pub struct StoreItemRow {
    pub item_code: String,
    pub item_name: String,
    pub manufacturer_name: Option<String>,
    pub item_price: f64,
    pub unit_of_measure: Option<String>,
    pub quantity: Option<String>,
}

/// Paginated item list returned by GET /api/stores/:id/items and GET /api/items.
#[derive(Debug, Serialize)]
pub struct PaginatedItems {
    pub items: Vec<StoreItemRow>,
    pub total: i64,
    pub page: usize,
    pub page_size: usize,
    pub has_more: bool,
}
```

**Step 2: Verify compilation**

```bash
cd backend && cargo check 2>&1
```

Expected: no errors (only `dead_code` warnings are fine).

**Step 3: Commit**

```bash
git add backend/src/models.rs
git commit -m "feat: add city filter to PriceComparisonRequest + StoreItemRow/PaginatedItems models"
```

---

### Task 2: Add `get_all_stores()` to database.rs

**Files:**
- Modify: `backend/src/database.rs`

**Step 1: Add the method inside the first `impl DatabaseManager` block, after `get_nearby_stores()`**

```rust
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
```

Note: `latitude` and `longitude` are stored as `DECIMAL` in the DB — the `::float8` cast is required so sqlx can read them as `Option<f64>`.

**Step 2: Verify compilation**

```bash
cd backend && cargo check 2>&1
```

**Step 3: Commit**

```bash
git add backend/src/database.rs
git commit -m "feat: add get_all_stores() for stores map endpoint"
```

---

### Task 3: Add `get_store_items()` to database.rs

**Files:**
- Modify: `backend/src/database.rs`

**Step 1: Add the method. Place it after `get_all_stores()`.**

Add to `use` imports at top of file: `use crate::models::{..., StoreItemRow};` (add `StoreItemRow` to the existing models import).

```rust
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
```

**Step 2: Verify compilation**

```bash
cd backend && cargo check 2>&1
```

**Step 3: Commit**

```bash
git add backend/src/database.rs
git commit -m "feat: add get_store_items() for store detail endpoint"
```

---

### Task 4: Add `search_items_paginated()` to database.rs

**Files:**
- Modify: `backend/src/database.rs`

**Step 1: Add method after `get_store_items()`**

```rust
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
```

**Step 2: Verify compilation**

```bash
cd backend && cargo check 2>&1
```

**Step 3: Commit**

```bash
git add backend/src/database.rs
git commit -m "feat: add search_items_paginated() for items browse endpoint"
```

---

### Task 5: Refactor `compare_prices()` — add city filter + location+item intersection

**Files:**
- Modify: `backend/src/database.rs`

**Step 1: Add private helper `get_stores_with_items_from_set()`**

Add this method inside the first `impl DatabaseManager` block, right before `compare_prices()`:

```rust
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
                s.latitude::float8, s.longitude::float8 \
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
```

**Step 2: Update `compare_prices()` to use the new helper**

Replace the `let (stores, total_stores) = if let Some(ref loc)...` block at the start of `compare_prices()`:

```rust
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
```

**Step 3: Verify compilation**

```bash
cd backend && cargo check 2>&1
```

**Step 4: Commit**

```bash
git add backend/src/database.rs
git commit -m "feat: refactor compare_prices to intersect location/city filter with item coverage"
```

---

### Task 6: Add route handlers to api.rs

**Files:**
- Modify: `backend/src/api.rs`

**Step 1: Update imports at the top of api.rs**

The current import is:
```rust
use std::{collections::HashMap, sync::Arc};
```

Add `Path` to the axum imports and add new model types:
```rust
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
    routing::{get, post},
    Router,
};
use std::{collections::HashMap, sync::Arc};
use tracing::error;

use crate::database::DatabaseManager;
use crate::models::{LocationQuery, PriceComparisonRequest, PaginatedItems};
```

**Step 2: Add three new handler functions after `search_items()`**

```rust
pub async fn get_all_stores_handler(
    State(db): State<Arc<DatabaseManager>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match db.get_all_stores().await {
        Ok(stores) => Ok(Json(serde_json::json!(stores))),
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
    let page: usize = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let limit: usize = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(20).min(100);

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

pub async fn search_items_handler(
    State(db): State<Arc<DatabaseManager>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<PaginatedItems>, StatusCode> {
    let q = params.get("q").map(|s| s.as_str()).unwrap_or("");
    let min_price: Option<f64> = params.get("min_price").and_then(|p| p.parse().ok());
    let max_price: Option<f64> = params.get("max_price").and_then(|p| p.parse().ok());
    let page: usize = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let limit: usize = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(20).min(100);

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
```

**Step 3: Register the new routes in `create_router()`**

```rust
pub fn create_router(db_manager: Arc<DatabaseManager>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/stores/nearby", get(get_nearby_stores))
        .route("/api/stores", get(get_all_stores_handler))           // ← NEW
        .route("/api/stores/:id/items", get(get_store_items_handler)) // ← NEW
        .route("/api/compare-prices", post(compare_prices))
        .route("/api/items/search", get(search_items))
        .route("/api/items", get(search_items_handler))               // ← NEW
        .with_state(db_manager)
}
```

Note: `/api/stores/nearby` must be registered BEFORE `/api/stores/:id/items` so Axum doesn't try to parse "nearby" as a store ID when hitting `:id/items`. The routes are distinct (different number of path segments) but keeping order explicit is safer.

**Step 4: Full build**

```bash
cd backend && cargo build 2>&1
```

Expected: build succeeds.

**Step 5: Start backend and smoke test all 3 new endpoints**

```bash
# In one terminal:
cd backend && cargo run --bin backend

# In another terminal (wait ~3s for startup):
curl -s http://127.0.0.1:3000/api/stores | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'{len(d)} stores')"
# Expected: e.g. "245 stores"

curl -s "http://127.0.0.1:3000/api/stores/1/items?limit=3" | python3 -m json.tool | head -30
# Expected: JSON with items array and total

curl -s "http://127.0.0.1:3000/api/items?q=חלב&limit=3" | python3 -m json.tool | head -30
# Expected: JSON with items array
```

**Step 6: Commit**

```bash
git add backend/src/api.rs backend/src/database.rs backend/src/models.rs
git commit -m "feat: add GET /api/stores, GET /api/stores/:id/items, GET /api/items endpoints"
```

---

## Phase 2 — Frontend Setup

### Task 7: Install react-leaflet dependencies

**Files:**
- Modify: `frontend/package.json` (via npm install)

**Step 1: Install packages**

```bash
cd frontend && npm install react-leaflet leaflet @types/leaflet
```

**Step 2: Verify no peer dependency errors**

```bash
npm ls react-leaflet 2>&1 | head -5
```

Expected: shows `react-leaflet@x.x.x` without errors.

**Step 3: Commit**

```bash
git add frontend/package.json frontend/package-lock.json
git commit -m "chore: add react-leaflet + leaflet dependencies"
```

---

### Task 8: Update `types/index.ts`

**Files:**
- Modify: `frontend/src/types/index.ts`

**Step 1: Add `city` to `PriceComparisonRequest` and new types for the three new endpoints**

Add to `PriceComparisonRequest`:
```typescript
export interface PriceComparisonRequest {
  user_location?: UserLocation;
  grocery_list: string[];
  page?: number;
  page_size?: number;
  city?: string;  // ← ADD
}
```

Add new interfaces at the end of the file:
```typescript
// Returned by GET /api/stores/:id/items and GET /api/items
export interface StoreItemRow {
  item_code: string;
  item_name: string;
  manufacturer_name?: string;
  item_price: number;
  unit_of_measure?: string;
  quantity?: string;
}

export interface PaginatedItemsResponse {
  items: StoreItemRow[];
  total: number;
  page: number;
  page_size: number;
  has_more: boolean;
}
```

**Step 2: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit 2>&1
```

Expected: no errors.

**Step 3: Commit**

```bash
git add frontend/src/types/index.ts
git commit -m "feat: add city to PriceComparisonRequest + StoreItemRow/PaginatedItemsResponse types"
```

---

### Task 9: Update `api.ts` with new methods

**Files:**
- Modify: `frontend/src/services/api.ts`

**Step 1: Update the import line to include new types**

```typescript
import { Store, Item, SearchFilters, ApiResponse, UserLocation, PriceComparisonRequest,
         PriceComparisonResponse, BackendStoreInfo, StoreItemRow, PaginatedItemsResponse } from '../types';
```

**Step 2: Add three new methods to the `apiService` object**

Add after `searchItemNames`:

```typescript
// GET /api/stores — all stores with coordinates (for map)
getAllStores: async (): Promise<BackendStoreInfo[]> => {
  const res = await api.get('/api/stores');
  return res.data as BackendStoreInfo[];
},

// GET /api/stores/:id/items — items for one store, paginated
getStoreItems: async (
  storeId: number,
  query?: string,
  page = 1,
  limit = 20
): Promise<PaginatedItemsResponse> => {
  const params: Record<string, string | number> = { page, limit };
  if (query) params.q = query;
  const res = await api.get(`/api/stores/${storeId}/items`, { params });
  return res.data as PaginatedItemsResponse;
},

// GET /api/items — paginated item search across all stores
searchItemsPaginated: async (
  query?: string,
  minPrice?: number,
  maxPrice?: number,
  page = 1,
  limit = 20
): Promise<PaginatedItemsResponse> => {
  const params: Record<string, string | number> = { page, limit };
  if (query) params.q = query;
  if (minPrice != null) params.min_price = minPrice;
  if (maxPrice != null) params.max_price = maxPrice;
  const res = await api.get('/api/items', { params });
  return res.data as PaginatedItemsResponse;
},
```

**Step 3: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit 2>&1
```

**Step 4: Commit**

```bash
git add frontend/src/services/api.ts
git commit -m "feat: add getAllStores, getStoreItems, searchItemsPaginated to apiService"
```

---

### Task 10: Update CartContext to use GroceryItem

The current CartContext stores `Item[]` (the full backend item). The grocery list needs `GroceryItem[]` (barcode + name). We replace the context's item type.

**Files:**
- Modify: `frontend/src/context/CartContext.tsx`

**Step 1: Rewrite CartContext.tsx**

```typescript
import React, { createContext, useContext, useMemo, useState } from 'react';
import { GroceryItem } from '../types';

export interface CartContextValue {
  items: GroceryItem[];
  addItem: (item: GroceryItem) => void;
  removeItem: (name: string) => void;
  clearCart: () => void;
  contains: (name: string) => boolean;
}

const CartContext = createContext<CartContextValue | undefined>(undefined);

export const CartProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [items, setItems] = useState<GroceryItem[]>([]);

  const addItem = (item: GroceryItem) => {
    setItems(prev => {
      if (prev.some(i => i.name === item.name)) return prev;
      return [item, ...prev];
    });
  };

  const removeItem = (name: string) => {
    setItems(prev => prev.filter(i => i.name !== name));
  };

  const clearCart = () => setItems([]);

  const contains = (name: string) => items.some(i => i.name === name);

  const value = useMemo<CartContextValue>(
    () => ({ items, addItem, removeItem, clearCart, contains }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [items]
  );

  return <CartContext.Provider value={value}>{children}</CartContext.Provider>;
};

export const useCart = (): CartContextValue => {
  const ctx = useContext(CartContext);
  if (!ctx) throw new Error('useCart must be used within a CartProvider');
  return ctx;
};
```

**Step 2: Verify TypeScript compiles (expect errors in files that used Item[] from cart)**

```bash
cd frontend && npx tsc --noEmit 2>&1
```

The errors you'll see at this point are expected — `CartPage.tsx` uses the old `Item` type. That file gets replaced in Task 14. Note the errors but don't fix them yet.

**Step 3: Update `ItemCard.tsx` to use `GroceryItem`**

Read the current `frontend/src/components/ItemCard.tsx`, then find where it calls `addItem(item)` and change it to:

```typescript
// Replace addItem(item) call with:
addItem({ barcode: item.item_code, name: item.item_name });
```

Also update the `contains` check — it currently uses `item_code`, but now uses `name`:

```typescript
// Replace contains(item.item_code) with:
contains(item.item_name)
```

**Step 4: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit 2>&1
```

Expected: fewer errors (only CartPage.tsx issues remain).

**Step 5: Commit**

```bash
git add frontend/src/context/CartContext.tsx frontend/src/components/ItemCard.tsx
git commit -m "refactor: CartContext uses GroceryItem[] instead of Item[]"
```

---

## Phase 3 — Component Rewrites

### Task 11: Redesign Header.tsx

**Files:**
- Modify: `frontend/src/components/Header.tsx`

**Step 1: Replace Header.tsx entirely**

```typescript
import React from 'react';
import {
  AppBar, Badge, BottomNavigation, BottomNavigationAction,
  Box, Paper, Toolbar, Typography, useMediaQuery, useTheme,
} from '@mui/material';
import CompareArrowsIcon from '@mui/icons-material/CompareArrows';
import SearchIcon from '@mui/icons-material/Search';
import StoreIcon from '@mui/icons-material/Store';
import ShoppingCartIcon from '@mui/icons-material/ShoppingCart';
import { useNavigate, useLocation } from 'react-router-dom';
import { useCart } from '../context/CartContext';

const NAV_ITEMS = [
  { path: '/', label: 'השוואה', icon: <CompareArrowsIcon /> },
  { path: '/items', label: 'מוצרים', icon: <SearchIcon /> },
  { path: '/stores', label: 'חנויות', icon: <StoreIcon /> },
];

const Header: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const { items } = useCart();
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('sm'));

  const isActive = (path: string) =>
    path === '/' ? location.pathname === '/' || location.pathname === '/compare' : location.pathname.startsWith(path);

  if (isMobile) {
    return (
      <>
        {/* Spacer so content isn't hidden behind bottom nav */}
        <Box sx={{ height: 56 }} />
        <Paper
          elevation={3}
          sx={{ position: 'fixed', bottom: 0, left: 0, right: 0, zIndex: 1200 }}
        >
          <BottomNavigation
            value={location.pathname}
            onChange={(_, val) => navigate(val)}
            showLabels
          >
            {NAV_ITEMS.map(item => (
              <BottomNavigationAction
                key={item.path}
                label={item.label}
                value={item.path}
                icon={item.icon}
              />
            ))}
            <BottomNavigationAction
              label="הסל"
              value="/cart"
              icon={
                <Badge badgeContent={items.length || undefined} color="primary">
                  <ShoppingCartIcon />
                </Badge>
              }
            />
          </BottomNavigation>
        </Paper>
      </>
    );
  }

  return (
    <AppBar
      position="sticky"
      color="default"
      elevation={0}
      sx={{ borderBottom: '1px solid', borderColor: 'divider', bgcolor: 'background.paper' }}
    >
      <Toolbar sx={{ gap: 1 }}>
        {/* Logo */}
        <Box
          sx={{ display: 'flex', alignItems: 'center', gap: 1, cursor: 'pointer', mr: 4 }}
          onClick={() => navigate('/')}
        >
          <ShoppingCartIcon color="primary" />
          <Typography variant="h6" fontWeight={700} color="primary">
            ShopSaver
          </Typography>
        </Box>

        {/* Nav links */}
        <Box sx={{ display: 'flex', flex: 1 }}>
          {NAV_ITEMS.map(item => (
            <Box
              key={item.path}
              onClick={() => navigate(item.path)}
              sx={{
                display: 'flex', alignItems: 'center', gap: 0.5,
                px: 2, py: 1, cursor: 'pointer',
                color: isActive(item.path) ? 'primary.main' : 'text.secondary',
                borderBottom: '2px solid',
                borderColor: isActive(item.path) ? 'primary.main' : 'transparent',
                fontWeight: isActive(item.path) ? 600 : 400,
                fontSize: 14,
                '&:hover': { color: 'primary.main' },
                transition: 'color 0.15s, border-color 0.15s',
              }}
            >
              {item.icon}
              <span>{item.label}</span>
            </Box>
          ))}
        </Box>

        {/* Cart badge */}
        <Box
          onClick={() => navigate('/')}
          sx={{
            display: 'flex', alignItems: 'center', gap: 0.5,
            px: 2, py: 1, cursor: 'pointer',
            color: 'text.secondary', '&:hover': { color: 'primary.main' },
          }}
        >
          <Badge badgeContent={items.length || undefined} color="primary">
            <ShoppingCartIcon />
          </Badge>
          <Typography variant="body2" sx={{ fontSize: 14 }}>הסל שלי</Typography>
        </Box>
      </Toolbar>
    </AppBar>
  );
};

export default Header;
```

**Step 2: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit 2>&1
```

**Step 3: Commit**

```bash
git add frontend/src/components/Header.tsx
git commit -m "feat: redesign Header with white sticky bar, active underline, mobile bottom nav"
```

---

### Task 12: Update ComparePage to use CartContext + auto-GPS + city fallback

**Files:**
- Modify: `frontend/src/pages/ComparePage.tsx`

The full ComparePage is 578 lines. The changes are surgical:

**Step 1: Replace the grocery list state with CartContext**

Near the top of `ComparePage`, the current state:
```typescript
const [items, setItems] = useState<GroceryItem[]>([]);
```
Replace with CartContext:
```typescript
const { items, addItem: cartAddItem, removeItem: cartRemoveItem, clearCart } = useCart();
```
Add the import at the top: `import { useCart } from '../context/CartContext';`

**Step 2: Replace `addItem` and `removeItem` local functions**

Find:
```typescript
const addItem = (name: string) => { ... };
const removeItem = (name: string) => setItems(prev => prev.filter(i => i !== name));
```
Replace with:
```typescript
const addItem = (result: ProductSearchResult | string) => {
  const name = typeof result === 'string' ? result.trim() : result.name.trim();
  const barcode = typeof result === 'string' ? null : result.barcode;
  if (!name || items.some(i => i.name === name)) return;
  cartAddItem({ barcode, name });
  setInputItem('');
  setSuggestions([]);
};

const removeItem = (name: string) => cartRemoveItem(name);
```

**Step 3: Add city state + auto-GPS on mount**

After the existing `const [location, setLocation]` line, add:
```typescript
const [city, setCity] = useState('');
const [gpsStatus, setGpsStatus] = useState<'idle' | 'active' | 'denied'>('idle');
```

Add a `useEffect` for auto-GPS on mount (place it after the existing useEffects that sync refs):
```typescript
// Auto-detect GPS on mount
useEffect(() => {
  if (!navigator.geolocation) { setGpsStatus('denied'); return; }
  navigator.geolocation.getCurrentPosition(
    pos => {
      setLocation({ latitude: pos.coords.latitude, longitude: pos.coords.longitude, radius_km: 10 });
      setGpsStatus('active');
    },
    () => setGpsStatus('denied')
  );
}, []);
```

**Step 4: Add `city` to the compare payload**

In `fetchPage`, update the payload construction:
```typescript
const payload: PriceComparisonRequest = {
  grocery_list: grocery.map(i => i.barcode ?? i.name),
  page,
  page_size: PAGE_SIZE,
  ...(location ? { user_location: location } : {}),
  ...(city.trim() ? { city: city.trim() } : {}),  // ← ADD
};
```

**Step 5: Replace the location button row in JSX**

Find the current location button area (around line 272-284) and replace with:
```tsx
{/* Location row */}
<Box sx={{ display: 'flex', gap: 1, alignItems: 'center', mb: 3, flexWrap: 'wrap' }}>
  {gpsStatus === 'active' && location ? (
    <Chip
      icon={<LocationOnIcon />}
      label={`GPS פעיל · ${location.radius_km ?? 10} ק"מ`}
      color="success"
      variant="outlined"
      onDelete={() => { setLocation(null); setGpsStatus('denied'); }}
    />
  ) : (
    <TextField
      label="עיר (אופציונלי)"
      size="small"
      value={city}
      onChange={e => setCity(e.target.value)}
      placeholder='לדוגמה: תל אביב'
      sx={{ width: 180 }}
      InputProps={{
        startAdornment: <LocationOnIcon sx={{ color: 'action.active', mr: 0.5 }} fontSize="small" />,
      }}
    />
  )}
  <Button
    variant="contained"
    size="large"
    disabled={items.length === 0 || loading}
    onClick={runSearch}
    startIcon={loading ? <CircularProgress size={18} color="inherit" /> : undefined}
  >
    {loading ? 'מחפש...' : 'השווה מחירים'}
  </Button>
</Box>
```

**Step 6: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit 2>&1
```

**Step 7: Commit**

```bash
git add frontend/src/pages/ComparePage.tsx
git commit -m "feat: ComparePage uses CartContext + auto-GPS on mount + city text fallback"
```

---

### Task 13: Replace CartPage with a redirect

CartPage is superseded — the grocery list now lives in ComparePage via CartContext.

**Files:**
- Modify: `frontend/src/pages/CartPage.tsx`

**Step 1: Replace CartPage with a simple redirect**

```typescript
import { Navigate } from 'react-router-dom';

const CartPage = () => <Navigate to="/" replace />;

export default CartPage;
```

**Step 2: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit 2>&1
```

Expected: no errors.

**Step 3: Commit**

```bash
git add frontend/src/pages/CartPage.tsx
git commit -m "refactor: replace CartPage with redirect to ComparePage"
```

---

### Task 14: Rewrite StoresPage with react-leaflet map

**Files:**
- Modify: `frontend/src/pages/StoresPage.tsx`

**Step 1: Add leaflet CSS import to `frontend/src/index.tsx`**

Open `frontend/src/index.tsx` and add this import near the top:
```typescript
import 'leaflet/dist/leaflet.css';
```

**Step 2: Fix leaflet default marker icon (webpack breaks the default)**

Create `frontend/src/utils/leafletIcons.ts`:
```typescript
import L from 'leaflet';
import markerIcon from 'leaflet/dist/images/marker-icon.png';
import markerIcon2x from 'leaflet/dist/images/marker-icon-2x.png';
import markerShadow from 'leaflet/dist/images/marker-shadow.png';

delete (L.Icon.Default.prototype as any)._getIconUrl;
L.Icon.Default.mergeOptions({
  iconUrl: markerIcon,
  iconRetinaUrl: markerIcon2x,
  shadowUrl: markerShadow,
});
```

Import this file in `frontend/src/index.tsx`:
```typescript
import './utils/leafletIcons';
```

**Step 3: Rewrite StoresPage.tsx**

```typescript
import React, { useEffect, useMemo, useState } from 'react';
import {
  Alert, Box, Card, CardActionArea, CardContent,
  CircularProgress, Container, TextField, Typography,
} from '@mui/material';
import LocationOnIcon from '@mui/icons-material/LocationOn';
import { MapContainer, TileLayer, Marker, Popup } from 'react-leaflet';
import { useNavigate } from 'react-router-dom';
import { apiService } from '../services/api';
import { BackendStoreInfo } from '../types';

const StoresPage: React.FC = () => {
  const navigate = useNavigate();
  const [stores, setStores] = useState<BackendStoreInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');

  useEffect(() => {
    apiService.getAllStores()
      .then(setStores)
      .catch(() => setError('שגיאה בטעינת החנויות'))
      .finally(() => setLoading(false));
  }, []);

  const filtered = useMemo(() => {
    if (!search.trim()) return stores;
    const q = search.toLowerCase();
    return stores.filter(s =>
      (s.store_name ?? '').toLowerCase().includes(q) ||
      (s.city ?? '').toLowerCase().includes(q)
    );
  }, [stores, search]);

  const mapStores = useMemo(
    () => filtered.filter(s => s.latitude != null && s.longitude != null),
    [filtered]
  );

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', mt: 8 }}>
        <CircularProgress />
      </Box>
    );
  }

  return (
    <Container maxWidth="lg" sx={{ mt: 3, mb: 8 }}>
      <Typography variant="h5" fontWeight={700} sx={{ mb: 2 }}>חנויות ורשתות</Typography>

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

      {/* Map */}
      {mapStores.length > 0 && (
        <Box sx={{ height: 400, borderRadius: 2, overflow: 'hidden', mb: 3, border: '1px solid', borderColor: 'divider' }}>
          <MapContainer
            center={[mapStores[0].latitude!, mapStores[0].longitude!]}
            zoom={8}
            style={{ height: '100%', width: '100%' }}
          >
            <TileLayer
              attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
              url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
            />
            {mapStores.map(s => (
              <Marker key={s.id} position={[s.latitude!, s.longitude!]}>
                <Popup>
                  <strong>{s.store_name || s.chain_id}</strong>
                  <br />
                  {[s.address, s.city].filter(Boolean).join(', ')}
                  <br />
                  <a href="#" onClick={e => { e.preventDefault(); navigate(`/stores/${s.id}`); }}>
                    פרטים ומחירים
                  </a>
                </Popup>
              </Marker>
            ))}
          </MapContainer>
        </Box>
      )}

      {/* Search */}
      <TextField
        fullWidth
        size="small"
        label="חפש לפי שם חנות או עיר"
        value={search}
        onChange={e => setSearch(e.target.value)}
        sx={{ mb: 2 }}
        InputProps={{ startAdornment: <LocationOnIcon sx={{ color: 'action.active', mr: 0.5 }} fontSize="small" /> }}
      />

      <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
        {filtered.length} חנויות
      </Typography>

      {/* List */}
      <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', sm: '1fr 1fr', md: '1fr 1fr 1fr' }, gap: 2 }}>
        {filtered.map(store => (
          <Card key={store.id} variant="outlined">
            <CardActionArea onClick={() => navigate(`/stores/${store.id}`)}>
              <CardContent>
                <Typography variant="subtitle1" fontWeight={600}>
                  {store.store_name || store.chain_id}
                </Typography>
                {store.city && (
                  <Typography variant="body2" color="text.secondary">
                    {[store.city, store.address].filter(Boolean).join(' · ')}
                  </Typography>
                )}
              </CardContent>
            </CardActionArea>
          </Card>
        ))}
      </Box>
    </Container>
  );
};

export default StoresPage;
```

**Step 4: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit 2>&1
```

**Step 5: Commit**

```bash
git add frontend/src/pages/StoresPage.tsx frontend/src/index.tsx frontend/src/utils/leafletIcons.ts
git commit -m "feat: StoresPage with react-leaflet map and searchable store list"
```

---

### Task 15: Create StoreDetailPage

**Files:**
- Create: `frontend/src/pages/StoreDetailPage.tsx`

**Step 1: Create the file**

```typescript
import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Alert, Box, Button, Chip, CircularProgress, Container,
  Divider, TextField, Typography,
} from '@mui/material';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import AddShoppingCartIcon from '@mui/icons-material/AddShoppingCart';
import { useNavigate, useParams } from 'react-router-dom';
import { apiService } from '../services/api';
import { BackendStoreInfo, StoreItemRow } from '../types';
import { useCart } from '../context/CartContext';

const PAGE_SIZE = 30;

const StoreDetailPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { addItem, contains } = useCart();

  const [store, setStore] = useState<BackendStoreInfo | null>(null);
  const [items, setItems] = useState<StoreItemRow[]>([]);
  const [query, setQuery] = useState('');
  const [page, setPage] = useState(1);
  const [total, setTotal] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Load store info from the stores list (no dedicated /api/stores/:id endpoint needed)
  useEffect(() => {
    if (!id) return;
    apiService.getAllStores().then(all => {
      const found = all.find(s => s.id === Number(id));
      setStore(found ?? null);
    });
  }, [id]);

  const loadItems = useCallback(async (pageNum: number, append: boolean, q: string) => {
    if (!id) return;
    if (pageNum === 1) setLoading(true); else setLoadingMore(true);
    try {
      const data = await apiService.getStoreItems(Number(id), q, pageNum, PAGE_SIZE);
      setItems(prev => append ? [...prev, ...data.items] : data.items);
      setTotal(data.total);
      setHasMore(data.has_more);
      setPage(pageNum);
    } catch {
      setError('שגיאה בטעינת פריטים');
    } finally {
      setLoading(false);
      setLoadingMore(false);
    }
  }, [id]);

  useEffect(() => { loadItems(1, false, ''); }, [loadItems]);

  // Debounced search
  const handleSearch = (val: string) => {
    setQuery(val);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => loadItems(1, false, val), 300);
  };

  // Infinite scroll sentinel
  useEffect(() => {
    const el = sentinelRef.current;
    if (!el) return;
    const observer = new IntersectionObserver(entries => {
      if (entries[0].isIntersecting && hasMore && !loadingMore) {
        loadItems(page + 1, true, query);
      }
    }, { threshold: 0.1 });
    observer.observe(el);
    return () => observer.disconnect();
  }, [hasMore, loadingMore, page, query, loadItems]);

  if (loading) {
    return <Box sx={{ display: 'flex', justifyContent: 'center', mt: 8 }}><CircularProgress /></Box>;
  }

  const storeLabel = store ? (store.store_name || store.chain_id) : `חנות ${id}`;
  const storeLocation = store ? [store.city, store.address].filter(Boolean).join(' · ') : '';

  return (
    <Container maxWidth="md" sx={{ mt: 3, mb: 8 }}>
      <Button startIcon={<ArrowBackIcon />} onClick={() => navigate('/stores')} sx={{ mb: 2 }}>
        חזרה לחנויות
      </Button>

      <Typography variant="h5" fontWeight={700}>{storeLabel}</Typography>
      {storeLocation && (
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>{storeLocation}</Typography>
      )}
      <Divider sx={{ mb: 2 }} />

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

      <TextField
        fullWidth
        size="small"
        label="חפש מוצר בחנות זו"
        value={query}
        onChange={e => handleSearch(e.target.value)}
        sx={{ mb: 2 }}
      />

      <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
        {total.toLocaleString()} מוצרים · מציג {items.length}
      </Typography>

      {items.map(item => (
        <Box
          key={item.item_code}
          sx={{
            display: 'flex', justifyContent: 'space-between', alignItems: 'center',
            py: 1.5, borderBottom: '1px solid', borderColor: 'divider',
          }}
        >
          <Box>
            <Typography variant="body1">{item.item_name}</Typography>
            {item.manufacturer_name && (
              <Typography variant="caption" color="text.secondary">{item.manufacturer_name}</Typography>
            )}
          </Box>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flexShrink: 0 }}>
            <Typography variant="subtitle1" fontWeight={700}>₪{item.item_price.toFixed(2)}</Typography>
            <Button
              size="small"
              variant={contains(item.item_name) ? 'contained' : 'outlined'}
              onClick={() => addItem({ barcode: null, name: item.item_name })}
              startIcon={<AddShoppingCartIcon fontSize="small" />}
              sx={{ minWidth: 0, px: 1 }}
            >
              {contains(item.item_name) ? 'בסל' : 'הוסף'}
            </Button>
          </Box>
        </Box>
      ))}

      <div ref={sentinelRef} style={{ height: 1 }} />

      {loadingMore && (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 3 }}>
          <CircularProgress size={24} />
        </Box>
      )}

      {!hasMore && items.length > 0 && !loadingMore && (
        <Typography variant="body2" color="text.secondary" textAlign="center" sx={{ mt: 2 }}>
          הוצגו כל {items.length} הפריטים
        </Typography>
      )}
    </Container>
  );
};

export default StoreDetailPage;
```

**Step 2: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit 2>&1
```

**Step 3: Commit**

```bash
git add frontend/src/pages/StoreDetailPage.tsx
git commit -m "feat: add StoreDetailPage with paginated item list and add-to-cart"
```

---

### Task 16: Rewrite ItemsPage with real API data

**Files:**
- Modify: `frontend/src/pages/ItemsPage.tsx`

**Step 1: Replace ItemsPage.tsx**

```typescript
import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Alert, Box, Button, CircularProgress, Container,
  Slider, TextField, Typography,
} from '@mui/material';
import AddShoppingCartIcon from '@mui/icons-material/AddShoppingCart';
import { apiService } from '../services/api';
import { StoreItemRow } from '../types';
import { useCart } from '../context/CartContext';

const PAGE_SIZE = 30;

const ItemsPage: React.FC = () => {
  const { addItem, contains } = useCart();
  const [items, setItems] = useState<StoreItemRow[]>([]);
  const [query, setQuery] = useState('');
  const [priceRange, setPriceRange] = useState<[number, number]>([0, 500]);
  const [page, setPage] = useState(1);
  const [total, setTotal] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searched, setSearched] = useState(false);

  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pageRef = useRef(page);
  const hasMoreRef = useRef(hasMore);
  const loadingMoreRef = useRef(loadingMore);
  const queryRef = useRef(query);
  const priceRangeRef = useRef(priceRange);

  useEffect(() => { pageRef.current = page; }, [page]);
  useEffect(() => { hasMoreRef.current = hasMore; }, [hasMore]);
  useEffect(() => { loadingMoreRef.current = loadingMore; }, [loadingMore]);
  useEffect(() => { queryRef.current = query; }, [query]);
  useEffect(() => { priceRangeRef.current = priceRange; }, [priceRange]);

  const fetchPage = useCallback(async (pageNum: number, append: boolean) => {
    const q = queryRef.current;
    const [min, max] = priceRangeRef.current;

    if (pageNum === 1) { setLoading(true); setError(null); }
    else setLoadingMore(true);

    try {
      const data = await apiService.searchItemsPaginated(
        q, min > 0 ? min : undefined, max < 500 ? max : undefined, pageNum, PAGE_SIZE
      );
      setItems(prev => append ? [...prev, ...data.items] : data.items);
      setTotal(data.total);
      setHasMore(data.has_more);
      setPage(pageNum);
      setSearched(true);
    } catch {
      setError('שגיאה בחיפוש מוצרים');
    } finally {
      setLoading(false);
      setLoadingMore(false);
    }
  }, []);

  const triggerSearch = () => {
    setItems([]);
    setPage(1);
    fetchPage(1, false);
  };

  const handleQueryChange = (val: string) => {
    setQuery(val);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      setItems([]);
      setPage(1);
      fetchPage(1, false);
    }, 400);
  };

  // Infinite scroll
  useEffect(() => {
    const el = sentinelRef.current;
    if (!el) return;
    const observer = new IntersectionObserver(entries => {
      if (entries[0].isIntersecting && hasMoreRef.current && !loadingMoreRef.current) {
        fetchPage(pageRef.current + 1, true);
      }
    }, { threshold: 0.1 });
    observer.observe(el);
    return () => observer.disconnect();
  }, [fetchPage, searched]);

  return (
    <Container maxWidth="md" sx={{ mt: 3, mb: 8 }}>
      <Typography variant="h5" fontWeight={700} sx={{ mb: 2 }}>חיפוש מוצרים</Typography>

      {/* Search bar */}
      <TextField
        fullWidth
        size="small"
        label="חפש מוצר"
        value={query}
        onChange={e => handleQueryChange(e.target.value)}
        onKeyDown={e => { if (e.key === 'Enter') triggerSearch(); }}
        sx={{ mb: 2 }}
      />

      {/* Price range */}
      <Box sx={{ px: 1, mb: 3 }}>
        <Typography variant="body2" color="text.secondary" gutterBottom>
          טווח מחיר: ₪{priceRange[0]} – ₪{priceRange[1] < 500 ? priceRange[1] : '500+'}
        </Typography>
        <Slider
          value={priceRange}
          onChange={(_, val) => setPriceRange(val as [number, number])}
          onChangeCommitted={() => triggerSearch()}
          min={0} max={500} step={5}
          valueLabelDisplay="auto"
          valueLabelFormat={v => `₪${v}`}
        />
      </Box>

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

      {loading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', mt: 4 }}><CircularProgress /></Box>
      ) : (
        <>
          {searched && (
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {total.toLocaleString()} מוצרים · מציג {items.length}
            </Typography>
          )}

          {items.map(item => (
            <Box
              key={`${item.item_code}-${item.item_name}`}
              sx={{
                display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                py: 1.5, borderBottom: '1px solid', borderColor: 'divider',
              }}
            >
              <Box>
                <Typography variant="body1">{item.item_name}</Typography>
                {item.manufacturer_name && (
                  <Typography variant="caption" color="text.secondary">{item.manufacturer_name}</Typography>
                )}
              </Box>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flexShrink: 0 }}>
                <Typography variant="subtitle1" fontWeight={700}>₪{item.item_price.toFixed(2)}</Typography>
                <Button
                  size="small"
                  variant={contains(item.item_name) ? 'contained' : 'outlined'}
                  onClick={() => addItem({ barcode: null, name: item.item_name })}
                  startIcon={<AddShoppingCartIcon fontSize="small" />}
                  sx={{ minWidth: 0, px: 1 }}
                >
                  {contains(item.item_name) ? 'בסל' : 'הוסף'}
                </Button>
              </Box>
            </Box>
          ))}

          <div ref={sentinelRef} style={{ height: 1 }} />

          {loadingMore && (
            <Box sx={{ display: 'flex', justifyContent: 'center', py: 3 }}>
              <CircularProgress size={24} />
            </Box>
          )}

          {searched && !hasMore && items.length > 0 && !loadingMore && (
            <Typography variant="body2" color="text.secondary" textAlign="center" sx={{ mt: 2 }}>
              הוצגו כל {items.length} הפריטים
            </Typography>
          )}

          {searched && items.length === 0 && !loading && (
            <Typography variant="body1" color="text.secondary" textAlign="center" sx={{ mt: 4 }}>
              לא נמצאו מוצרים. נסה מילת חיפוש אחרת.
            </Typography>
          )}
        </>
      )}
    </Container>
  );
};

export default ItemsPage;
```

**Step 2: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit 2>&1
```

**Step 3: Commit**

```bash
git add frontend/src/pages/ItemsPage.tsx
git commit -m "feat: ItemsPage wired to real GET /api/items with infinite scroll and add-to-cart"
```

---

### Task 17: Update App.tsx routes

**Files:**
- Modify: `frontend/src/App.tsx`

**Step 1: Add StoreDetailPage import and `/stores/:id` route**

```typescript
// Add import:
import StoreDetailPage from './pages/StoreDetailPage';

// Add route inside <Routes>:
<Route path="/stores/:id" element={<StoreDetailPage />} />
```

The `/compare` route and `/cart` route (which now redirects) can stay as-is.

**Step 2: Verify TypeScript compiles**

```bash
cd frontend && npx tsc --noEmit 2>&1
```

Expected: no errors.

**Step 3: Commit**

```bash
git add frontend/src/App.tsx
git commit -m "feat: add /stores/:id route for StoreDetailPage"
```

---

## Phase 4 — End-to-End Verification

### Task 18: Full build + smoke test

**Step 1: Build backend**

```bash
cd backend && cargo build --release 2>&1 | tail -5
```

Expected: `Finished release [optimized] target(s)`

**Step 2: Build frontend**

```bash
cd frontend && npm run build 2>&1 | tail -10
```

Expected: `Compiled successfully.`

**Step 3: Start services and manually verify each feature**

```bash
# Terminal 1 — backend
cd backend && cargo run --bin backend

# Terminal 2 — frontend
cd frontend && npm start
```

Open http://localhost:3001 and check:

| Feature | What to verify |
|---|---|
| Navigation | White bar, active underline on current route, cart badge |
| ComparePage | Auto-prompts for GPS; if denied shows city input; adding items updates badge |
| Compare results | With city "תל אביב" — results show only stores in that city |
| Stores map | `/stores` shows a Leaflet map with store markers |
| Store search | Typing filters the list below the map |
| Store detail | Click a store → `/stores/:id` shows item list with Add to cart |
| Items page | `/items` search loads real items; infinite scroll works; Add to cart |
| Mobile | Resize to < 600px — bottom nav appears instead of top nav |

**Step 4: Final commit + push**

```bash
git add -A
git status  # verify no .env or sensitive files
git commit -m "feat: complete UI redesign — location filtering, stores map, store detail, real items API"
git push origin main
```
