# ADR-0011: On-device OCR via tesseract-wasm

- **Status**: Accepted
- **Date**: 2026-08-30
- **Builds on**: [ADR-0009](0009-browser-rasterization.md) (page
  rasterization), [ADR-0010](0010-ffmpeg-wasm-integration.md) (the
  lazy-big-module pattern)

## Context

OCR sat in the ROADMAP's parity-gap list with a "~15 MB" price tag. The
owner's constraint when green-lighting it was explicit: size. The 15 MB
figure came from tesseract's *best* (float LSTM) English model; it is
not what has to ship.

## Decision

Two tools — **Image to Text** (Image category, multi-file) and **OCR
PDF** (PDF category) — as `ToolPipeline::BrowserOcr`:
`app/assets/ocrtool.js` + `app/src/ocr.rs` drive Bloomberg's
`tesseract-wasm` (BSD-2-Clause) in a Web Worker. OCR PDF composes two
pipelines: pages are rasterized by the ADR-0009 renderer, then each
page's pixels are recognized. `pz_engine::run` refuses the slugs, as
with every browser pipeline.

### The size strategy (the point of this ADR)

1. **The engine is small**: `tesseract-core.wasm` is 1.8 MB raw
   (~0.9 MB wired, gzip_static), plus ~190 KB of JS. A same-sized
   non-SIMD fallback is staged but never downloaded by any browser
   newer than early 2023.
2. **`tessdata_fast`, not `tessdata_best`**: integer-quantized LSTM
   models — English is 3.9 MB instead of 15 MB, with accuracy loss that
   doesn't matter for screenshots and scans.
3. **Per-language lazy fetch**: nothing OCR-related loads until an OCR
   tool page runs, and only the *selected* language's model is ever
   fetched. Staged languages (eng + ind today) cost Docker image bytes,
   never user bandwidth. Adding one = a pinned entry in
   `scripts/fetch-ocr.sh`, an arm in `app/src/ocr.rs::safe_lang`, and
   an option in the widget.
4. **Total first-use wire cost ≈ 4–5 MB** (engine + English model),
   less than half the video engine's 10.2 MB — then the service
   worker's runtime cache makes it offline and free.

The language code is an allowlist (`safe_lang`), not a passthrough — it
is spliced into a URL path.

### Serving

Same treatment as `/ffmpeg/` and for the same reason: the ESM lib
resolves its worker relative to `import.meta.url`, and the worker its
wasm relative to itself, so `scripts/fetch-ocr.sh` stages everything
(version- and sha256-pinned; models pinned to a tessdata_fast commit)
into `app/ocr/` (gitignored) and `build-web.sh` copies it to the bundle
root unhashed. No CSP change: module import, worker and the worker-side
model fetch are all same-origin, and the deploy smoke exercises them.

## Consequences

- "Scanned PDF → text" and "copy text out of a picture" work fully
  on-device; `extract-text-pdf` keeps the digital-text fast path and its
  SEO copy now cross-references the difference.
- Recognition quality scales with the render: OCR PDF exposes the
  ADR-0009 resolution option (more pixels, better reads).
- Headless-testable end to end: the text fixture is drawn on a canvas
  inside the test browser, so expected strings are exact and no binary
  fixture enters the repo.
- Upgrading = bumping pins in `scripts/fetch-ocr.sh`.
