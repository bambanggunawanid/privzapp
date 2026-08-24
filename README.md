<p align="center">
  <img src="app/assets/logo.png" width="112" alt="PrivZapp logo" />
</p>

# PrivZapp

**Every file tool. Zero uploads.**

PrivZapp is a free, privacy-first suite of file utilities — merge PDFs, convert
images, compress anything — that runs **entirely on your device**. On the web
it's Rust compiled to WebAssembly; on Windows, macOS, iOS and Android it's the
same Rust code running natively via [Dioxus](https://dioxuslabs.com). There is
no processing server, so your files can't leak: there is nowhere for them to go.

## Why

Popular "I love X" file-tool sites upload your documents to their servers.
That's a privacy tax nobody should pay for rotating a PDF. PrivZapp does the
same jobs at native speed, offline-capable, and free forever — funded by
donations, not by your data.

## Principles

1. **Files never leave the device.** All processing is local (WASM/native).
   The web app keeps working with the network unplugged.
2. **Free forever, no dark patterns.** No accounts, no ads, no premium tier,
   no data sales. Revenue is donations only.
3. **Telemetry is opt-in, anonymous, and bucketed.** Current builds send
   *nothing at all*. The future opt-in schema (see `crates/pz-telemetry`) can
   only carry enumerable, coarse facts — never filenames, contents, or ids.
4. **Security first.** `#![forbid(unsafe_code)]` across all engine crates,
   AES-256-GCM for anything PII-adjacent (`pz-crypto`), zip-slip and zip-bomb
   guards in `pz-archive`.

## Architecture

```
privzapp/
├── app/                Dioxus UI — one codebase for Web / Windows / macOS / iOS / Android
│   └── src/pages/      Home (tool grid), generic ToolPage, Privacy, Support
├── crates/
│   ├── pz-core         Shared types, tool registry, option parsing (zero deps)
│   ├── pz-engine       One dispatch API the UI calls: run(slug, files, opts)
│   ├── pz-pdf          Merge / split / rotate / compress   (lopdf, pure Rust)
│   ├── pz-img          Convert / resize / compress         (image, pure Rust)
│   ├── pz-archive      ZIP create / extract with safety guards
│   ├── pz-crypto       AES-256-GCM sealing, SHA-256, CSPRNG helpers
│   └── pz-telemetry    Opt-in anonymous telemetry (schema-constrained)
```

Everything in `crates/` is pure computation on in-memory bytes — no I/O, no
network — which is what makes the same code correct on wasm32 and native.

## Current tools (v0.1)

| Category | Tools |
|----------|-------|
| PDF      | Edit PDF (sign by hand, handwriting brush, image stamps), Merge PDF, Split PDF (ranges or burst), Rotate PDF, Compress PDF, Images to PDF, Watermark PDF, Reorder PDF, Add Page Numbers, Crop PDF, PDF to Text, Repair PDF, Protect PDF (AES-256, opens anywhere), Unlock PDF |
| Image    | Convert Image (PNG/JPG/WebP/GIF/BMP/TIFF/ICO/QOI), Resize Image, Compress Image, Crop Image, Rotate Image, Flip Image, Upscale Image (2x/4x), Grayscale Image, Blur Image, Watermark Image, Strip Metadata (EXIF), Favicon Generator (full .zip pack), Batch Rename |
| Compress | Create ZIP, Extract ZIP |
| Protect  | Encrypt File / Decrypt File (AES-256-GCM `.pzv` vaults, PBKDF2 password keys) |

A doc-sync test (`crates/pz-core/tests/docs_sync.rs`) fails the build if a
registered tool is missing from this table — the docs can't silently rot.

## Development

```bash
# One-time after cloning: pre-commit secret guard (this repo is public —
# credentials never go in code; put them in .env, template in .env.example)
git config core.hooksPath .githooks

# Everything CI runs: secret scan, fmt, clippy -D warnings, tests, wasm32 check
./scripts/verify.sh

# Playwright UI tests: drives the real wasm bundle in headless Chromium
# (needs node; first run downloads Chromium and builds the web bundle)
./scripts/ui-test.sh

# Web (dev server with hot reload; PWA bits are release-only)
cd app && dx serve --platform web

# Web production bundle + PWA (manifest, service worker, icons at site root)
./scripts/build-web.sh

# Desktop (needs webkit2gtk/gtk dev packages on Linux)
cd app && dx serve --platform desktop

# Android APK (needs JDK 17+, Android SDK 34 + NDK, aarch64 Rust target;
# see the env vars in the script) → dist/PrivZapp-<version>-android.apk
./scripts/build-android.sh

# iOS
cd app && dx build --platform ios   # (on macOS)
```

## Self-hosting

The web app is pure static files — the container just serves bytes; your
files still never leave your browser.

```bash
docker compose up -d --build     # or: podman-compose up -d --build
# → http://localhost:8080
```

Portainer: *Stacks → Add stack → Repository*, point at this repo with
compose path `docker-compose.yml`. The image builds the WASM bundle in a
Rust stage and serves it with nginx (access logs off, hashed assets cached
immutable, SPA fallback, PWA files at the root — see `deploy/nginx.conf`).

**Set your domain for SEO**: canonicals, Open Graph URLs and the sitemap
are baked in at build time from `BASE_URL`
(`docker build --build-arg BASE_URL=https://your.domain .` or
`BASE_URL=https://your.domain ./scripts/build-web.sh`). Every tool route is
prerendered as a real HTML page with structured data (see ADR-0006);
after deploying, submit `https://your.domain/sitemap.xml` in Google Search
Console and Bing Webmaster Tools.

Requires stable Rust with the `wasm32-unknown-unknown` target and
[`dioxus-cli`](https://crates.io/crates/dioxus-cli) (`cargo binstall dioxus-cli`).

## Roadmap

Details, designs and rationale live in [docs/ROADMAP.md](docs/ROADMAP.md).

- [ ] Web Workers so huge files never block the UI thread ([design](docs/adr/0004-web-worker-offloading.md))
- [x] PWA manifest + service worker → installable, explicitly offline
- [x] Drag-and-drop (folder drops still to come)
- [x] PDF: reorder pages, images → PDF, watermark
- [ ] PDF: page → image export (blocked on a wasm-safe rasterizer)
- [x] Image: batch rename, EXIF strip (privacy!), crop
- [ ] Video/GIF tools via ffmpeg compiled to WASM ([design](docs/adr/0005-ffmpeg-wasm.md))
- [x] Password-protect any file (AES-256 `.pzv` vaults via `pz-crypto`)
- [ ] Opt-in telemetry wiring + public dashboard of the little we collect
- [x] Donation integrations ([Ko-fi](https://ko-fi.com/S7F125OT18), [GitHub Sponsors](https://github.com/sponsors/bambanggunawanid))

## License

AGPL-3.0-or-later — free forever, and improvements stay free.
