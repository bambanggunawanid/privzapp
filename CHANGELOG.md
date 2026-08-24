# Changelog

All notable changes to PrivZapp. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[SemVer](https://semver.org). Every user-visible change lands here in the
same commit that makes it (see docs/CONTINUOUS_DOCUMENTATION.md).

## [Unreleased]

### Added
- Pre-commit secret guard: `.githooks/pre-commit` + `scripts/check-secrets.py`
  block credentials, key files and `.env` from ever being committed (the
  repo is public); the same scan runs in `verify.sh`/CI. Secrets belong in
  `.env` (gitignored, sourced by `build-web.sh`; template `.env.example`).
- **PDF editor** (30 tools total): draw and sign by hand with a brush
  (pressure-thinned vector ink, color/size controls), stamp images by
  dragging a rectangle, undo, then bake everything into the PDF on-device.
  Pages render via a locally bundled PDF.js (display only — all mutation
  is pure Rust; ADR-0007).
- Twelve more tools (29 total), closing most of the gap to iLovePDF/
  iLoveIMG: Add Page Numbers, Crop PDF, PDF to Text, Repair PDF,
  Protect PDF (standard AES-256 PDF encryption — opens in any viewer)
  and Unlock PDF; Rotate, Flip, Upscale (2x/4x Lanczos), Grayscale,
  Blur and Watermark Image (embedded Liberation Sans, SIL OFL 1.1).
- Eight new tools (17 total): Images to PDF, Watermark PDF, Reorder PDF,
  Strip Metadata (EXIF), Crop Image, Batch Rename, Encrypt File and
  Decrypt File (new "Protect" category).
- Password vaults: AES-256-GCM `.pzv` format with PBKDF2-HMAC-SHA256
  (600k rounds) key derivation in `pz-crypto`.
- Drag-and-drop onto the tool page dropzone.
- PWA: web manifest, offline service worker and app icons; the release
  bundle is installable and works with the network unplugged.
- `scripts/verify.sh` (fmt/clippy/tests/wasm check) and
  `scripts/build-web.sh` (release bundle + PWA files).
- Self-hosting: multi-stage `Dockerfile` (Rust/dx build → nginx static
  serving, access logs off) and `docker-compose.yml` compatible with
  Docker Compose, Podman Compose and Portainer stacks.
- Android: `scripts/build-android.sh` produces an installable APK
  (arm64 + x86_64, release-optimized Rust, debug-signed for testing;
  bundle id `app.privzapp`).

- SEO: every route is now prerendered at build time (`tools/seo-gen`) with
  unique titles, meta descriptions, canonicals, Open Graph tags, JSON-LD
  (WebApplication + FAQPage) and crawlable FAQ content, plus sitemap.xml,
  robots.txt and a 1200×630 social card. Tool pages show the same FAQ and
  related-tools sections in-app; copy lives in `pz_core::seo`
  (test-enforced).

- Live before/after preview on Compress Image and Convert Image: result
  image plus size savings, updating as quality/format changes.
- Favicon Generator (31 tools): any PNG/JPG becomes a standard favicon
  pack ZIP — multi-size favicon.ico, all PNG sizes, apple-touch-icon,
  site.webmanifest and a README with the paste-in HTML snippet.
- PDF editor: Add Text tool — type multi-line text, pick size (pen
  color applies), tap the page to place it; baked into the PDF as real
  Helvetica text. (Rewriting text already inside a PDF needs font
  re-embedding/reflow and stays out of scope — same as iLovePDF.)
- PDF editor is now a full workspace: besides drawing/signing and image
  stamps, you can rotate, add page numbers, watermark, crop margins,
  reorganize/delete/duplicate pages and append another PDF — each
  operation applies to the working copy and returns to the editor, with
  operation-level undo. Export downloads plain, compressed, or AES-256
  password-protected. Pending drawings are baked automatically before
  document operations.

### Fixed
- Refreshing any page flashed unstyled prerender content: the app
  stylesheet is now linked statically in prerendered pages, and a dark
  splash screen (pulsing logo) covers loading until the app mounts,
  with an 8s no-wasm fallback that reveals the styled content.
- PDF editor failed silently on open (errors weren't shown on the
  chooser screen; large files broke the JS handoff — now blob URLs).
- nginx served `.mjs` without a JavaScript MIME type, blocking the
  editor's PDF.js module import entirely.
- Prerendered SEO content stayed visible above the app after load.

### Added
- Playwright UI test suite (`scripts/ui-test.sh`, `tests/ui/`): drives
  the real wasm bundle in headless Chromium and pins every editor
  behavior that has ever regressed (tool switching, text editability,
  undo shortcuts, zoom, drag-reorder, export). Runs as its own CI job.
- Editor text editing, reworked: the text tool places an editable box
  wherever you click; boxes stay selectable, movable (drag with the
  cursor tool) and re-editable until export. Clicking text that's
  already in the PDF converts it to an editable box (white-out +
  retype — best on white backgrounds).
- Editor highlighter ("stabilo"): translucent yellow by default,
  baked into the PDF with a real Multiply blend so text stays readable.
- Editor undo/redo: Ctrl+Z / Ctrl+Shift+Z plus top-bar buttons —
  canvas edits first, then document operations.
- Editor views: ruler (PDF points) and grid toggles.
- Reorder pages by dragging thumbnails in the left rail (the old
  "3,1,2" text input is gone).

### Added
- Compress Image: Resolution control (10–100% of the original, ±10
  buttons) — downscaling is the biggest size lever once quality has
  done its part; the filename keeps the original name.
- Live previews are cached in memory per image + settings, so switching
  between thumbnails is instant instead of recompressing every click.
  Deliberately not persisted across refreshes: uploads never touch
  storage, and neither do results — nothing is left behind.
- Compress/Convert Image: before/after comparison panes; click any
  thumbnail to pick which image the live preview shows.
- Compress Image now genuinely shrinks PNGs at lower quality: palette
  quantization (NeuQuant) with an automatic keep-the-smaller fallback,
  so quality can never make a file bigger.
- Editor images are live objects: inserting places the image at its
  natural size on the current page — drag to move, corner handle to
  resize (hold Shift for proportional), per-image opacity slider
  (baked with real PDF transparency),
  ✕/Delete to remove, all undoable.

### Fixed
- Editor retype inherits the source style: clicking existing PDF text
  now samples its color from the render and detects bold from the text
  metrics — editing changes the content, not the look (bold bakes as
  Helvetica-Bold).
- Compress/Convert Image: the page froze while dragging the quality
  slider (the wasm engine recompressed on every tick, blocking the main
  thread). Dragging now only updates the number; the preview recomputes
  once when the slider is released or via the new +/−10 buttons.
- Compress Image: Clear left the old preview image on screen.
- Image uploads now show real thumbnails (one per file) instead of
  name-and-size rows, so you can see what actually got picked.

### Changed
- Editor defaults: the cursor (select/move/edit) tool is active on
  open — drawing is now an explicit choice.
- The PDF editor is now a Figma-style workspace: full-viewport dark
  shell with page thumbnails on the left (click to jump), a dot-grid
  canvas with zoom (buttons or Ctrl+scroll, drawings rescale losslessly),
  a hand/pan tool, a floating page indicator, and a properties inspector
  on the right holding pen settings, text, and all document operations.
  On phones the inspector slides in on demand.
- Home page got category filter chips and per-category colored icon
  tiles; the nav gained quick links (Merge/Compress/Edit PDF, Compress
  Image) and an "All tools" mega-menu listing every tool by category.
- Support page now links the real donation accounts (Ko-fi and GitHub
  Sponsors); the Liberapay placeholder is gone.
- Quality sliders move in steps of 10 (10–100).
- Official bolt-P logo everywhere: app icon set (incl. maskable and
  apple-touch), favicon, nav brand and README, all derived from
  `app/brand/logo-master.png` by `scripts/gen-icons.py`.

## [0.1.0] — 2026-08-24

### Added
- Initial release: Cargo workspace with pure engine crates
  (`pz-core`, `pz-pdf`, `pz-img`, `pz-archive`, `pz-crypto`,
  `pz-telemetry`, `pz-engine`) and a Dioxus 0.7 app (web/desktop/mobile).
- Nine tools: Merge/Split/Rotate/Compress PDF, Convert/Resize/Compress
  Image, Create/Extract ZIP.
- All processing on-device; engine crates compile for native and wasm32.
