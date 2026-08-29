# ADR-0009: Browser rasterization for PDF → image tools

- **Status**: Accepted
- **Date**: 2026-08-29
- **Amends**: [ADR-0007](0007-pdf-editor-pdfjs.md) (rule 3, "isolated in the editor")

## Context

"PDF to JPG" is one of the most-searched file tools there is, and it sat on
the roadmap marked *blocked: no wasm-safe rasterizer*. That framing is out
of date. Rasterization is still impossible in pure Rust — `lopdf` parses
PDFs but does not render them, and every production renderer (pdfium,
mupdf) is C — but ADR-0007 already bundled PDF.js for the editor, and the
editor already ships `⬇ Pages as PNG` on top of it.

So the capability exists; what was missing was a way to expose it as a
*tool*. Two things stood in the way:

1. **ADR-0007 rule 3** scoped PDF.js to the editor: "only the editor page
   loads it, so every other tool stays wasm-only."
2. **The registry contract assumes purity.** Every tool is
   `pz_engine::run(slug, files, opts)` — bytes in, bytes out, on a Web
   Worker (ADR-0002/0004). Rasterization can't be: it needs a canvas on the
   main thread.

Leaving it in the editor only was the alternative considered: users
searching "pdf to jpg" would land in a full editing workspace to do a
one-click job, and the tool would be invisible to the tool grid, the
mega menu and the prerendered SEO pages (ADR-0006).

## Decision

Allow browser rasterization for tool pages, under the same conditions that
made ADR-0007 acceptable, plus one more:

1. **Render only.** `app/assets/pdfrender.js` opens a document, draws pages
   to offscreen canvases and returns encoded bytes. It never writes a PDF —
   every mutation stays in the Rust engine, as ADR-0007 rule 1 requires.
2. **Same bundled PDF.js.** No CDN, no second copy: the module imports
   `assets/pdfjs/` like the editor does, so the offline and no-third-party
   promises hold and the bundle doesn't grow.
3. **Lazy and opt-in.** The `<script>` is mounted only for tools whose
   pipeline says they need it, so every other tool page stays wasm-only.
4. **A separate module from the editor.** `editor.js` owns the editing
   workspace — overlay canvases, strokes, undo, a `#pz-pages` container it
   writes into — and would throw on a tool page that has no such container.
   `pdfrender.js` is ~90 lines with no DOM dependency.
5. **The exception is declared, not inferred.** `ToolMeta` gains
   `pipeline: ToolPipeline`. `Engine` is every tool but one; `BrowserRender`
   marks this path. The app dispatches on that field instead of matching
   slugs, and `pz_engine::run` *rejects* `BrowserRender` slugs with a clear
   `Unsupported` error rather than looking like an unimplemented tool.

## Consequences

- PDF to Image ships as a normal tool: registry entry, generic tool page,
  SEO landing page, home grid tile, mega-menu row.
- There is no headless path for it. `pz_engine::run("pdf-to-images", …)`
  errors by design, and a future CLI would have to say so. Web and
  desktop/mobile all run a WebView, so every shipped build can do it.
- Adding a tool now includes choosing a pipeline. The `add-tool` skill
  says so, and a wrong choice fails loudly (engine rejection) rather than
  silently.
- The privacy promise is unchanged: rendering happens in the user's own
  browser, the images are packaged on-device by the engine, and nothing is
  uploaded. No new CSP directives are needed — PDF.js and its worker are
  same-origin (`worker-src 'self' blob:` already covers it).
- Page-range validation splits: Rust checks the *syntax* of "1-3,5", the JS
  side checks it against the real page count (unknown until the document is
  open) and reports the range in the error.
