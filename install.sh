#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# ggnmem installer
#
# Usage:
#   curl -sSf https://raw.githubusercontent.com/ggnmem/ggnmem/main/install.sh | bash
#   ./install.sh
#
# This script installs ggnmem binaries and sets up shell integration.
# Target: Linux / WSL only.
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

# ─── Colors ───────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

info()  { echo -e "${CYAN}[info]${RESET}  $*"; }
ok()    { echo -e "${GREEN}[ok]${RESET}    $*"; }
warn()  { echo -e "${YELLOW}[warn]${RESET}  $*"; }
err()   { echo -e "${RED}[error]${RESET} $*"; }
step()  { echo -e "\n${BOLD}$*${RESET}"; }

# ─── Platform detection ──────────────────────────────────────────────────────

step "Detecting platform..."

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)  info "OS: Linux" ;;
    *)
        err "Unsupported OS: $OS"
        err "ggnmem currently supports Linux and WSL only."
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64)         info "Arch: x86_64" ;;
    aarch64|arm64)  ARCH="aarch64"; info "Arch: aarch64" ;;
    *)
        err "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Detect WSL.
if grep -qi microsoft /proc/version 2>/dev/null; then
    info "Environment: WSL"
else
    info "Environment: native Linux"
fi

# ─── Directories ─────────────────────────────────────────────────────────────

BIN_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/ggnmem"
DATA_DIR="$HOME/.local/share/ggnmem"
STATE_DIR="$HOME/.local/state/ggnmem"

step "Creating directories..."

for dir in "$BIN_DIR" "$CONFIG_DIR" "$DATA_DIR" "$STATE_DIR"; do
    if [ -d "$dir" ]; then
        ok "$dir (exists)"
    else
        mkdir -p "$dir"
        ok "$dir (created)"
    fi
done

# ─── Find binaries ───────────────────────────────────────────────────────────

step "Looking for binaries..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLI_BIN=""
DAEMON_BIN=""

# Priority 1: release/ directory next to this script.
if [ -f "$SCRIPT_DIR/release/ggnmem" ] && [ -f "$SCRIPT_DIR/release/ggnmem-daemon" ]; then
    CLI_BIN="$SCRIPT_DIR/release/ggnmem"
    DAEMON_BIN="$SCRIPT_DIR/release/ggnmem-daemon"
    info "Found binaries in release/"
# Priority 2: release/ directory in current directory.
elif [ -f "./release/ggnmem" ] && [ -f "./release/ggnmem-daemon" ]; then
    CLI_BIN="./release/ggnmem"
    DAEMON_BIN="./release/ggnmem-daemon"
    info "Found binaries in ./release/"
# Priority 3: target/release/ (cargo build --release output).
elif [ -f "$SCRIPT_DIR/target/release/ggnmem-cli" ] && [ -f "$SCRIPT_DIR/target/release/ggnmem-daemon" ]; then
    CLI_BIN="$SCRIPT_DIR/target/release/ggnmem-cli"
    DAEMON_BIN="$SCRIPT_DIR/target/release/ggnmem-daemon"
    info "Found binaries in target/release/"
elif [ -f "./target/release/ggnmem-cli" ] && [ -f "./target/release/ggnmem-daemon" ]; then
    CLI_BIN="./target/release/ggnmem-cli"
    DAEMON_BIN="./target/release/ggnmem-daemon"
    info "Found binaries in ./target/release/"
else
    err "Cannot find ggnmem binaries."
    err ""
    err "Build from source first:"
    err "  cargo build --release"
    err ""
    err "Or run the release script:"
    err "  bash scripts/build_release.sh"
    exit 1
fi

# ─── Install binaries ────────────────────────────────────────────────────────

step "Installing binaries..."

cp "$CLI_BIN" "$BIN_DIR/ggnmem"
chmod +x "$BIN_DIR/ggnmem"
ok "ggnmem -> $BIN_DIR/ggnmem"

cp "$DAEMON_BIN" "$BIN_DIR/ggnmem-daemon"
chmod +x "$BIN_DIR/ggnmem-daemon"
ok "ggnmem-daemon -> $BIN_DIR/ggnmem-daemon"

# ─── Default config ──────────────────────────────────────────────────────────

step "Setting up config..."

CONFIG_FILE="$CONFIG_DIR/config.toml"

