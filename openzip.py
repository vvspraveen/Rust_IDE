import zipfile
import os
import sys
import subprocess
 
def unzip_and_open(zip_path):
    if not zipfile.is_zipfile(zip_path):
        print("Error: Not a valid ZIP file.")
        return
 
    extract_dir = zip_path.replace('.zip', '')
 
    with zipfile.ZipFile(zip_path, 'r') as zip_ref:
        zip_ref.extractall(extract_dir)
        print(f"Extracted to: {extract_dir}")
 
    # Use the full path to code.cmd
    vscode_cmd = r"/snap/bin/code"
    try:
        subprocess.run([vscode_cmd, extract_dir])
    except Exception as e:
        print(f"Error launching VSCode: {e}")
 
if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python open_zip_in_vscode.py <zip_file_path>")
    else:
        unzip_and_open(sys.argv[1])