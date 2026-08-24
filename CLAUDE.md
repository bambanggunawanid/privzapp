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
- **Telemetry schema is a privacy contract.** Adding any field to
  `pz_telemetry::Event` needs explicit user sign-off. No free-form strings.
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
- `crates/pz-telemetry` — opt-in event queue; not wired into the UI yet
  (v1 builds send nothing).
- `docs/` — ARCHITECTURE, ROADMAP, CONTINUOUS_DOCUMENTATION (the "docs land
  with the code" contract), `adr/` decision records.

## Commands

- Everything CI runs: `./scripts/verify.sh` (fmt --check, clippy -D
  warnings, workspace tests, wasm32 check). Run it before every commit.
- Wasm check alone (the important one):
  `cargo check -p privzapp --target wasm32-unknown-unknown`
- Dev server: `cd app && dx serve --platform web`
- Release bundle incl. PWA files: `./scripts/build-web.sh`
  (output: `target/dx/privzapp/release/web/public/`)
- Icons/branding: `python3 scripts/gen-icons.py` regenerates every icon
  (PWA set, favicon, in-app logo) from `app/brand/logo-master.png` —
  never hand-edit the derived PNGs.
- Container: `docker compose up -d --build` (also Portainer/podman-compatible).
  Runtime image is nginx serving the static bundle — keep it that way; a
  server that processes files would violate the core promise.
- SEO: per-tool copy (titles/descriptions/FAQs) lives in
  `crates/pz-core/src/seo.rs` and is test-enforced (every tool needs an
  entry; snippet length limits). `tools/seo-gen` prerenders all routes
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
- This container is headless: service-worker, drag-drop and download
  behavior can't be verified here — flag what needs a manual browser pass
  instead of claiming it works.

## Gotchas

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
- The PDF editor (`app/src/pages/editor.rs` + `app/assets/editor.js`) is
  the one sanctioned JS-library exception: bundled PDF.js renders pages,
  Rust does ALL mutation (`pz_pdf::annotate`). Don't add other JS libs or
  fetch PDF.js from a CDN (ADR-0007). Its canvas flow can't be tested in
  this headless container — flag for a manual browser pass when touched.
