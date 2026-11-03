#!/usr/bin/env bash
set -euo pipefail

# Universal One-Step Repository Automation Bootstrapper (Bash wrapper)
# Usage: ./tools/repository-automation/bootstrap.sh -t <path-to-target-repo> [options]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOOLKIT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

if ! command -v node >/dev/null 2>&1; then
  echo "Error: Node.js is required to run the bootstrapper." >&2
  echo "Please install Node.js 18+ and retry." >&2
  exit 1
fi

node "${SCRIPT_DIR}/bootstrap.mjs" "$@"
