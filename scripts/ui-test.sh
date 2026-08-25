#!/usr/bin/env bash
# Playwright UI tests against the real wasm bundle in headless Chromium.
# Heavier than verify.sh (needs the release web build + node), so it runs
# separately — before releases and on editor/UI changes.
set -euo pipefail
cd "$(dirname "$0")/.."

BUNDLE="target/dx/privzapp/release/web/public"
# On CI, always rebuild: rust-cache restores a stale (and cleanup-mangled)
# target/dx from the previous run — reusing it means testing the wrong
# commit's bundle (or a broken one; that's how the ui-tests job went red
# while every spec passed locally).
if [[ ! -d "$BUNDLE" || "${FRESH_BUNDLE:-}" == "1" || -n "${CI:-}" ]]; then
  echo "==> building web bundle (missing, FRESH_BUNDLE=1 or CI)"
  ./scripts/build-web.sh
fi

cd tests/ui
if [[ ! -d node_modules ]]; then
  echo "==> npm install (first run)"
  npm install --no-fund --no-audit
fi
if ! npx playwright install --dry-run chromium 2>/dev/null | grep -q "is already installed"; then
  echo "==> installing headless chromium (first run)"
  npx playwright install chromium
fi

npx playwright test "$@"
