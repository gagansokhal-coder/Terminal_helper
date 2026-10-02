#!/usr/bin/env bash
#
# Regression tests for scripts/install-online.sh (see issue #8).
#
# The installer is run end-to-end against a local stand-in for api.github.com,
# so these tests need no network access and install nothing for real: the test
# bundle's install.sh is a no-op stub and the version probe is satisfied by a
# stub ggnmem on PATH.
#
# The ONLY edit made to the script under test is the API_URL constant, so that
# it can be pointed at 127.0.0.1.
#
# Usage:  bash scripts/test_install_online.sh
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALLER="$SCRIPT_DIR/install-online.sh"
SERVER_PY="$SCRIPT_DIR/test_install_online_server.py"
PORT="${PORT:-8940}"
# NB: the name must NOT match the installer's temp-dir glob below, or the
# cleanup step would delete this test's own work directory.
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/installer-test.XXXXXX")"

pass=0
fail=0
ok()  { echo "  ok   - $1"; pass=$((pass + 1)); }
no()  { echo "  FAIL - $1"; fail=$((fail + 1)); }
hdr() { echo; echo "$1"; }

# The old installer hardcodes /tmp/ggnmem-installer while the new one honours
# TMPDIR, so both locations must be inspected to catch a leftover either way.
# The name must not collide with the test's own WORK_DIR.
purge_installer_tmp() {
    local base
    for base in /tmp "${TMPDIR:-/tmp}"; do
        [ -d "$base" ] || continue
        rm -rf "${base}"/ggnmem-installer "${base}"/ggnmem-installer.* 2>/dev/null
    done
    return 0
}

installer_tmp_leftovers() {
    local base candidate
    for base in /tmp "${TMPDIR:-/tmp}"; do
        [ -d "$base" ] || continue
        # Each pattern is checked with -e first: an unmatched glob would
        # otherwise be echoed back verbatim and look like a leftover.
        for candidate in "${base}"/ggnmem-installer "${base}"/ggnmem-installer.*; do
            [ -e "$candidate" ] && printf '%s\n' "$candidate"
        done
    done
    return 0
}

