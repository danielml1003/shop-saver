import datetime
import os
from .cerberus import CerberusStoreDownloader

_config = {
    "ChainId": 7290058140886,
    "ftp_username": "ramilevi",
    # Retailers publish these credentials; env var overrides for consistency (§5.2)
    "ftp_password": os.environ.get("RAMILEVY_FTP_PASSWORD", "ramilevi"),
    "ftp_active_mode": True,   # passive mode is blocked on many networks; active works
    "WFileType": ["StoresFull", "Price", "PriceFull"],
}


class RamiLevy(CerberusStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
