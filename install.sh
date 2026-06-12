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
#
# Features:
#   - Detects existing installations and upgrades in place
#   - Preserves config, database, and installed AI models
#   - Verifies binary checksums if checksums.txt is available
#   - Verifies binaries after install
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
MODELS_DIR="$DATA_DIR/models"

step "Creating directories..."

for dir in "$BIN_DIR" "$CONFIG_DIR" "$DATA_DIR" "$STATE_DIR"; do
    if [ -d "$dir" ]; then
        ok "$dir (exists)"
    else
        mkdir -p "$dir"
        ok "$dir (created)"
    fi
done

# ─── Detect existing installation ────────────────────────────────────────────

step "Checking for existing installation..."

EXISTING_VERSION=""
UPGRADE_MODE=false

if [ -f "$BIN_DIR/ggnmem" ]; then
    EXISTING_VERSION=$("$BIN_DIR/ggnmem" version 2>/dev/null | head -1 || echo "unknown")
    info "Found existing installation: $EXISTING_VERSION"
    UPGRADE_MODE=true
else
    info "No existing installation found (fresh install)"
fi

# ─── Find binaries ───────────────────────────────────────────────────────────

step "Looking for binaries..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLI_BIN=""
DAEMON_BIN=""
CHECKSUMS_FILE=""

# Priority 1: release/ directory next to this script.
if [ -f "$SCRIPT_DIR/release/ggnmem" ] && [ -f "$SCRIPT_DIR/release/ggnmem-daemon" ]; then
    CLI_BIN="$SCRIPT_DIR/release/ggnmem"
    DAEMON_BIN="$SCRIPT_DIR/release/ggnmem-daemon"
    [ -f "$SCRIPT_DIR/release/checksums.txt" ] && CHECKSUMS_FILE="$SCRIPT_DIR/release/checksums.txt"
    info "Found binaries in release/"
# Priority 2: same directory as install.sh (extracted tarball).
elif [ -f "$SCRIPT_DIR/ggnmem" ] && [ -f "$SCRIPT_DIR/ggnmem-daemon" ]; then
    CLI_BIN="$SCRIPT_DIR/ggnmem"
    DAEMON_BIN="$SCRIPT_DIR/ggnmem-daemon"
    [ -f "$SCRIPT_DIR/checksums.txt" ] && CHECKSUMS_FILE="$SCRIPT_DIR/checksums.txt"
    info "Found binaries alongside install.sh"
# Priority 3: release/ directory in current directory.
elif [ -f "./release/ggnmem" ] && [ -f "./release/ggnmem-daemon" ]; then
    CLI_BIN="./release/ggnmem"
    DAEMON_BIN="./release/ggnmem-daemon"
    [ -f "./release/checksums.txt" ] && CHECKSUMS_FILE="./release/checksums.txt"
    info "Found binaries in ./release/"
# Priority 4: target/release/ (cargo build --release output).
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

# ─── Verify checksums (if available) ────────────────────────────────────────

if [ -n "$CHECKSUMS_FILE" ]; then
    step "Verifying binary checksums..."

    CHECKSUMS_DIR="$(dirname "$CHECKSUMS_FILE")"
    cd "$CHECKSUMS_DIR"
    if sha256sum --check checksums.txt > /dev/null 2>&1; then
        ok "all checksums verified"
    else
        warn "checksum verification failed — some files may be corrupt"
        sha256sum --check checksums.txt 2>&1 | head -10
        echo ""
        echo -e "${YELLOW}Continue anyway? (y/N)${RESET}"
        read -r REPLY
        if [ "$REPLY" != "y" ] && [ "$REPLY" != "Y" ]; then
            err "installation aborted"
            exit 1
        fi
    fi
    cd - > /dev/null
else
    info "No checksums.txt found (skipping verification)"
fi

# ─── Stop daemon if upgrading ────────────────────────────────────────────────

if $UPGRADE_MODE; then
    step "Stopping daemon for upgrade..."

    # Check if daemon is running.
    if pgrep -x ggnmem-daemon > /dev/null 2>&1; then
        "$BIN_DIR/ggnmem" stop 2>/dev/null || kill "$(pgrep -x ggnmem-daemon)" 2>/dev/null || true
        sleep 1
        ok "daemon stopped"
    else
        ok "daemon not running"
    fi
fi

# ─── Install binaries ────────────────────────────────────────────────────────

step "Installing binaries..."

if $UPGRADE_MODE; then
    # Backup existing binaries.
    if [ -f "$BIN_DIR/ggnmem" ]; then
        cp "$BIN_DIR/ggnmem" "$BIN_DIR/ggnmem.old"
        ok "backed up ggnmem → ggnmem.old"
    fi
    if [ -f "$BIN_DIR/ggnmem-daemon" ]; then
        cp "$BIN_DIR/ggnmem-daemon" "$BIN_DIR/ggnmem-daemon.old"
        ok "backed up ggnmem-daemon → ggnmem-daemon.old"
    fi
fi

