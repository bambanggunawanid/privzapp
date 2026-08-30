# PrivZapp — agent notes

Privacy-first, fully client-side file-tools suite (think iLovePDF, but nothing
ever leaves the device). Full-stack Rust: Dioxus 0.7 UI + pure-Rust engine
crates that compile to both native and wasm32.

## Ground rules

- **No server-side processing, ever.** Any feature that needs bytes to leave
  the device is rejected by design. No network calls in engine crates.
- **Engine crates are pure**: bytes in → bytes out, `#![forbid(unsafe_code)]`,
  no I/O, no threads (wasm32-unknown-unknown has none), no `std::time::Instant`
  (panics on wasm). Keep them compiling for wasm32 at all times.
- **Dependencies must be wasm-safe**: no rayon, no C build deps unless they
  compile to wasm (future ffmpeg-wasm is a deliberate exception, isolated).
  `image` and `lopdf` are used with `default-features = false` for exactly
  this reason — don't re-enable defaults.
- **There is no telemetry, and adding any is a product decision.** The app
  makes no network requests of its own; `tests/ui/no-phone-home.spec.js`
  fails if it ever does. The `pz-telemetry` crate was deleted on purpose
  (a dormant one still invites "so what do you collect?" from every new
  contributor). Do not re-add telemetry, analytics or a beacon without
  explicit owner sign-off.
  Analytics were built and then deliberately removed (ADR-0012, Reverted):
  the app sends NO requests of its own, and there is no analytics code in
  the repo at all — the sidecar that briefly existed was deleted too, so
  nothing here can prompt "do you track?" from a contributor or a secret
  scanner. Re-introducing any beacon, counter or third-party script needs
  explicit owner sign-off plus a Privacy page update. ADR-0012 has the
  reasoning and the traps if it ever comes back.
- **UI copy promises** ("nothing is uploaded", "works offline") are product
  claims — don't add code that makes them false.

## Layout

- `app/` — Dioxus app. Platform features: `web` (default), `desktop`, `mobile`.
  Routes in `app/src/main.rs`; the generic tool page is
  `app/src/pages/tool.rs`; per-platform download/save in `app/src/save.rs`;
  `app/pwa/` holds manifest/service-worker/icons (served from the site root,
  copied there by `scripts/build-web.sh` — release builds only).
- `crates/pz-core` — tool registry (`TOOLS`), shared types, option parsing.
  Adding a tool: follow the `add-tool` skill (`.claude/skills/add-tool/`).
  Short form: `ToolMeta` here + engine impl + match arm in
  `crates/pz-engine/src/lib.rs::run` + README table + CHANGELOG.
- `crates/pz-{pdf,img,archive}` — the actual operations, unit-tested with
  in-memory fixtures (see `sample_pdf` / `sample_png` helpers).
- `crates/pz-crypto` — AES-256-GCM `seal`/`open`, password vaults
  (`seal_with_password`, `.pzv` format — ADR-0003), `sha256_hex`, CSPRNG.
