#!/usr/bin/env bash
# setup.sh — one-time server setup for Shop-Saver.
#
# Run as root on a fresh Linux server:
#   bash setup.sh
#
# What it does:
#   1. Creates a dedicated system user (shop-saver)
#   2. Installs Python dependencies
#   3. Builds the Rust backend (release mode)
#   4. Installs the systemd service for the API
#   5. Installs the cron schedule for the download pipeline
#   6. Creates log directories
#   7. Starts everything

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICE_DIR="$PROJECT_ROOT/service"
BACKEND_DIR="$PROJECT_ROOT/backend"
INSTALL_DIR="/opt/shop-saver"
LOG_DIR="/var/log/shop-saver"
SYSTEM_USER="shop-saver"

log() { echo "==> $*"; }
die() { echo "ERROR: $*" >&2; exit 1; }

# ---------------------------------------------------------------------------
# 0. Pre-flight checks
# ---------------------------------------------------------------------------
[[ $EUID -eq 0 ]] || die "Run this script as root (sudo bash setup.sh)"
command -v python3 >/dev/null || die "python3 not found"
command -v cargo   >/dev/null || die "cargo not found — install Rust via https://rustup.rs"
command -v psql    >/dev/null || die "psql not found — install PostgreSQL first"

[[ -f "$BACKEND_DIR/.env" ]] || die ".env not found. Copy backend/.env.example to backend/.env and fill in DATABASE_URL."

# ---------------------------------------------------------------------------
# 1. System user
# ---------------------------------------------------------------------------
log "Creating system user: $SYSTEM_USER"
if ! id "$SYSTEM_USER" &>/dev/null; then
    useradd --system --no-create-home --shell /usr/sbin/nologin "$SYSTEM_USER"
fi

# ---------------------------------------------------------------------------
# 2. Log directory
# ---------------------------------------------------------------------------
log "Creating log directory: $LOG_DIR"
mkdir -p "$LOG_DIR"
chown "$SYSTEM_USER:$SYSTEM_USER" "$LOG_DIR"

# ---------------------------------------------------------------------------
# 3. Python dependencies
# ---------------------------------------------------------------------------
log "Installing Python dependencies"
pip3 install -r "$PROJECT_ROOT/requirements.txt" --quiet

# ---------------------------------------------------------------------------
# 4. Build Rust backend
# ---------------------------------------------------------------------------
log "Building Rust backend (release — this may take a few minutes)"
cd "$BACKEND_DIR"
cargo build --release

# ---------------------------------------------------------------------------
# 5. Install project to /opt/shop-saver (symlink or copy)
# ---------------------------------------------------------------------------
log "Installing project to $INSTALL_DIR"
if [[ ! -L "$INSTALL_DIR" ]]; then
    ln -s "$PROJECT_ROOT" "$INSTALL_DIR"
fi
chown -R "$SYSTEM_USER:$SYSTEM_USER" "$PROJECT_ROOT/service/downloads" 2>/dev/null || true
mkdir -p "$PROJECT_ROOT/service/downloads"
chown "$SYSTEM_USER:$SYSTEM_USER" "$PROJECT_ROOT/service/downloads"

# ---------------------------------------------------------------------------
# 6. systemd service for the API
# ---------------------------------------------------------------------------
log "Installing systemd service"
cp "$BACKEND_DIR/shop-saver-api.service" /etc/systemd/system/shop-saver-api.service
systemctl daemon-reload
systemctl enable shop-saver-api
systemctl restart shop-saver-api
log "API service status: $(systemctl is-active shop-saver-api)"

# ---------------------------------------------------------------------------
# 7. Cron job for the download pipeline
# ---------------------------------------------------------------------------
log "Installing cron schedule"
# Replace PROJECT placeholder with actual path
sed "s|/opt/shop-saver|$INSTALL_DIR|g" "$SERVICE_DIR/cronjob" > /etc/cron.d/shop-saver
chmod 644 /etc/cron.d/shop-saver

# ---------------------------------------------------------------------------
# 8. Initial geocoding run (non-fatal)
# ---------------------------------------------------------------------------
log "Running initial geocoding (this may take a while for large store lists)"
python3 "$SERVICE_DIR/geocode_stores.py" || log "WARNING: Geocoding failed — run manually later"

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
log ""
log "Setup complete!"
log ""
log "  API server: systemctl status shop-saver-api"
log "  API logs:   journalctl -u shop-saver-api -f"
log "  Pipeline:   bash $SERVICE_DIR/run_pipeline.sh"
log "  Pipe logs:  tail -f $LOG_DIR/pipeline.log"
log ""
log "Run the first download manually to populate the database:"
log "  bash $SERVICE_DIR/run_pipeline.sh"
