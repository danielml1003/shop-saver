import datetime
from .original import OriginalStoreDownloader

_FILE_TYPES = ["StoresFull", "Price", "Promo", "PriceFull", "PromoFull"]


def _recent_timestamps():
    now = datetime.datetime.now()
    return [
        now.replace(hour=h, minute=10, second=0, microsecond=0).strftime("%Y%m%d%H%M")
        for h in range(max(0, now.hour - 1), now.hour + 1)
    ]


_config = {
    "Url": "https://goodpharm.binaprojects.com/Download/",
    "WFileType": _FILE_TYPES,
    "ChainId": 7290058197699,
    "WStoresFullHours": [6, 10, 13],  # verified upload times for StoresFull
    "StoreId": [
        "001", "002", "003", "004", "005", "006", "007", "008", "009", "010",
        "011", "012", "013", "014", "015", "016", "017", "018", "019", "020",
        "021", "022", "023", "024", "025", "026", "027", "028", "029", "030",
        "031", "032", "033", "034", "035", "036", "037", "038", "039", "040",
        "041", "042", "043", "044", "045", "046", "047", "048", "049", "050",
        "051", "052", "053", "054", "055", "056", "057", "058", "059", "060",
        "061", "062", "063",
    ],
    "WDate": _recent_timestamps(),
}


class GoodPharm(OriginalStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
