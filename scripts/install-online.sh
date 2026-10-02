#!/usr/bin/env bash

# Bootstrap installer prototype for ggnmem
# This script detects the platform and finds the correct release asset
# without downloading it yet.

set -euo pipefail

# ---------------------------------------------------------------------------
# Dependency check
# ---------------------------------------------------------------------------
MISSING_DEPS=()
for dep in curl tar mktemp; do
    if ! command -v "$dep" >/dev/null 2>&1; then
        MISSING_DEPS+=("$dep")
    fi
done

if [ ${#MISSING_DEPS[@]} -gt 0 ]; then
    echo "Error: Missing required command(s): ${MISSING_DEPS[*]}"
    echo "DEBUG: install them with your package manager, e.g.:"
    echo "DEBUG:   Debian/Ubuntu: sudo apt-get install -y ${MISSING_DEPS[*]}"
    echo "DEBUG:   Fedora/RHEL:   sudo dnf install -y ${MISSING_DEPS[*]}"
    echo "DEBUG:   Alpine:        sudo apk add ${MISSING_DEPS[*]}"
    exit 1
fi

# ---------------------------------------------------------------------------
# Cleanup
#
# The temp directory is created with mktemp -d, so another local user cannot
# pre-create it or plant a symlink inside it. The trap then guarantees that a
# partial download is removed on every exit path, including Ctrl-C.
# ---------------------------------------------------------------------------
TMP_DIR=""
cleanup() {
    if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
        rm -rf "$TMP_DIR"
    fi
}
trap cleanup EXIT INT TERM

# 1. Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

# Normalize platform
if [ "$OS" = "Linux" ]; then
    if grep -qi microsoft /proc/version 2>/dev/null; then
        # This is WSL
        PLATFORM="linux"
    else
        PLATFORM="linux"
    fi
else
    echo "Error: Unsupported OS $OS"
    case "$OS" in
        Darwin)
            echo "DEBUG: ggnmem does not ship a macOS release asset."
            echo "DEBUG: Build from source instead:"
            echo "DEBUG:   git clone https://github.com/gagansokhal-coder/Terminal_helper"
            echo "DEBUG:   cd Terminal_helper && cargo build --release --workspace"
            echo "DEBUG: See INSTALL.md for details."
            ;;
        *)
            echo "DEBUG: Supported platforms are Linux and WSL (x86_64, aarch64)."
            echo "DEBUG: See https://github.com/gagansokhal-coder/Terminal_helper/blob/main/INSTALL.md"
            ;;
    esac
    exit 1
fi

# Normalize architecture
if [ "$ARCH" = "x86_64" ]; then
    TARGET_ARCH="x86_64"
elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
    TARGET_ARCH="aarch64"
else
    echo "Error: Unsupported architecture $ARCH"
    echo "DEBUG: Supported architectures are x86_64 and aarch64/arm64."
    echo "DEBUG: No prebuilt binary exists for $ARCH; build from source with:"
    echo "DEBUG:   cargo build --release --workspace"
    exit 1
fi

DETECTED_TARGET="${PLATFORM}-${TARGET_ARCH}"
echo "Detected platform: ${DETECTED_TARGET}"
echo ""

# 2. Query GitHub API for the releases
REPO="gagansokhal-coder/Terminal_helper"
API_URL="https://api.github.com/repos/${REPO}/releases"

# Send the token when the user has one: an authenticated request gets a much
# higher rate limit, which is the usual fix for the 403 handled below.
AUTH_HEADER=()
if [ -n "${GITHUB_TOKEN:-}" ]; then
    AUTH_HEADER=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
    echo "DEBUG: Using GITHUB_TOKEN for the GitHub API request."
else
    echo "DEBUG: GITHUB_TOKEN not set, using unauthenticated API request (60 req/h)."
fi