- `docs/` — ARCHITECTURE, ROADMAP, CONTINUOUS_DOCUMENTATION (the "docs land
  with the code" contract), `adr/` decision records.

## Commands

- Everything CI runs: `./scripts/verify.sh` (secret scan, fmt --check,
  clippy -D warnings, workspace tests, wasm32 check). Run it before every
  commit.
- UI tests: `./scripts/ui-test.sh` — Playwright drives the real wasm
  bundle in headless Chromium (works in this container). Run it after
  touching the editor, tool pages, or nav; every owner-reported UI
  regression gets a test in `tests/ui/` so it can't come back. Own CI
  job. App code calls the engine through `crate::engine::run` (async —
  it dispatches to a Web Worker in the built bundle, inline in `dx
  serve`; ADR-0004), never `pz_engine::run` directly. Still never
  trigger engine runs from `oninput` on sliders — use `onchange`
  (inline fallback freezes mid-drag, and worker mode would churn).
- Secrets: the repo is public. Never hardcode credentials — they go in
  `.env` (gitignored, template `.env.example`). `.githooks/pre-commit`
  (installed via `git config core.hooksPath .githooks`) and the verify
  script both run `scripts/check-secrets.py`; avoid scanner-bait patterns
  like `password: "..."` (a quoted literal) even in tests (build test
  passphrases with `join()` — see pz-engine). In shell, never combine a
  credential-shaped variable name with the `:?` error-if-unset expansion:
  GitGuardian reads the message after `?` as an assigned value and opens
  an incident on a line containing no secret (this happened — commit
  60bf108). Use an explicit `[ -z "$VAR" ]` test instead. check-secrets.py
  blocks that shape via BAIT_PATTERNS, which bypasses the placeholder
  filter on purpose — and yes, it will block your documentation too if you
  spell the pattern out. Commits carry NO Co-Authored-By/AI trailer.
- Wasm check alone (the important one):
  `cargo check -p privzapp --target wasm32-unknown-unknown`
- Dev server: `cd app && dx serve --platform web`
- `scripts/build-web.sh` wipes `target/dx/privzapp/release/web` before
  building on purpose: dx leaves a hashed copy of the ~4 MB app wasm per
  build and never prunes, which took the LOCAL bundle to 248 MB / 54
  copies and is the stale-bundle hazard behind the ADR-0004 worker
  gotcha (ui-test.sh serves that bundle). Don't remove it. Container
  images were never affected — .dockerignore excludes target/, so the
  image build always starts clean.
- Release bundle incl. PWA files: `./scripts/build-web.sh`
  (output: `target/dx/privzapp/release/web/public/`)
- Icons/branding: `python3 scripts/gen-icons.py` regenerates every icon
  (PWA set, favicon, in-app logo) from `app/brand/logo-master.png` —
  never hand-edit the derived PNGs.
- Container: `docker compose up -d --build` (also Portainer/podman-compatible).
  Runtime image is nginx serving the static bundle — keep it that way; a
  server that processes files would violate the core promise.
- i18n (ADR-0014): `crates/pz-core/src/i18n.rs` + `i18n_id.rs` +
  `i18n_seo_id.rs`. UI strings are keyed by their ENGLISH text
  (`tr("...")` in the app, `i18n::t(loc, "...")` elsewhere), so editing an
  English string silently orphans its translation — update both. English
  is unprefixed and `Locale::from_str` must NEVER accept a route name
  ("tool"/"privacy"/"support") or every tool page 404s; tests pin that.
  Adding a language = a `Locale` variant + rows in the three tables.
- SEO: per-tool copy (titles/descriptions/FAQs) lives in
  `crates/pz-core/src/seo.rs` and is test-enforced (every tool needs an
  entry; snippet length limits, applied to every locale). `tools/seo-gen` prerenders all routes
  during `scripts/build-web.sh`; `BASE_URL` env sets canonical origin.
  Adding a tool now also means writing its `ToolSeo` entry (ADR-0006).
- Desktop build needs webkit2gtk + gtk3 dev packages on Linux (not installed
  in this container — wasm/web is the verified path here).

## Working agreement (agents)

- Skills in `.claude/skills/` (add-tool, verify, release, continuous-docs)
  are the playbooks for the common tasks — follow them.
- Docs are part of the diff: see `docs/CONTINUOUS_DOCUMENTATION.md`. A
  docs-sync test fails the build if the README tools table or CHANGELOG
  lags the registry.
- Run the `privacy-reviewer` agent on telemetry/dependency/UI-copy changes
  and the `wasm-guard` agent on dependency/engine changes
  (`.claude/agents/`).
- Manual-only checks live in `docs/MANUAL_QA.md` — add to it whenever you
  ship something headless can't reach, and tell the owner which section
  to run.
- The editor autosaves its working document, encrypted, to IndexedDB
  (ADR-0013: `app/src/autosave.rs` + `app/assets/autosave.js`; key in
  localStorage so Discard is a crypto-shred). Any test touching the
  `pz-editor` database MUST create the object store in `onupgradeneeded`
  exactly like autosave.js does — a probe that opens it bare can win the
  race, create an empty v1 database and silently break autosave.
- This container is headless: service-worker and real drag-gesture
  behavior can't be verified here — flag what needs a manual browser pass
  instead of claiming it works. (Folder-drop logic IS testable: the specs
  call `pzIngestEntries` with duck-typed entry trees; only the physical
  gesture isn't. `readEntries` returns ≤100 entries per call — the walker
  must loop, and the test pins it.)

## Gotchas

- Prod serves a strict CSP from `deploy/security-headers.conf` (ADR-0008).
  Anything that loads a new kind of resource (fonts, workers, media)
  must extend the policy there, or it works in `dx serve` and breaks
  only in the container.

- `.cargo/config.toml` sets `getrandom_backend="wasm_js"` for wasm builds
  (getrandom 0.3 via lopdf). pz-crypto separately enables getrandom 0.2 `js`.
  Both are needed; don't "clean up" either.
- `zip` is pinned with `default-features = false, features = ["deflate"]` —
  default features pull lzma/zstd/bzip2 and break/bloat wasm.
- Compress tools must never return bytes larger than the input (both pz-img
  and pz-pdf enforce this — keep the invariant for new compressors).
- lopdf `Object::type_name()` returns `&[u8]` (match on `b"Catalog"` etc.);
  `get_page_content()` returns plain `Vec<u8>`, not a `Result`.
- pz-crypto tests take ~14s: PBKDF2 at 600k rounds is deliberate (ADR-0003).
  Never lower the rounds to make tests faster.
- `pz-pdf` uses `image` as a **dev-dependency only** (JPEG test fixtures);
  don't promote it — runtime image work belongs in pz-img, composed via
  pz-engine (see the images-to-pdf arm).
- The `OptionKind` match in `app/src/pages/tool.rs` is exhaustive on
  purpose: adding a variant breaks the build until its widget exists.
- PWA files must sit at the site root (SW scope rule) — that's why they're
  copied by `scripts/build-web.sh` instead of going through `asset!()`
  hashing. In `dx serve` dev they 404 and degrade silently; that's expected.
- Rasterizing a PDF page is the one job the engine can't do: no pure-Rust
  renderer exists. `pdf-to-images` is therefore
  `ToolPipeline::BrowserRender` — `app/src/render.rs` +
  `app/assets/pdfrender.js` render via the bundled PDF.js and the engine
  only zips the result (ADR-0009). `pz_engine::run` rejects such slugs on
  purpose; don't "fix" that by adding an arm. Every other tool stays
  `ToolPipeline::Engine`.
- Video tools run on ffmpeg.wasm (ADR-0010): `ToolPipeline::BrowserFfmpeg`,
  `app/src/video.rs` + `app/assets/videotool.js`. The ~31 MB core is NOT
  in git — `scripts/fetch-ffmpeg.sh` fetches it sha256-pinned into
  `app/ffmpeg/` (gitignored) and `scripts/build-web.sh` copies it to the
  bundle root UNhashed (the wrapper resolves its worker chunk relative to
  its own URL). In `dx serve` dev, /ffmpeg/ 404s like the PWA files —
  exercise video tools through the built bundle. Single-threaded core on
  purpose (the mt build needs cross-origin isolation, rejected in
  ADR-0004). Never "upgrade" it to a CDN load, and never add the COOP/COEP
  headers just to get threads.
- OCR (ADR-0011) mirrors the ffmpeg setup exactly: `ToolPipeline::BrowserOcr`,
  `app/src/ocr.rs` + `app/assets/ocrtool.js`, runtime fetched sha256-pinned
  by `scripts/fetch-ocr.sh` into `app/ocr/` (gitignored), served unhashed
  from `/ocr/`, 404s in `dx serve`. Language codes are an ALLOWLIST
  (`safe_lang`) because they're spliced into a URL; adding a language =
  fetch-script pin + allowlist arm + widget option. OCR PDF composes the
  ADR-0009 renderer, so its page mounts pdfrender.js too.
- The PDF editor (`app/src/pages/editor.rs` + `app/assets/editor.js`) is
  the one sanctioned JS-library exception: bundled PDF.js renders pages,
  Rust does ALL mutation (`pz_pdf::annotate`). Don't add other JS libs or
  fetch PDF.js from a CDN (ADR-0007). Its canvas flow can't be tested in
  this headless container — flag for a manual browser pass when touched.
