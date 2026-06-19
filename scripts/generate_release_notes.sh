#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# generate_release_notes.sh — Produce RELEASE_NOTES.md for GitHub Releases
#
# Called by the release workflow after tarballs are assembled.
# Writes RELEASE_NOTES.md to the project root.
#
# Usage:
#   bash scripts/generate_release_notes.sh \
#     --version 0.4.0-alpha \
#     --tag     v0.4.0-alpha \
#     --dist    dist
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

# ─── Parse arguments ────────────────────────────────────────────────────────

VERSION=""
TAG=""
DIST_DIR=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)  VERSION="$2";  shift 2 ;;
        --tag)      TAG="$2";      shift 2 ;;
        --dist)     DIST_DIR="$2"; shift 2 ;;
        *) echo "Unknown argument: $1" >&2; exit 1 ;;
    esac
done

for var in VERSION TAG DIST_DIR; do
    if [ -z "${!var}" ]; then
        echo "Missing required argument: --$(echo $var | tr '[:upper:]' '[:lower:]')" >&2
        exit 1
    fi
done

# ─── Paths ───────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# ─── Gather metadata ────────────────────────────────────────────────────────

GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_DATE=$(date +%Y-%m-%d)
RUSTC_VER=$(rustc --version 2>/dev/null | sed 's/rustc \([^ ]*\).*/\1/' || echo "unknown")

# ─── Build checksums table ──────────────────────────────────────────────────

CHECKSUMS_TABLE=""
if [ -f "$DIST_DIR/checksums.txt" ]; then
    while IFS= read -r line; do
        HASH=$(echo "$line" | awk '{print $1}')
        FILE=$(echo "$line" | awk '{print $2}')
        # Remove leading ./ or */ prefix if present
        FILE=$(basename "$FILE")
        CHECKSUMS_TABLE="${CHECKSUMS_TABLE}| \`${FILE}\` | \`${HASH}\` |
"
    done < "$DIST_DIR/checksums.txt"
fi

# ─── Detect available architectures ─────────────────────────────────────────

INSTALL_EXAMPLES=""
DOWNLOAD_TABLE=""

for arch in x86_64 aarch64; do
    TARBALL="ggnmem-linux-${arch}.tar.gz"
    if [ -f "$DIST_DIR/$TARBALL" ]; then
        SIZE=$(du -h "$DIST_DIR/$TARBALL" | cut -f1)
        DOWNLOAD_TABLE="${DOWNLOAD_TABLE}| \`${TARBALL}\` | ${SIZE} |
"
    fi
done

# ─── Build changelog from commits since last tag ───────────────────────────

CHANGELOG=""
PREV_TAG=$(git describe --tags --abbrev=0 "${TAG}^" 2>/dev/null || echo "")
if [ -n "$PREV_TAG" ]; then
    CHANGELOG=$(git log --pretty=format:"- %s (%h)" "${PREV_TAG}..${TAG}" 2>/dev/null || echo "")
fi

if [ -z "$CHANGELOG" ]; then
    CHANGELOG="<!-- Add changelog entries here -->"
fi

# ─── Write RELEASE_NOTES.md ─────────────────────────────────────────────────

cat > "$PROJECT_ROOT/RELEASE_NOTES.md" <<EOF
# ggnmem ${TAG}

## 🌐 Website

**[ggnmem.mytechy.in](https://ggnmem.mytechy.in)**

## What's New

${CHANGELOG}

## Installation

### One-Line Install

\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/gagansokhal-coder/Terminal_helper/main/scripts/install-online.sh | bash
\`\`\`

### Upgrade Existing Installation

\`\`\`bash
ggnmem self-update
\`\`\`

### Manual Install (from tarball)

\`\`\`bash
tar xzf ggnmem-linux-x86_64.tar.gz
bash install.sh
\`\`\`

### Verify Installation

\`\`\`bash
ggnmem version
ggnmem doctor
\`\`\`

## Downloads

| Asset | Size |
|-------|------|
${DOWNLOAD_TABLE}
## Checksums (SHA256)

| File | SHA256 |
|------|--------|
${CHECKSUMS_TABLE}
## Build Info

| Field | Value |
|-------|-------|
| Version | ${VERSION} |
| Commit | ${GIT_COMMIT} |
| Date | ${BUILD_DATE} |
| Rust | ${RUSTC_VER} |

## Requirements

- Linux (x86_64 or aarch64) or WSL
- No Rust toolchain required (pre-built binaries)
- ~100 MB disk space (with AI model)

## Preserved During Upgrade

- \`~/.config/ggnmem/config.toml\` — configuration
- \`~/.local/share/ggnmem/ggnmem.db\` — command history database
- \`~/.local/share/ggnmem/models/\` — installed AI models
EOF

echo "RELEASE_NOTES.md generated for ${TAG}"

