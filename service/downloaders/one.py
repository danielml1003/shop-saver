import os # For os.path.basename in print statements if you add them
from .base import StoreDownloader

class OneStoreDownloader(StoreDownloader):
    def _generate_urls(self):
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

        # For 'one' type, WDate is typically a single timestamp string
        timestamp = store_cfg.get("WDate")
        if timestamp is None:
            print(f"  Warning: 'WDate' missing for OneStoreDownloader, ChainId {chain_id}.")
            return []
        
        ts_str = str(timestamp)

        store_ids = store_cfg.get("StoreId", [])
        if not store_ids:
            print(f"  Warning: 'StoreId' missing or empty for OneStoreDownloader, ChainId {chain_id}.")

        file_types = store_cfg.get("WFileType", [])
        if not file_types:
            print(f"  Warning: 'WFileType' missing or empty for OneStoreDownloader, ChainId {chain_id}.")

        for store_id in store_ids:
            for file_type in file_types:
                # URL from storesJsonUrl.py for 'victory' (type 'one') ends with .zip
                # Our base downloader will still save it as .gz and try to Gzip extract.
                url = f"{base_url}{file_type}{chain_id}-{store_id}-{ts_str}.zip" 
                urls_to_process.append({
                    'url': url,
                    'store_id': store_id,
                    'file_type': file_type,
                    'timestamp': ts_str
                })
        
        if not urls_to_process and (store_ids and file_types and timestamp):
             print(f"  Note: No URLs generated for OneStoreDownloader, ChainId {chain_id}, despite config values.")

        return urls_to_process
