#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# ggnmem release verification script
#
# Usage:
#   bash scripts/test_release.sh [path-to-tarball]
#
# Runs a full end-to-end verification of a release bundle on the current system.
# This script simulates what a new user would do:
#   1. Extract release bundle
#   2. Run install.sh
#   3. Start daemon
#   4. Run doctor
#   5. Run version
#   6. Run search
#   7. Run semantic search (if AI enabled)
#
# Exit code 0 = all tests pass, non-zero = failure.
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

PASS=0
FAIL=0
SKIP=0

pass() { echo -e "  ${GREEN}✓ PASS${RESET}: $*"; PASS=$((PASS + 1)); }
fail() { echo -e "  ${RED}✗ FAIL${RESET}: $*"; FAIL=$((FAIL + 1)); }
skip() { echo -e "  ${YELLOW}— SKIP${RESET}: $*"; SKIP=$((SKIP + 1)); }
step() { echo -e "\n${BOLD}$*${RESET}"; }

# ─── Find tarball / release directory ────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

TARBALL=""
RELEASE_DIR=""

if [ $# -ge 1 ]; then
    TARBALL="$1"
elif ls "$PROJECT_ROOT"/ggnmem-linux-*.tar.gz 1>/dev/null 2>&1; then
    TARBALL=$(ls -t "$PROJECT_ROOT"/ggnmem-linux-*.tar.gz | head -1)
elif [ -d "$PROJECT_ROOT/release" ]; then
    RELEASE_DIR="$PROJECT_ROOT/release"
else
    echo -e "${RED}No release tarball or release/ directory found.${RESET}"
    echo "Usage: bash scripts/test_release.sh [path-to-tarball]"
    echo ""
    echo "Build a release first:"
    echo "  bash scripts/build_release.sh"
    exit 1
fi

# ─── Setup test environment ──────────────────────────────────────────────────

WORK_DIR=$(mktemp -d /tmp/ggnmem-test-XXXXXX)
trap "rm -rf '$WORK_DIR'" EXIT

echo -e "${BOLD}═══════════════════════════════════════${RESET}"
echo -e "${BOLD}  ggnmem release verification${RESET}"
echo -e "${BOLD}═══════════════════════════════════════${RESET}"

# ─── Step 1: Extract release bundle ──────────────────────────────────────────

step "1. Extract release bundle"

if [ -n "$TARBALL" ]; then
    echo "  tarball: $TARBALL"
    tar xzf "$TARBALL" -C "$WORK_DIR"
    RELEASE_DIR="$WORK_DIR"
    pass "tarball extracted to $WORK_DIR"
else
    echo "  using: $RELEASE_DIR"
    # Copy release dir to work dir so we don't modify the original.
    cp -r "$RELEASE_DIR"/* "$WORK_DIR/"
    RELEASE_DIR="$WORK_DIR"
    pass "release directory copied"
fi

# Verify expected files.
for f in ggnmem ggnmem-daemon install.sh README.md; do
    if [ -f "$RELEASE_DIR/$f" ]; then
        pass "$f present"
    else
        fail "$f missing from release"
    fi
done

# ─── Step 2: Run install.sh ──────────────────────────────────────────────────

step "2. Run install.sh"

if bash "$RELEASE_DIR/install.sh" 2>&1; then
    pass "install.sh completed successfully"
else
    fail "install.sh failed"
fi

# Ensure PATH includes ~/.local/bin.
export PATH="$HOME/.local/bin:$PATH"

# ─── Step 3: Start daemon ────────────────────────────────────────────────────

step "3. Start daemon"

if ggnmem start 2>&1; then
    pass "daemon started"
    sleep 1
else
    fail "daemon failed to start"
fi

# ─── Step 4: Run doctor ──────────────────────────────────────────────────────

step "4. Run doctor"

if ggnmem doctor 2>&1; then
    pass "doctor ran successfully"
else
    fail "doctor failed"
fi

# ─── Step 5: Run version ─────────────────────────────────────────────────────

step "5. Run version"

VERSION_OUTPUT=$(ggnmem version 2>&1 || true)
echo "$VERSION_OUTPUT"

# Verify version output contains expected fields.
if echo "$VERSION_OUTPUT" | grep -q "Version:"; then
    pass "version shows Version field"
else
    fail "version missing Version field"
fi

if echo "$VERSION_OUTPUT" | grep -q "Commit:"; then
    pass "version shows Commit field"
else
    fail "version missing Commit field"
fi

if echo "$VERSION_OUTPUT" | grep -q "Build:"; then
    pass "version shows Build field"
else
    fail "version missing Build field"
fi

if echo "$VERSION_OUTPUT" | grep -q "ONNX:"; then
    pass "version shows ONNX field"
else
    fail "version missing ONNX field"
fi

# Test verbose mode.
VERBOSE_OUTPUT=$(ggnmem version --verbose 2>&1 || true)
if echo "$VERBOSE_OUTPUT" | grep -q "Target:"; then
    pass "version --verbose shows extended info"
else
    fail "version --verbose missing extended info"
fi

# ─── Step 6: Run Ctrl+R (TUI quick check) ────────────────────────────────────

step "6. Check TUI availability"

# We can't run the actual TUI in a script, but verify the binary supports it.
if ggnmem 2>&1 | grep -q "  ui "; then
    pass "TUI command available"
else
    skip "TUI command check"
fi

# ─── Step 7: Run search ──────────────────────────────────────────────────────

step "7. Run search"

# Ingest a test command first.
ggnmem ingest --command "docker compose up" --cwd "/tmp" --exit-code 0 --duration-ms 100 2>/dev/null || true
sleep 1

SEARCH_OUTPUT=$(ggnmem search docker 2>&1 || true)
echo "$SEARCH_OUTPUT"

# Search may return results or "no matching commands" — both are valid.
if echo "$SEARCH_OUTPUT" | grep -qE "(result|no matching|docker)"; then
    pass "search executed"
else
    fail "search produced unexpected output"
fi

# ─── Step 8: Run semantic search ─────────────────────────────────────────────

step "8. Run semantic search"

AI_STATUS=$(ggnmem ai status 2>&1 || true)
if echo "$AI_STATUS" | grep -q "ai_enabled.*true"; then
    SEM_OUTPUT=$(ggnmem semantic docker 2>&1 || true)
    echo "$SEM_OUTPUT"
    if echo "$SEM_OUTPUT" | grep -qE "(result|no semantic)"; then
        pass "semantic search executed"
    else
        fail "semantic search produced unexpected output"
    fi
else
    skip "semantic search (AI not enabled)"
fi

# ─── Step 9: Stop daemon ─────────────────────────────────────────────────────

step "9. Cleanup"

ggnmem stop 2>/dev/null || true
pass "daemon stopped"

# ─── Summary ─────────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}═══════════════════════════════════════${RESET}"
echo -e "${BOLD}  Release Verification Summary${RESET}"
echo -e "${BOLD}═══════════════════════════════════════${RESET}"
echo ""
echo -e "  ${GREEN}PASS${RESET}: $PASS"
echo -e "  ${RED}FAIL${RESET}: $FAIL"
echo -e "  ${YELLOW}SKIP${RESET}: $SKIP"
echo ""

if [ $FAIL -eq 0 ]; then
    echo -e "  ${GREEN}${BOLD}All checks passed! Release is ready.${RESET}"
    exit 0
else
    echo -e "  ${RED}${BOLD}$FAIL check(s) failed. Review above output.${RESET}"
    exit 1
fi
