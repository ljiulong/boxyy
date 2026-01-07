#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FRONTEND_DIR="${ROOT_DIR}/gui-frontend"

if ! command -v pnpm >/dev/null 2>&1; then
  echo "pnpm not found in PATH" >&2
  exit 1
fi

cd "${FRONTEND_DIR}"
pnpm install
pnpm build
export PATH="${FRONTEND_DIR}/node_modules/.bin:${PATH}"
cd "${ROOT_DIR}/boxy-gui"
tauri build --bundles app,dmg --updater
