#!/usr/bin/env bash

# Bootstrap installer prototype for ggnmem
# This script detects the platform and finds the correct release asset
# without downloading it yet.

set -euo pipefail

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
    exit 1
fi

# Normalize architecture
if [ "$ARCH" = "x86_64" ]; then
    TARGET_ARCH="x86_64"
elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
    TARGET_ARCH="aarch64"
else
    echo "Error: Unsupported architecture $ARCH"
    exit 1
fi

DETECTED_TARGET="${PLATFORM}-${TARGET_ARCH}"
echo "Detected platform: ${DETECTED_TARGET}"
echo ""

# 2. Query GitHub API for the releases
REPO="gagansokhal-coder/Terminal_helper"
API_URL="https://api.github.com/repos/${REPO}/releases"

echo "DEBUG: Fetching release information from $API_URL..."
if ! RELEASES_JSON=$(curl -sSL -f "$API_URL" 2>/dev/null); then
    echo "Error: Failed to fetch releases from GitHub API."
    echo "DEBUG: Ensure you have internet connectivity and can reach api.github.com."
    exit 1
fi

# Verify the response is not empty
if [ -z "$RELEASES_JSON" ] || [ "$RELEASES_JSON" = "[]" ]; then
    echo "Error: GitHub API returned an empty release list."
    exit 1
fi

# Check for API rate limit or other error messages in JSON
set +o pipefail
HAS_MESSAGE=$(echo "$RELEASES_JSON" | grep -c '"message":' || true)
set -o pipefail

if [ "$HAS_MESSAGE" -gt 0 ] && [[ "$RELEASES_JSON" != *"tag_name"* ]]; then
    echo "Error: GitHub API returned an error message (possibly rate limiting)."
    echo "DEBUG: API Response:"
    echo "$RELEASES_JSON" | head -n 10
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

# Verify asset actually exists in the response
set +o pipefail
HAS_ASSET=$(echo "$RELEASES_JSON" | grep -c "$EXPECTED_ASSET" || true)
set -o pipefail

if [ "$HAS_ASSET" -eq 0 ]; then
    echo "Error: Asset $EXPECTED_ASSET not found in the latest release."
    exit 1
fi

echo "Selected asset:"
echo "$EXPECTED_ASSET"
echo ""

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

if [ -z "$ASSET_URL" ]; then
    echo "Error: Could not extract download URL for $EXPECTED_ASSET."
    exit 1
fi

if [ -z "$CHECKSUM_URL" ]; then
    echo "Error: Could not extract download URL for checksums.txt in the latest release."
    exit 1
fi

# 5. Create temp directory
TMP_DIR="/tmp/ggnmem-installer"

if ! mkdir -p "$TMP_DIR"; then
    echo "Error: Failed to create temporary directory $TMP_DIR."
    echo "DEBUG: Check your disk space and permissions."
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

set +o pipefail
EXPECTED_HASH=$(grep "$EXPECTED_ASSET" "$CHECKSUM_PATH" | awk '{print $1}')
set -o pipefail

if [ -z "$EXPECTED_HASH" ]; then
    echo "Error: Could not find checksum entry for $EXPECTED_ASSET in checksums.txt"
    rm -rf "$TMP_DIR"
    exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL_HASH=$(sha256sum "$ASSET_PATH" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
    ACTUAL_HASH=$(shasum -a 256 "$ASSET_PATH" | awk '{print $1}')
else
    echo "Error: Neither sha256sum nor shasum is installed. Cannot verify checksum."
    rm -rf "$TMP_DIR"
    exit 1
fi

echo "Expected SHA256: $EXPECTED_HASH"
echo "Actual SHA256:   $ACTUAL_HASH"

if [ "$EXPECTED_HASH" != "$ACTUAL_HASH" ]; then
    echo "Error: Checksum mismatch! The downloaded file may be corrupted or tampered with."
    rm -rf "$TMP_DIR"
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
    rm -rf "$EXTRACT_DIR"
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
    rm -rf "$EXTRACT_DIR"
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
    rm -rf "$TMP_DIR"
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
    rm -rf "$TMP_DIR"
    exit 1
fi

echo "Installed version: $INSTALLED_VERSION"

# Clean up
rm -rf "$TMP_DIR"

exit 0
