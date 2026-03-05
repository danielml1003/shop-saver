from .mega import MegaStoreDownloader

# Only Price/PriceFull — Promo files use a different XML schema (<Promotions> not <Items>)
_FILE_TYPE_PREFIXES = ("Price", "PriceFull")

_config = {
    "ChainId": 7290055700007,
    "WFileTypePrefixes": _FILE_TYPE_PREFIXES,
}


class Carrefour(MegaStoreDownloader):
    def __init__(self):
        super().__init__(_config)

    def download(self):
        self.process_store()