cp "$CLI_BIN" "$BIN_DIR/ggnmem"
chmod +x "$BIN_DIR/ggnmem"
ok "ggnmem -> $BIN_DIR/ggnmem"

cp "$DAEMON_BIN" "$BIN_DIR/ggnmem-daemon"
chmod +x "$BIN_DIR/ggnmem-daemon"
ok "ggnmem-daemon -> $BIN_DIR/ggnmem-daemon"

# ─── Verify binaries ────────────────────────────────────────────────────────

step "Verifying binaries..."

# Source PATH for this session.
export PATH="$BIN_DIR:$PATH"

# Verify CLI binary.
if "$BIN_DIR/ggnmem" version > /dev/null 2>&1; then
    NEW_VERSION=$("$BIN_DIR/ggnmem" version 2>/dev/null | head -1 || echo "unknown")
    ok "ggnmem binary verified: $NEW_VERSION"
else
    err "ggnmem binary verification failed!"
    err "The binary may be incompatible with this system."
    err "Try building from source: cargo build --release"
    exit 1
fi

# Verify daemon binary (just check it's executable — don't run it as it starts the daemon).
if [ -x "$BIN_DIR/ggnmem-daemon" ]; then
    ok "ggnmem-daemon binary verified"
else
    warn "ggnmem-daemon may not be executable — check manually"
fi

# ─── Upgrade / fresh install messaging ───────────────────────────────────────

if $UPGRADE_MODE; then
    step "Upgrade status..."
    info "Previous: $EXISTING_VERSION"
    info "New:      $NEW_VERSION"
fi

# ─── Default config ──────────────────────────────────────────────────────────

step "Setting up config..."

CONFIG_FILE="$CONFIG_DIR/config.toml"

if [ -f "$CONFIG_FILE" ]; then
    ok "config.toml exists (preserved — not overwriting)"
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

# ─── Preserve database notice ────────────────────────────────────────────────

DB_FILE="$DATA_DIR/ggnmem.db"
if [ -f "$DB_FILE" ]; then
    DB_SIZE=$(du -h "$DB_FILE" | cut -f1)
    ok "database preserved: $DB_FILE ($DB_SIZE)"
else
    info "database will be created when daemon starts"
fi

# ─── Preserve models notice ─────────────────────────────────────────────────

if [ -d "$MODELS_DIR" ]; then
    MODEL_COUNT=$(find "$MODELS_DIR" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)
    if [ "$MODEL_COUNT" -gt 0 ]; then
        ok "AI models preserved: $MODELS_DIR ($MODEL_COUNT model(s))"
        # List installed models.
        for model_dir in "$MODELS_DIR"/*/; do
            if [ -d "$model_dir" ]; then
                MODEL_NAME=$(basename "$model_dir")
                MODEL_SIZE=$(du -sh "$model_dir" 2>/dev/null | cut -f1)
                info "  model: $MODEL_NAME ($MODEL_SIZE)"
            fi
        done
    fi
else
    info "no AI models installed (install with: ggnmem ai install)"
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

step "Final verification..."

if command -v ggnmem &>/dev/null; then
    VERSION="$(ggnmem version 2>/dev/null | head -1 || echo 'unknown')"
    ok "ggnmem in PATH: $VERSION"
else
    warn "ggnmem not found in PATH after install"
    warn "You may need to open a new terminal or run:"
    warn "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# ─── Summary ─────────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}═══════════════════════════════════════${RESET}"
if $UPGRADE_MODE; then
    echo -e "${GREEN}${BOLD}  ggnmem upgraded successfully!${RESET}"
else
    echo -e "${GREEN}${BOLD}  ggnmem installed successfully!${RESET}"
fi
echo -e "${BOLD}═══════════════════════════════════════${RESET}"
echo ""
echo "  binaries:  $BIN_DIR/ggnmem"
echo "             $BIN_DIR/ggnmem-daemon"
echo "  config:    $CONFIG_FILE"
echo "  data:      $DATA_DIR/"
echo ""
if $UPGRADE_MODE; then
    echo "  ┌─────────────────────────────────┐"
    echo "  │ Previous: $EXISTING_VERSION"
    echo "  │ Current:  $NEW_VERSION"
    echo "  ├─────────────────────────────────┤"
    echo "  │ ✓ config preserved              │"
    echo "  │ ✓ database preserved            │"
    if [ -d "$MODELS_DIR" ] && [ "$(find "$MODELS_DIR" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)" -gt 0 ]; then
        echo "  │ ✓ AI models preserved           │"
    fi
    echo "  └─────────────────────────────────┘"
    echo ""
    echo "  next steps:"
    echo "    1. Restart daemon:    ggnmem restart"
    echo "    2. Verify:            ggnmem doctor"
    echo "    3. Check version:     ggnmem version"
else
    echo "  next steps:"
    echo "    1. Open a new terminal (or: source ~/.${SHELL_NAME}rc)"
    echo "    2. Start the daemon:  ggnmem start"
    echo "    3. Verify:            ggnmem doctor"
    echo "    4. Try it:            Ctrl+R"
fi
echo ""
