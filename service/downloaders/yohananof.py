from .ceberus import CeberusStoreDownloader

_config = {
    "ChainId": 7290803800003,
    "ftp_username": "yohananof",
    # password defaults to username in CeberusStoreDownloader
    "ftp_active_mode": True,
    "WFileType": ["StoresFull", "PriceFull", "Price"],
}


class Yohananof(CeberusStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
