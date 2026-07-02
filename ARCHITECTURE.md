# Shop-Saver — Architecture, Refactoring Notes & Completion Plan

> Last updated: 2026-07-02.
> Companion docs: [`ROADMAP.md`](ROADMAP.md) (feature roadmap, phases 1–6),
> [`docs/plans/2026-03-10-ui-redesign.md`](docs/plans/2026-03-10-ui-redesign.md) (the active implementation plan).

---

## 1. What the project does

A user builds a grocery list (by typing, autocomplete, or scanning a barcode), optionally shares
their location or city, and the app ranks every matching supermarket branch in Israel by the
**total price of that exact basket** — so they can see the same list costs less at one chain
than another a few streets away.

---

## 2. How the architecture is built

### 2.1 High-level data flow

```
Israeli retailer price feeds (XML published daily — gov. price-transparency law)
        │
        ▼
[Python service]  service/downloaders/*  — 12 chains
   downloads .gz/.zip/.xml → extracts → service/downloads/*.xml   (or /downloads volume in Docker)
        │
        ▼  (filesystem watcher, auto-triggered)
[Rust backend — Axum]  backend/src/xml_processor.rs
   parses PriceFull/StoresFull XML → inserts into PostgreSQL, dedups via processed_files
        │
        ▼
[PostgreSQL 16]
   stores · items · products (EAN-13 catalog) · chain_names · processed_files
        │
        ▼
[REST API]  backend/src/api.rs
   GET  /health
   GET  /api/stores/nearby        — stores within a radius (haversine)
   GET  /api/stores               — all stores (map page)          ← DB layer done, route pending
   GET  /api/stores/:id/items     — paginated store inventory      ← DB layer done, route pending
   GET  /api/items                — paginated cross-store search   ← pending
   GET  /api/items/search         — autocomplete (name → barcode)
   POST /api/compare-prices       — basket vs all/nearby/city stores, paginated
        │
        ▼
[React 19 / TypeScript frontend — MUI, Hebrew RTL, PWA]
   ComparePage   — main feature: build list, compare, per-item breakdown, barcode scanner, share link
   CartPage      — grocery list management (being merged into ComparePage via CartContext)
   ItemsPage     — browse products (currently mock data — being wired to /api/items)
   StoresPage    — store list (being rewritten with a react-leaflet map)
```

### 2.2 The three tiers

#### Python ingestion service (`service/`)

| Piece | Role |
|---|---|
| `downloaders/base.py` | Abstract downloader: URL generation, download with retries/timeouts, magic-number detection of `.gz` / `.zip` / plain `.xml`, extraction, cleanup. |
| `downloaders/original.py`, `one.py`, `ceberus.py`, `shufersal.py`, `carrefour.py`, … | Per-publishing-platform subclasses: "original"-type HTTP hosts, ZIP hosts, Cerberus FTP (Rami Levy, Yohananof, Osher Ad — some need active FTP mode), Shufersal's paginated Azure-blob listing, etc. |
| `downloaders/__init__.py` | `ALL_CHAINS` — the 12 wired chains: King, Maayan, GoodPharm, DorAlon, TivTaam, Victory, ZolVeGadol, Carrefour, Shufersal, RamiLevy, Yohananof, OsherAd. |
| `storesJsonUrl.py` | Per-retailer config dictionaries (base URL, chain ID, store IDs, timestamp format). |
| `geocode_stores.py` | Fills NULL lat/lng in `stores` via Nominatim (full address → city fallback). Re-run safe. |
| `run_pipeline.sh` + `cronjob` | Runs all downloaders then geocoding at 06:00 and 13:00; logs to `/var/log/shop-saver/`. |

#### Rust backend (`backend/src/`)

| File | Role |
|---|---|
| `main.rs` | Entry point: env config, DB connect, spawns the XML watcher + background scan of existing files, mounts the Axum router with CORS/tracing. |
| `xml_processor.rs` | `notify`-based directory watcher; parses store & price XML, upserts stores/items, populates the `products` catalog for every valid EAN-13, records files in `processed_files`. |
| `database.rs` | **All business logic lives here.** Nearby-store haversine query, barcode-first basket matching (`is_ean13()` → exact `item_code` lookup; non-barcodes fall back to `LIKE`), store-coverage ranking + pagination, chain-name COALESCE joins. Largest file (~700 lines). |
| `api.rs` | Thin Axum handlers — validate input, call `DatabaseManager`, map errors to status codes. |
| `models.rs` | Serde structs for XML parsing, requests, and responses. |
| `main_new.rs` | ⚠️ Leftover duplicate of `main.rs` — not referenced by `Cargo.toml`, dead code (see §3). |

**Core matching design** (the heart of the app): each chain publishes its own `ItemCode`, but most
packaged goods use the international EAN-13 barcode, so the same code identifies the same product in
every chain. The compare endpoint partitions the grocery list into *barcodes* (exact indexed lookup,
cross-store correct) and *free-text names* (LIKE fallback for produce/store brands). Autocomplete
returns `{barcode, name}` pairs so the frontend sends barcodes whenever possible.

