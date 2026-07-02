import ftplib
import os
import datetime
from .base import StoreDownloader


class CerberusStoreDownloader(StoreDownloader):
    FTP_HOST = "url.retail.publishedprices.co.il"

    def _generate_urls(self):
        return []  # not used; process_store overrides entirely

    def _open_ftp(self, username, password, active):
        ftp = ftplib.FTP(self.FTP_HOST, timeout=60)
        ftp.login(username, password)
        ftp.set_pasv(not active)
        return ftp

    def process_store(self):
        username = self.config["ftp_username"]
        password = self.config.get("ftp_password", username)
        chain_id = str(self.config["ChainId"])
        file_types = self.config.get("WFileType", [])
        today = datetime.date.today().strftime("%Y%m%d")

        self.success_count = 0
        self.failure_count = 0

        print(f"\nProcessing Store (FTP): {chain_id}")

        # Try the configured mode first, then the other one. Active mode fails
        # behind NAT (e.g. inside Docker: '500 Port command invalid'); passive
        # mode is blocked on some direct networks — so neither can be pinned.
        preferred_active = self.config.get("ftp_active_mode", False)
        modes = (preferred_active, not preferred_active)

        all_files = None
        working_mode = None
        for active in modes:
            mode_name = "active" if active else "passive"
            try:
                with self._open_ftp(username, password, active) as ftp:
                    all_files = ftp.nlst()
                working_mode = active
                print(f"  FTP listing OK in {mode_name} mode ({len(all_files)} entries).")
                break
            except ftplib.all_errors as e:
                print(f"  FTP {mode_name} mode failed for {chain_id}: {e}")

        if all_files is None:
            print(f"FTP listing error for {chain_id}: both modes failed")
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
                with self._open_ftp(username, password, working_mode) as ftp:
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
