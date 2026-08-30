# Roadmap

Status and rationale for everything on the README roadmap. Checkboxes live
in README.md; this file carries the detail. Constraints that shape all of
it: client-side only, wasm32-safe, free forever (see ADR-0001).

## Done

- **35 tools** across PDF / Image / Compress / Protect / Video (see README
  table),
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
- **Video/GIF tools** (2026-08-30, ADR-0005 design + ADR-0010
  implementation): Video to GIF, lossless Trim, MP4 ↔ WebM conversion via
  the prebuilt ffmpeg.wasm — fetched pinned by sha256, served same-origin
  from /ffmpeg/, loaded lazily in a Web Worker, single-threaded core so no
  cross-origin isolation is needed. The engine crates stay pure; the slugs
  are `ToolPipeline::BrowserFfmpeg` and `pz_engine::run` refuses them.
- **Web Worker offloading** (2026-08-25, ADR-0004 Accepted): the engine
  runs off the main thread in the built bundle, transferable
  `ArrayBuffer`s avoid copies, and the UI falls back to inline execution
  where workers are unavailable — big files no longer freeze the tab.

## Approved, designed, not yet built

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
