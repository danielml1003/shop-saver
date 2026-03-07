# Shop-Saver — Architecture Roadmap

## What the app does

A user builds a grocery list, optionally shares their location, and the app shows them every nearby supermarket ranked by total basket price — so they can see at a glance that the exact same list costs ₪23 less at Rami Levy vs Shufersal two streets away.

---

## Current Architecture

```
Retailer websites (XML price lists published daily)
        |
        v
[Python service] — downloads & extracts XML files --> service/downloads/*.xml
        |
        v  (filesystem watcher, auto-triggered)
[Rust backend / Axum] — parses XML, inserts into PostgreSQL
        |
        v
[PostgreSQL]
  stores (chain_id, store_id, address, city, lat, lng)
  items  (store_pk, item_code, item_name, item_price, ...)
  processed_files (dedup log)
        |
        v
[REST API]
  GET  /api/stores/nearby        — stores within radius
  POST /api/compare-prices       — basket vs all/nearby stores
  GET  /api/items/search         — autocomplete
        |
        v
[React / TypeScript frontend]
  ComparePage  — main feature: build list, compare, see best store
  CartPage     — save/manage list
  ItemsPage    — browse products
  StoresPage   — list all branches
```

---

## The Core Problem: Item Identity Across Stores

Each chain publishes its own `ItemCode`. Most packaged goods use the international EAN-13/GS1 barcode (13-digit), so `ItemCode = 7290000066768` means Tnuva Milk 1L in **every** store. But some chains use internal codes for store-brand or weighted items.

### Current matching (what exists now)
The compare endpoint does `LIKE '%user_typed_name%'` per store. This works but is fragile — it misses items if the store writes the name differently, and it can false-positive on partial matches.

### Target matching (what we need)
1. **Primary: barcode match** — if `item_code` is a valid EAN-13, it uniquely identifies the product. One query: `SELECT store_pk, MIN(item_price) FROM items WHERE item_code = $1 GROUP BY store_pk`. Instant, exact, cross-store.
2. **Fallback: fuzzy name match** — for non-standard codes (store brands, produce), use the existing trigram LIKE search as a fallback.

---

## Roadmap

### Phase 1 — Make cross-store comparison actually work
**Status: done**

- [x] **Product catalog table** (`products`) — `barcode VARCHAR(13) PRIMARY KEY`, `canonical_name`, `manufacturer`, `quantity`, `unit_of_measure`. Auto-populated during XML ingest.
- [x] **EAN-13 validator** — `is_ean13()` in `database.rs`, validates 13 digits + check digit.
- [x] **Barcode-first compare logic** — `find_items_for_stores` partitions grocery list into barcodes (exact `item_code =` lookup) and name terms (LIKE fallback). Missing items tracked by term index, not string match.
- [x] **Autocomplete returns barcodes** — `search_item_names` LEFT JOINs `items → products`, returning `{barcode, name}`. Frontend sends barcodes to API; UI shows names.
- [x] **Products upserted on ingest** — `xml_processor.rs` calls `upsert_product` for every EAN-13 item inserted.

---

### Phase 2 — Store geocoding (needed for "near me" to work)
**Status: done**

- [x] **Geocode stores** — `service/geocode_stores.py` reads all stores with NULL lat/lng from the DB, queries Nominatim (OpenStreetMap, free, no API key), and writes coordinates back. Re-run safe. Two-step fallback: full address → city only.
- [x] **Chain ID → display name mapping** — `chain_names` table in PostgreSQL, seeded at backend startup. All store queries LEFT JOIN it and COALESCE `store_name` with the chain display name. Mapping also stored in `chain_names.json` at project root.
- [x] **New dependencies** — `psycopg2-binary`, `python-dotenv` added to `requirements.txt`.

---

### Phase 3 — Full retailer coverage
**Status: done**

- [x] **Shufersal** — `shufersal.py`: scrapes paginated HTML listing at `prices.shufersal.co.il`, downloads signed Azure Blob `.gz` files. Fetches PriceFull (catID=2) + StoresFull (catID=5).
- [x] **Rami Levy** — `rami_levy.py`: uses CeberusStoreDownloader with FTP credentials `ramilevi/ramilevi`, active mode enabled.
- [x] **ceberus.py updated** — added optional `ftp_active_mode` config key for chains that need active FTP.
- [x] **Yohananof** — `yohananof.py`: Cerberus FTP, username `yohananof`, ChainId `7290803800003`, active mode. Verified 20 files/day. Live XML download tested.
- [x] **Osher Ad** — `osher_ad.py`: Cerberus FTP, username `osherad`, ChainId `7290103152017`, active mode. Stores files served as plain `.xml` (handled by base.py).
- [x] **base.py patched** — `_extract_file` now returns pre-extracted `.xml` files immediately instead of failing.
- [x] **All 12 chains wired** — `ALL_CHAINS` in `__init__.py` includes King, Maayan, GoodPharm, DorAlon, TivTaam, Victory, ZolVeGadol, Carrefour, Shufersal, RamiLevy, Yohananof, OsherAd.
- [ ] **Cron job** wired up and running on a production server (Phase 4 covers this for Linux; Windows requires Task Scheduler)

