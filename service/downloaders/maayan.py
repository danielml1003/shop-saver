import datetime
from .original import OriginalStoreDownloader

# Promo/PromoFull intentionally excluded — the backend discards promo files (ARCHITECTURE.md §4.2).
_FILE_TYPES = ["StoresFull", "Price", "PriceFull"]


def _recent_timestamps():
    now = datetime.datetime.now()
    return [
        now.replace(hour=h, minute=10, second=0, microsecond=0).strftime("%Y%m%d%H%M")
        for h in range(max(0, now.hour - 1), now.hour + 1)
    ]


_config = {
    "Url": "https://maayan2000.binaprojects.com/Download/",
    "WFileType": _FILE_TYPES,
    "ChainId": 7290058159628,
    "WStoresFullHours": [5, 10],  # verified upload times for StoresFull
    "StoreId": [
        "001", "002", "003", "004", "005", "006", "007", "008", "009", "010",
        "011", "013", "016", "017", "018", "019", "020", "021", "022", "023",
        "026", "027", "028", "029", "035", "038", "039", "040", "041", "043",
        "044", "045", "046", "047", "048", "049", "050", "060", "061", "062", "063",
    ],
    "WDate": _recent_timestamps(),
}


class Maayan(OriginalStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