#### React frontend (`frontend/src/`)

- `pages/` — ComparePage (main flow), CartPage, ItemsPage, StoresPage (+ StoreDetailPage planned).
- `components/` — Header (nav), ItemCard, SearchFilters.
- `context/CartContext.tsx` — shared grocery-list state.
- `services/api.ts` — Axios wrapper; compare/nearby/autocomplete hit the real backend, **the rest still returns hardcoded mock data** (see §3).
- `types/index.ts` — all shared interfaces.
- PWA: `manifest.json` + cache-first/network-first `service-worker.js`; barcode scanning via the browser `BarcodeDetector` API.

### 2.3 Deployment

Two supported modes:

1. **Docker Compose** (`docker-compose.yml`): 4 services — `db` (postgres:16 + healthcheck),
   `api` (multi-stage Rust build), `service` (Python + cron in-container), `frontend`
   (React build served by nginx, which also proxies `/api/` and `/health` to the api container).
   Shared `downloads` and `pgdata` volumes. Config via `.env` (`POSTGRES_PASSWORD`, `APP_PORT`).
2. **Bare-metal Linux** (`setup.sh`): system user, Python deps, Rust release build,
   `backend/shop-saver-api.service` systemd unit (hardened, auto-restart), `/etc/cron.d/shop-saver`.

---

## 3. What needs to be changed or refactored

Ordered roughly by importance.

### 3.1 Correctness / hygiene

1. **Delete `backend/src/main_new.rs`.** It's an older copy of `main.rs`, not wired into
   `Cargo.toml`, and will silently drift. One binary, one entry point.
2. **Remove mock data & dead methods from `frontend/src/services/api.ts`.**
   `getStores`, `getItems`, `getItemsByStore`, `searchItems`, `getLatestPrices` all return
   hardcoded `mockStores`/`mockItems`, plus there's a commented-out duplicate Axios instance.
   Once the three new endpoints land (§4, tasks 6–9 of the UI plan), delete the mocks and the
   methods that duplicate them.
3. **Panic-prone URL masking in `main.rs:37-38`.** The `safe_db_url` string slicing assumes the URL
   contains `://` *and* `@`; a URL without credentials slices with mismatched indices. Replace with
   a proper parse (or just log host/db name).
4. **Consolidate schema management.** Today the schema comes from three places: tables created at
   runtime by the Rust app, `init.sql` (essentially empty), and `backend/migrations/001_add_store_locations.sql`.
   Move everything into `sqlx` migrations (`sqlx::migrate!`) run at startup, and delete the ad-hoc
   `CREATE TABLE` code paths. This makes schema changes reviewable and reproducible.
5. **Duplicate/overlapping search endpoints.** `/api/items/search` (autocomplete) and the planned
   `/api/items` (paginated browse) are similar; keep both but document the distinction clearly in
   `api.rs`, or fold autocomplete into `/api/items?mode=suggest`.

### 3.2 Performance

6. **N+1 queries in the compare path** (`database.rs`): both `get_stores_with_items` and the new
   `get_stores_with_items_from_set` issue **one query per grocery-list term** (per barcode and per
   name). A 20-item basket = 20 round trips. Refactor to a single query using
   `item_code = ANY($1)` for barcodes and a joined `unnest()` of name patterns, grouping by
   `store_pk` to compute coverage in SQL.
7. **Unindexed `LIKE '%…%'` name search.** Every fallback name match is a sequential scan over
   `items` (millions of rows once all 12 chains ingest daily). Add the `pg_trgm` extension and a
   GIN trigram index on `LOWER(item_name)`; the existing queries then use it with no code changes.
