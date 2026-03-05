import requests
import re
import json
import os
import datetime
from .base import StoreDownloader


class MegaStoreDownloader(StoreDownloader):
    LISTING_URL = "https://prices.carrefour.co.il/"

    def _generate_urls(self):
        return []  # Not used — process_store is overridden

    def process_store(self):
        chain_id = str(self.config["ChainId"])
        self.success_count = 0
        self.failure_count = 0

        print(f"\nProcessing Store (Mega): {chain_id}")

        # Fetch the listing page which embeds all available files in JS
        try:
            response = requests.get(self.LISTING_URL, timeout=30)
            response.raise_for_status()
            content = response.text
        except requests.exceptions.RequestException as e:
            print(f"  Error fetching Mega listing page: {e}")
            return

        # Parse the embedded JS: const path = 'YYYYMMDD'; const files = [...];
        path_match = re.search(r"const path = '(\d{8})'", content)
        files_match = re.search(r"const files = (\[.*?\]);", content, re.DOTALL)

        if not path_match or not files_match:
            print("  Error: Could not parse file listing from Mega page.")
            return

        date_path = path_match.group(1)
        all_files = json.loads(files_match.group(1))

        # Filter to files belonging to this chain modified in the last 2 hours
        now = datetime.datetime.now()
        cutoff = now - datetime.timedelta(hours=2)
        allowed_prefixes = self.config.get("WFileTypePrefixes")

        def _is_recent(file_info):
            """Parse 'HH:MM DD-MM-YYYY' and check if within last 2 hours."""
            try:
                modified = datetime.datetime.strptime(file_info["modified"], "%H:%M %d-%m-%Y")
                return modified >= cutoff
            except (ValueError, KeyError):
                return True  # Include if we can't parse the date

        def _allowed_type(file_info):
            if not allowed_prefixes:
                return True
            name = file_info["name"]
            return any(name.startswith(p) for p in allowed_prefixes)

        matching = [
            f for f in all_files
            if chain_id in f["name"] and _is_recent(f) and _allowed_type(f)
        ]
        print(f"  Found {len(matching)} recent files for chain {chain_id} (out of {len(all_files)} total)")

        if not matching:
            print("  No files found for this chain today.")
            return

        for file_info in matching:
            filename = file_info["name"]
            download_url = f"{self.LISTING_URL}{date_path}/{filename}"
            local_path = os.path.join(self.download_dir, filename)

            print(f"\n  Downloading: {filename}")
            try:
                dl = requests.get(download_url, timeout=60, stream=True)
                dl.raise_for_status()
                with open(local_path, "wb") as f:
                    for chunk in dl.iter_content(chunk_size=8192):
                        f.write(chunk)
                print(f"  Saved: {filename}")
            except requests.exceptions.RequestException as e:
                print(f"  Error downloading {filename}: {e}")
                self.failure_count += 1
                continue

            extracted = self._extract_file(local_path)
            if extracted:
                print(f"  Extracted: {os.path.basename(extracted)}")
                self.success_count += 1
            else:
                self.failure_count += 1

        print(f"\nFinished processing Store (Mega): {chain_id}")
        print(f"  Successful: {self.success_count}  Failed: {self.failure_count}")
