import datetime
from .ceberus import CeberusStoreDownloader

_config = {
    "ChainId": 7290058140886,
    "ftp_username": "ramilevi",
    # password defaults to username in CeberusStoreDownloader
    "ftp_active_mode": True,   # passive mode is blocked on many networks; active works
    "WFileType": ["StoresFull", "Price", "PriceFull"],
}


class RamiLevy(CeberusStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
