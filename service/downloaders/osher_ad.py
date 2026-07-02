import os
from .cerberus import CerberusStoreDownloader

_config = {
    "ChainId": 7290103152017,
    "ftp_username": "osherad",
    # Retailers publish these credentials; env var overrides for consistency (§5.2)
    "ftp_password": os.environ.get("OSHERAD_FTP_PASSWORD", "osherad"),
    "ftp_active_mode": True,
    "WFileType": ["StoresFull", "PriceFull", "Price"],
}


class OsherAd(CerberusStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
