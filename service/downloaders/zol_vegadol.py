import datetime
from .original import OriginalStoreDownloader

# Promo/PromoFull intentionally excluded — the backend discards promo files (ARCHITECTURE.md §4.2).
_FILE_TYPES = ["Price", "PriceFull"]  # StoresFull not published by this chain


def _recent_timestamps():
    now = datetime.datetime.now()
    return [
        now.replace(hour=h, minute=10, second=0, microsecond=0).strftime("%Y%m%d%H%M")
        for h in range(max(0, now.hour - 1), now.hour + 1)
    ]


_config = {
    "Url": "https://zolvebegadol.binaprojects.com/Download/",
    "WFileType": _FILE_TYPES,
    "ChainId": 7290058173198,
    "StoreId": [
        "002", "003", "004", "006", "007", "008", "009", "011", "012", "013",
        "014", "015", "016", "018", "019", "020", "021", "023", "024", "025",
        "026", "027", "028", "030", "031", "032", "033", "035", "036", "037",
        "038", "039", "040", "041", "042", "089",
    ],
    "WDate": _recent_timestamps(),
}


class ZolVeGadol(OriginalStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
