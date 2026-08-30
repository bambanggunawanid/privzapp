#!/usr/bin/env bash
# Release build for the web target, with PWA files installed at the origin
# root (required: a service worker's scope is capped at its own path, so
# sw.js cannot live under the hashed /assets/ tree).
set -euo pipefail
cd "$(dirname "$0")/.."

# Local config/secrets live in .env (gitignored; template: .env.example).
if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

# dx does NOT clean its output directory: every build drops another
# hashed copy of the ~4 MB app wasm (and its .gz sibling) into assets/
# and leaves the old ones. Left alone that reached 54 stale binaries and
# a 248 MB bundle in one working session — all of which would ship
# inside the container image. The directory is pure build output, so
# wiping it before each release build is safe and keeps the bundle
# honest. (It also removes the stale-bundle trap behind the Web Worker
# gotcha in ADR-0004.)
rm -rf "target/dx/privzapp/release/web"

(cd app && dx build --platform web --release)

# dx emits the static site under target/dx/<app>/release/web/public.
OUT="target/dx/privzapp/release/web/public"
if [[ ! -d "$OUT" ]]; then
  echo "error: expected bundle at $OUT — did the dx output layout change?" >&2
  exit 1
fi

cp app/pwa/* "$OUT/"

# ffmpeg.wasm for the video tools (ADR-0010): fetched pinned, served
# UNhashed from /ffmpeg/ — the wrapper resolves its worker chunk relative
# to its own URL, which dx asset hashing would break. Lazily loaded, so
# it costs nothing until a video tool runs.
./scripts/fetch-ffmpeg.sh
mkdir -p "$OUT/ffmpeg"
cp app/ffmpeg/*.js app/ffmpeg/*.wasm app/ffmpeg/LICENSE.md "$OUT/ffmpeg/"

# OCR runtime (ADR-0011): same unhashed-root treatment, same reason.
./scripts/fetch-ocr.sh
mkdir -p "$OUT/ocr/tessdata"
cp app/ocr/*.js app/ocr/*.wasm app/ocr/LICENSE.md "$OUT/ocr/"
cp app/ocr/tessdata/*.traineddata "$OUT/ocr/tessdata/"

# Prerender SEO pages (per-tool HTML, sitemap, robots) from the registry.
# BASE_URL must be the real production origin for canonicals to be valid.
BASE_URL="${BASE_URL:-https://privzapp.com}" cargo run --quiet --release -p seo-gen -- "$OUT"

echo "Web bundle with PWA + SEO pages ready: $OUT"
