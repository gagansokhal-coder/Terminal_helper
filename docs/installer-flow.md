# Installer Flow

1. **Detect platform**
   - linux-x86_64
   - linux-aarch64
   - wsl
   - windows-x86_64 (via PowerShell)

2. **Query GitHub API**
   - URL: `https://api.github.com/repos/gagansokhal-coder/Terminal_helper/releases`

3. **Select newest prerelease**
   - Parse the JSON response to find the latest available prerelease version.

4. **Select matching asset**
   - Find the appropriate `.tar.gz` (Linux) or `.zip` (Windows) asset based on the detected platform.

5. **Download bundle**
   - Fetch the selected asset from its `browser_download_url`.

6. **Download checksums.txt**
   - Fetch the `checksums.txt` file from the same release.

7. **Verify SHA256**
   - Compute the SHA256 hash of the downloaded bundle and ensure it matches the expected hash in `checksums.txt`.

8. **Extract bundle**
   - Extract the contents of the verified tarball/zip.

9. **Execute install script**
   - Run the included `install.sh` (Linux) or `install.ps1` (Windows) script to set up binaries and shell hooks.

10. **Run doctor**
    - Execute `ggnmem doctor` to verify the installation and system health.

11. **Print success message**
    - Notify the user that the installation completed successfully.

## Failure Modes

* **No internet**: The installer cannot reach GitHub API or download assets.
* **Unsupported architecture**: The detected platform does not have a matching pre-compiled release.
* **Checksum mismatch**: The downloaded bundle's hash does not match `checksums.txt` (possible corruption or tampering).
* **Missing asset**: The release exists, but the required `.tar.gz`/`.zip` or `checksums.txt` asset is missing.
* **Install failure**: The `install.sh`/`install.ps1` script encounters an error (e.g., missing permissions, unable to modify shell rc files).
