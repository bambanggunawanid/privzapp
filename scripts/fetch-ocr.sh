#!/usr/bin/env bash
# Fetch the OCR runtime for the text-recognition tools (ADR-0011).
#
# Same contract as fetch-ffmpeg.sh: nothing lands in git, every byte is
# version- AND sha256-pinned, and scripts/build-web.sh copies the result
# to the bundle root UNhashed (the ESM lib resolves its worker — and the
# worker its wasm — relative to their own URLs, which dx hashing would
# break). Served same-origin always, never a CDN.
#
# Size strategy: tessdata_fast models (integer-quantized LSTM — eng is
# 3.9 MB vs 15 MB for tessdata_best) and per-language lazy fetch: only
# the language the user picks ever downloads. Staging another language
# here grows the Docker image, never the user's first load.
set -euo pipefail
cd "$(dirname "$0")/.."

WASM_VERSION="0.11.0"
WASM_SHA256="0d1b399a55028330a3779c331725993ad727db219b9112a899e7bf215ca55014"
# tesseract-ocr/tessdata_fast, pinned by commit.
TESSDATA_COMMIT="87416418657359cb625c412a48b6e1d6d41c29bd"
ENG_SHA256="7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
IND_SHA256="69786901da87ab8766c1ea7fbb10b28f2110c14da3f6c8f2735df131fba95d88"

DEST="app/ocr"
STAMP="$DEST/.version"
WANT="tesseract-wasm=$WASM_VERSION tessdata=$TESSDATA_COMMIT langs=eng,ind"

if [[ -f "$STAMP" && "$(cat "$STAMP")" == "$WANT" ]]; then
  echo "OCR runtime already fetched ($WANT)"
  exit 0
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fetch() { # url sha256 out
  curl -fsSL --proto '=https' --tlsv1.2 -o "$3" "$1"
  echo "$2  $3" | sha256sum -c - >/dev/null
}

echo "==> fetching tesseract-wasm@$WASM_VERSION + tessdata_fast eng/ind (pinned sha256)"
fetch "https://registry.npmjs.org/tesseract-wasm/-/tesseract-wasm-$WASM_VERSION.tgz" "$WASM_SHA256" "$TMP/tw.tgz"
fetch "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/$TESSDATA_COMMIT/eng.traineddata" "$ENG_SHA256" "$TMP/eng.traineddata"
fetch "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/$TESSDATA_COMMIT/ind.traineddata" "$IND_SHA256" "$TMP/ind.traineddata"

tar xzf "$TMP/tw.tgz" -C "$TMP"

rm -rf "$DEST"
mkdir -p "$DEST/tessdata"
# lib.js (ESM) spawns tesseract-worker.js relative to itself; the worker
# picks tesseract-core.wasm (SIMD) or the -fallback (pre-2023 browsers)
# relative to ITSELF. All four must sit side by side.
cp "$TMP/package/dist/lib.js" \
   "$TMP/package/dist/tesseract-worker.js" \
   "$TMP/package/dist/tesseract-core.wasm" \
   "$TMP/package/dist/tesseract-core-fallback.wasm" "$DEST/"
cp "$TMP/package/LICENSE.md" "$DEST/LICENSE.md"
cp "$TMP/eng.traineddata" "$TMP/ind.traineddata" "$DEST/tessdata/"

cat >> "$DEST/LICENSE.md" <<'LIC'

---

The `tessdata/*.traineddata` recognition models are from the
tesseract-ocr/tessdata_fast repository, Apache-2.0.
LIC

echo "$WANT" > "$STAMP"
echo "OCR runtime ready in $DEST ($(du -sh "$DEST" | cut -f1))"
