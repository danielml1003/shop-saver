import ftplib
import os
import datetime
from .base import StoreDownloader


class CeberusStoreDownloader(StoreDownloader):
    FTP_HOST = "url.retail.publishedprices.co.il"

    def _generate_urls(self):
        return []  # not used; process_store overrides entirely

    def process_store(self):
        username = self.config["ftp_username"]
        password = self.config.get("ftp_password", username)
        chain_id = str(self.config["ChainId"])
        file_types = self.config.get("WFileType", [])
        today = datetime.date.today().strftime("%Y%m%d")

        self.success_count = 0
        self.failure_count = 0

        print(f"\nProcessing Store (FTP): {chain_id}")

        use_active = self.config.get("ftp_active_mode", False)
        try:
            with ftplib.FTP(self.FTP_HOST) as ftp:
                ftp.login(username, password)
                if use_active:
                    ftp.set_pasv(False)
                all_files = ftp.nlst()
        except ftplib.all_errors as e:
            print(f"FTP listing error for {chain_id}: {e}")
            return

        matching = [
            f for f in all_files
            if chain_id in f and today in f
            and any(f.lower().startswith(ft.lower()) for ft in file_types)
        ]

        print(f"  Found {len(matching)} matching file(s) for today.")

        for filename in matching:
            local_path = os.path.join(self.download_dir, filename)
            print(f"\n  Downloading: {filename}")
            try:
                with ftplib.FTP(self.FTP_HOST) as ftp:
                    ftp.login(username, password)
                    if use_active:
                        ftp.set_pasv(False)
                    with open(local_path, "wb") as f:
                        ftp.retrbinary(f"RETR {filename}", f.write)
                extracted = self._extract_file(local_path)
                if extracted:
                    print(f"    Successfully extracted: {os.path.basename(extracted)}")
                    self.success_count += 1
                else:
                    self.failure_count += 1
            except ftplib.all_errors as e:
                print(f"  FTP download error for {filename}: {e}")
                self.failure_count += 1

        print(f"\nFinished FTP store: {chain_id}")
        print(f"  Successful: {self.success_count}")
        print(f"  Failed: {self.failure_count}")
