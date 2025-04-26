import requests
import os
import datetime

def job_that_runs_every_day():
    now = datetime.datetime.now()
    return now

reshet  = {
    "WStore": 1,
    "WDate": job_that_runs_every_day(),
    "WFileType":2
}

def makeUrl(date):
    dataUrl = "https://kingstore.binaprojects.com/MainIO_Hok.aspx"
    dataUrl += reshet.items
    print(dataUrl)

if __name__ == "__main__":
    makeUrl(job_that_runs_every_day())