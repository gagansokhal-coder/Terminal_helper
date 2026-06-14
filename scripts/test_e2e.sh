#!/usr/bin/env bash
set -e

# Resolve project root relative to this script.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Source cargo env if available.
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

# Use temp directories so we don't pollute real XDG dirs.
TEST_DIR=$(mktemp -d)
export XDG_RUNTIME_DIR="$TEST_DIR/runtime"
export XDG_DATA_HOME="$TEST_DIR/data"
mkdir -p "$XDG_RUNTIME_DIR/ggnmem"
mkdir -p "$XDG_DATA_HOME/ggnmem"

cleanup() {
    kill "$DAEMON_PID" 2>/dev/null || true
    wait "$DAEMON_PID" 2>/dev/null || true
    rm -rf "$TEST_DIR"
    echo "Cleanup done."
}
trap cleanup EXIT

echo "=== Starting daemon in background ==="
cargo run --bin ggnmem-daemon &
DAEMON_PID=$!
sleep 2

echo ""
echo "=== Ping daemon ==="
cargo run --bin ggnmem-cli -- ping

echo ""
echo "=== Ingest: ls ==="
cargo run --bin ggnmem-cli -- ingest \
    --command "ls" \
    --cwd "/home/user/projects" \
    --exit-code 0 \
    --duration-ms 5 \
    --shell zsh \
    --session-id "test-session-001" \
    --hostname "devbox" \
    --started-at-ms 1716364800000 \
    --completed-at-ms 1716364800005

echo ""
echo "=== Ingest: git status ==="
cargo run --bin ggnmem-cli -- ingest \
    --command "git status" \
    --cwd "/home/user/projects/ggnmem" \
    --exit-code 0 \
    --duration-ms 12 \
    --shell zsh \
    --session-id "test-session-001" \
    --hostname "devbox" \
    --started-at-ms 1716364810000 \
    --completed-at-ms 1716364810012

echo ""
echo "=== Ingest: docker ps ==="
cargo run --bin ggnmem-cli -- ingest \
    --command "docker ps" \
    --cwd "/home/user/projects" \
    --exit-code 1 \
    --duration-ms 230 \
    --shell bash \
    --session-id "test-session-001" \
    --hostname "devbox" \
    --started-at-ms 1716364820000 \
    --completed-at-ms 1716364820230

# Give queue worker a moment to persist.
sleep 1

echo ""
echo "=== ggnmem recent ==="
cargo run --bin ggnmem-cli -- recent

echo ""
echo "=== ggnmem count ==="
cargo run --bin ggnmem-cli -- count

echo ""
echo "=== ggnmem doctor ==="
cargo run --bin ggnmem-cli -- doctor

echo ""
echo "=== ALL TESTS PASSED ==="
