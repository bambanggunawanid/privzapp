# ADR-0002: Engine crates are pure bytes-in/bytes-out

- **Status**: Accepted
- **Date**: 2026-08-24

## Context

The same processing code must run on wasm32-unknown-unknown (no threads, no
filesystem, no reliable clock; `std::time::Instant` panics) and on native
desktop/mobile. Platform-conditional engine code would fork behavior and
multiply the test matrix.

## Decision

Every `crates/pz-*` engine crate is pure computation:

- Signature shape: `(&[u8], options) -> Result<Vec<u8> | OutputFile, Error>`.
- No I/O, no network, no threads, no time, no global state.
- `#![forbid(unsafe_code)]`.
- Dependencies must compile for wasm32 with the features we use:
  `image`/`lopdf`/`zip` are pinned with `default-features = false`
  (defaults pull rayon/lzma/zstd, which either don't build or bloat wasm).
- All platform specifics (file pickers, downloads, save paths) live in
  `app/`, behind one dispatch call: `pz_engine::run`.

## Consequences

- One test suite, run natively, is authoritative for every platform;
  `cargo check --target wasm32-unknown-unknown` proves portability.
- Unit tests use in-memory fixtures (`sample_pdf`/`sample_png` helpers) —
  no test data files, no tmp dirs.
- Long operations block their thread; responsiveness must come from the
  app layer (ADR-0004), never from threads inside engine crates.
- The ffmpeg exception (ADR-0005) must be isolated so this rule survives
  it.
