# Changelog

All notable changes to PrivZapp. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[SemVer](https://semver.org). Every user-visible change lands here in the
same commit that makes it (see docs/CONTINUOUS_DOCUMENTATION.md).

## [Unreleased]

## [1.0.0] — 2026-08-30

First tagged release. Everything below shipped since 0.1.0: the PDF
editor workspace, the video and OCR tool families, folder drag-and-drop,
editor autosave — and, deliberately, no telemetry.

### Fixed
- Folder drag-and-drop is now covered by real automated drops rather than
  a manual checklist. The tests previously entered through the walker's
  entry point with synthetic objects, because `webkitGetAsEntry()` returns
  null for programmatically built items — but Chrome DevTools Protocol's
  `Input.dispatchDragEvent` takes real filesystem paths and Chrome builds
  a genuine entry tree, directories included. The suite drops actual
  folders and covers the >100-file batching, clutter skipping, structure
  in the archive, mixed drops, and single-file tools staying inert.
- The prerendered home page never linked any video tool: `seo-gen`
  listed categories by hand and was not updated when Video was added.
  It now derives them from the registry, so a new category can't be
  forgotten again.
- Local release builds no longer accumulate every previous build. `dx`
  never cleans its output directory, so each build left another hashed
  copy of the ~4 MB app wasm in `assets/`; 54 had piled up, taking the
  local bundle to 248 MB. `scripts/build-web.sh` now wipes the output
  directory first (248 MB → 47 MB), which also removes the stale-bundle
  trap that once booted an old wasm in the engine Web Worker (ADR-0004)
  and is what `scripts/ui-test.sh` serves. Container images were never
  affected: `.dockerignore` excludes `target/`, so the Docker build
  always started from a clean context — verified, one wasm in both the
  old and new images.
- Mobile nav: the "All tools" and "Support us" labels wrapped to a second
  line and spilled out of the fixed-height bar on phones. Below 700px
  every nav chip is now icon-only (a grid glyph, the GitHub mark, a
  heart) with the wording kept as `title`/`aria-label`; below 380px the
  logo carries the brand alone. A UI test pins the one-row layout at
  390px so it can't come back.
- PDF editor: retyping existing text no longer changes its look. The bake
  now rewrites the clicked run through its ORIGINAL font resource —
  family, bold, italics, color and scale survive because the same
  embedded font keeps rendering it — after stripping the old glyphs
  (true removal, not a white cover). When the font can't be reused
  safely (CID/symbolic fonts, custom encodings, non-Latin replacement
  text, text inside form XObjects) the editor falls back to the previous
  white-out + Helvetica so the edit still lands.
- PDF editor: redacting a page that contains an inline image (BI..EI —
  some scanners emit these) now fails with a clear error instead of
  silently corrupting the page on rewrite; the message suggests
  flattening the PDF first. Font-preserving text edits refuse such pages
  automatically and use the cover fallback.
- CI: the ui-tests job tested a stale, cache-mangled bundle instead of
  the pushed commit (the Rust cache restores `target/dx` from the
  previous run, so the build step was skipped). The script now always
  rebuilds the bundle on CI.

### Changed
- Tool tile icons are now coloured by category — PDF rose, image sky,
  archive amber, security violet, hue-matched to the `.cat-*` tints the
  grid already used — instead of every tile sharing one plum. The colour
  is baked into each SVG (they load via `<img>`, so page CSS can't reach
  inside); the palette and its 3:1 contrast floor are documented in
  `app/assets/icons/README.md`.
- All 32 tools now show custom SVG tile icons (gold/blue on a plum tile,
  `app/assets/icons/`) on the home grid, the all-tools menu, tool page
  headers and related-tools links — the 14 PDF tiles plus the image,
  archive and security sets. A tool added without an icon still falls
  back to its emoji tile; giving it one is dropping `<slug>.svg` in the
  folder and one match arm in `app/src/icons.rs`.

### Added
- **PrivZapp speaks Indonesian, and it is built to rank** (ADR-0014).
  English keeps its canonical unprefixed URLs; Indonesian lives under
  `/id/` — 82 prerendered pages across both, each with `<html lang>`, a
  self-referencing canonical, reciprocal `hreflang` (plus `x-default`)
  and a sitemap covering every locale. Tool names, taglines, the UI, and
  the complete SEO copy including all 118 FAQ pairs are translated. A
  language switcher sits in the nav; it navigates to the mirrored URL
  rather than swapping strings in place, so crawlers can follow it.
  Adding another language is data, not code: one `Locale` variant plus
  rows in three tables — routing costs nothing extra.
- **The PDF editor survives a refresh.** It autosaves the working
  document to your browser's storage, encrypted with AES-256 and with
  the key held separately, then offers it back by name when you return —
  nothing is reopened without you clicking Restore (ADR-0013). Discard
  erases the key immediately, which makes the copy unreadable, and
  anything untouched for a day is dropped automatically. Entirely
  on-device: no server, no account.
- **`docs/MANUAL_QA.md`**: a checklist of the things headless tests
  genuinely cannot cover — real folder drags, service-worker offline and
  update cycles, stylus input, Android — with why each one resists
  automation and exactly what to click.
- **"Zero telemetry" is now a stated promise, and it is enforced.** The
  home page carries it as a badge, both the prerendered and runtime meta
  descriptions say it, and the Privacy page leads with "What we collect:
  nothing". A UI test drives the app across the home grid, a tool page,
  the editor and the Privacy page and fails if it contacts any other
  host — or hits a same-origin path that even looks like a counter — so
  the claim cannot quietly stop being true.
- **OCR — Image to Text and OCR PDF**: read the text out of pictures,
  screenshots and scanned PDFs, entirely on-device (tesseract-wasm,
  ADR-0011). English and Indonesian models; scanned PDFs are rendered
  page by page (resolution and page-range options) and download as .txt
  with page markers. Size was the design constraint: a 1.8 MB engine +
  3.9 MB fast English model — ~4-5 MB once, only when first used, only
  for the language picked — then cached for offline.
- **Extract Audio** (36th tool): pull the soundtrack out of any video as
  MP3, WAV, OGG or M4A, with a quality slider and optional time range —
  all on-device via the bundled ffmpeg.wasm.
- **Convert Video learned more formats**: MKV, MOV and AVI join MP4 and
  WebM as outputs, and animated GIFs now work as *input* — a GIF becomes
  a real (far smaller) video.
- **Folder drag-and-drop**: dropping a directory on a multi-file tool now
  walks its whole tree (subfolders included, OS clutter like `.DS_Store`
  skipped) into the file list, and **Create ZIP preserves the folder
  structure** inside the archive — relative paths are kept, traversal
  tricks (`../`, absolute, drive letters) are neutralized. Plain file
  drops behave as before; single-file tools are untouched.
- **Video tools** — a fifth category, powered by FFmpeg compiled to
  WebAssembly (ADR-0005 design, ADR-0010 implementation), fetched
  version- and sha256-pinned and served from this origin, never a CDN:
  - **Video to GIF**: two-pass palette for clean colors, frame rate,
    width and time-range controls.
  - **Trim Video**: lossless stream copy — original quality, near
    instant; the cut snaps to the keyframe before the start time.
  - **Convert Video**: MP4 (H.264 + AAC) ↔ WebM (VP8 + Opus) with a
    quality slider.
  Everything runs in a Web Worker on the user's device; the ~10 MB
  (compressed) engine downloads on first use and is cached for offline
  after that. The pure engine crates are untouched: these slugs are
  `ToolPipeline::BrowserFfmpeg` and `pz_engine::run` refuses them.
- **PDF to Image**: every page of a PDF as a PNG, JPG or WebP, at 1x-4x
  (72-288 DPI), with an optional page range — one page downloads as an
  image, several as a .zip. Pure-Rust rasterization still doesn't exist,
  so the pages are rendered by the browser's bundled PDF.js and packaged
  by the engine (ADR-0009, amending ADR-0007). `ToolMeta` gains a
  `pipeline` field so this exception is declared rather than inferred from
  the slug: `pz_engine::run` rejects browser-rendered tools outright
  instead of looking unimplemented.
- CONTRIBUTING.md: from-scratch dev setup (Rust + wasm target,
  dioxus-cli, verify/UI-test scripts) and the ground rules that keep the
  privacy promise intact. The app now links the GitHub repo from the
  footer and the Support page, and the nav carries a "Star on GitHub"
  button (a plain link on purpose — a live star-count badge would phone
  home, which the CSP forbids).
- PDF editor: true redaction (▓ tool). Drag a box over sensitive text and
  on bake the glyphs inside are REMOVED from the content stream — copy,
  search and text extraction find nothing — then the area is painted
  black. Glyph-precise where the font carries width metrics (surrounding
  text keeps its exact layout via kerning compensation); fonts we can't
  measure are redacted conservatively, dropping the whole text run —
  over-redaction is the acceptable failure mode, under-redaction never
  is. Pending boxes stay movable/deletable/undoable until baked. (The
  editor's older white-out retype was never redaction and still isn't —
  this is the tool for removing content.)
- PDF editor: "⬇ Pages as PNG" export — every page rendered at 2x and
  downloaded as a ZIP (single page: a plain .png), with pending edits
  baked in first. First step toward a standalone PDF→image tool.
- The engine now runs in a dedicated Web Worker on the web (ADR-0004):
  compressing a huge file no longer freezes the tab — the UI stays
  interactive while the wasm works. Files cross as transferable buffers
  (no copies beyond the unavoidable one), a crashed worker takes the
  blame instead of the whole tab and respawns, and browsers or dev
  setups where the worker can't boot fall back to the old inline path
  automatically. A Playwright test pins worker mode in the built bundle.

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
- Compress and Convert Image: Resolution control (10–100% of the
  original, ±10 buttons) — downscaling is the biggest size lever once
  quality has done its part.
- The live before/after preview now covers every image tool — Flip,
  Upscale, Grayscale, Blur, Watermark, Strip Metadata, Crop and Rotate
  join Compress and Convert; option changes recompute on release/commit,
  never mid-drag.
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

### Added
- Security hardening pass (ADR-0008): nginx now serves a
  Content-Security-Policy that makes "files never leave your device"
  browser-enforced — `connect-src 'self'` means the page cannot phone
  home anywhere — plus nosniff/frame-deny/referrer/permissions headers.
  Engine parsers reject decompression bombs with clear errors instead of
  exhausting tab memory (20 000 px image decode cap, 64 MP resize/upscale
  ceiling, ZIP limits enforced on actual inflated bytes — lying size
  headers are caught). A `cargo audit` RustSec scan runs as its own CI job.
- `llms.txt` is prerendered alongside sitemap/robots: an agent-facing
  site map with the tool list and the privacy model.
- PDF editor: "＋ Append PDF" tile at the bottom of the page rail —
  merge another PDF into the open one; its pages stack below the last
  page. (The operation existed buried in the inspector; now it is
  discoverable, and a UI test pins the stacking order.)

### Changed
- Lighthouse pass: the nav brand ships as a 4 KB 56-px logo cut (it was
  reusing the 76 KB 256-px favicon), the loading splash as a right-sized
  168-px asset, and `gen-icons.py` writes adaptively-filtered PNGs — every
  derived icon shrinks, losslessly. The container precompresses the
  bundle at build time and nginx serves the `.gz` siblings
  (`gzip_static`), so the multi-MB wasm goes over the wire ~3x smaller.
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
