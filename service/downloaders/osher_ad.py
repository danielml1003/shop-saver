from .cerberus import CerberusStoreDownloader

_config = {
    "ChainId": 7290103152017,
    "ftp_username": "osherad",
    # password defaults to username in CerberusStoreDownloader
    "ftp_active_mode": True,
    "WFileType": ["StoresFull", "PriceFull", "Price"],
}


class OsherAd(CerberusStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
