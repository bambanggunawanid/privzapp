# Roadmap

Status and rationale for everything on the README roadmap. Checkboxes live
in README.md; this file carries the detail. Constraints that shape all of
it: client-side only, wasm32-safe, free forever (see ADR-0001).

## Done

- **29 tools** across PDF / Image / Compress / Protect (see README table),
  covering most of the iLovePDF/iLoveIMG catalog that is feasible fully
  client-side — including standard AES-256 PDF password protection and
  removal, page numbers, PDF crop/repair/text-extraction, and the common
  image edits (rotate/flip/upscale/grayscale/blur/text watermark).
- **PWA**: installable, offline-capable release bundle
  (`scripts/build-web.sh` puts manifest/service-worker/icons at the site
  root; cache-first is safe because dx fingerprints assets).
- **Drag-and-drop** onto the tool page dropzone.
- **PDF page → image export** (2026-08-29): the PDF to Image tool renders
  pages with the bundled PDF.js and packages them in the engine — PNG/JPG/
  WebP, 1x-4x, optional page range (ADR-0009). Pure-Rust rasterization
  still doesn't exist; this borrows the editor's renderer instead of
  waiting for one.
- **Password vaults**: AES-256-GCM `.pzv` files (ADR-0003).

## Approved, designed, not yet built

### Web Worker offloading (ADR-0004, Proposed)
Large files currently run the engine on the main thread; a 200 MB zip can
freeze the tab for seconds. Design: a second wasm binary (worker entry
point) exposing `run(slug, files, opts)` behind `postMessage`, transferable
`ArrayBuffer`s to avoid copies, UI falls back to inline execution where
workers are unavailable. Deliberately **not** wasm-threads: those need
cross-origin isolation headers and nightly features. Blocked only on being
able to verify in a real browser — do not land blind; needs a manual
smoke-test pass (`dx serve` + large file) before merge.

### ffmpeg-to-WASM video/GIF tools (ADR-0005, Proposed)
Video convert/trim/GIF-ify, still fully client-side. Design: ffmpeg.wasm
(or a trimmed custom emscripten build) loaded **lazily** as a separate
multi-MB module only when a video tool opens; isolated in a `pz-video`
crate/JS shim so the C exception never leaks into the pure engine crates;
same bytes-in/bytes-out contract. Needs: emscripten toolchain, bundle-size
budget (~25 MB), and a licensing pass (LGPL build config, no GPL codecs) —
none of which fit the current container. First concrete step when picked
up: prototype `ffmpeg.wasm` npm package behind a feature-flagged route.

### Folder support for drag-and-drop
Dropping a directory needs `webkitGetAsEntry` recursion (JS interop beyond
what Dioxus events expose today). Plan: small `wasm-bindgen` helper walking
`DataTransferItem` entries, feeding the same `InputFile` list.

## iLove-parity gaps and why they're blocked

Competitor features that need capabilities pure-Rust wasm doesn't have yet.
None of them justify a processing server; each waits for a client-side
path:

- **PDF ↔ Office (Word/Excel/PowerPoint)**: faithful conversion needs a
  layout engine (LibreOffice-class). Candidate: a trimmed
  LibreOffice-WASM or docx-targeted generator fed by `extract_text` —
  research item, large.
- **PDF → JPG**: done (ADR-0009). What remains in this family is
  *thumbnail sheets* (contact-sheet style, several pages per image), which
  is a compositing job on top of the same render.
- **OCR (scanned PDFs → text)**: tesseract-wasm exists (~15 MB); viable
  later behind lazy loading, same isolation rules as ffmpeg.
- **AI features (background removal, face blur, AI upscale)**: need ONNX
  models in the browser (~5-40 MB each); possible via ort-wasm/tract —
  evaluate after ffmpeg lands the "big lazy module" pattern.
- **HTML → PDF, certificate-based e-signatures**: still out of scope.
  (The hand-drawn sign/draw/stamp editor shipped — see ADR-0007; what
  remains is cryptographic digital signatures, which need a cert UX.)

## Known issues — Android build (first device test, 2026-08-24)

- **File picker does nothing**: `<input type=file>` in the Android WebView
  needs `WebChromeClient.onShowFileChooser` wired up in the generated
  shell; the dx/wry defaults don't do it, so taps are silently dropped (no
  permission prompt either — SAF pickers don't need one). Fix candidates:
  newer wry with file-chooser support, or a patched MainActivity in the
  gradle template. Until then the web app over the network is the mobile
  path.
- **Saves land app-private**: `dirs::download_dir()` is None on Android →
  temp-dir fallback. Needs MediaStore/SAF integration in `app/src/save.rs`.

## Needs owner action (not code)

- ~~**Donation integrations**~~ — done 2026-08-24: `app/src/pages/support.rs`
  links the real Ko-fi (`ko-fi.com/S7F125OT18`) and GitHub Sponsors
  (`github.com/sponsors/bambanggunawanid`) accounts.
- **Opt-in telemetry wiring**: the queue exists (`pz-telemetry`) but
  transport + settings UI need (a) an endpoint the owner controls and
  (b) explicit sign-off on the exact `Event` schema — it's a privacy
  contract (CLAUDE.md ground rule), so no agent should wire it
  unilaterally. The public dashboard ships with it, not after it.

## Explicitly rejected

- Server-side processing of any kind, accounts, ads, premium tiers — by
  design, forever (ADR-0001).
