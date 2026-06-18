# Install Online Design

## Why the Bootstrap Installer Exists

The `install-online.sh` script serves as the primary entry point for users to get `ggnmem` running on their systems via a single `curl | bash` command. Its purpose is to securely bootstrap the environment by detecting the system platform, resolving the correct binary bundle, verifying its integrity, and managing the initial installation setup—all before `ggnmem` itself is available to take over.

## Why GitHub Releases are Used

We utilize GitHub Releases as our source of truth for distribution for several key reasons:

* **Reliability:** GitHub provides high-availability infrastructure for hosting and distributing binary assets.
* **Simplicity:** It avoids the need for maintaining separate package repositories or mirrored hosting endpoints.
* **API Access:** The GitHub REST API allows the script to dynamically query for the latest version and discover download links without hardcoding versions in the script.
* **Security:** Version tags and release assets on GitHub offer a verifiable chain of custody, especially when combined with our `checksums.txt` SHA256 hashes.

## How Platform Selection Works

The installer utilizes standard UNIX utilities (`uname`) to automatically detect the user's environment:

* `uname -s` is used to detect the operating system (e.g., Linux). It also checks `/proc/version` for WSL contexts to provide accurate telemetry or handling if needed.
* `uname -m` identifies the system architecture, specifically targeting `x86_64` and `aarch64` (or `arm64`).
* These values are combined to form a target string (e.g., `linux-x86_64` or `linux-aarch64`), which is then mapped directly to the naming convention of the `.tar.gz` assets uploaded to the GitHub Release. If a system is unsupported, the script intentionally fails fast before any download attempts.

## Download Strategy

Once the correct release and asset URL are identified, the script proceeds to the download phase:

* **Temporary Directory:** All artifacts are safely downloaded to `/tmp/ggnmem-installer/` to prevent cluttering the user's working directory or interfering with existing installations prematurely.
* **Asset Retrieval:** The script downloads both the `.tar.gz` release bundle and the `checksums.txt` file using `curl` with fail-fast flags.
* **Integrity Validation:** Before any extraction happens, the script confirms that the downloaded files exist on disk and are not empty, protecting against silent network drops or "disk full" scenarios.
* **Safe Staging:** At this stage, no system modifications have been made, meaning the script can safely abort without leaving the system in a broken state.

## Verification Strategy

After a successful download, the script performs a strict checksum verification:

* **Hash Parsing:** The script locates the specific hash for the downloaded asset within `checksums.txt`. If the hash is missing, the script aborts immediately.
* **Hash Computation:** It computes the actual SHA256 hash of the downloaded tarball using either `sha256sum` or `shasum -a 256`, depending on what is available on the user's system.
* **Comparison:** The expected hash is compared against the actual computed hash.
* **Failure Handling:** If the hashes do not match, this indicates a corrupted download or tampering. The script immediately deletes the staged files in `/tmp/ggnmem-installer`, prints a clear error message, and exits with a non-zero status code. No installation occurs.

## Extraction Strategy

Once the download is verified, the script carefully unpacks the bundle:

* **Staging Area:** The tarball is extracted into `/tmp/ggnmem-installer/extracted` to keep the operation completely isolated from the system binaries.
* **Content Validation:** The script verifies the internal structure of the extracted archive. It strictly asserts the presence of all required artifacts: `ggnmem`, `ggnmem-daemon`, `install.sh`, `VERSION`, and `checksums.txt`.
* **Version Identification:** It reads the included `VERSION` file to expose exactly which version bundle is being prepared for installation.
* **Abortion Safety:** If any expected files are missing or the tarball extraction fails, the script deletes the extracted directory and aborts. No partial state is left behind.

## Installation Strategy

Once extraction is complete and validated, the script defers to the official bundled installer:

* **Delegation:** The `install-online.sh` script executes `install.sh` found inside the extracted bundle. This ensures the installation logic is tightly coupled with the specific release version, rather than relying on a static script that might go out-of-sync with future release structures.
* **Output Forwarding:** The standard output and standard error from `install.sh` are passed directly to the terminal, keeping the user informed of setup operations (e.g., copying binaries, installing shell hooks).
* **Failure Handling:** If `install.sh` returns a non-zero exit code, the bootstrap script prints a helpful error message and aborts. The rollback mechanisms (restoring `.old` backups) are expected to be handled internally by `install.sh`.
* **Final Verification:** Upon successful installation, the script runs `ggnmem version` to prove the executable is correctly placed in the system `PATH` and fully operational.
* **Cleanup:** The `/tmp/ggnmem-installer` directory is wiped clean to leave no traces.
