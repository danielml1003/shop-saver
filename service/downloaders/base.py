from abc import ABC, abstractmethod
import requests
import os
import json
import gzip
import shutil
import zipfile # Import the zipfile module

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
        print("starting attempt on extraction")
        output_file_path = None # Initialize to None

        try:
            # Read the first few bytes to determine file type
            with open(compressed_file, 'rb') as f:
                signature = f.read(2)

            if signature == b'PK':
                # It's a ZIP file
                print("  Detected ZIP file.")
                with zipfile.ZipFile(compressed_file, 'r') as zip_ref:
                    file_list = zip_ref.namelist()
                    if not file_list:
                        print(f"  Error: ZIP file {os.path.basename(compressed_file)} is empty.")
                        return None

                    # Construct the desired output file path based on the compressed file's base name
                    output_file_path = os.path.splitext(compressed_file)[0] + '.xml'
                    print(f"  Attempting to write extracted content to: {output_file_path}") # Added logging

                    # Read the content of the first file in the zip
                    with zip_ref.open(file_list[0]) as zf:
                        file_content = zf.read()

                    # Write the content to the desired output file
                    with open(output_file_path, 'wb') as f_out:
                        f_out.write(file_content)

                    print("  Extracted and saved successfully (ZIP)!")
                    if os.path.exists(output_file_path): # Added check
                        print(f"  Confirmed: Extracted file exists at {output_file_path}") # Added logging
                    else:
                        print(f"  Warning: Extracted file NOT found at {output_file_path} immediately after writing.") # Added logging


            elif signature == b'\x1f\x8b':
                # It's a GZIP file (standard gzip magic number)
                print("  Detected GZIP file.")
                output_file_path = os.path.splitext(compressed_file)[0] + '.xml'
                print(f"  Attempting to write extracted content to: {output_file_path}") # Added logging
                with gzip.open(compressed_file, 'rb') as f_in:
                    with open(output_file_path, 'wb') as f_out:
                        shutil.copyfileobj(f_in, f_out)
                print("  Extracted successfully (GZIP)!")
                if os.path.exists(output_file_path): # Added check
                    print(f"  Confirmed: Extracted file exists at {output_file_path}") # Added logging
                else:
                    print(f"  Warning: Extracted file NOT found at {output_file_path} immediately after writing.") # Added logging


            else:
                print(f"  Error: Unknown file signature for {os.path.basename(compressed_file)}: {signature}")
                return None

            return output_file_path

        except (OSError, gzip.BadGzipFile) as e_gz:
            print(f"  Error: GZ extraction failed for {os.path.basename(compressed_file)}. Error: {e_gz}")
            if output_file_path and os.path.exists(output_file_path):
                try:
                    os.remove(output_file_path)
                except Exception as e_remove:
                    print(f"  Error removing incomplete output file {os.path.basename(output_file_path)}: {e_remove}")
            return None
        except zipfile.BadZipFile as e_zip:
             print(f"  Error: ZIP extraction failed for {os.path.basename(compressed_file)}. Error: {e_zip}")
             if output_file_path and os.path.exists(output_file_path):
                try:
                    os.remove(output_file_path)
                except Exception as e_remove:
                    print(f"  Error removing incomplete output file {os.path.basename(output_file_path)}: {e_remove}")
             return None
        except Exception as e:
            print(f"  Unexpected error during extraction of {os.path.basename(compressed_file)}: {e}")
            if output_file_path and os.path.exists(output_file_path):
                try:
                    os.remove(output_file_path)
                except Exception as e_remove:
                    print(f"  Error removing output file {os.path.basename(output_file_path)} after error: {e_remove}")
            return None
        finally:
            # Clean up the compressed file after extraction attempt
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

    def process_store(self):
        """
        Main method to process all downloads for the configured store.
        It generates URLs, downloads files, extracts them, and counts successes/failures.
        """
        print(f"\nProcessing Store: {self.config.get('ChainId', 'Unknown Chain ID')}")
        # Reset counters for this processing run
        self.success_count = 0
        self.failure_count = 0

        # This will be implemented by subclasses to provide specific URLs
        urls_to_process = self._generate_urls()

        if not urls_to_process:
            print("  No URLs generated for this store. Skipping.")
            return

        print(f"  Generated {len(urls_to_process)} URLs to process.")

        for url_info in urls_to_process:
            # url_info is expected to be a dictionary, e.g.,
            # {'url': 'http://example.com/file.gz', 'store_id': '001', 'file_type': 'Price', 'timestamp': '202301010000'}
            url = url_info.get('url')
            if not url:
                print("  Skipping entry with no URL in url_info.")
                self.failure_count +=1
                continue

            filename_base = (
                f"{self.config.get('ChainId', 'NA')}-"
                f"{url_info.get('store_id', 'NA')}-"
                f"{url_info.get('file_type', 'NA')}-"
                f"{url_info.get('timestamp', 'NA')}"
            )

            print(f"\n  Processing URL: {url}")
            compressed_filepath = self._request_and_save_file(url, filename_base)

            if compressed_filepath:
                extracted_xml_path = self._extract_file(compressed_filepath)
                if extracted_xml_path:
                    print(f"    Successfully processed and extracted: {os.path.basename(extracted_xml_path)}")
                    self.success_count += 1
                else:
                    print(f"    Extraction failed for file from URL: {url}")
                    self.failure_count += 1
            else:
                print(f"    Download failed for URL: {url}")
                self.failure_count += 1

        print(f"\nFinished processing Store: {self.config.get('ChainId', 'Unknown Chain ID')}")
        print(f"  Successful operations: {self.success_count}")
        print(f"  Failed operations: {self.failure_count}")
        print(f"  Total attempts: {self.success_count + self.failure_count}")
