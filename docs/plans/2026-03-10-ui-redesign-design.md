# Design: UI Redesign + Location Filtering + Cart Integration

**Date:** 2026-03-10
**Status:** Approved

---

## Problem Statement

The current app has four interrelated issues:

1. **Shows all 300+ stores across Israel** — no location filtering by default; users in Tel Aviv see stores from Be'er Sheva.
2. **Location button doesn't help** — when GPS location is provided, the backend returns all nearby stores regardless of whether they carry any of the requested items. Pages fill with stores showing "0/3 items found."
3. **Outdated, inconsistent UI** — CartPage and ComparePage have duplicate grocery-list state that doesn't stay in sync; ItemsPage and StoresPage use entirely mock data.
4. **No store detail view** — no way to browse a specific store's items or add them to the grocery list.

---

## Design

### 1. Navigation Redesign (`Header.tsx`)

- White sticky top bar with bottom border (no more flat blue AppBar)
- Logo: shopping cart icon + "ShopSaver" bold text on the left
- Nav links on the right: "השוואה", "מוצרים", "חנויות", "הסל שלי" + item-count badge
- Active link: blue underline + primary color text
- Mobile (< 600px): bottom navigation bar with 4 icon buttons replacing top links

### 2. ComparePage + Cart Integration

**Unified grocery list state:**
- `CartContext` becomes the single source of truth for the grocery list
- `ComparePage` reads and writes items via `CartContext` (no more local `items` state)
- List persists across navigation (go to Stores page, come back, list is still there)
- Cart badge in nav reflects live count

**Location UX:**
- On mount, silently call `navigator.geolocation.getCurrentPosition()`
- If granted: store GPS coordinates in state, show green "📍 GPS active" indicator
- If denied: show a small city text input field inline below the item input
- Either GPS or city is passed to the compare API

**`CartPage` removal:**
- CartPage is removed; `/cart` route redirects to `/compare`
- All comparison functionality lives in ComparePage

### 3. Backend: `POST /api/compare-prices` Changes

Add optional `city: Option<String>` to `PriceComparisonRequest`.

New routing logic in `compare_prices()`:

```
if user_location present:
    nearby_ids  = get_nearby_store_ids(lat, lon, radius_km)
    item_sets   = per-term barcode/LIKE queries → Vec<HashSet<i32>>
    candidates  = nearby_ids ∩ union(item_sets)
    sort by coverage DESC, total_price ASC → paginate

else if city present:
    city_ids    = SELECT id FROM stores WHERE LOWER(city) LIKE '%{city}%'
    item_sets   = per-term queries
    candidates  = city_ids ∩ union(item_sets)
    sort + paginate

else:
    existing behavior (all stores, item-filtered)
```

Key change: the location path pre-filters by item coverage before pagination, so pages no longer contain stores with 0 items found.

### 4. Stores Page — Map View (`StoresPage.tsx`)

- Uses `react-leaflet` + OpenStreetMap tiles (free, no API key)
- Full-width map (400px tall) at top with a marker per store that has lat/lng
- Below map: searchable/filterable list of stores (by city or name)
- Clicking a marker or list row → navigates to `/stores/:id`

**New backend endpoint:** `GET /api/stores`
Returns all stores: `id, chain_id, store_id, store_name, address, city, latitude, longitude`.

### 5. Store Detail Page (new `StoreDetailPage.tsx`, route `/stores/:id`)

- Store header: name, chain, address, city
- Paginated item list with search input
- Each item has an "+ Add to cart" button (adds to CartContext grocery list)

**New backend endpoint:** `GET /api/stores/:id/items?q=&page=&limit=`
Returns paginated items for one store: `{items: [...], total, has_more}`.

### 6. Items Page — Real Data + Filter (`ItemsPage.tsx`)

- Replace mock `getItems()` call with real `GET /api/items` backend endpoint
- Filter bar: search box (existing autocomplete), price range slider
- Each item card has "+ Add to cart" button
- Infinite scroll (same pattern as ComparePage results)

**New backend endpoint:** `GET /api/items?q=&min_price=&max_price=&page=&limit=`
Returns paginated items across all stores: `{items: [...], total, has_more}`.

---

## Visual Style

- **Background:** white (`#FFFFFF`) page, light grey surface (`#F5F5F5`) for cards
- **Primary:** `#1976d2` (existing blue)
- **Success/Savings:** `#2e7d32` green
- **Cards:** `elevation=1`, `borderRadius: 2` (8px), subtle shadow
- **Typography:** MUI defaults, `h5`/`h6` for card titles, `body2` for secondary info
- **Spacing:** consistent 8px grid throughout

---

## Files Changed

### Backend (`backend/src/`)
| File | Change |
|---|---|
| `models.rs` | Add `city: Option<String>` to `PriceComparisonRequest` |
| `database.rs` | Refactor `compare_prices()` for location+item intersection; add `get_all_stores()`, `get_store_items()`, `search_items_paginated()` |
| `api.rs` | Add `GET /api/stores`, `GET /api/stores/:id/items`, `GET /api/items` handlers |

### Frontend (`frontend/src/`)
| File | Change |
|---|---|
| `components/Header.tsx` | Full redesign — white bar, underline active, bottom nav on mobile |
| `context/CartContext.tsx` | Expose `GroceryItem[]` grocery list + `addItem`/`removeItem`/`clearCart` |
| `pages/ComparePage.tsx` | Use CartContext for list; auto-GPS on mount; city input fallback |
| `pages/CartPage.tsx` | Remove — replace with redirect to `/compare` |
| `pages/StoresPage.tsx` | Add react-leaflet map; wire to real `GET /api/stores`; click → detail |
| `pages/StoreDetailPage.tsx` | **New** — store info + paginated items with "Add to cart" |
| `pages/ItemsPage.tsx` | Wire to real `GET /api/items`; infinite scroll; "Add to cart" per item |
| `services/api.ts` | Add `getAllStores()`, `getStoreItems()`, `searchItems()` methods; update `comparePrices()` to accept `city` |
| `types/index.ts` | Add `city?: string` to `PriceComparisonRequest`; add response types for new endpoints |
| `App.tsx` | Add `/stores/:id` route; redirect `/cart` → `/compare` |

### Dependencies to add
- `react-leaflet` + `leaflet` (map rendering)
- `@types/leaflet` (TypeScript types)

---

## What This Does NOT Change

- Barcode scanner (stays as-is in ComparePage)
- Share list button (stays as-is)
- Infinite scroll on compare results (stays as-is)
- Backend XML processor / downloader pipeline
- Database schema (no migrations needed)