8. **Items table growth.** Daily full-price ingestion appends forever. Decide the retention policy:
   either upsert on `(store_pk, item_code)` keeping only the latest price, or keep history in a
   separate `price_history` table (needed anyway for the roadmap's price-history feature) and keep
   `items` as "current prices only".

### 3.3 Security / production hardening

9. **CORS is wide open** (`CorsLayer::new().allow_origin(Any)` in `main.rs`). Fine in dev; in the
   Docker deployment nginx proxies same-origin, so restrict allowed origins via env var.
10. **Rate limiting** on `POST /api/compare-prices` (it's the most expensive endpoint) — e.g.
    `tower-governor`. Currently anyone can hammer it.
11. **FTP credentials in source** (`rami_levy.py` etc. use the retailers' published public
    credentials — acceptable since they're public, but move them into `storesJsonUrl.py` config or
    env vars so all chain config lives in one place).

### 3.4 Code organization / naming

12. **`ceberus.py` → `cerberus.py`.** The platform is called Cerberus; the typo propagates into
    class names (`CeberusStoreDownloader`) and imports across 4+ files. Rename once, now, before
    more chains are added.
13. **Split `database.rs` (~700 lines).** It mixes schema setup, store queries, item queries, and
    compare logic. Suggested split: `db/mod.rs` (pool + migrations), `db/stores.rs`, `db/items.rs`,
    `db/compare.rs`.
14. **Audit unused downloaders.** `mega.py` and `one.py` exist but Mega isn't in `ALL_CHAINS`.
    Either finish wiring them or delete them — half-wired modules confuse contributors.
15. **`README.md` is stale.** It still describes the service as "prints raw XML" and the frontend
    as fully mocked; both are long superseded. Rewrite it as a short landing page pointing at
    `QUICK_START.md`, this file, and `ROADMAP.md`.

### 3.5 Testing (currently ~zero)

16. **Backend**: unit tests for `is_ean13()`, XML parsing fixtures (one sample PriceFull +
    StoresFull file per publishing platform checked into `backend/tests/fixtures/`), and
    integration tests for `compare_prices` against a dockerized Postgres (`sqlx::test`).
17. **Service**: `base.py` extraction logic (gz/zip/plain-xml magic numbers) is pure and easy to
    test; URL generators per chain are also pure functions.
18. **Frontend**: `App.test.tsx` is the untouched CRA default. Minimum: a render test per page and
    a test for the grocery-list/CartContext behavior (add/remove/dedupe/share-link encode-decode).

---

## 4. How to finish the project

### Step 1 — Finish the in-flight UI redesign (the active work)

The plan in [`docs/plans/2026-03-10-ui-redesign.md`](docs/plans/2026-03-10-ui-redesign.md) is
**3 of 18 tasks done** (models + `get_all_stores()` + `get_store_items()` are committed). Remaining,
in order — each task in the plan has exact code, verification commands, and a commit message:

- [ ] **Task 4** — `search_items_paginated()` in `database.rs`
- [ ] **Task 5** — `compare_prices()` city filter + location∩items intersection helper
- [ ] **Task 6** — register the 3 new routes in `api.rs` (`GET /api/stores`, `GET /api/stores/:id/items`, `GET /api/items`) + smoke test with curl
- [ ] **Task 7** — `npm install react-leaflet leaflet @types/leaflet`
- [ ] **Task 8–9** — frontend types + `apiService` methods for the new endpoints
- [ ] **Task 10** — CartContext switches to `GroceryItem[]` (barcode + name)
- [ ] **Task 11** — Header redesign (sticky bar, active underline, mobile bottom nav)
- [ ] **Task 12** — ComparePage: CartContext + auto-GPS on mount + city fallback input
- [ ] **Task 13** — CartPage becomes a redirect to `/`
- [ ] **Task 14** — StoresPage rewrite with react-leaflet map
- [ ] **Task 15** — new StoreDetailPage (`/stores/:id`, infinite scroll, add-to-cart)
- [ ] **Task 16** — ItemsPage wired to real `GET /api/items` (kills the mock data)
- [ ] **Task 17** — routes in `App.tsx`
- [ ] **Task 18** — full build + manual end-to-end verification checklist

### Step 2 — Clean up (from §3)

Do these right after Step 1, while the code is warm:

1. Delete `main_new.rs`; strip mocks from `api.ts`; rewrite `README.md`. *(1 hour)*
2. `pg_trgm` index + collapse the compare N+1 into set-based queries. *(half a day, biggest runtime win)*
3. sqlx migrations; decide items retention (upsert-latest + `price_history` table). *(a day)*
4. Rename `ceberus` → `cerberus`; delete or wire `mega.py`/`one.py`. *(1 hour)*
5. Add the test baseline from §3.5. *(1–2 days, pays for itself immediately)*

### Step 3 — Go to production

1. Server with Docker: `cp .env.example .env` (set a real password) → `docker compose up -d --build`.
2. Let the `service` container's cron do its first 06:00 run (or exec `run_pipeline.sh` manually
   once) → verify rows in `stores`/`items` → verify geocoding filled lat/lng.
3. Point a domain at it, put HTTPS in front (Caddy/Traefik or nginx + certbot), restrict CORS,
   wire `/health` to an uptime monitor (the endpoint already exists).
4. Open the remaining `ROADMAP.md` checkboxes as issues:
   - **Error reporting** — alert when a downloader fails (retailers change URL formats often; this
     is the most likely silent-failure mode).
   - **Stale-data detection** — flag items with `price_update_date` > 2 days old in the UI.
   - **Rate limiting** (§3.3).
   - **Price history** sparkline — unblocked once the `price_history` table from Step 2.3 exists.
   - **Authentication / saved lists** — optional; the share-by-URL feature already covers the core need.

### Definition of done

The project is "finished" (v1.0) when:

1. All 18 UI-redesign tasks are merged and the manual checklist in Task 18 passes.
2. Zero mock data anywhere in `frontend/src/services/api.ts`.
3. The pipeline runs unattended for a week on a real server with all 12 chains ingesting,
   and a failure of any downloader produces an alert instead of silence.
4. A compare of a 20-item basket over the full dataset responds in under ~1s (needs §3 items 6–7).
5. `cargo test`, `pytest`, and `npm test` all exist and pass in CI.
