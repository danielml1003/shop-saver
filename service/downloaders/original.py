from .base import StoreDownloader
import datetime
import json
import re
import requests
# --- Concrete Subclasses ---

class OriginalStoreDownloader(StoreDownloader):
    """Downloader for binaprojects-hosted chains (King, Maayan, GoodPharm, ZolVeGadol).

    Primary strategy: ask the site's MainIO_Hok.aspx endpoint for the list of
    every file published today. This covers ALL stores the chain publishes —
    including new branches that are not in the static StoreId config — and all
    upload times, not just the last couple of hours.

    Fallback strategy: the original URL guessing (StoreId x WDate timestamps),
    used only when the listing endpoint is unavailable or returns nothing.
    """

    LISTING_ENDPOINT = "MainIO_Hok.aspx"

    def _site_root(self):
        # Config URLs look like https://kingstore.binaprojects.com/Download/
        base_url = self.config["Url"]
        idx = base_url.find("/Download")
        return base_url[:idx] if idx != -1 else base_url.rstrip("/")

    def _discover_urls(self):
        """Return url_info dicts for every file the server lists, or None if listing failed."""
        chain_id = str(self.config["ChainId"])
        file_types = self.config.get("WFileType", [])
        listing_url = f"{self._site_root()}/{self.LISTING_ENDPOINT}"

        try:
            resp = requests.get(listing_url, timeout=30)
            resp.raise_for_status()
            entries = resp.json()
        except (requests.exceptions.RequestException, json.JSONDecodeError, ValueError) as e:
            print(f"  Listing endpoint unavailable ({listing_url}): {e}")
            return None

        if not isinstance(entries, list):
            print(f"  Unexpected listing response from {listing_url} — falling back.")
            return None

        return self._urls_from_names(
            (entry.get("FileNm", "") for entry in entries if isinstance(entry, dict)),
            chain_id,
            file_types,
        )

    def _urls_from_names(self, names, chain_id, file_types):
        """Build url_info dicts from server-listed filenames, filtered by chain + file type."""
        base_url = self.config["Url"]
        urls_to_process = []
        seen = set()
        for name in names:
            if not name or chain_id not in name or name in seen:
                continue
            if file_types and not any(name.startswith(ft) for ft in file_types):
                continue
            seen.add(name)
            m = re.match(rf"^([A-Za-z]+){chain_id}-(\d+)-(\d+)", name)
            urls_to_process.append({
                'url': f"{base_url}{name}",
                'store_id': m.group(2) if m else 'NA',
                'file_type': m.group(1) if m else 'NA',
                'timestamp': m.group(3) if m else 'NA',
            })

        print(f"  Server listing: {len(urls_to_process)} file(s) for chain {chain_id} (all stores).")
        return urls_to_process

    def _generate_urls(self):
        discovered = self._discover_urls()
        if discovered:
            return discovered
        if discovered is not None:
            print("  Listing returned no matching files — falling back to static URL generation.")
        return self._generate_urls_static()

    def _generate_urls_static(self):
        """
        Generates download URLs for stores with 'original' siteType.
        Example URL: {Url}{WFileType}{ChainId}-{StoreId}-{WDate}.gz
        """
        urls_to_process = []
        store_cfg = self.config # self.config is the store's dictionary

        base_url = store_cfg["Url"]
        chain_id = store_cfg["ChainId"]

        # WDate for 'original' type stores like kingStore can be a list of timestamps
        # or a single timestamp for others. Ensure it's a list.
        timestamps = store_cfg.get("WDate") # Use .get for safety
        if timestamps is None: # Handle case where WDate might be missing
            print(f"  Warning: 'WDate' missing in config for OriginalStoreDownloader, ChainId {chain_id}. No URLs will be generated.")
            return []

        if not isinstance(timestamps, list):
            timestamps = [timestamps] # Make it a list if it's a single value

        store_ids = store_cfg.get("StoreId", [])
        if not store_ids:
            print(f"  Warning: 'StoreId' missing or empty in config for OriginalStoreDownloader, ChainId {chain_id}.")

        file_types = store_cfg.get("WFileType", [])
        if not file_types:
            print(f"  Warning: 'WFileType' missing or empty in config for OriginalStoreDownloader, ChainId {chain_id}.")


        seen_urls = set()

        for file_type in file_types:
            for timestamp in timestamps:
                ts_str = str(timestamp)

                if file_type.startswith("Stores"):
                    # Chain-level file: use the known upload hours from WStoresFullHours config.
                    stores_hours = store_cfg.get("WStoresFullHours", [])
                    today = datetime.datetime.now().strftime("%Y%m%d")
                    for hour in stores_hours:
                        day_ts = f"{today}{hour:02d}10"
                        url = f"{base_url}{file_type}{chain_id}-000-{day_ts}.gz"
                        if url not in seen_urls:
                            seen_urls.add(url)
                            urls_to_process.append({
                                'url': url,
                                'store_id': '000',
                                'file_type': file_type,
                                'timestamp': day_ts
                            })
                else:
                    for store_id in store_ids:
                        url = f"{base_url}{file_type}{chain_id}-{store_id}-{ts_str}.gz"
                        urls_to_process.append({
                            'url': url,
                            'store_id': store_id,
                            'file_type': file_type,
                            'timestamp': ts_str
                        })

        if not urls_to_process and (store_ids and file_types and timestamps): # Only print warning if inputs were present but still no URLs
            print(f"  Note: No URLs generated for OriginalStoreDownloader, ChainId {chain_id}, despite having config values. This might be expected if timestamp list was empty.")

        return urls_to_process
