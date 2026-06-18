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

RELEASES_JSON=$(curl -sSL "$API_URL")

# 3. Select newest prerelease
# GitHub API returns releases in chronological order, so the first "tag_name" is the newest.
LATEST_VERSION=$(echo "$RELEASES_JSON" | grep -m 1 '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_VERSION" ]; then
    echo "Error: Could not determine latest version from GitHub API"
    exit 1
fi

echo "Latest release:"
echo "$LATEST_VERSION"
echo ""

# 4. Select matching asset
# The asset name format is expected to be ggnmem-{platform}-{arch}.tar.gz
EXPECTED_ASSET="ggnmem-${DETECTED_TARGET}.tar.gz"

echo "Selected asset:"
echo "$EXPECTED_ASSET"

# We exit successfully without downloading anything for this prototype.
exit 0
