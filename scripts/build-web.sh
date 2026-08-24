#!/usr/bin/env bash
# Release build for the web target, with PWA files installed at the origin
# root (required: a service worker's scope is capped at its own path, so
# sw.js cannot live under the hashed /assets/ tree).
set -euo pipefail
cd "$(dirname "$0")/.."

(cd app && dx build --platform web --release)

# dx emits the static site under target/dx/<app>/release/web/public.
OUT="target/dx/privzapp/release/web/public"
if [[ ! -d "$OUT" ]]; then
  echo "error: expected bundle at $OUT — did the dx output layout change?" >&2
  exit 1
fi

cp app/pwa/* "$OUT/"

# Prerender SEO pages (per-tool HTML, sitemap, robots) from the registry.
# BASE_URL must be the real production origin for canonicals to be valid.
BASE_URL="${BASE_URL:-https://privzapp.com}" cargo run --quiet --release -p seo-gen -- "$OUT"

echo "Web bundle with PWA + SEO pages ready: $OUT"