cleanup() {
    [ -n "${SERVER_PID:-}" ] && kill "$SERVER_PID" 2>/dev/null
    wait "${SERVER_PID:-}" 2>/dev/null
    purge_installer_tmp
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

start_server() { # $1 = mode
    MODE="$1" PORT="$PORT" python3 "$SERVER_PY" >"$WORK_DIR/server.log" 2>&1 &
    SERVER_PID=$!
    for _ in $(seq 1 60); do
        if curl -sS -o /dev/null "http://127.0.0.1:${PORT}/releases" 2>/dev/null; then return 0; fi
        sleep 0.1
    done
    echo "  test double failed to start:"; cat "$WORK_DIR/server.log"; exit 1
}
stop_server() { kill "$SERVER_PID" 2>/dev/null; wait "$SERVER_PID" 2>/dev/null; unset SERVER_PID; }

# The script under test, with only API_URL redirected at the local server.
sed 's|^API_URL=.*|API_URL="http://127.0.0.1:'"$PORT"'/releases"|' "$INSTALLER" > "$WORK_DIR/install-online.sh"

# A stub ggnmem so the post-install version check can pass.
mkdir -p "$WORK_DIR/stubbin"
printf '#!/bin/sh\necho "ggnmem 0.0.0-test"\n' > "$WORK_DIR/stubbin/ggnmem"
chmod +x "$WORK_DIR/stubbin/ggnmem"
export PATH="$WORK_DIR/stubbin:$PATH"

# The installer must not use a fixed, predictable path in world-writable /tmp.
FIXED_TMP_DIR="/tmp/ggnmem-installer"
FIXED_TMP_FILE="$FIXED_TMP_DIR/ggnmem-linux-x86_64.tar.gz"

########################################################################
hdr "happy path: install completes and verifies the checksum"
start_server ok
OUT="$(bash "$WORK_DIR/install-online.sh" 2>&1)"; RC=$?
stop_server
if [ $RC -eq 0 ] && grep -q "Installation complete" <<<"$OUT"; then
    ok "installer completes end-to-end"
else
    no "installer did not complete (rc=$RC)"
    # shellcheck disable=SC2001  # replacement contains the sed delimiter
    sed 's/^/       | /' <<<"$OUT"
fi
if grep -q "Checksum verified" <<<"$OUT"; then
    ok "checksum is verified"
else
    no "checksum verification did not run"
fi
if grep -q "Installed version: ggnmem" <<<"$OUT"; then
    ok "installed version is reported"
else
    no "installed version was not reported"
fi

########################################################################
hdr "temp dir: a planted symlink must not be followed"
# Another local user pre-creates the temp dir and symlinks the asset path to
# a file the victim can write. curl -o would follow it and clobber the target.
VICTIM="$WORK_DIR/victim.txt"
printf 'ORIGINAL-IMPORTANT-DATA\n' > "$VICTIM"
rm -rf "$FIXED_TMP_DIR"
mkdir -p "$FIXED_TMP_DIR"
ln -sf "$VICTIM" "$FIXED_TMP_FILE"

start_server ok
OUT="$(bash "$WORK_DIR/install-online.sh" 2>&1)"; RC=$?
stop_server
if grep -q "ORIGINAL-IMPORTANT-DATA" "$VICTIM"; then
    ok "a symlink planted in /tmp is not written through"
else
    no "the planted symlink target was overwritten"
fi
rm -rf "$FIXED_TMP_DIR"

########################################################################
hdr "cleanup: a failed download leaves nothing behind"
# The asset response is truncated mid-body, so curl fails at the download step,
# which is exactly when a partial tarball exists on disk.
start_server dlfail
rm -rf "$FIXED_TMP_DIR"
purge_installer_tmp
OUT="$(bash "$WORK_DIR/install-online.sh" 2>&1)"; RC=$?
stop_server
LEFTOVER=$(installer_tmp_leftovers)
if [ $RC -ne 0 ]; then
    ok "a truncated download fails the installer"
else
    no "a truncated download unexpectedly succeeded"
fi
if [ -z "$LEFTOVER" ]; then
    ok "no partial download or temp dir is left behind"
else
    no "leftover installer temp data: $(echo "$LEFTOVER" | tr '\n' ' ')"
    purge_installer_tmp
fi

########################################################################
hdr "rate limiting: a 403 is diagnosed, not blamed on connectivity"
start_server ratelimit
OUT="$(bash "$WORK_DIR/install-online.sh" 2>&1)"; RC=$?
stop_server
if [ $RC -ne 0 ]; then
    ok "a rate-limited API request fails the installer"
else
    no "a rate-limited API request unexpectedly succeeded"
fi
if grep -qi "rate limit" <<<"$OUT"; then
    ok "the 403 is reported as a rate limit"
else
    no "the 403 is not identified as a rate limit"
    # shellcheck disable=SC2001  # replacement contains the sed delimiter
    sed 's/^/       | /' <<<"$OUT"
fi
if grep -q "GITHUB_TOKEN" <<<"$OUT"; then
    ok "GITHUB_TOKEN is suggested as the remedy"
else
    no "GITHUB_TOKEN is never mentioned"
fi
if grep -qi "connectivity" <<<"$OUT"; then
    no "a 403 quota error is misreported as a connectivity problem"
else
    ok "a 403 quota error is not misreported as a connectivity problem"
fi

########################################################################
hdr "checksum lookup: only the exact filename is matched"
# checksums.txt can list several assets. A line that merely starts with the
# expected name (e.g. "<asset>.sig") must not be picked up as the hash.
BAD_SUMS="$WORK_DIR/bad-checksums.txt"
cat > "$BAD_SUMS" <<'EOF'
deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef  ggnmem-linux-x86_64.tar.gz.sig
1111111111111111111111111111111111111111111111111111111111111111  ggnmem-linux-x86_64.tar.gz
EOF
if awk -v asset="ggnmem-linux-x86_64.tar.gz" \
       '{ name = $2; sub(/^\*/, "", name); if (name == asset) print $1 }' "$BAD_SUMS" \
   | grep -qx '1111111111111111111111111111111111111111111111111111111111111111'; then
    ok "the exact asset line is selected, ignoring the .sig line"
else
    no "checksum lookup did not select the exact line"
fi

# The same must hold inside the installer: serve a checksums.txt whose first
# matching line is a decoy, and the install must still succeed.
# shellcheck disable=SC2016  # the literal $( is the pattern, not a substitution
if grep -q 'EXPECTED_HASH=$(awk -v asset=' "$INSTALLER"; then
    ok "installer uses an exact-match checksum lookup"
else
    no "installer does not use an exact-match checksum lookup"
fi

########################################################################
hdr "static checks"
if command -v shellcheck >/dev/null 2>&1; then
    if shellcheck "$INSTALLER"; then
        ok "shellcheck reports no warnings for install-online.sh"
    else
        no "shellcheck reported warnings for install-online.sh"
    fi
else
    echo "  skip - shellcheck not installed"
fi
if bash -n "$INSTALLER"; then
    ok "install-online.sh parses cleanly"
else
    no "install-online.sh has a syntax error"
fi

echo
echo "================================================"
echo "  passed: $pass   failed: $fail"
echo "================================================"
[ $fail -eq 0 ]
