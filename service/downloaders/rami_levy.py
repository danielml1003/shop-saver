import datetime
from .cerberus import CerberusStoreDownloader

_config = {
    "ChainId": 7290058140886,
    "ftp_username": "ramilevi",
    # password defaults to username in CerberusStoreDownloader
    "ftp_active_mode": True,   # passive mode is blocked on many networks; active works
    "WFileType": ["StoresFull", "Price", "PriceFull"],
}


class RamiLevy(CerberusStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
