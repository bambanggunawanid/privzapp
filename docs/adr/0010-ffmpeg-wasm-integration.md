# ADR-0010: ffmpeg.wasm integration for the video tools

- **Status**: Accepted
- **Date**: 2026-08-30
- **Implements**: [ADR-0005](0005-ffmpeg-wasm.md) (the approved design)

## Context

ADR-0005 approved video/GIF tools via FFmpeg compiled to WASM but stalled
on three "needs": an emscripten toolchain, a bundle-size budget, and a
licensing pass. The toolchain need evaporated by not building a custom
core: the prebuilt `@ffmpeg/core` npm artifact is a full FFmpeg compiled
to a single wasm, and the `@ffmpeg/ffmpeg` wrapper runs it in a Web
Worker. What remained was deciding how to bundle, serve, license and
constrain it.

## Decision

Three tools — **Video to GIF**, **Trim Video**, **Convert Video** — in a
new Video category, all `ToolPipeline::BrowserFfmpeg`: the app-side
module (`app/src/video.rs` + `app/assets/videotool.js`) does the work and
`pz_engine::run` refuses the slugs, exactly like the ADR-0009 pattern.
The engine crates stay pure; the C exception never leaks into them
(ADR-0002 holds).

1. **Fetched pinned, served same-origin, never a CDN.** The ~31 MB core
   doesn't belong in git; `scripts/fetch-ffmpeg.sh` downloads exact
   versions from the npm registry, verifies sha256 pins, and stages
   `app/ffmpeg/` (gitignored). `scripts/build-web.sh` copies it to the
   bundle root — the same treatment as the PWA files, and for the same
   class of reason: the wrapper resolves its worker chunk relative to its
   own URL, which dx asset hashing would break.
2. **Single-threaded core, deliberately.** The multithreaded build needs
   SharedArrayBuffer and therefore COOP/COEP cross-origin isolation,
   which ADR-0004 already rejected. Slower, but it runs everywhere under
   the existing CSP: the worker and `importScripts` are same-origin
   (`script-src 'self'`, `worker-src 'self'`), wasm compiles under
   `wasm-unsafe-eval`. **No CSP change was needed** — the post-deploy
   smoke (`tests/ui/csp-smoke.mjs`) proves it against the real container.
3. **Lazy by construction.** Nothing loads until a video tool page runs;
   the first run fetches the core (~10 MB gzipped over the wire) and the
   service worker's runtime cache keeps it for offline use afterwards.
   The SEO FAQ copy says this out loud rather than hiding it.
4. **UMD builds, not ESM.** The wrapper's ESM artifact bakes a broken
   `file://` base URL into its worker path; the UMD build resolves
   correctly and stays inside the CSP.
5. **Arguments are built from typed values only.** `app/src/video.rs`
   assembles every ffmpeg invocation from parsed numbers, validated
   timecodes (`pz_core::parse_timecode`) and a sanitized extension —
   user strings are never spliced into commands. Trim uses stream copy
   (`-c copy`): lossless and near-instant, snapping to the keyframe
   before the start time (documented in the tool FAQ). GIF uses the
   two-pass palettegen/paletteuse recipe.

## Licensing

`@ffmpeg/ffmpeg` (wrapper) is MIT. `@ffmpeg/core` is **GPL-2.0-or-later**
because it links x264 among others. PrivZapp is AGPL-3.0-or-later, and
GPLv3 §13 / AGPLv3 §13 explicitly permit conveying the combination — so
ADR-0005's "no GPL codecs" caution, written with LGPL-purity in mind,
does not bind an AGPL project. `app/ffmpeg/LICENSE.md` ships in the
bundle. Separate from copyright: H.264 *encoding* (Convert Video's MP4
path) touches the AVC patent pool like every FFmpeg distribution does;
WebM/VP8 and GIF are royalty-free. If that risk ever feels wrong, the
mp4 arm is one match branch to remove.

## Consequences

- The Docker image grows ~11 MB (gzipped core; nginx serves the
  precompressed sibling). Repo size is unchanged — the artifact is
  fetched, pinned, at build time.
- `dx serve` dev mode 404s `/ffmpeg/` (same as the PWA files); video
  tools are exercised through the built bundle (`scripts/ui-test.sh`),
  which is also what CI runs.
- Everything is testable headless: the UI suite feeds in-memory fixtures
  (a raw `.y4m`, a tiny real WebM) through the real worker and asserts
  on downloaded bytes (GIF/EBML/ftyp magics, GIF logical screen width).
- Upgrading ffmpeg = bumping two versions + two sha256 pins in
  `scripts/fetch-ffmpeg.sh`.
