import os
import re
import requests
from urllib.parse import urljoin
from .base import StoreDownloader

class OneStoreDownloader(StoreDownloader):
    """Downloader for laibcatalog-hosted chains (Victory).

    Primary strategy: scrape the site's listing page(s) for download links so
    we fetch every file the chain actually published — all stores, all upload
    times — instead of guessing filenames.

    Fallback strategy: static URL guessing (StoreId x WDate timestamps), used
    only when no listing page can be parsed.
    """

    LISTING_PAGES = ["", "NBCompetitionRegulations.aspx"]

    def _discover_urls(self):
        """Return url_info dicts scraped from the listing page, or None if none found."""
        base_url = self.config.get("Url")
        chain_id = str(self.config.get("ChainId"))
        file_types = self.config.get("WFileType", [])
        if not base_url:
            return None

        for page in self.LISTING_PAGES:
            listing_url = urljoin(base_url, page)
            try:
                resp = requests.get(listing_url, timeout=30)
                resp.raise_for_status()
            except requests.exceptions.RequestException as e:
                print(f"  Listing page unavailable ({listing_url}): {e}")
                continue

            urls = self._urls_from_html(resp.text, listing_url, chain_id, file_types)
            if urls:
                print(f"  Listing page: {len(urls)} file(s) for chain {chain_id} (all stores).")
                return urls

        return None

    @staticmethod
    def _urls_from_html(html, listing_url, chain_id, file_types):
        """Extract download url_info dicts from a listing page's hrefs."""
        hrefs = re.findall(r'href="([^"]+?\.(?:zip|gz|xml)(?:\?[^"]*)?)"', html, re.IGNORECASE)
        matching = [h for h in hrefs if chain_id in h]
        if file_types:
            matching = [
                h for h in matching
                if any(os.path.basename(h.split("?")[0]).startswith(ft) for ft in file_types)
            ]

        urls_to_process = []
        seen = set()
        for href in matching:
            name = os.path.basename(href.split("?")[0])
            if name in seen:
                continue
            seen.add(name)
            full_url = href if href.lower().startswith("http") else urljoin(listing_url, href)
            m = re.match(rf"^([A-Za-z]+){chain_id}-(\d+)-(\d+)", name)
            urls_to_process.append({
                'url': full_url,
                'store_id': m.group(2) if m else 'NA',
                'file_type': m.group(1) if m else 'NA',
                'timestamp': m.group(3) if m else 'NA',
            })
        return urls_to_process

    def _generate_urls(self):
        discovered = self._discover_urls()
        if discovered:
            return discovered
        print("  No listing found — falling back to static URL generation.")
        return self._generate_urls_static()

    def _generate_urls_static(self):
        """
        Generates download URLs for stores with 'one' siteType.
        Example URL from config: {Url}{WFileType}{ChainId}-{StoreId}-{WDate}.zip
        """
        urls_to_process = []
        store_cfg = self.config

        base_url = store_cfg.get("Url")
        chain_id = store_cfg.get("ChainId")

        if not base_url or not chain_id:
            print(f"  Warning: 'Url' or 'ChainId' missing in config for OneStoreDownloader. Config: {store_cfg}")
            return []

        # WDate may be a single timestamp or a list of candidate timestamps.
        timestamps = store_cfg.get("WDate")
        if timestamps is None:
            print(f"  Warning: 'WDate' missing for OneStoreDownloader, ChainId {chain_id}.")
            return []
        if not isinstance(timestamps, list):
            timestamps = [timestamps]

        store_ids = store_cfg.get("StoreId", [])
        if not store_ids:
            print(f"  Warning: 'StoreId' missing or empty for OneStoreDownloader, ChainId {chain_id}.")

        file_types = store_cfg.get("WFileType", [])
        if not file_types:
            print(f"  Warning: 'WFileType' missing or empty for OneStoreDownloader, ChainId {chain_id}.")

        for store_id in store_ids:
            for file_type in file_types:
                for timestamp in timestamps:
                    ts_str = str(timestamp)
                    # URLs for 'one'-type chains end with .zip; the base downloader
                    # saves by magic number, so the extension mismatch is harmless.
                    url = f"{base_url}{file_type}{chain_id}-{store_id}-{ts_str}.zip"
                    urls_to_process.append({
                        'url': url,
                        'store_id': store_id,
                        'file_type': file_type,
                        'timestamp': ts_str
                    })

        if not urls_to_process and (store_ids and file_types and timestamps):
             print(f"  Note: No URLs generated for OneStoreDownloader, ChainId {chain_id}, despite config values.")

        return urls_to_process
