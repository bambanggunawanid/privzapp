# ADR-0001: All processing is client-side, forever

- **Status**: Accepted
- **Date**: 2026-08-24

## Context

File-tool sites conventionally upload user documents to servers for
processing. That model leaks the most sensitive data users own (contracts,
IDs, medical scans) to a third party, requires trust in retention promises,
and costs money that pushes products toward ads, accounts and premium tiers.

## Decision

PrivZapp performs **all** file processing on the user's device: Rust
compiled to WebAssembly in the browser, the same Rust compiled natively on
desktop/mobile. There is no processing backend at all. Any feature that
requires bytes to leave the device is rejected at design time — the answer
is "no", not "opt-in".

Revenue is donations only. No accounts, no ads, no data sales, no premium
tier.

## Consequences

- The web app must stay offline-capable; UI copy ("nothing is uploaded",
  "works offline") is a product claim that code must never falsify.
- Engine crates must compile for wasm32-unknown-unknown at all times, which
  forbids threads, I/O, clocks and C dependencies that don't compile to
  wasm (see ADR-0002).
- Heavy capabilities (video transcoding, PDF rasterization) must ship as
  client-side wasm modules or not at all (ADR-0005).
- Hosting is static-file-only and effectively free; there is no server to
  breach, subpoena or maintain.
