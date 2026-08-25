# Contributing to PrivZapp

Thanks for helping! PrivZapp is a privacy-first, fully client-side file-tools
suite: Dioxus 0.7 UI on top of pure-Rust engine crates that compile to both
native targets and `wasm32`. The repo lives at
**<https://github.com/bambanggunawanid/privzapp>** — issues and pull requests
are welcome there.

## The one rule that shapes everything

**Files never leave the device.** There is no processing server and there will
never be one. Any feature that needs bytes to leave the browser/app is
rejected by design; engine crates make no network calls, and the UI's promises
("nothing is uploaded", "works offline") are product claims your code must
keep true.

## Setup from scratch

Everything below works on a fresh Linux/macOS machine (Windows: use WSL).

```bash
# 1. Rust (stable) + the wasm target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
rustup component add rustfmt clippy

# 2. The Dioxus CLI (builds/serves the web app)
cargo install cargo-binstall        # prebuilt binaries, much faster
cargo binstall -y dioxus-cli@0.7.10

# 3. Node 18+ and Python 3 (UI tests / dev static server) — from your
#    package manager, e.g. apt install nodejs npm python3

# 4. Clone and wire up the repo hooks
git clone https://github.com/bambanggunawanid/privzapp.git
cd privzapp
git config core.hooksPath .githooks   # pre-commit secret scan (repo is public)
cp .env.example .env                  # local config; .env is gitignored

# 5. Run it
cd app && dx serve --platform web     # http://127.0.0.1:8080
```

Useful builds:

```bash
./scripts/build-web.sh    # release bundle + PWA files + SEO prerender
                          # → target/dx/privzapp/release/web/public/
docker compose up -d --build   # the self-hosted container (nginx, static)
```

The desktop build additionally needs `webkit2gtk` + `gtk3` dev packages on
Linux; Android needs a JDK + Android SDK/NDK (see `scripts/build-android.sh`).
Neither is required for web work.

## Verifying your change

```bash
./scripts/verify.sh       # what CI runs: secret scan, fmt --check,
                          # clippy -D warnings, workspace tests, wasm32 check
./scripts/ui-test.sh      # Playwright drives the real wasm bundle in
                          # headless Chromium (builds the bundle first run;
                          # FRESH_BUNDLE=1 forces a rebuild)
```

Run `verify.sh` before every commit. Run `ui-test.sh` when you touch the
editor, tool pages, or navigation — every UI regression that gets reported
grows a test in `tests/ui/` so it can't come back.

## Project layout in one minute

- `app/` — Dioxus app (routes in `app/src/main.rs`, generic tool page in
  `app/src/pages/tool.rs`, PDF editor in `app/src/pages/editor.rs`).
  App code calls the engine through `crate::engine::run` (async; Web Worker
  in release bundles) — never `pz_engine::run` directly.
- `crates/pz-core` — tool registry + shared types; per-tool SEO copy in
  `seo.rs` (test-enforced).
- `crates/pz-{pdf,img,archive,crypto}` — the actual operations. Pure: bytes
  in → bytes out, `#![forbid(unsafe_code)]`, no I/O, no threads, no
  `std::time::Instant` — they must always compile for wasm32.
- `crates/pz-engine` — dispatches a tool slug + options to the right crate.
- `docs/` — architecture, roadmap, ADRs, and the continuous-documentation
  contract (`docs/CONTINUOUS_DOCUMENTATION.md`).

To add a tool: `ToolMeta` in `pz-core` + engine implementation + match arm in
`pz-engine::run` + `ToolSeo` entry + README table + CHANGELOG. A docs-sync
test fails the build if the README tools table or CHANGELOG lags the registry
— docs land in the same commit as the code.

## Ground rules for changes

- **Engine crates stay pure and wasm-safe.** No network, no filesystem, no
  threads, no `unsafe`. New dependencies must compile to
  `wasm32-unknown-unknown` (that's why `image`, `lopdf` and `zip` are pinned
  with `default-features = false` — don't re-enable defaults).
- **No secrets in the repo.** The repo is public; credentials go in `.env`.
  The pre-commit hook and CI both run `scripts/check-secrets.py`.
- **Compress tools never return more bytes than they got.** Keep that
  invariant for anything new that claims to shrink files.
- **Telemetry schema changes need explicit owner sign-off** — the event
  schema is a privacy contract (enumerable values only, no free-form
  strings). Current builds send nothing at all.
- **CSP:** production serves a strict Content-Security-Policy from
  `deploy/security-headers.conf`. If your change loads a new kind of
  resource (fonts, workers, media), extend the policy there or it will work
  in `dx serve` and break only in the container.
- **JS libraries:** the bundled PDF.js in the editor is the single
  sanctioned exception (display only — all mutation is Rust, ADR-0007).
  Don't add others, and never fetch from a CDN.

## Pull requests

- Keep `./scripts/verify.sh` green; add/adjust tests with the behavior they
  pin.
- Update `CHANGELOG.md` (and the README table for new tools) in the same
  commit.
- Plain commits from you as the author, please — no generated attribution
  trailers.
