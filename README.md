# Shop-Saver 🛒

Compare the price of your grocery basket across Israeli supermarket chains — and see
which nearby branch sells the *exact same list* for less.

Israeli retailers are required by law to publish their full price lists daily.
Shop-Saver downloads those feeds from 12 chains, loads them into PostgreSQL, and serves
a Hebrew-RTL web app where you build a list (typing, autocomplete, or barcode scanning),
share your location or city, and get every matching branch ranked by total basket price.

## How it works

```
retailer price feeds (XML) → Python downloaders → Rust/Axum API + PostgreSQL → React PWA
```

- **`service/`** — Python downloaders for all 12 chains (binaprojects, Cerberus FTP,
  Shufersal, laibcatalog, Carrefour platforms) + Nominatim geocoding, on a cron schedule.
- **`backend/`** — Rust API: watches the download directory, parses XML into PostgreSQL,
  serves price comparison (barcode-first matching), store map, and item search endpoints.
- **`frontend/`** — React 19 / TypeScript / MUI PWA: compare page, stores map
  (react-leaflet), store detail, and item browse pages.

## Quick start

```bash
cp .env.example .env        # set a real POSTGRES_PASSWORD
docker compose up -d --build
```

Then open http://localhost (or `APP_PORT`). The pipeline container downloads price data
at 06:00 and 13:00; trigger a first run manually with:

```bash
docker compose exec service bash -c "cd /app/service && python3 main.py && python3 geocode_stores.py"
```

For local development without Docker, see [`QUICK_START.md`](QUICK_START.md).

## Documentation

| Doc | What's in it |
|---|---|
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Full architecture, refactoring notes, store-coverage design, security checklist, completion plan |
| [`ROADMAP.md`](ROADMAP.md) | Feature roadmap and history (phases 1–6) |
| [`QUICK_START.md`](QUICK_START.md) | Local development setup |

## Tests & checks

```bash
cd backend && cargo test          # Rust unit tests
python3 -m pytest service/tests/  # downloader tests
cd frontend && npm test           # React tests
./scripts/security_check.sh       # gitleaks + cargo/npm/pip audits
```

CI runs all of the above on every push and PR (`.github/workflows/ci.yml`,
`.github/workflows/security.yml`).
