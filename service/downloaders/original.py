from .base import StoreDownloader
import os
# --- Concrete Subclasses ---

class OriginalStoreDownloader(StoreDownloader):
    def _generate_urls(self):
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


        for store_id in store_ids:
            for file_type in file_types:
                for timestamp in timestamps:
                    # Ensure timestamp is a string for URL construction
                    ts_str = str(timestamp) 
                    
                    # Construct the URL
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

# We will add other subclasses (MegaStoreDownloader, etc.) here later.
