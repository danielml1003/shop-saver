import requests
import os
import storesJsonUrl




def makeUrls():
    for store in storesJsonUrl.stores:
        print(store)

if __name__ == "__main__":
    makeUrls()