---

### Phase 4 — Pipeline automation
**Status: done**

- [x] **`service/run_pipeline.sh`** — wrapper script: runs all downloaders then geocodes new stores, full logging to `/var/log/shop-saver/pipeline.log`, loads `.env`, dry-run mode supported.
- [x] **`service/cronjob`** — updated cron schedule: 6 AM (main run after chains publish) + 1 PM (afternoon update chains). Installed to `/etc/cron.d/shop-saver` by setup.
- [x] **`backend/shop-saver-api.service`** — systemd unit for the Rust API: auto-restart on crash, depends on PostgreSQL, security hardened (`NoNewPrivileges`, `PrivateTmp`, etc.).
- [x] **`setup.sh`** — one-time server setup: creates system user, installs Python deps, builds Rust release binary, installs systemd service + cron, runs initial geocoding.
- [ ] **Error reporting** — if a downloader fails (retailer changed URL format), send an alert (future)
- [ ] **Stale data detection** — flag items where `price_update_date` > 2 days old in the UI (future)

---

### Phase 5 — UX improvements
**Status: done (core features)**

- [x] **Barcode scanner** — camera icon next to search field (shown only when `BarcodeDetector` API is supported — Chrome/Android). Opens a live camera dialog, auto-detects EAN-13/QR, resolves to product name via search API, adds to list.
- [x] **Save & share list** — "שתף רשימה" button encodes grocery list (barcodes + names) as base64 JSON in URL (`?q=...`). Copies to clipboard with confirmation tick. On load, URL is automatically decoded to pre-populate the list.
- [x] **Per-item price breakdown** — store cards now show a proper table: item name + manufacturer on the left, price on the right. Missing items shown inline in amber with "לא נמצא בסניף זה".
- [x] **Mobile PWA** — `manifest.json` updated (Hebrew name, RTL, theme color). `service-worker.js` added (cache-first for app shell, network-first for API). Registered in `index.tsx` for production builds.
- [ ] **Price history** — sparkline showing price trend per item per store (future — requires historical data accumulation)

---

### Phase 6 — Production readiness (Docker deployment)
**Status: done (containerization complete)**

- [x] **`.env.example`** — template with `POSTGRES_PASSWORD` and `APP_PORT`
- [x] **`.dockerignore`** — excludes `backend/target`, `frontend/node_modules`, `service/downloads`, `__pycache__`
- [x] **`backend/Dockerfile`** — multi-stage Rust build (`rust:1.82-slim` → `debian:bookworm-slim`); dependency caching layer; binary exposed on port 3000
- [x] **`frontend/Dockerfile`** — multi-stage React build (`node:20-alpine` → `nginx:1.27-alpine`); serves compiled SPA
- [x] **`frontend/nginx.conf`** — SPA fallback routing, `/api/` and `/health` proxied to `api:3000`, gzip + 1y cache for hashed assets
- [x] **`service/Dockerfile`** — Python 3.12-slim + cron; runs pipeline at 06:00 and 13:00 UTC; shares `downloads` volume with API
- [x] **`docker-compose.yml`** — four services: `db` (postgres:16, healthcheck), `api` (Rust backend), `service` (Python pipeline), `frontend` (nginx); shared `downloads` and `pgdata` volumes
- [ ] **Authentication** — optional user accounts to save lists and preferences (future)
- [ ] **Rate limiting** on the API (future)
- [ ] **Monitoring** — `/health` endpoint exists; wire to uptime monitor (future)

---

## Priority Order

| Priority | Task | Why |
|---|---|---|
| ~~1~~ | ~~Product catalog + barcode-first matching~~ | ~~Core feature doesn't work correctly without it~~ ✓ |
| ~~2~~ | ~~Store geocoding~~ | ~~"Near me" is broken without coordinates~~ ✓ |
| ~~3~~ | ~~Chain ID display names~~ | ~~UX is confusing with raw numeric IDs~~ ✓ |
| ~~4~~ | ~~Shufersal + Rami Levy downloaders~~ | ~~Covers ~60% of Israeli supermarkets~~ ✓ |
| ~~5~~ | ~~Automated cron~~ | ~~Data goes stale without it~~ ✓ |
| ~~6~~ | ~~Barcode scanner UI~~ | ~~Big UX win on mobile~~ ✓ |
| 7 | Price history | Differentiating feature |

---

## Key Files Reference

| File | What it does |
|---|---|
| `service/downloaders/base.py` | Abstract downloader — download, decompress, save XML |
| `service/storesJsonUrl.py` | Store configurations (URLs, chain IDs, store IDs) |
| `backend/src/xml_processor.rs` | Watches download dir, parses XML, inserts into DB |
| `backend/src/database.rs` | All DB queries — compare logic lives here |
| `backend/src/models.rs` | Rust structs for XML parsing and API responses |
| `backend/src/api.rs` | Axum route handlers |
| `frontend/src/pages/ComparePage.tsx` | Main UI — basket input, compare, results |
| `frontend/src/services/api.ts` | Frontend HTTP calls to the backend |
