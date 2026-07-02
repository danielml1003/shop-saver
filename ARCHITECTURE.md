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

## 4. Full store coverage — make the service and backend go through ALL stores

Goal: every branch of every chain is downloaded, parsed, and queryable — no store silently
missing from price comparisons.

### 4.1 Current coverage per chain (audited)

| Chain(s) | Downloader | How stores are found | Coverage today |
|---|---|---|---|
| Rami Levy, Yohananof, Osher Ad, Dor Alon, Tiv Taam | `ceberus.py` (Cerberus FTP) | `ftp.nlst()` lists **every** file on the server for today | ✅ **All stores** (the `StoreId` lists in `dor_alon.py`/`tiv_taam.py` are dead config — the FTP downloader ignores them) |
| Shufersal | `shufersal.py` | Paginated HTML listing, walks **all pages** | ✅ All stores |
| Carrefour | `mega.py` | Parses the site's embedded full file listing… | ⚠️ …but then **drops files modified more than 2 hours ago** (`cutoff = now - 2h`). Files uploaded outside the cron window are lost. |
| King, Maayan, GoodPharm, ZolVeGadol | `original.py` (binaprojects) | **Guesses URLs** from a hardcoded `StoreId` list × timestamps of only the **last 2 hours** (`_recent_timestamps()`) | ❌ New branches never appear (list is stale by definition), and any file uploaded outside the 2-hour window is missed |
| Victory | `one.py` (laibcatalog) | Guesses URLs from a hardcoded `StoreId` list × a **to-the-minute "now" timestamp** | ❌ Effectively broken — a guessed `YYYYMMDDHHMM` of "right now" almost never matches a published filename |

### 4.2 Service-side fixes (download everything)

The principle: **stop guessing filenames; ask the server what exists.** Every publishing
platform used by these chains has a listing mechanism:

1. **binaprojects chains** (`original.py` — King, Maayan, GoodPharm, ZolVeGadol):
   each site exposes a JSON file-listing endpoint (`MainIO_Hok.aspx` at the site root)
   that returns every file published today with its exact filename. Add a
   `_discover_urls()` step: fetch the listing, filter by `ChainId` + configured
   `WFileType`s, download `{Url}{FileNm}` for each entry. Keep the current
   StoreId×timestamp guessing only as a **fallback** when the listing endpoint fails,
   so the worst case is today's behavior.
2. **Victory** (`one.py`): scrape the laibcatalog listing page for `href`s ending in
   `.zip/.gz/.xml` that contain the chain ID, and download those. Fallback: replace the
   broken to-the-minute timestamp with hourly candidates over a ~26-hour window.
3. **Carrefour** (`mega.py`): the full listing is already parsed — just widen the
   hardcoded 2-hour cutoff to a configurable lookback (default ~26h, i.e. the whole
   publishing day). Re-downloading is harmless: the backend dedups by filename + size
   via `processed_files`.
4. **Cerberus chains**: nothing to do — already full coverage. Delete the unused
   `StoreId` lists from `dor_alon.py`/`tiv_taam.py` so nobody thinks they matter.
5. **Stop downloading Promo files.** Every chain config requests `Promo`/`PromoFull`,
   but the backend skips them (`xml_processor.rs` marks them processed without parsing).
   Roughly half the download bandwidth and time is spent on files that get thrown away —
   trim every `WFileType` list to `StoresFull`, `Price`, `PriceFull`.
6. **Per-chain observability**: log "chain X: N files downloaded" at the end of each
   `run_pipeline.sh` run and alert when any chain returns 0 files. Retailers change their
   sites without notice — a chain silently going to zero is the most common failure mode
   in this domain, and it's invisible without this counter. Verify coverage with
   `SELECT chain_id, COUNT(*) FROM stores GROUP BY chain_id` — the counts should roughly
   match each chain's real branch count.
7. **Fallback-first rollout**: implement discovery as the primary path and keep the current
   URL-guessing as the fallback, so a broken listing endpoint degrades to today's behavior
   instead of zero files. (Note: the retailer listing endpoints can't be reached from every
   network/sandbox — verify each one with a real pipeline run on the target server.)

### 4.2b Alternative: replace the downloaders with a maintained library

Instead of maintaining 12 custom scrapers, consider the open-source
**`il-supermarket-scraper`** package (PyPI; GitHub: `erlichsefi/israeli-supermarket-scrapers`).
It already implements listing-based discovery for all the publishing platforms used here
(binaprojects, Cerberus, Shufersal, laibcatalog, Carrefour and more), tracks retailer
format changes, and covers additional chains this project doesn't have yet.

- **What you'd keep**: only the glue that drops extracted XML into the watch directory —
  the Rust backend doesn't care who downloaded the file.
- **What you'd gain**: someone else absorbs the ongoing breakage when retailers change
  their sites — the single biggest maintenance cost of this project.
- **What you'd lose**: control over the download pattern, and the custom code as a
  learning artifact. A reasonable middle ground: keep `service/downloaders/` but use the
  library's source as the reference for each platform's listing endpoint and quirks.

