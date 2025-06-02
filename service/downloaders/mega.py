import requests
import json
import os # For os.path.basename in print statements
from .base import StoreDownloader

class MegaStoreDownloader(StoreDownloader):
    def _generate_urls(self):
        """
        Generates initial request URLs for stores with 'mega' siteType.
        These URLs point to an API returning JSON with the actual file path.
        Example initial URL: {Url}{WFileType}{ChainId}-{StoreId}-{WDate}.json
        """
        urls_to_process = []
        store_cfg = self.config

        base_url = store_cfg.get("Url")
        chain_id = store_cfg.get("ChainId")

        if not base_url or not chain_id:
            print(f"  Warning: 'Url' or 'ChainId' missing in config for MegaStoreDownloader. Config: {store_cfg}")
            return []

        timestamp = store_cfg.get("WDate")
        if timestamp is None:
            print(f"  Warning: 'WDate' missing for MegaStoreDownloader, ChainId {chain_id}.")
            return []
        
        ts_str = str(timestamp)

        store_ids = store_cfg.get("StoreId", [])
        if not store_ids:
            print(f"  Warning: 'StoreId' missing or empty for MegaStoreDownloader, ChainId {chain_id}.")

        file_types = store_cfg.get("WFileType", [])
        if not file_types:
            print(f"  Warning: 'WFileType' missing or empty for MegaStoreDownloader, ChainId {chain_id}.")

        for store_id in store_ids:
            for file_type in file_types:
                url = f"{base_url}{file_type}{chain_id}-{store_id}-{ts_str}.json"
                urls_to_process.append({
                    'url': url,
                    'store_id': store_id,
                    'file_type': file_type,
                    'timestamp': ts_str
                })
        
        if not urls_to_process and (store_ids and file_types and timestamp):
             print(f"  Note: No URLs generated for MegaStoreDownloader, ChainId {chain_id}, despite config values.")

        return urls_to_process

    def _request_and_save_file(self, initial_url, filename_base):
        """
        Overrides base method to handle Mega's two-step download.
        """
        print(f"  MegaStoreDownloader: Attempting initial request to: {initial_url}")
        try:
            response = requests.get(initial_url, timeout=30)
            response.raise_for_status()
            data = response.json()
            
            if isinstance(data, list) and len(data) > 0 and "SPath" in data[0]:
                actual_file_url = data[0]["SPath"]
                print(f"    MegaStoreDownloader: Found actual file URL in SPath: {actual_file_url}")
                return super()._request_and_save_file(actual_file_url, filename_base)
            else:
                print(f"    Error: MegaStoreDownloader - Invalid JSON response format from {initial_url}. Expected list with 'SPath'. Response: {data}")
                return None

        except json.JSONDecodeError as e_json:
            print(f"    Error: MegaStoreDownloader - Failed to parse JSON from {initial_url}. Error: {e_json}. Response: {response.text[:200]}")
            return None
        except requests.exceptions.RequestException as e_req:
            print(f"    Error: MegaStoreDownloader - Initial request failed for {initial_url}. Error: {e_req}")
            return None
        except Exception as e_other:
            print(f"    Error: MegaStoreDownloader - Unexpected error for {initial_url}. Error: {e_other}")
            return None
