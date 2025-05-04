from abc import ABC, abstractmethod
import requests
import os
import json
import gzip
import shutil

class StoreDownloader(ABC):

    def __init__(self, store_config):
        self.config = store_config
        self.download_dir = self._ensure_download_dir()
        self.success_count = 0
        self.failure_count = 0


    @abstractmethod
    def _generate_urls(self):
        pass
    
    def _ensure_download_dir(self):
        download_dir = os.path.join("service", "downloads")
        if not os.path.exists(download_dir):
            os.makedirs(download_dir)
            print("created directory for downloaded xml files")

        return download_dir
    
    def _clean_old_files(self, filepath_without_extension):
        extensions_to_check = ['.gz', '']
        print("start cleaning gz extention files or incomplete files")

        for ext in extensions_to_check:
            file_to_remove = filepath_without_extension + ext

            if os.path.exists(file_to_remove):
                try:
                    os.remove(file_to_remove)
                    print(f"  Removed existing file: {os.path.basename(file_to_remove)}")
                except Exception as e:
                    print(f"  Error removing file {os.path.basename(file_to_remove)}: {e}")
                
    def _extract_file(self, compressed_file):
        output_file_path = os.path.splitext(compressed_file)[0] + '.xml'

        try:
            print("starting attempt on extraction")
            with gzip.open(compressed_file, 'rb') as f_in:
                with open(output_file_path, 'wb') as f_out:
                    shutil.copyfileobj(f_in, f_out)
            print("extracted successfully!")
            return output_file_path
        except (OSError, gzip.BadGzipFile) as e_gz:
            print(f"  Error: GZ extraction failed for {os.path.basename(compressed_file)}. Error: {e_gz}")
            if os.path.exists(output_file_path):
                try:
                    os.remove(output_file_path)
                except Exception as e_remove:
                    print(f"  Error removing incomplete output file {os.path.basename(output_file_path)}: {e_remove}")
            return None
        except Exception as e:
            print(f"  Unexpected error during extraction of {os.path.basename(compressed_file)}: {e}")
            if os.path.exists(output_file_path):
                try:
                    os.remove(output_file_path)
                except Exception as e_remove:
                    print(f"  Error removing output file {os.path.basename(output_file_path)} after error: {e_remove}")
            return None
        finally:
            if os.path.exists(compressed_file):
                try:
                    os.remove(compressed_file)
                except Exception as e_remove:
                    print(f"  Error removing compressed file {os.path.basename(compressed_file)}: {e_remove}")
   

    def _request_and_save_file(self, url, filename_base):
        """Downloads a file from a URL, saves it, and returns the saved path or None on failure."""
        print(f"Attempting download from: {url}")
        try:
            # Make the HTTP GET request with a timeout
            response = requests.get(url, timeout=30)
            # Raise an exception for bad status codes (like 404, 500)
            response.raise_for_status()
            print(f"  Download successful (Status: {response.status_code})")

            # --- Determine filename ---
            # Basic approach: use the base name provided and assume .gz
            filename = filename_base + ".gz" # Assuming .gz for the base class
            filepath = os.path.join(self.download_dir, filename)
            filepath_without_extension = os.path.splitext(filepath)[0]

            # --- Clean old files before saving ---
            self._clean_old_files(filepath_without_extension)

            # --- Save the downloaded content ---
            with open(filepath, "wb") as file:
                file.write(response.content)
            print(f"  Successfully saved file: {filename}")
            # Return the full path to the saved file
            return filepath

        except requests.exceptions.RequestException as e:
            # Handle network-related errors (DNS failure, refused connection, timeout, bad status code etc.)
            print(f"  Error: Download failed for {url}. Error: {e}")
            return None # Indicate failure
        except Exception as e:
            # Handle any other unexpected errors during download/saving
            print(f"  An unexpected error occurred during download from {url}: {e}")
            # Clean up potentially partially saved file if error occurred during save
            if 'filepath' in locals() and os.path.exists(filepath):
                 try:
                     os.remove(filepath)
                     print(f"  Removed partially saved file: {filename}")
                 except Exception as e_remove:
                     print(f"  Error removing partially saved file {filename}: {e_remove}")
            return None # Indicate failure


