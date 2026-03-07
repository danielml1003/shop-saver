#!/usr/bin/env bash
# run_pipeline.sh — download all chain price files then geocode any new stores.
#
# Called by cron. Designed to be idempotent and safe to re-run at any time.
# Logs are written to /var/log/shop-saver/pipeline.log (or $LOG_FILE).
#
# Usage:
#   ./service/run_pipeline.sh             # normal cron run
#   DRY_RUN=1 ./service/run_pipeline.sh   # print what would happen, don't download

set -euo pipefail

# ---------------------------------------------------------------------------
# Paths — all relative to the project root (one directory up from service/)
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SERVICE_DIR="$PROJECT_ROOT/service"
LOG_DIR="${LOG_DIR:-/var/log/shop-saver}"
LOG_FILE="$LOG_DIR/pipeline.log"
PYTHON="${PYTHON:-python3}"

# ---------------------------------------------------------------------------
# Logging helpers
# ---------------------------------------------------------------------------
mkdir -p "$LOG_DIR"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

log_section() {
    log "------------------------------------------------------------"
    log "$*"
    log "------------------------------------------------------------"
}

# ---------------------------------------------------------------------------
# Load environment (DATABASE_URL etc.)
# ---------------------------------------------------------------------------
ENV_FILE="$PROJECT_ROOT/backend/.env"
if [[ -f "$ENV_FILE" ]]; then
    set -a
    # shellcheck disable=SC1090
    source "$ENV_FILE"
    set +a
fi

# ---------------------------------------------------------------------------
# Dry-run guard
# ---------------------------------------------------------------------------
if [[ "${DRY_RUN:-0}" == "1" ]]; then
    log "[DRY RUN] Would run: $PYTHON $SERVICE_DIR/main.py"
    log "[DRY RUN] Would run: $PYTHON $SERVICE_DIR/geocode_stores.py"
    exit 0
fi

# ---------------------------------------------------------------------------
# Step 1 — Download price XML files from all chains
# ---------------------------------------------------------------------------
log_section "STEP 1: Downloading price files from all chains"

DOWNLOAD_START=$(date +%s)

cd "$SERVICE_DIR"
if "$PYTHON" main.py >> "$LOG_FILE" 2>&1; then
    DOWNLOAD_END=$(date +%s)
    log "Download complete in $((DOWNLOAD_END - DOWNLOAD_START))s"
else
    EXIT_CODE=$?
    log "ERROR: Download step failed with exit code $EXIT_CODE (continuing to geocoding)"
fi

# ---------------------------------------------------------------------------
# Step 2 — Geocode any stores that are missing coordinates
# ---------------------------------------------------------------------------
log_section "STEP 2: Geocoding new stores"

if "$PYTHON" "$SERVICE_DIR/geocode_stores.py" >> "$LOG_FILE" 2>&1; then
    log "Geocoding complete"
else
    log "WARNING: Geocoding step failed (non-fatal)"
fi

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
log_section "Pipeline run finished"
