# ADR-0007: PDF editor rendered with bundled PDF.js

- **Status**: Accepted
- **Date**: 2026-08-24

## Context

An editor (hand-drawn signatures, handwriting brush, image stamps) needs to
*show* the page being edited. Pure-Rust PDF rasterization doesn't exist at
usable fidelity (the same blocker as PDF→JPG, see ROADMAP), and iframe/embed
browser viewers give no coordinate control for overlays.

## Decision

Bundle Mozilla **PDF.js** (Apache-2.0) as a static asset — the project's
first JavaScript library dependency, accepted under strict conditions:

1. **Display only.** PDF.js renders pages into canvases. All mutation stays
   in the Rust engine: `pz_pdf::annotate` bakes ink strokes (as vector
   stroke operators — crisp at any zoom, tiny output) and images (JPEG
   XObjects) into the PDF.
2. **Bundled, never CDN.** `app/assets/pdfjs/` ships with the app, hashed
   by dx and cached by the service worker; the offline and no-third-party
   promises hold. License file included.
3. **Isolated in the editor.** Only the editor page loads it (dynamic
   `import()` on first use), so every other tool stays wasm-only and the
   initial bundle doesn't grow.

Architecture: `assets/editor.js` owns rendering + overlay canvases
(pointer-drawn strokes, drag-rect image placement, undo) in canvas
coordinates; `pzExport()` converts to PDF points (origin bottom-left) and
hands JSON to the Dioxus page, which builds typed `PageEdit`s and calls
`pz_engine::edit_pdf`. Signatures are vector ink, not raster stamps, so no
alpha-channel embedding is needed.

## Consequences

- Editing works fully client-side incl. offline; nothing about the file,
  signature or drawings leaves the device.
- ~1.7 MB extra assets, loaded lazily; native (desktop/mobile) builds need
  a different render path later (pdfium via FFI is the candidate).
- The PDF.js instance also unblocks future PDF→JPG export (same render,
  canvas → PNG) — noted in ROADMAP.
- Canvas interop can't be tested headless; the editor needs a manual
  browser pass before any release that touches it.
