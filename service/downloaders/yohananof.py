import os
from .cerberus import CerberusStoreDownloader

_config = {
    "ChainId": 7290803800003,
    "ftp_username": "yohananof",
    # Retailers publish these credentials; env var overrides for consistency (§5.2)
    "ftp_password": os.environ.get("YOHANANOF_FTP_PASSWORD", "yohananof"),
    "ftp_active_mode": True,
    "WFileType": ["StoresFull", "PriceFull", "Price"],
}


class Yohananof(CerberusStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
