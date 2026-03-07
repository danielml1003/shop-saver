from .ceberus import CeberusStoreDownloader

_config = {
    "ChainId": 7290103152017,
    "ftp_username": "osherad",
    # password defaults to username in CeberusStoreDownloader
    "ftp_active_mode": True,
    "WFileType": ["StoresFull", "PriceFull", "Price"],
}


class OsherAd(CeberusStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
