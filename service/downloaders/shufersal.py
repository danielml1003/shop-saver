"""
Shufersal price data downloader.

Shufersal publishes price files at prices.shufersal.co.il via a paginated HTML
listing. Each row contains a signed Azure Blob Storage URL for a .gz file.

Categories fetched:
  catID=2  →  PriceFull (complete price list per store branch)
  catID=5  →  StoresFull (store metadata — address, name, etc.)
"""

import html
import os
import re
import datetime
import requests
from .base import StoreDownloader

_CHAIN_ID   = 7290027600007
_BASE_URL   = "http://prices.shufersal.co.il/FileObject/UpdateCategory"
_CATEGORIES = [2, 5]   # 2=PriceFull, 5=StoresFull  (skip 6=Price — PriceFull is a superset)
_PAGE_SIZE  = 20       # items per page as served by Shufersal

_config = {
    "ChainId": _CHAIN_ID,
}


def _fetch_page_links(session: requests.Session, cat_id: int, date_str: str, page: int) -> list[str]:
    """Return all Azure Blob download URLs from one listing page."""
    resp = session.get(
        _BASE_URL,
        params={
            "catID": cat_id,
            "storeId": "0",
            "reqLastData": date_str,
            "page": page,
        },
        timeout=30,
    )
    resp.raise_for_status()
    # Signed Azure URLs look like: https://pricesprodpublic.blob.core.windows.net/...
    # HTML escapes & as &amp; inside href attributes — unescape or the SAS
    # signature in the query string is corrupted and every download 403s.
    return [
        html.unescape(u)
        for u in re.findall(r'href="(https://pricesprodpublic\.blob\.core\.windows\.net/[^"]+)"', resp.text)
    ]


def _filename_from_url(url: str) -> str:
    """Extract the bare filename (no query string) from a signed Azure Blob URL."""
    path = url.split("?")[0]
    return path.rsplit("/", 1)[-1]


class Shufersal(StoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def _generate_urls(self):
        return []  # not used — process_store is overridden

    def process_store(self):
        chain_id = str(_CHAIN_ID)
        self.success_count = 0
        self.failure_count = 0
        print(f"\nProcessing Store (Shufersal): {chain_id}")

        # Shufersal shows files from reqLastData onwards.
        # Use yesterday to ensure today's files are included.
        date_str = (datetime.date.today() - datetime.timedelta(days=1)).strftime("%Y-%m-%d")

        seen_filenames: set[str] = set()
        all_links: list[str] = []

        with requests.Session() as session:
            session.headers["User-Agent"] = "Mozilla/5.0"

            for cat_id in _CATEGORIES:
                cat_label = "PriceFull" if cat_id == 2 else "StoresFull"
                print(f"  Fetching {cat_label} listing (catID={cat_id})...")
                page = 1
                while True:
                    try:
                        links = _fetch_page_links(session, cat_id, date_str, page)
                    except requests.RequestException as e:
                        print(f"    Error fetching page {page}: {e}")
                        break

                    # Filter to today's files and deduplicate
                    today = datetime.date.today().strftime("%Y%m%d")
                    new_links = []
                    for url in links:
                        fname = _filename_from_url(url)
                        if today in fname and fname not in seen_filenames:
                            seen_filenames.add(fname)
                            new_links.append(url)

                    all_links.extend(new_links)

                    # Stop when the page returned fewer items than the page size
                    # (means we've reached the end of the listing)
                    if len(links) < _PAGE_SIZE:
                        break
                    page += 1

                print(f"    {len([l for l in all_links if _filename_from_url(l).startswith(cat_label[:5])])} files so far")

        print(f"  Total files to download: {len(all_links)}")

        for url in all_links:
            filename = _filename_from_url(url)
            local_path = os.path.join(self.download_dir, filename)

            print(f"\n  Downloading: {filename}")
            try:
                # fresh request — the listing session is closed by this point
                dl = requests.get(url, timeout=60, stream=True, headers={"User-Agent": "Mozilla/5.0"})
                dl.raise_for_status()
                with open(local_path, "wb") as f:
                    for chunk in dl.iter_content(chunk_size=65536):
                        f.write(chunk)
                print(f"  Saved: {filename}")
            except requests.RequestException as e:
                print(f"  Error downloading {filename}: {e}")
                self.failure_count += 1
                continue

            extracted = self._extract_file(local_path)
            if extracted:
                print(f"  Extracted: {os.path.basename(extracted)}")
                self.success_count += 1
            else:
                self.failure_count += 1

        print(f"\nFinished Shufersal.")
        print(f"  Successful: {self.success_count}  Failed: {self.failure_count}")

    def download(self):
        self.process_store()
