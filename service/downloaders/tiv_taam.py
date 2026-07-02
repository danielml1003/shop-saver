import os
from .cerberus import CerberusStoreDownloader

# Promo/PromoFull intentionally excluded — the backend discards promo files (ARCHITECTURE.md §4.2).
_FILE_TYPES = ["StoresFull", "Price", "PriceFull"]

_config = {
    "ChainId": 7290873255550,
    "WFileType": _FILE_TYPES,
    "ftp_username": "TivTaam",
    "ftp_password": os.environ.get("TIVTAAM_FTP_PASSWORD", "TivTaam"),
    # StoreId list removed — the Cerberus FTP downloader lists every file on the server,
    # so all stores are covered automatically (ARCHITECTURE.md §4.2).
}


class TivTaam(CerberusStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
