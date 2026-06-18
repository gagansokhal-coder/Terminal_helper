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
