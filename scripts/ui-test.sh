#!/usr/bin/env bash
# Playwright UI tests against the real wasm bundle in headless Chromium.
# Heavier than verify.sh (needs the release web build + node), so it runs
# separately — before releases and on editor/UI changes.
set -euo pipefail
cd "$(dirname "$0")/.."

BUNDLE="target/dx/privzapp/release/web/public"
if [[ ! -d "$BUNDLE" || "${FRESH_BUNDLE:-}" == "1" ]]; then
  echo "==> building web bundle (missing or FRESH_BUNDLE=1)"
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
