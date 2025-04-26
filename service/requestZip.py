import requests
import os
import datetime

def job_that_runs_every_day():
    now = datetime.datetime.now()
    return now.date()

kingStore  = {
    "WStore": [1,2,3,5,6,7,8,9,10,12,13,14,15,16,17,18,19,27,28,31,50,200,334,335,336,337,338,339],
    "WDate": job_that_runs_every_day(),
    "WFileType":[1,2,3,4,5]
}

def makeUrls(date):
    base_url = "https://kingstore.binaprojects.com/MainIO_Hok.aspx"
    params = {
        "WStore": kingStore["WStore"],
        "WDate": date,
        "WFileType": kingStore["WFileType"]
    }
    query_string = "&".join(f"{key}={value}" for key, value in params.items())
    return f"{base_url}?{query_string}"

if __name__ == "__main__":
    print(makeUrls(job_that_runs_every_day()))