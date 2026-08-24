# ADR-0005: Video/GIF tools via ffmpeg compiled to WASM

- **Status**: Proposed (approved direction; needs toolchain + licensing pass)
- **Date**: 2026-08-24

## Context

Video convert/trim/compress and video→GIF are top user asks. No pure-Rust
codec stack covers this. ffmpeg compiled to WebAssembly does, and the
project's ground rules already carve out ffmpeg-wasm as the one deliberate
C-dependency exception — provided it stays isolated and client-side.

## Decision (design)

1. **Isolation**: a new `pz-video` boundary that talks to an
   ffmpeg-wasm module (initially the prebuilt `@ffmpeg/ffmpeg` package via
   JS interop; later possibly a trimmed custom emscripten build). No other
   engine crate may depend on it; the pure-crate rule (ADR-0002) is
   unchanged everywhere else.
2. **Lazy loading**: the multi-megabyte ffmpeg module loads only when a
   video tool page opens, never on the home page. The service worker caches
   it after first use so video tools also work offline thereafter.
3. **Same contract**: bytes in → bytes out via a virtual FS
   (`writeFile → exec(args) → readFile`); no network, no telemetry.
4. **Native targets** use ffmpeg via sidecar/library later; web ships
   first.

## Prerequisites (why this isn't built yet)

- Emscripten toolchain and a real browser for verification — neither exists
  in the current dev container.
- **Licensing pass**: LGPL build configuration only (no `--enable-gpl`, no
  x264/x265); document the exact configure flags; AGPL app + LGPL ffmpeg
  is fine, GPL codecs would change the app's obligations.
- Bundle-size budget: ~25 MB wasm is acceptable *only* lazy-loaded +
  SW-cached; decide a cap before building.

## First step when picked up

Feature-flagged `video-to-gif` route using `@ffmpeg/ffmpeg` from npm,
manual browser test, measure: load time, memory ceiling on a 100 MB input,
output parity with native ffmpeg.
