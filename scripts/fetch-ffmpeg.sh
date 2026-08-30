#!/usr/bin/env bash
# Fetch the prebuilt ffmpeg.wasm runtime for the video tools (ADR-0010).
#
# The ~31 MB compiled core doesn't belong in git; like every other
# dependency it's fetched at build time, pinned by version AND sha256 so
# the bytes we serve are exactly the bytes we reviewed. Output goes to
# app/ffmpeg/ (gitignored) and scripts/build-web.sh copies it to the
# bundle root — UNhashed, because the wrapper resolves its worker chunk
# (814.ffmpeg.js) relative to its own URL, which dx asset hashing would
# break. Serving is always same-origin: never a CDN (ADR-0007 rule 2).
set -euo pipefail
cd "$(dirname "$0")/.."

WRAPPER_VERSION="0.12.15"
CORE_VERSION="0.12.10"
WRAPPER_SHA256="c8a23365fb39b46d3d1d9baa2e74b522d00ce5d57e8b20471ad2665eaad38e3e"
CORE_SHA256="d00089ce82e1bdf637ddbe42e0c3d41a1ba8cf4c9e825e7fa4d0bb970e844bd4"

DEST="app/ffmpeg"
STAMP="$DEST/.version"
WANT="ffmpeg=$WRAPPER_VERSION core=$CORE_VERSION"

if [[ -f "$STAMP" && "$(cat "$STAMP")" == "$WANT" ]]; then
  echo "ffmpeg.wasm already fetched ($WANT)"
  exit 0
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fetch() { # url sha256 out
  curl -fsSL --proto '=https' --tlsv1.2 -o "$3" "$1"
  echo "$2  $3" | sha256sum -c - >/dev/null
}

echo "==> fetching @ffmpeg/ffmpeg@$WRAPPER_VERSION + @ffmpeg/core@$CORE_VERSION (pinned sha256)"
fetch "https://registry.npmjs.org/@ffmpeg/ffmpeg/-/ffmpeg-$WRAPPER_VERSION.tgz" "$WRAPPER_SHA256" "$TMP/wrapper.tgz"
fetch "https://registry.npmjs.org/@ffmpeg/core/-/core-$CORE_VERSION.tgz" "$CORE_SHA256" "$TMP/core.tgz"

tar xzf "$TMP/wrapper.tgz" -C "$TMP"
mv "$TMP/package" "$TMP/wrapper"
tar xzf "$TMP/core.tgz" -C "$TMP"

rm -rf "$DEST"
mkdir -p "$DEST"
# UMD builds on purpose: classic worker + importScripts stay inside
# script-src 'self'/worker-src 'self'; the ESM build bakes a broken
# file:// base URL into its worker path.
cp "$TMP/wrapper/dist/umd/ffmpeg.js" "$TMP/wrapper/dist/umd/814.ffmpeg.js" "$DEST/"
cp "$TMP/package/dist/umd/ffmpeg-core.js" "$TMP/package/dist/umd/ffmpeg-core.wasm" "$DEST/"

cat > "$DEST/LICENSE.md" <<'LIC'
# Bundled ffmpeg.wasm licensing

- `ffmpeg.js`, `814.ffmpeg.js` — @ffmpeg/ffmpeg (JS wrapper): MIT.
- `ffmpeg-core.js`, `ffmpeg-core.wasm` — @ffmpeg/core: GPL-2.0-or-later
  (FFmpeg is LGPL-2.1+; this build links GPL components such as x264).

PrivZapp itself is AGPL-3.0-or-later; conveying it combined with the
GPLv2+ core is permitted via the GPLv3 §13 / AGPLv3 §13 compatibility
clause. Sources: https://github.com/ffmpegwasm/ffmpeg.wasm and
https://ffmpeg.org — no PrivZapp-side modifications.
LIC

echo "$WANT" > "$STAMP"
echo "ffmpeg.wasm ready in $DEST ($(du -sh "$DEST" | cut -f1))"
