"""
geocode_stores.py

Fetches all stores with missing coordinates from the database and geocodes them
using the Nominatim (OpenStreetMap) API — free, no API key required.

Re-run safe: only touches stores where latitude IS NULL.

Usage:
    python service/geocode_stores.py

Requires (add to requirements.txt):
    psycopg2-binary
    python-dotenv
"""

import os
import sys
import time
from pathlib import Path
from typing import Optional, Tuple

import requests
import psycopg2
from dotenv import load_dotenv

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

# Load DATABASE_URL from backend/.env (or from the environment directly)
_env_path = Path(__file__).parent.parent / "backend" / ".env"
load_dotenv(dotenv_path=_env_path)

DATABASE_URL = os.environ.get("DATABASE_URL")
if not DATABASE_URL:
    sys.exit(
        "ERROR: DATABASE_URL not set.\n"
        "Copy backend/.env.example to backend/.env and fill in your credentials."
    )

NOMINATIM_URL = "https://nominatim.openstreetmap.org/search"
# Nominatim requires a descriptive User-Agent and >= 1 second between requests.
HEADERS = {"User-Agent": "shop-saver/1.0 (Israeli grocery price comparison)"}
RATE_LIMIT = 1.1  # seconds between API calls

# ---------------------------------------------------------------------------
# Geocoding
# ---------------------------------------------------------------------------

def _nominatim_query(query: str) -> Optional[Tuple[float, float]]:
    """Fire one Nominatim request, return (lat, lng) or None."""
    try:
        resp = requests.get(
            NOMINATIM_URL,
            params={"q": query, "format": "json", "countrycodes": "il", "limit": 1},
            headers=HEADERS,
            timeout=10,
        )
        resp.raise_for_status()
        results = resp.json()
        if results:
            return float(results[0]["lat"]), float(results[0]["lon"])
    except Exception as e:
        print(f"    [warn] Nominatim error: {e}")
    return None


def geocode(address: Optional[str], city: Optional[str]) -> Optional[Tuple[float, float]]:
    """
    Try to geocode a store. Strategy:
      1. Full query: "{address}, {city}, ישראל"
      2. Fallback:   "{city}, ישראל"  (in case address is too specific / garbled)
    """
    addr = (address or "").strip()
    cty  = (city or "").strip()

    if addr and cty:
        result = _nominatim_query(f"{addr}, {cty}, ישראל")
        if result:
            return result
        time.sleep(RATE_LIMIT)  # extra call = extra sleep

    if cty:
        return _nominatim_query(f"{cty}, ישראל")

    if addr:
        return _nominatim_query(f"{addr}, ישראל")

    return None


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    conn = psycopg2.connect(DATABASE_URL)
    cur  = conn.cursor()

    cur.execute("""
        SELECT id, chain_id, store_id, store_name, address, city
        FROM   stores
        WHERE  latitude IS NULL
          AND  (address IS NOT NULL OR city IS NOT NULL)
        ORDER  BY id
    """)
    stores = cur.fetchall()
    total  = len(stores)

    if total == 0:
        print("All stores already have coordinates. Nothing to do.")
        cur.close()
        conn.close()
        return

    print(f"Geocoding {total} stores (≈{total * RATE_LIMIT:.0f}s at Nominatim rate limit)...\n")

    success = 0
    failed  = 0

    for idx, (store_pk, chain_id, store_id, store_name, address, city) in enumerate(stores, 1):
        label = store_name or f"chain={chain_id} store={store_id}"
        location_str = ", ".join(filter(None, [address, city]))
        print(f"[{idx}/{total}] {label}  |  {location_str} ... ", end="", flush=True)

        coords = geocode(address, city)

        if coords:
            lat, lng = coords
            cur.execute(
                "UPDATE stores SET latitude = %s, longitude = %s WHERE id = %s",
                (lat, lng, store_pk),
            )
            conn.commit()
            print(f"✓  ({lat:.5f}, {lng:.5f})")
            success += 1
        else:
            print("✗  not found")
            failed += 1

        time.sleep(RATE_LIMIT)

    cur.close()
    conn.close()

    print(f"\n{'─'*50}")
    print(f"Done.  Geocoded: {success}  |  Failed: {failed}  |  Total: {total}")
    if failed:
        print(
            "Tip: stores that failed often have garbled Hebrew addresses.\n"
            "Re-run after fixing the address/city data, or set coordinates manually."
        )


if __name__ == "__main__":
    main()
