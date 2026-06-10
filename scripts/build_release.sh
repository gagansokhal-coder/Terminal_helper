#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# ggnmem release build script
#
# Usage:
#   bash scripts/build_release.sh
#
# Creates a release/ directory with:
#   - ggnmem        (CLI binary, renamed from ggnmem-cli)
#   - ggnmem-daemon (daemon binary)
#   - install.sh    (installer)
#   - README.md     (quick-start docs)
#   - VERSION       (build metadata)
#
# Then bundles everything into:
#   ggnmem-linux-<arch>.tar.gz
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'
BOLD='\033[1m'
RESET='\033[0m'

info()  { echo -e "${CYAN}[info]${RESET}  $*"; }
ok()    { echo -e "${GREEN}[ok]${RESET}    $*"; }
warn()  { echo -e "${YELLOW}[warn]${RESET}  $*"; }
err()   { echo -e "${RED}[error]${RESET} $*"; }

# Find project root (parent of scripts/).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

RELEASE_DIR="$PROJECT_ROOT/release"

# ─── Detect architecture ────────────────────────────────────────────────────

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64)         ARCH_TAG="x86_64" ;;
    aarch64|arm64)  ARCH_TAG="aarch64" ;;
    *)
        err "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

info "Architecture: $ARCH_TAG"

# ─── Capture build metadata ──────────────────────────────────────────────────

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_DATE=$(date +%Y-%m-%d)

info "Version: $VERSION"
info "Commit: $GIT_COMMIT"
info "Date: $BUILD_DATE"

# ─── Build ────────────────────────────────────────────────────────────────────

info "Building release binaries..."
cargo build --release

if [ ! -f "target/release/ggnmem-cli" ]; then
    err "ggnmem-cli binary not found after build"
    exit 1
fi

if [ ! -f "target/release/ggnmem-daemon" ]; then
    err "ggnmem-daemon binary not found after build"
    exit 1
fi

ok "cargo build --release succeeded"

# ─── Assemble release ────────────────────────────────────────────────────────

info "Assembling release directory..."

rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

# Copy and rename CLI binary.
cp "target/release/ggnmem-cli" "$RELEASE_DIR/ggnmem"
chmod +x "$RELEASE_DIR/ggnmem"
ok "release/ggnmem"

# Copy daemon binary.
cp "target/release/ggnmem-daemon" "$RELEASE_DIR/ggnmem-daemon"
chmod +x "$RELEASE_DIR/ggnmem-daemon"
ok "release/ggnmem-daemon"

# Copy installer.
cp "install.sh" "$RELEASE_DIR/install.sh"
chmod +x "$RELEASE_DIR/install.sh"
ok "release/install.sh"

# Strip debug symbols if strip is available.
if command -v strip &>/dev/null; then
    info "Stripping debug symbols..."
    strip "$RELEASE_DIR/ggnmem" 2>/dev/null || true
    strip "$RELEASE_DIR/ggnmem-daemon" 2>/dev/null || true
    ok "binaries stripped"
else
    info "strip not found, skipping (binaries will be larger)"
fi

# Generate VERSION file.
cat > "$RELEASE_DIR/VERSION" <<EOF
version=$VERSION
commit=$GIT_COMMIT
date=$BUILD_DATE
arch=$ARCH_TAG
EOF
ok "release/VERSION"

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
# From a new release bundle
ggnmem upgrade --bundle ./path/to/release

# Or from an extracted tarball
ggnmem upgrade --bundle ggnmem-linux-x86_64.tar.gz
```

## Uninstall

```bash
ggnmem uninstall          # keeps database
ggnmem uninstall --full   # removes everything
```

## Directory Layout

| Path | Purpose |
|------|---------|
| `~/.local/bin/ggnmem` | CLI binary |
| `~/.local/bin/ggnmem-daemon` | Background daemon |
| `~/.config/ggnmem/config.toml` | Configuration |
| `~/.local/share/ggnmem/ggnmem.db` | Command database |
| `~/.local/state/ggnmem/` | Runtime state |

## License

MIT OR Apache-2.0
EOF
ok "release/README.md"

# ─── Create tarball ──────────────────────────────────────────────────────────

TARBALL_NAME="ggnmem-linux-${ARCH_TAG}.tar.gz"
TARBALL_PATH="$PROJECT_ROOT/$TARBALL_NAME"

info "Creating release tarball: $TARBALL_NAME"

# Create tarball from the release directory contents.
# Files are placed at the top level inside the tarball (no enclosing directory).
tar czf "$TARBALL_PATH" -C "$RELEASE_DIR" \
    ggnmem \
    ggnmem-daemon \
    install.sh \
    README.md \
    VERSION

ok "$TARBALL_NAME"

# ─── Summary ─────────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}═══════════════════════════════════════${RESET}"
echo -e "${GREEN}${BOLD}  Release build complete${RESET}"
echo -e "${BOLD}═══════════════════════════════════════${RESET}"
echo ""

# Show binary sizes.
echo "  Binary sizes:"
for f in "$RELEASE_DIR/ggnmem" "$RELEASE_DIR/ggnmem-daemon"; do
    SIZE=$(du -h "$f" | cut -f1)
    echo "    $(basename "$f"): $SIZE"
done
echo ""

# Show tarball size.
TARBALL_SIZE=$(du -h "$TARBALL_PATH" | cut -f1)
echo "  Tarball: $TARBALL_NAME ($TARBALL_SIZE)"
echo ""

echo "  Version:   $VERSION"
echo "  Commit:    $GIT_COMMIT"
echo "  Date:      $BUILD_DATE"
echo "  Arch:      $ARCH_TAG"
echo ""
echo "  Release directory: $RELEASE_DIR/"
echo ""
echo "  To install:"
echo "    cd release && bash install.sh"
echo ""
echo "  To distribute:"
echo "    Share $TARBALL_NAME"
echo ""
