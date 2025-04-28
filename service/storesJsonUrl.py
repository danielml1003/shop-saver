import datetime


def job_that_runs_every_hour():
    now = datetime.datetime.now().strftime("%Y%m%d%H%M")
    return now

fileType = ["StoresFull", "Price", "Promo", "PriceFull", "PromoFull"]


kingStore  = {
    "Url": "https://kingstore.binaprojects.com/Download.aspx?FileNm=",
    "WFileType": fileType,
    "ChainId":7290058108879,
    "StoreId": ["001","002","003","005","006","007","008","009","010","012","013","014","015","016","017","018","019","027","028","031","050","200","334","335","336","337","338","339"],
    "WDate": job_that_runs_every_hour()
}

mayaanStore = {
    "Url": "https://maayan2000.binaprojects.com/Download.aspx?FileNm=",
    "WFileType": fileType,
    "ChainId":7290058159628,
    "StoreId": ["001","002","003","004","005","006","007","008","009","010","011","013","016","017","018","019","020","021","022","023","026","027","028","029","035","038","039","040","041","043","044","045","046","047","048","049","050","060","061","062","063"],
    "WDate": job_that_runs_every_hour()
}

goodPharm = {
    "Url": "https://goodpharm.binaprojects.com/Download.aspx?FileNm=",
    "WFileType": fileType,
    "ChainId":7290058197699,
    "StoreId": ["001","002","003","004","005","006","007","008","009","010","011","012","013","014","015","016","017","018","019","020","021","022","023","024","025","026","027","028","029","030","031","032","033","034","035","036","037","038","039","040","041","042","043","044","045","046","047","048","049","050","051","052","053","054","055","056","057","058","059","060","061","062","063"],
    "WDate": job_that_runs_every_hour()
}

#remember to update the storeId
dorAlon ={
    "Url": "https://url.publishedprices.co.il/file/d/",
    "WFileType": fileType,
    "ChainId":7290492000005,
    "StoreId": ["001","002","003","004","005","006","007","008","009","010","011","012","013","014","015","016","017","018","019","020","021","022","023","024","025","026","027","028","029","030","031","032","033","034","035","036","037","038","039","040","041","042","043","044","045","046","047","048","049","050","051","052","053","054","055","056","057","058","059","060","061","062","063"],
    "WDate": job_that_runs_every_hour()
}

stores = [kingStore, mayaanStore, goodPharm]

if __name__ == "__main__":
    print(job_that_runs_every_hour())


