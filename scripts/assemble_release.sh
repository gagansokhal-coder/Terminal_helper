#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# assemble_release.sh — Package built binaries into a release tarball
#
# Called by the release workflow after `cargo build --release --target <target>`.
# Produces: ggnmem-linux-<arch>.tar.gz in the project root.
#
# Usage:
#   bash scripts/assemble_release.sh \
#     --target x86_64-unknown-linux-gnu \
#     --arch   x86_64 \
#     --version 0.4.0-alpha \
#     --commit  abc1234 \
#     --date    2026-06-14 \
#     --rustc   1.95.0
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

# ─── Parse arguments ────────────────────────────────────────────────────────

TARGET=""
ARCH=""
VERSION=""
COMMIT=""
BUILD_DATE=""
RUSTC_VER=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)   TARGET="$2";    shift 2 ;;
        --arch)     ARCH="$2";      shift 2 ;;
        --version)  VERSION="$2";   shift 2 ;;
        --commit)   COMMIT="$2";    shift 2 ;;
        --date)     BUILD_DATE="$2"; shift 2 ;;
        --rustc)    RUSTC_VER="$2"; shift 2 ;;
        *) echo "Unknown argument: $1" >&2; exit 1 ;;
    esac
done

for var in TARGET ARCH VERSION COMMIT BUILD_DATE RUSTC_VER; do
    if [ -z "${!var}" ]; then
        echo "Missing required argument: --$(echo $var | tr '[:upper:]' '[:lower:]')" >&2
        exit 1
    fi
done

# ─── Paths ───────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

BINARY_DIR="target/${TARGET}/release"
RELEASE_DIR="$(mktemp -d /tmp/ggnmem-assemble-XXXXXX)"
trap "rm -rf '$RELEASE_DIR'" EXIT

# ─── Verify binaries exist ──────────────────────────────────────────────────

for bin in ggnmem-cli ggnmem-daemon; do
    if [ ! -f "$BINARY_DIR/$bin" ]; then
        echo "ERROR: $BINARY_DIR/$bin not found" >&2
        exit 1
    fi
done

# ─── Assemble ────────────────────────────────────────────────────────────────

echo "Assembling release for linux-${ARCH}..."

# Copy and rename CLI binary.
cp "$BINARY_DIR/ggnmem-cli" "$RELEASE_DIR/ggnmem"
chmod +x "$RELEASE_DIR/ggnmem"

# Copy daemon binary.
cp "$BINARY_DIR/ggnmem-daemon" "$RELEASE_DIR/ggnmem-daemon"
chmod +x "$RELEASE_DIR/ggnmem-daemon"

# Copy installer.
cp "install.sh" "$RELEASE_DIR/install.sh"
chmod +x "$RELEASE_DIR/install.sh"

# Strip debug symbols if strip is available.
if [ "$ARCH" = "aarch64" ]; then
    STRIP_CMD="aarch64-linux-gnu-strip"
else
    STRIP_CMD="strip"
fi

if command -v "$STRIP_CMD" &>/dev/null; then
    echo "Stripping debug symbols with $STRIP_CMD..."
    "$STRIP_CMD" "$RELEASE_DIR/ggnmem" 2>/dev/null || true
    "$STRIP_CMD" "$RELEASE_DIR/ggnmem-daemon" 2>/dev/null || true
fi

# Generate VERSION file.
cat > "$RELEASE_DIR/VERSION" <<EOF
version=$VERSION
commit=$COMMIT
date=$BUILD_DATE
arch=$ARCH
rust=$RUSTC_VER
EOF

# Generate README.
cat > "$RELEASE_DIR/README.md" << 'EOF'
# ggnmem — Semantic Terminal Memory

A local-first, privacy-focused terminal history intelligence system.

## Quick Install

```bash
bash install.sh
```

## Manual Install

```bash
# Copy binaries
cp ggnmem ggnmem-daemon ~/.local/bin/

# Set up shell integration
ggnmem install

# Start the daemon
ggnmem-daemon &

# Verify
ggnmem doctor
```

## Usage

```bash
# Search your command history
ggnmem search docker

# Interactive search (also bound to Ctrl+R)
ggnmem ui

# Show recent commands
ggnmem recent

# Check health
ggnmem doctor

# Show version info
ggnmem version
ggnmem version --verbose
```

## Upgrade

```bash
ggnmem upgrade --bundle ./path/to/release
ggnmem upgrade --bundle ggnmem-linux-x86_64.tar.gz
```

## Uninstall

```bash
ggnmem uninstall          # keeps database
ggnmem uninstall --full   # removes everything
```

## License

MIT OR Apache-2.0
EOF

# Generate checksums inside the release directory.
cd "$RELEASE_DIR"
sha256sum ggnmem ggnmem-daemon install.sh README.md VERSION > checksums.txt
cd "$PROJECT_ROOT"

# ─── Create tarball ──────────────────────────────────────────────────────────

TARBALL_NAME="ggnmem-linux-${ARCH}.tar.gz"
TARBALL_PATH="$PROJECT_ROOT/$TARBALL_NAME"

echo "Creating $TARBALL_NAME..."

tar czf "$TARBALL_PATH" -C "$RELEASE_DIR" \
    ggnmem \
    ggnmem-daemon \
    install.sh \
    README.md \
    VERSION \
    checksums.txt

# ─── Verify tarball integrity ────────────────────────────────────────────────

VERIFY_DIR=$(mktemp -d /tmp/ggnmem-verify-XXXXXX)

tar xzf "$TARBALL_PATH" -C "$VERIFY_DIR"
cd "$VERIFY_DIR"
if sha256sum --check checksums.txt > /dev/null 2>&1; then
    echo "Tarball checksums verified."
else
    echo "ERROR: Tarball checksum verification FAILED" >&2
    sha256sum --check checksums.txt
    rm -rf "$VERIFY_DIR"
    exit 1
fi
cd "$PROJECT_ROOT"
rm -rf "$VERIFY_DIR"

# ─── Summary ─────────────────────────────────────────────────────────────────

TARBALL_SIZE=$(du -h "$TARBALL_PATH" | cut -f1)
CLI_SIZE=$(du -h "$RELEASE_DIR/ggnmem" | cut -f1)
DAEMON_SIZE=$(du -h "$RELEASE_DIR/ggnmem-daemon" | cut -f1)

echo ""
echo "═══════════════════════════════════════"
echo "  Release assembled: $TARBALL_NAME"
echo "═══════════════════════════════════════"
echo "  Version:   $VERSION"
echo "  Commit:    $COMMIT"
echo "  Arch:      $ARCH"
echo "  CLI:       $CLI_SIZE"
echo "  Daemon:    $DAEMON_SIZE"
echo "  Tarball:   $TARBALL_SIZE"
echo ""
