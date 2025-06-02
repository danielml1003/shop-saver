'''from service.downloaders.base import BaseDownloader    BaseDownloader'''
from downloaders.original import OriginalStoreDownloader
import datetime

fileType = ["StoresFull", "Price", "Promo", "PriceFull", "PromoFull"]

def time_for_kingStore(timeNow):
    now = datetime.datetime.now()
    urls = []
    for hour in range(24):
        time = now.replace(hour=hour, minute=24, second=0, microsecond=0)
        urls.append(time.strftime("%Y%m%d%H%M"))
    return urls

kingStore  = {
    "Url": "https://kingstore.binaprojects.com/Download/",
    "WFileType": fileType,
    "ChainId":7290058108879,
    "StoreId": ["001","002","003","005","006","007","008","009","010","012","013","014","015","016","017","018","019","027","028","031","050","200","334","335","336","337","338","339"],
    "WDate": time_for_kingStore(datetime.date.today()),
    "siteType": "original"
}


class King(OriginalStoreDownloader):
    def __init__(self):
        # Initialize any necessary attributes or configurations
        super().__init__(kingStore)


    def download(self):
        # Implement download logic here
        self.process_store()
