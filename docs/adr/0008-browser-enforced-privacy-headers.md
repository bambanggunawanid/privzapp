# ADR-0008: Browser-enforced privacy via CSP + hardened input handling

- **Status**: Accepted
- **Date**: 2026-08-24

## Context

"Your files never leave your device" was, until now, a promise backed by
code review: the engine crates have no network access by construction
(ADR-0001/0002), but nothing stopped a future bug, a compromised
dependency, or injected markup from calling `fetch()` in the app layer.
Separately, every engine crate parses fully attacker-controlled bytes
(PDFs, images, archives) inside the user's tab, where an unchecked
allocation aborts the wasm instance — a denial of service against the
user's own session.

The historical blocker for a CSP — "this container can't be
browser-tested headlessly" — is gone: the Playwright suite drives the
real bundle in headless Chromium, so header changes are verifiable.

## Decision

1. **Content-Security-Policy at the web server** (`deploy/
   security-headers.conf`, included by `deploy/nginx.conf`): the pivotal
   directive is `connect-src 'self' blob: data:` — the browser itself
   refuses any request to another origin, making exfiltration impossible
   even if application code were compromised. `object-src 'none'`,
   `frame-ancestors 'none'`, `base-uri 'self'` close the classic
   injection amplifiers. Accepted compromises, revisit later:
   - `script-src 'unsafe-inline'`: dx emits an inline wasm bootstrap and
     the prerender a tiny splash-fallback script; both vary per build.
     Tightening to hashes needs build-time header generation.
   - `script-src 'unsafe-eval'`: Dioxus's `document::eval` bridge is how
     the entire app talks to JS (previews, downloads, editor channel);
     the wasm aborts on the first interop call without it. Dropping this
     means replacing the framework's interop mechanism — not worth it
     while `connect-src` already closes the exfiltration path.
   - No COEP/COOP isolation requirement yet; COOP `same-origin` is set,
     COEP waits for wasm-thread work (ADR-0004).
2. **Companion headers**: `X-Content-Type-Options: nosniff`,
   `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, minimal
   `Permissions-Policy`.
3. **Engine input ceilings** — parsers reject hostile inputs with a clear
   `PzError` instead of exhausting tab memory:
   - pz-img: decode capped at 20 000 px per side + the `image` crate's
     512 MiB allocation limit; resize/upscale outputs capped at 64 MP.
   - pz-archive: extraction ceilings (512 MiB/file, 1 GiB total) are
     enforced on the *actual* inflated bytes, not just the declared
     header sizes, which a crafted archive can understate.
4. **Supply-chain gate**: `cargo audit` (RustSec advisory DB) runs as its
   own CI job and fails on known-vulnerable dependencies.

## Consequences

- The privacy claim is now enforced by the user's browser, not just our
  discipline; the Playwright smoke must pass against the built container
  whenever headers change.
- Header edits carry breakage risk for new asset types (e.g. a future
  worker or font) — the CSP must be updated deliberately, which is the
  point.
- Self-hosters behind their own TLS proxy still own HSTS at the proxy.
