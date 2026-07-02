from .cerberus import CerberusStoreDownloader

_config = {
    "ChainId": 7290803800003,
    "ftp_username": "yohananof",
    # password defaults to username in CerberusStoreDownloader
    "ftp_active_mode": True,
    "WFileType": ["StoresFull", "PriceFull", "Price"],
}


class Yohananof(CerberusStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
