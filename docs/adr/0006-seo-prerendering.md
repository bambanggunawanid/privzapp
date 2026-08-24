# ADR-0006: Build-time SEO prerendering from the tool registry

- **Status**: Accepted
- **Date**: 2026-08-24

## Context

PrivZapp is a wasm SPA: the dx bundle serves one `index.html` with an empty
`<div id="main">` and a single generic title for every route. Crawlers that
don't execute wasm see nothing; even Google's renderer gets no per-route
titles, descriptions or canonicals. Competing with established file-tool
sites for queries like "compress image" or "merge pdf" requires every tool
to be a real, unique, crawlable page.

Dioxus fullstack SSG was rejected for now: it drags server-function
machinery into a deliberately serverless app and couples rendering to a
framework feature still moving fast.

## Decision

A small native tool (`tools/seo-gen`, runs as the last step of
`scripts/build-web.sh`) rewrites the dx bundle:

1. Per route (`/`, `/privacy`, `/support`, `/tool/<slug>`) it emits a real
   HTML file: unique `<title>`, meta description, canonical URL, Open
   Graph/Twitter tags, JSON-LD (`WebApplication` + `FAQPage` per tool,
   `WebSite` on home) and crawlable body content — headline, description,
   FAQ, related-tool links — injected inside `<div id="main">`.
2. The wasm app replaces `#main` on load and renders the *same* copy
   (description, FAQ, related links now part of `ToolPage`), so prerender
   and hydrated page match — prerender-then-hydrate, not cloaking.
3. All copy lives in `pz_core::seo` next to the registry; tests enforce
   every tool has an entry and that titles/descriptions fit snippet
   limits. seo-gen also emits `sitemap.xml` and `robots.txt`.
4. `BASE_URL` (env / Docker build arg) sets the canonical origin;
   nginx serves prerendered pages via `try_files $uri $uri/index.html`.

## Consequences

- Every tool page is independently indexable with structured data eligible
  for rich results; content parity keeps it within search guidelines.
- Adding a tool without SEO copy fails tests — copy is part of "done".
- Canonicals are wrong until the real production domain is set in
  `BASE_URL`; deploys must pass it.
- The dx template is patched by string replacement; if dx changes its
  index.html shape, seo-gen's `expect`/`replacen` calls surface it at
  build time, not silently.
