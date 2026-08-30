# Architecture

One sentence: **a Dioxus UI over a pure-Rust engine that turns bytes into
bytes, compiled to wasm32 on the web and to native code everywhere else, so
files never leave the device.**

## Crate graph

```
                    ┌─────────────────────────────┐
                    │  app (Dioxus: web/desktop/  │
                    │  mobile) — UI, file pickers,│
                    │  downloads, PWA shell       │
                    └──────────────┬──────────────┘
                                   │ pz_engine::run(slug, files, opts)
                    ┌──────────────▼──────────────┐
                    │          pz-engine          │  one dispatch point
                    └──┬───────┬───────┬───────┬──┘
                       │       │       │       │
                  ┌────▼──┐ ┌──▼───┐ ┌─▼─────┐ ┌▼─────────┐
                  │pz-pdf │ │pz-img│ │pz-    │ │pz-crypto │
                  │(lopdf)│ │(image│ │archive│ │(aes-gcm, │
                  │       │ │crate)│ │(zip)  │ │ pbkdf2)  │
                  └───┬───┘ └──┬───┘ └─┬─────┘ └┬─────────┘
                      └────────┴───┬───┴────────┘
                              ┌────▼────┐
                              │ pz-core │  types, registry, parsing
                              └─────────┘

```

## Invariants (why it's built this way)

1. **Engine crates are pure**: bytes in → bytes out. No I/O, no network, no
   threads, no clocks. This single constraint is what makes the same code
   correct on wasm32-unknown-unknown (which has none of those) and native.
2. **One dispatch point** (`pz-engine::run`): the UI knows tool *slugs*, not
   implementations. Adding a tool never touches page code — the generic
   `ToolPage` renders whatever `OptionKind`s the registry declares.
3. **The registry is the single source of truth** (`pz-core::TOOLS`): home
   grid, tool pages, engine dispatch and the README tools table (enforced by
   `crates/pz-core/tests/docs_sync.rs`) all derive from it.
4. **`#![forbid(unsafe_code)]`** in every engine crate; dependencies chosen
   for wasm-safety (`image`/`lopdf`/`zip` with default features off).

## Data flow for one tool run (web)

1. User picks/drops files → browser File API → bytes copied into WASM memory
   (`InputFile { name, bytes }`).
2. `ToolPage` collects options into `ToolOptions` and calls
   `pz_engine::run` — synchronous pure computation inside the WASM module.
3. Results (`OutputFile { name, mime, bytes }`) become Blob object-URLs and
   download via a synthetic anchor click (`app/src/save.rs`). On native, the
   same struct is written to `~/Downloads/PrivZapp` instead.
4. Nothing is retained: signals are dropped when the page unmounts; there is
   no storage layer and no network layer.

## Platform split

- `app/src/save.rs` — the only file with per-platform code (cfg on
  `target_arch = "wasm32"`).
- `app/pwa/` — web-only shell files (manifest, service worker, icons),
  copied to the site root by `scripts/build-web.sh` because a service
  worker's scope is capped at its own URL path.
- Feature flags on the `privzapp` crate: `web` (default) / `desktop` /
  `mobile` select the Dioxus renderer.

## Security-relevant corners

- `pz-archive`: zip-slip (path sanitization) and zip-bomb (size-ratio)
  guards on extraction.
- `pz-crypto`: AES-256-GCM only; PBKDF2-HMAC-SHA256 at 600k rounds for
  password vaults (see ADR-0003); CSPRNG via `getrandom` (OS or browser).
- Compress tools never return more bytes than they were given (invariant
  tested in pz-img and pz-pdf).
- There is no telemetry crate and no analytics of any kind: the app makes
  no network requests of its own, enforced by
  `tests/ui/no-phone-home.spec.js`. The dormant `pz-telemetry` crate was
  deleted on 2026-08-30 rather than left switched off (see ADR-0012).
