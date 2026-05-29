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
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

info()  { echo -e "${CYAN}[info]${RESET}  $*"; }
ok()    { echo -e "${GREEN}[ok]${RESET}    $*"; }
err()   { echo -e "${RED}[error]${RESET} $*"; }

# Find project root (parent of scripts/).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

RELEASE_DIR="$PROJECT_ROOT/release"

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
echo "  Release directory: $RELEASE_DIR/"
echo ""
echo "  To install:"
echo "    cd release && bash install.sh"
echo ""
