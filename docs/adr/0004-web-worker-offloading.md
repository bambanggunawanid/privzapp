# ADR-0004: Web Worker offloading for the engine (web target)

- **Status**: Accepted (implemented 2026-08-25)
- **Date**: 2026-08-24

## Context

`pz_engine::run` is synchronous and ran on the main thread in the web
build. Small files finish in milliseconds, but a 200 MB zip or a photo-heavy
PDF can freeze the tab for seconds — the "Working…" state can't even paint
reliably. Engine crates are forbidden from threading (ADR-0002), so
responsiveness must come from the app layer.

## Decision (as implemented)

Run the engine in a **dedicated module Web Worker** with message-passing,
not wasm threads. The original design called for a second wasm binary;
what shipped is simpler: **the worker is the same dx-built module loaded
a second time.**

1. `seo-gen` writes a stable shim, `/pz-worker.js`, containing one line:
   an import of the hashed entry module **that index.html references**.
   Two hard-won constraints live here:
   - The worker cannot be created from a `blob:` URL — the wasm-bindgen
     glue fetches its `.wasm` by relative path, and `blob:` is not a
     valid base URL, so init dies silently.
   - The entry must be parsed out of index.html, not picked from the
     assets dir: stale hashed bundles from previous builds accumulate
     there, and a stale entry boots a stale wasm whose `main()` predates
     the worker guard — it launches the UI in the worker and aborts with
     a bare `unreachable`.
2. `main()` detects the worker context (no `Window`) and registers the
   engine message handler instead of launching Dioxus
   (`app/src/engine.rs::maybe_worker_main`). A worker-side panic hook
   forwards panic text to the client, because a worker abort is
   otherwise an anonymous `RuntimeError: unreachable`.
3. All app engine calls go through **`crate::engine::run` (async)** —
   never `pz_engine::run` directly. File bytes cross the boundary as
   **transferable `ArrayBuffer`s**, so peak memory stays ~1× file size;
   options cross as a small JSON mirror struct (engine crates stay
   serde-free).
4. Fallback: if the worker can't boot (`dx serve` has no shim → 404;
   the boot watchdog fires after 20 s; two consecutive worker crashes),
   jobs run inline on the main thread exactly as before. A single crash
   respawns the worker — isolation is a feature: a hostile file kills
   the worker, not the tab.
5. The active mode is published as `window.pzEngineMode`
   ("worker"/"inline") and pinned by a Playwright test.

Explicitly rejected: wasm threads / `rayon-wasm` (need COOP/COEP
cross-origin isolation headers, restrict hosting, and violate ADR-0002
inside engines); `gloo-worker` codegen (extra dependency for what is one
message shape); a second wasm binary (needs a version-matched
`wasm-bindgen-cli` in every build environment for ~40 lines of saved
guard code).

## Consequences

- The tab stays responsive during any engine run; the CSP smoke test
  (`tests/ui/csp-smoke.mjs`) proves the worker boots under the deployed
  headers (`worker-src 'self'`; the shim is served with the same CSP).
- The whole app module loads twice (~1.6 MB compressed, served from the
  service-worker cache after first load) — the cost of not maintaining a
  second binary.
- `pz_engine::edit_pdf` (editor annotation baking) still runs inline —
  its typed edit structs don't cross the wire yet. Offload it if baking
  ever shows up as jank.
- Progress messages (coarse, per-file) remain future work; the protocol
  has room for them.
