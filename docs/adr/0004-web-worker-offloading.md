# ADR-0004: Web Worker offloading for the engine (web target)

- **Status**: Proposed (approved direction; blocked on browser verification)
- **Date**: 2026-08-24

## Context

`pz_engine::run` is synchronous and runs on the main thread in the web
build. Small files finish in milliseconds, but a 200 MB zip or a photo-heavy
PDF can freeze the tab for seconds — the "Working…" state can't even paint
reliably. Engine crates are forbidden from threading (ADR-0002), so
responsiveness must come from the app layer.

## Decision (design)

Run the engine in a **dedicated Web Worker** with message-passing, not wasm
threads:

1. A second small binary target (`app/src/bin/worker.rs` or a `pz-worker`
   crate) compiled to wasm32 with `wasm-bindgen`, exposing one entry point
   that receives `{slug, files, opts}` and posts back
   `{outputs} | {error}`.
2. File bytes cross the boundary as **transferable `ArrayBuffer`s** (zero
   copy), so peak memory stays ~1× file size.
3. The UI keeps the current synchronous path as fallback (feature
   detection): desktop/mobile builds and browsers without module-worker
   support just run inline as today.
4. Progress: coarse per-file progress messages first; intra-operation
   progress only if an engine op ever exposes it (registry-driven flag).

Explicitly rejected: wasm threads / `rayon-wasm` (need COOP/COEP
cross-origin isolation headers, restrict hosting, and violate ADR-0002
inside engines); `gloo-worker` codegen (extra dependency for what is one
message shape).

## Consequences

- dx must build two wasm artifacts; the worker script must be served
  unhashed or referenced via the asset manifest — same class of problem
  already solved for the service worker in `scripts/build-web.sh`.
- The engine API stays untouched; only `app/` changes.
- **Must not land without a manual browser smoke test** (headless container
  CI can't validate worker wiring): `dx serve`, feed a >100 MB file,
  confirm the UI stays interactive and output bytes match the inline path.
