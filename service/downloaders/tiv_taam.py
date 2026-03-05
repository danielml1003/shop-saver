import os
from .ceberus import CeberusStoreDownloader

_FILE_TYPES = ["StoresFull", "Price", "Promo", "PriceFull", "PromoFull"]

_config = {
    "ChainId": 7290873255550,
    "WFileType": _FILE_TYPES,
    "ftp_username": "TivTaam",
    "ftp_password": os.environ.get("TIVTAAM_FTP_PASSWORD", "TivTaam"),
    "StoreId": [
        "002", "003", "006", "007", "010", "012", "014", "015", "017", "018",
        "019", "020", "021", "022", "023", "024", "025", "030", "051", "052",
        "054", "056", "057", "058", "059", "061", "063", "068", "070", "071",
        "073", "074", "075", "079", "080", "082", "083", "084", "085", "087",
        "088", "089", "091", "092", "156", "502", "503", "512", "514", "515",
        "519", "523",
    ],
}


class TivTaam(CeberusStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