### 4.3 Backend-side fixes (process everything that lands)

The Rust processor already handles every `.xml` in the watch directory, but two behaviors
can silently drop stores:

1. **Failed files are never retried** — `scan_existing_files()` marks a file as processed
   *"regardless of success or failure"* (`xml_processor.rs:106-110`). A transient DB error
   during one scan permanently skips that store's prices. Fix: only mark permanently on
   success or on *parse* errors (those never succeed on retry); leave transient failures
   unmarked so the next scan retries them.
2. **Promo files are only skipped in the scan path** — the live file watcher tries to parse
   `Promo*/PromoFull*` files and logs errors. Apply the same promo skip in the watcher
   callback for consistency (and mark them processed immediately).
3. **StoresFull ingestion** must run for every chain so branches get addresses/cities
   (needed for geocoding and "near me"). ZolVeGadol doesn't publish StoresFull — those
   branches are created bare from price files; geocoding then has nothing to geocode.
   Track such stores (`address IS NULL`) and backfill from a manual mapping if needed.

---

## 5. Cyber-security check

### 5.1 Automated security checks (add to CI)

Add a GitHub Actions workflow (`.github/workflows/security.yml`) that runs on every push,
every PR, and weekly on a schedule:

| Check | Tool | What it catches |
|---|---|---|
| Secret scanning | `gitleaks` | Committed passwords, API keys, tokens (run on full git history) |
| Rust dependency audit | `cargo audit` (RustSec) | Known CVEs in `backend/Cargo.lock` |
| Node dependency audit | `npm audit --audit-level=high` | Known CVEs in `frontend/package-lock.json` |
| Python dependency audit | `pip-audit -r requirements.txt` | Known CVEs in the service's Python deps |

Mirror the same four checks in a local `scripts/security_check.sh` so they can be run
before pushing. The project is "secure-checked" when this workflow is green and required
on the main branch.

### 5.2 Manual security checklist (current findings)

Reviewed against the current code — status of each item:

- [x] **SQL injection** — safe: every query in `database.rs`/`xml_processor.rs` uses sqlx
  bind parameters (`$1, $2…`); user input is never string-concatenated into SQL.
- [x] **Secrets in git** — `.env` is gitignored; only `.env.example` templates are committed.
- [ ] **CORS wide open** — `main.rs` uses `allow_origin(Any)`. In production nginx proxies
  same-origin, so lock allowed origins down via an env var.
- [ ] **No rate limiting** — `POST /api/compare-prices` is expensive and unauthenticated;
  add `tower-governor` (or nginx `limit_req`) before public exposure.
- [ ] **Input bounds** — cap `grocery_list` length (e.g. 100 items), string lengths, and
  `radius_km`/`page_size` ranges in `api.rs`; currently only "non-empty" is checked.
- [ ] **Untrusted XML** — retailer feeds are external input. `serde-xml-rs` doesn't expand
  external entities (XXE-safe), but add a file-size cap before `read_to_string` so a huge
  or malicious file can't exhaust memory.
- [ ] **FTP credentials** — Cerberus usernames/passwords are the retailers' *published public*
  credentials, but they live in source; move to env vars (as `dor_alon.py` already does)
  so all credentials follow one pattern.
- [ ] **TLS/headers at the edge** — put HTTPS in front (Caddy/Traefik/certbot) and add
  `X-Content-Type-Options`, `X-Frame-Options`, and a CSP to `frontend/nginx.conf`.
- [ ] **Docker hardening** — containers currently run as root; add non-root `USER`s to the
  three Dockerfiles, and don't expose the API or Postgres ports on the host (already true
  in `docker-compose.yml` — keep it that way).
- [ ] **Geocoding privacy** — `geocode_stores.py` sends store addresses to Nominatim (fine,
  public data), but user coordinates from `/api/stores/nearby` must never be logged or
  sent to third parties; audit `TraceLayer` log output.

---

## 6. How to finish the project

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
5. Implement full store coverage from §4 — first decide custom scrapers vs the
   `il-supermarket-scraper` library (§4.2b); then either dynamic file discovery + promo
   trimming in the service, or the library swap. Backend retry fix either way. *(1 day)*
6. Add the security checks from §5.1 (CI workflow + local script) and work through the
   §5.2 checklist items. *(half a day for the workflow; checklist items are small PRs)*
7. Add the test baseline from §3.5. *(1–2 days, pays for itself immediately)*

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
4. **Every store of every chain is covered** — the §4.2 discovery fixes are in, the §4.3
   backend retry fix is in, and the per-chain store counts in the DB match reality.
5. A compare of a 20-item basket over the full dataset responds in under ~1s (needs §3 items 6–7).
6. **The security workflow (§5.1) is green and required on `main`**, and every unchecked
   item in the §5.2 checklist is either fixed or consciously accepted.
7. `cargo test`, `pytest`, and `npm test` all exist and pass in CI.
