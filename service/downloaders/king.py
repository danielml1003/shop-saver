'''from service.downloaders.base import BaseDownloader    BaseDownloader'''
from .original import OriginalStoreDownloader
import datetime

fileType = ["StoresFull", "Price", "Promo", "PriceFull", "PromoFull"]

def _recent_timestamps():
    now = datetime.datetime.now()
    return [
        now.replace(hour=h, minute=10, second=0, microsecond=0).strftime("%Y%m%d%H%M")
        for h in range(max(0, now.hour - 1), now.hour + 1)
    ]

kingStore  = {
    "Url": "https://kingstore.binaprojects.com/Download/",
    "WFileType": fileType,
    "ChainId":7290058108879,
    "StoreId": ["001","002","003","005","006","007","008","009","010","012","013","014","015","016","017","018","019","027","028","031","050","200","334","335","336","337","338","339"],
    "WDate": _recent_timestamps(),
    "WStoresFullHours": [5, 10],  # verified upload times for StoresFull
    "siteType": "original"
}


class King(OriginalStoreDownloader):
    def __init__(self):
        # Initialize any necessary attributes or configurations
        super().__init__(kingStore)


    def download(self):
        # Implement download logic here
        self.process_store()
