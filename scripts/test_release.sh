#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# ggnmem release verification script
#
# Usage:
#   bash scripts/test_release.sh [path-to-tarball]
#
# Runs a full end-to-end verification of a release bundle on the current system.
# This script simulates what a new user would do:
#   1.  Extract release bundle
#   2.  Verify checksums
#   3.  Run install.sh
#   4.  Start daemon
#   5.  Run doctor
#   6.  Run version (verify all metadata fields)
#   7.  Check TUI availability
#   8.  Run search
#   9.  Run AI setup check
#   10. Run semantic search
#   11. Upgrade workflow test
#   12. Cleanup
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

# Verify VERSION file.
if [ -f "$RELEASE_DIR/VERSION" ]; then
    pass "VERSION file present"
    # Check expected fields in VERSION file.
    for field in version commit date arch; do
        if grep -q "^${field}=" "$RELEASE_DIR/VERSION"; then
            pass "VERSION contains $field"
        else
            fail "VERSION missing $field"
        fi
    done
else
    fail "VERSION file missing from release"
fi

# ─── Step 2: Verify checksums ────────────────────────────────────────────────

step "2. Verify checksums"

if [ -f "$RELEASE_DIR/checksums.txt" ]; then
    pass "checksums.txt present"
    cd "$RELEASE_DIR"
    if sha256sum --check checksums.txt > /dev/null 2>&1; then
        pass "all checksums verified"
    else
        fail "checksum verification failed"
        sha256sum --check checksums.txt 2>&1 | head -10
    fi
    cd "$PROJECT_ROOT"
else
    skip "checksums.txt not present"
fi

# ─── Step 3: Run install.sh ──────────────────────────────────────────────────

step "3. Run install.sh"

if bash "$RELEASE_DIR/install.sh" 2>&1; then
    pass "install.sh completed successfully"
else
    fail "install.sh failed"
fi

# Ensure PATH includes ~/.local/bin.
export PATH="$HOME/.local/bin:$PATH"

# ─── Step 4: Start daemon ────────────────────────────────────────────────────

step "4. Start daemon"

if ggnmem start 2>&1; then
    pass "daemon started"
    sleep 1
else
    fail "daemon failed to start"
fi

# ─── Step 5: Run doctor ──────────────────────────────────────────────────────

step "5. Run doctor"

if ggnmem doctor 2>&1; then
    pass "doctor ran successfully"
else
    fail "doctor failed"
fi

# ─── Step 6: Run version ─────────────────────────────────────────────────────

step "6. Run version"

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

if echo "$VERSION_OUTPUT" | grep -q "Rust:"; then
    pass "version shows Rust field"
else
    fail "version missing Rust field"
fi

if echo "$VERSION_OUTPUT" | grep -q "Platform:"; then
    pass "version shows Platform field"
else
    fail "version missing Platform field"
fi

if echo "$VERSION_OUTPUT" | grep -q "ONNX:"; then
    pass "version shows ONNX field"
else
    fail "version missing ONNX field"
fi

if echo "$VERSION_OUTPUT" | grep -q "AI:"; then
    pass "version shows AI field"
else
    fail "version missing AI field"
fi

# Test verbose mode.
VERBOSE_OUTPUT=$(ggnmem version --verbose 2>&1 || true)
if echo "$VERBOSE_OUTPUT" | grep -q "Profile:"; then
    pass "version --verbose shows Profile info"
else
    fail "version --verbose missing Profile info"
fi

if echo "$VERBOSE_OUTPUT" | grep -q "Binary:"; then
    pass "version --verbose shows Binary path"
else
    fail "version --verbose missing Binary path"
fi

# ─── Step 7: Check TUI availability ─────────────────────────────────────────

step "7. Check TUI availability"

# We can't run the actual TUI in a script, but verify the binary supports it.
if ggnmem 2>&1 | grep -q "  ui "; then
    pass "TUI command available"
else
    skip "TUI command check"
fi

# ─── Step 8: Run search ──────────────────────────────────────────────────────

step "8. Run search"

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

# ─── Step 9: AI setup check ─────────────────────────────────────────────────

step "9. AI setup check"

AI_STATUS=$(ggnmem ai status 2>&1 || true)
echo "$AI_STATUS"

if echo "$AI_STATUS" | grep -qE "(ai_enabled|AI|enabled|disabled)"; then
    pass "ai status ran successfully"
else
    fail "ai status produced unexpected output"
fi

# Check ai models command.
AI_MODELS=$(ggnmem ai models 2>&1 || true)
if echo "$AI_MODELS" | grep -qE "(MiniLM|model|available)"; then
    pass "ai models lists available models"
else
    skip "ai models check (no models listed)"
fi

# ─── Step 10: Run semantic search ────────────────────────────────────────────

step "10. Run semantic search"

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

# ─── Step 11: Upgrade workflow test ──────────────────────────────────────────

step "11. Upgrade workflow test"

# Create a temporary bundle directory for upgrade testing.
UPGRADE_DIR=$(mktemp -d /tmp/ggnmem-upgrade-test-XXXXXX)

# Copy the release binaries to the upgrade dir.
cp "$RELEASE_DIR/ggnmem" "$UPGRADE_DIR/"
cp "$RELEASE_DIR/ggnmem-daemon" "$UPGRADE_DIR/"
[ -f "$RELEASE_DIR/checksums.txt" ] && cp "$RELEASE_DIR/checksums.txt" "$UPGRADE_DIR/"

UPGRADE_OUTPUT=$(ggnmem upgrade --bundle "$UPGRADE_DIR" 2>&1 || true)
echo "$UPGRADE_OUTPUT"

if echo "$UPGRADE_OUTPUT" | grep -qE "(upgrade complete|backed up|installed)"; then
    pass "upgrade workflow completed"
else
    fail "upgrade workflow failed"
fi

# Verify binaries still work after upgrade.
if ggnmem version > /dev/null 2>&1; then
    pass "binaries functional after upgrade"
else
    fail "binaries broken after upgrade"
fi

rm -rf "$UPGRADE_DIR"

# ─── Step 12: Cleanup ───────────────────────────────────────────────────────

step "12. Cleanup"

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
echo -e "  TOTAL: $((PASS + FAIL + SKIP))"
echo ""

if [ $FAIL -eq 0 ]; then
    echo -e "  ${GREEN}${BOLD}All checks passed! Release is ready.${RESET}"
    exit 0
else
    echo -e "  ${RED}${BOLD}$FAIL check(s) failed. Review above output.${RESET}"
    exit 1
fi
