import requests
import os
import storesJsonUrl
import json
import gzip
import shutil
import xml.dom.minidom
import zipfile

def ensure_download_dir():
    download_dir = os.path.join("service", "downloads")
    if not os.path.exists(download_dir):
        os.makedirs(download_dir)
    return download_dir

def clean_old_files(filepath_without_extension):
    """Remove any existing files with the same name but different extensions"""
    for ext in ['.gz', '.zip', '.xml', '']:
        file_to_remove = filepath_without_extension + ext
        if os.path.exists(file_to_remove):
            try:
                os.remove(file_to_remove)
                print(f"Removed existing file: {file_to_remove}")
            except Exception as e:
                print(f"Error removing file {file_to_remove}: {e}")

def display_xml_content(xml_file):
    try:
        with open(xml_file, 'r', encoding='utf-8') as f:
            content = f.read()
            # Parse and pretty print the XML
            dom = xml.dom.minidom.parseString(content)
            pretty_xml = dom.toprettyxml(indent="  ")
            print("\nXML Content:")
            print("=" * 80)
            print(pretty_xml)
            print("=" * 80)
    except Exception as e:
        print(f"Error displaying XML file {xml_file}: {e}")

def extract_file(compressed_file):
    try:
        # Get the base name without extension
        output_file = os.path.splitext(compressed_file)[0]
        
        # Try to extract as gzip first
        try:
            with gzip.open(compressed_file, 'rb') as f_in:
                with open(output_file + '.xml', 'wb') as f_out:
                    shutil.copyfileobj(f_in, f_out)
            print(f"Extracted GZ file: {output_file}.xml")
            extracted_file = output_file + '.xml'
        except OSError:
            # If gzip fails, try ZIP
            try:
                with zipfile.ZipFile(compressed_file, 'r') as zip_ref:
                    # Get the first file in the ZIP
                    first_file = zip_ref.namelist()[0]
                    zip_ref.extract(first_file, os.path.dirname(output_file))
                    # Rename the extracted file to .xml
                    extracted_name = os.path.splitext(first_file)[0] + '.xml'
                    extracted_file = os.path.join(os.path.dirname(output_file), extracted_name)
                    os.rename(os.path.join(os.path.dirname(output_file), first_file), extracted_file)
                print(f"Extracted ZIP file: {extracted_file}")
            except zipfile.BadZipFile:
                print(f"File is neither a valid GZ nor ZIP file: {compressed_file}")
                if os.path.exists(compressed_file):
                    os.remove(compressed_file)
                return False
        
        # Remove the original compressed file only after successful extraction
        if os.path.exists(compressed_file):
            os.remove(compressed_file)
            print(f"Removed compressed file: {compressed_file}")
        
        # Display the XML content
        display_xml_content(extracted_file)
        return True
    except Exception as e:
        print(f"Error extracting {compressed_file}: {e}")
        if os.path.exists(compressed_file):
            os.remove(compressed_file)
        return False

def requestZip(url):
    print(f"Trying to download: {url}")
    response = requests.get(url)
    print(f"Response status: {response.status_code}")
    if response.status_code == 200:
        try:
            # Parse the JSON response
            data = response.json()
            if isinstance(data, list) and len(data) > 0 and "SPath" in data[0]:
                # Get the actual file URL from the JSON
                file_url = data[0]["SPath"]
                print(f"Found file URL: {file_url}")
                
                # Download the actual file
                file_response = requests.get(file_url)
                if file_response.status_code == 200:
                    # Ensure download directory exists
                    download_dir = ensure_download_dir()
                    filename = file_url.split("/")[-1]
                    filepath = os.path.join(download_dir, filename)
                    
                    # Clean up any existing files with the same name
                    base_path = os.path.splitext(filepath)[0]
                    for ext in ['.gz', '.zip', '.xml', '']:
                        existing_file = base_path + ext
                        if os.path.exists(existing_file):
                            os.remove(existing_file)
                            print(f"Cleaned up existing file: {existing_file}")
                    
                    # Save the downloaded file
                    with open(filepath, "wb") as file:
                        file.write(file_response.content)
                    print(f"Successfully downloaded: {filename}")
                    
                    # Extract the compressed file
                    return extract_file(filepath)
                else:
                    print(f"Failed to download file from {file_url} - Status code: {file_response.status_code}")
            else:
                print("Invalid JSON response format")
        except json.JSONDecodeError:
            print("Failed to parse JSON response")
        return False
    else:
        print(f"Failed to download {url} - Status code: {response.status_code}")
        return False

def makeUrls():
    store = storesJsonUrl.kingStore  # Use only kingStore
    success_count = 0
    failure_count = 0
    
    print(f"\nProcessing KingStore (Chain ID: {store['ChainId']})")
    for storeId in store["StoreId"]:
        print(f"\nTrying Store ID: {storeId}")
        for fileType in store["WFileType"]:
            print(f"\nFile Type: {fileType}")
            for timestamp in store["WDate"]:
                # Make sure we're using the correct URL format
                url = f"{store['Url']}{fileType}{store['ChainId']}-{storeId}-{timestamp}.gz"
                if requestZip(url):
                    success_count += 1
                else:
                    failure_count += 1
                
    print(f"\nDownload Summary:")
    print(f"Successful downloads: {success_count}")
    print(f"Failed downloads: {failure_count}")
    print(f"Total attempts: {success_count + failure_count}")

if __name__ == "__main__":
    makeUrls()