echo "DEBUG: Fetching release information from $API_URL..."
# Capture the HTTP status code alongside the body. curl -f would discard the
# body on any 4xx/5xx, which is exactly the case we need to diagnose below
# (a rate-limited 403), so the status code is requested explicitly instead.
# The ${arr[@]+...} form keeps the unset/empty array safe under `set -u` on
# bash < 4.4 (e.g. CentOS 7), which a bootstrap installer must survive.
API_CURL_RC=0
API_RESPONSE=$(curl -sSL -w $'\n%{http_code}' ${AUTH_HEADER[@]+"${AUTH_HEADER[@]}"} "$API_URL" 2>/dev/null) || API_CURL_RC=$?

API_STATUS="${API_RESPONSE##*$'\n'}"
RELEASES_JSON="${API_RESPONSE%$'\n'*}"

# A 403/429 is a quota or policy problem, not a connectivity problem.
if [ "$API_STATUS" = "403" ] || [ "$API_STATUS" = "429" ]; then
    if printf '%s' "$RELEASES_JSON" | grep -qi "rate limit"; then
        echo "Error: GitHub API rate limit exceeded (HTTP $API_STATUS)."
        echo "DEBUG: Unauthenticated requests are limited to 60 per hour per IP address."
        echo "DEBUG: Wait and retry, or set a token to raise the limit:"
        echo "DEBUG:   GITHUB_TOKEN=<your token> $0"
        echo "DEBUG: Create one at https://github.com/settings/tokens"
    else
        echo "Error: GitHub API request was refused (HTTP $API_STATUS)."
        echo "DEBUG: Response body:"
        printf '%s\n' "$RELEASES_JSON" | head -n 10 | sed 's/^/DEBUG: /'
    fi
    exit 1
fi

if [ "$API_CURL_RC" -ne 0 ] || [ "$API_STATUS" != "200" ]; then
    echo "Error: Failed to fetch releases from GitHub API."
    if [ -n "$API_STATUS" ] && [ "$API_STATUS" != "000" ]; then
        echo "DEBUG: GitHub responded with HTTP $API_STATUS."
        printf '%s\n' "$RELEASES_JSON" | head -n 10 | sed 's/^/DEBUG: /'
    else
        echo "DEBUG: Ensure you have internet connectivity and can reach api.github.com."
    fi
    exit 1
fi

# Verify the response is not empty
if [ -z "$RELEASES_JSON" ] || [ "$RELEASES_JSON" = "[]" ]; then
    echo "Error: GitHub API returned an empty release list."
    exit 1
fi

# 3. Select newest prerelease
# GitHub API returns releases in chronological order, so the first "tag_name" is the newest.
if command -v jq >/dev/null 2>&1; then
    echo "DEBUG: jq detected, using jq to parse JSON."
    LATEST_VERSION=$(echo "$RELEASES_JSON" | jq -r '.[0].tag_name')