if [ -f "$CONFIG_FILE" ]; then
    ok "config.toml exists (not overwriting)"
else
    cat > "$CONFIG_FILE" << 'EOF'
# ggnmem configuration
# See: https://github.com/ggnmem/ggnmem

[features]
capture = true
search = true
tui = true
ai = false

[daemon]
autostart = false

[appearance]
theme = "auto"

[limits]
max_history = 100000
max_memory_mb = 40
max_db_size_mb = 1024

[search]
index_mode = "balanced"

[retention]
retention_days = 365
max_commands = 1000000
auto_cleanup = true

[ai]
ai_enabled = false
embedding_provider = "local"
semantic_search = false
model_name = "all-MiniLM-L6-v2"
EOF
    ok "config.toml created"
fi

# ─── PATH ────────────────────────────────────────────────────────────────────

step "Checking PATH..."

if echo "$PATH" | tr ':' '\n' | grep -qx "$BIN_DIR"; then
    ok "~/.local/bin is in PATH"
    PATH_NEEDS_UPDATE=false
else
    warn "~/.local/bin is NOT in PATH"
    PATH_NEEDS_UPDATE=true
fi

# ─── Shell integration ───────────────────────────────────────────────────────

step "Setting up shell integration..."

MARKER_START="# ggnmem shell integration"
MARKER_END="# end ggnmem"

add_shell_hook() {
    local rc_file="$1"
    local shell_name="$2"

    if [ ! -f "$rc_file" ]; then
        touch "$rc_file"
    fi

    # Check if already configured.
    if grep -q "ggnmem init" "$rc_file" 2>/dev/null; then
        ok "$rc_file (already configured)"
        return
    fi

    # Add integration.
    {
        echo ""
        echo "$MARKER_START"
        echo "eval \"\$(ggnmem init $shell_name)\""
        echo "$MARKER_END"
    } >> "$rc_file"

    ok "$rc_file (added ggnmem hook)"
}

add_path_line() {
    local rc_file="$1"

    if [ ! -f "$rc_file" ]; then
        return
    fi

    if grep -q '\.local/bin' "$rc_file" 2>/dev/null; then
        return
    fi

    {
        echo ""
        echo "# Added by ggnmem installer"
        echo 'export PATH="$HOME/.local/bin:$PATH"'
    } >> "$rc_file"

    ok "Added PATH export to $rc_file"
}

SHELL_NAME="$(basename "${SHELL:-/bin/bash}")"

case "$SHELL_NAME" in
    zsh)
        add_shell_hook "$HOME/.zshrc" "zsh"
        if $PATH_NEEDS_UPDATE; then
            add_path_line "$HOME/.zshrc"
        fi
        ;;
    bash)
        add_shell_hook "$HOME/.bashrc" "bash"
        if $PATH_NEEDS_UPDATE; then
            add_path_line "$HOME/.bashrc"
        fi
        ;;
    *)
        warn "Unknown shell: $SHELL_NAME"
        warn "Add manually to your shell rc:"
        warn "  eval \"\$(ggnmem init <bash|zsh>)\""
        ;;
esac

# ─── Verify ──────────────────────────────────────────────────────────────────

step "Verifying install..."

# Source the PATH update for this session.
export PATH="$BIN_DIR:$PATH"

if command -v ggnmem &>/dev/null; then
    VERSION="$(ggnmem version 2>/dev/null || echo 'unknown')"
    ok "$VERSION"
else
    warn "ggnmem not found in PATH after install"
    warn "You may need to open a new terminal or run:"
    warn "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# ─── Summary ─────────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}═══════════════════════════════════════${RESET}"
echo -e "${GREEN}${BOLD}  ggnmem installed successfully!${RESET}"
echo -e "${BOLD}═══════════════════════════════════════${RESET}"
echo ""
echo "  binaries:  $BIN_DIR/ggnmem"
echo "             $BIN_DIR/ggnmem-daemon"
echo "  config:    $CONFIG_FILE"
echo "  data:      $DATA_DIR/"
echo ""
echo "  next steps:"
echo "    1. Open a new terminal (or: source ~/.${SHELL_NAME}rc)"
echo "    2. Start the daemon:  ggnmem start"
echo "    3. Verify:            ggnmem doctor"
echo "    4. Try it:            Ctrl+R"
echo ""