else
    echo "DEBUG: jq not found, using grep/sed fallback."
    set +o pipefail
    LATEST_VERSION=$(echo "$RELEASES_JSON" | grep -m 1 '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    set -o pipefail
fi

if [ -z "$LATEST_VERSION" ] || [ "$LATEST_VERSION" = "null" ]; then
    echo "Error: Could not determine latest version from GitHub API response."
    echo "DEBUG: API Response Preview:"
    echo "$RELEASES_JSON" | head -n 10
    exit 1
fi

echo "Latest release:"
echo "$LATEST_VERSION"
echo ""

# 4. Select matching asset
# The asset name format is expected to be ggnmem-{platform}-{arch}.tar.gz
EXPECTED_ASSET="ggnmem-${DETECTED_TARGET}.tar.gz"

# Extract asset download URLs
if command -v jq >/dev/null 2>&1; then
    ASSET_URL=$(echo "$RELEASES_JSON" | jq -r ".[0].assets[] | select(.name == \"$EXPECTED_ASSET\") | .browser_download_url")
    CHECKSUM_URL=$(echo "$RELEASES_JSON" | jq -r '.[0].assets[] | select(.name == "checksums.txt") | .browser_download_url')
else
    set +o pipefail
    ASSET_URL=$(echo "$RELEASES_JSON" | grep '"browser_download_url":' | grep "$EXPECTED_ASSET" | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/')
    CHECKSUM_URL=$(echo "$RELEASES_JSON" | grep '"browser_download_url":' | grep "checksums.txt" | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/')
    set -o pipefail
fi

if [ -z "$ASSET_URL" ] || [ "$ASSET_URL" = "null" ]; then
    echo "Error: Asset $EXPECTED_ASSET not found in release $LATEST_VERSION."
    echo "DEBUG: This release may not ship a binary for $DETECTED_TARGET."
    echo "DEBUG: List the available assets at:"
    echo "DEBUG:   https://github.com/${REPO}/releases/tag/${LATEST_VERSION}"
    exit 1
fi

if [ -z "$CHECKSUM_URL" ] || [ "$CHECKSUM_URL" = "null" ]; then
    echo "Error: Could not find checksums.txt in release $LATEST_VERSION."
    echo "DEBUG: Without it the download cannot be verified, so installation is aborted."
    echo "DEBUG: Check the release assets at:"
    echo "DEBUG:   https://github.com/${REPO}/releases/tag/${LATEST_VERSION}"
    exit 1
fi

echo "Selected asset:"
echo "$EXPECTED_ASSET"
echo ""

# 5. Create temp directory
# mktemp -d creates the directory atomically with owner-only permissions, so
# another local user cannot pre-create it or symlink files into it.
if ! TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/ggnmem-installer.XXXXXXXX"); then
    echo "Error: Failed to create a temporary directory."
    echo "DEBUG: Check your disk space and that ${TMPDIR:-/tmp} is writable."
    exit 1
fi

ASSET_PATH="$TMP_DIR/$EXPECTED_ASSET"
CHECKSUM_PATH="$TMP_DIR/checksums.txt"

# 6. Download files
echo "Downloading release bundle..."
if ! curl -sSL -f -o "$ASSET_PATH" "$ASSET_URL"; then
    echo "Error: Failed to download release bundle from $ASSET_URL."
    echo "DEBUG: Check your internet connection or if GitHub API rate limiting blocked the download."
    exit 1
fi

echo "Downloading checksums..."
if ! curl -sSL -f -o "$CHECKSUM_PATH" "$CHECKSUM_URL"; then
    echo "Error: Failed to download checksums from $CHECKSUM_URL."
    echo "DEBUG: Check your internet connection."
    exit 1
fi

# Verify files exist after download
if [ ! -s "$ASSET_PATH" ] || [ ! -s "$CHECKSUM_PATH" ]; then
    echo "Error: Files missing or empty after download. Disk might be full or write failed silently."
    exit 1
fi

# 7. Print success
echo ""
echo "Download complete"
echo ""
echo "Bundle:"
echo "$ASSET_PATH"
echo ""
echo "Checksums:"
echo "$CHECKSUM_PATH"
echo ""

# 8. Checksum verification
echo "Verifying checksum..."

# Match the filename field exactly. An unanchored grep also matches longer
# names that merely start with the same string (e.g. "<asset>.sig"), which
# would capture several hashes and make the comparison below fail spuriously.
# The leading '*' is stripped to also accept sha256sum's binary-mode marker.
set +o pipefail
EXPECTED_HASH=$(awk -v asset="$EXPECTED_ASSET" '{ name = $2; sub(/^\*/, "", name); if (name == asset) print $1 }' "$CHECKSUM_PATH")
set -o pipefail

if [ -z "$EXPECTED_HASH" ]; then
    echo "Error: Could not find checksum entry for $EXPECTED_ASSET in checksums.txt"
    exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL_HASH=$(sha256sum "$ASSET_PATH" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
    ACTUAL_HASH=$(shasum -a 256 "$ASSET_PATH" | awk '{print $1}')
else
    echo "Error: Neither sha256sum nor shasum is installed. Cannot verify checksum."
    echo "DEBUG: Install coreutils, or run: sudo apt-get install -y coreutils"
    exit 1
fi

echo "Expected SHA256: $EXPECTED_HASH"
echo "Actual SHA256:   $ACTUAL_HASH"

if [ "$EXPECTED_HASH" != "$ACTUAL_HASH" ]; then
    echo "Error: Checksum mismatch! The downloaded file may be corrupted or tampered with."
    echo "DEBUG: Nothing was installed. Re-run the installer, or download"
    echo "DEBUG: $EXPECTED_ASSET manually from:"
    echo "DEBUG:   https://github.com/${REPO}/releases/tag/${LATEST_VERSION}"
    exit 1
fi

echo ""
echo "✓ Checksum verified"
echo ""

# 9. Extract and validate bundle
echo "Extracting bundle..."
EXTRACT_DIR="$TMP_DIR/extracted"

if [ -d "$EXTRACT_DIR" ]; then
    rm -rf "$EXTRACT_DIR"
fi
mkdir -p "$EXTRACT_DIR"

if ! tar -xzf "$ASSET_PATH" -C "$EXTRACT_DIR"; then
    echo "Error: Failed to extract $ASSET_PATH"
    echo "DEBUG: The archive may be truncated even though its checksum matched,"
    echo "DEBUG: or the local tar implementation may be too old for this format."
    exit 1
fi

echo ""
echo "Archive extracted successfully"
echo ""
echo "Found:"

REQUIRED_FILES=("ggnmem" "ggnmem-daemon" "install.sh" "VERSION" "checksums.txt")
MISSING_FILES=0

for f in "${REQUIRED_FILES[@]}"; do
    if [ -f "$EXTRACT_DIR/$f" ]; then
        echo "✓ $f"
    else
        echo "✗ $f (Missing)"
        MISSING_FILES=1
    fi
done

if [ "$MISSING_FILES" -ne 0 ]; then
    echo "Error: Extracted bundle is missing required files."
    echo "DEBUG: The release bundle for $DETECTED_TARGET looks incomplete or corrupt."
    echo "DEBUG: Nothing was installed. Please report this at:"
    echo "DEBUG:   https://github.com/${REPO}/issues"
    exit 1
fi

echo ""
# Handle VERSION file which might be 'version=v0.3.6-alpha' or just 'v0.3.6-alpha'
if grep -q '^version=' "$EXTRACT_DIR/VERSION"; then
    BUNDLE_VERSION=$(grep '^version=' "$EXTRACT_DIR/VERSION" | sed -E 's/^version=v?//' | tr -d '[:space:]')
else
    BUNDLE_VERSION=$(head -n 1 "$EXTRACT_DIR/VERSION" | sed -E 's/^v?//' | tr -d '[:space:]')
fi
echo "Bundle version: $BUNDLE_VERSION"
echo ""
echo "✓ Bundle validation passed"
echo ""

# 10. Execute installation
echo "Starting installation..."
echo "----------------------------------------"

set +e
bash "$EXTRACT_DIR/install.sh"
INSTALL_STATUS=$?
set -e

if [ $INSTALL_STATUS -ne 0 ]; then
    echo "----------------------------------------"
    echo "Error: Installation script failed with exit code $INSTALL_STATUS."
    echo "DEBUG: System modifications have been rolled back by the installer if it failed mid-flight."
    echo "DEBUG: Re-run the installer to retry, or follow the manual steps in INSTALL.md:"
    echo "DEBUG:   https://github.com/${REPO}/blob/main/INSTALL.md"
    exit 1
fi

echo "----------------------------------------"
echo "✓ Installation complete"
echo ""

# 11. Verify installed version
INSTALLED_BIN="$HOME/.local/bin/ggnmem"

if command -v ggnmem >/dev/null 2>&1; then
    INSTALLED_VERSION=$(ggnmem --version 2>/dev/null || echo "unknown")
elif [ -x "$INSTALLED_BIN" ]; then
    INSTALLED_VERSION=$("$INSTALLED_BIN" --version 2>/dev/null || echo "unknown")
else
    echo "Error: Could not locate installed ggnmem binary to verify version."
    echo "DEBUG: The installer reported success, but no ggnmem binary is on PATH"
    echo "DEBUG: or at $INSTALLED_BIN. Check the installer output above for errors."
    exit 1
fi

echo "Installed version: $INSTALLED_VERSION"

# Clean up (the EXIT trap would do this too; explicit here for clarity)
cleanup
TMP_DIR=""

exit 0
