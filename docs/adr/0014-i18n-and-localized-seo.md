# ADR-0014: Multi-language support, and localized URLs for SEO

- **Status**: Accepted
- **Date**: 2026-08-30

## Context

The product's goal is to rank against iLovePDF-class sites. Every tool
already has its own prerendered landing page (ADR-0006), but only in
English — which leaves the entire non-English search market unaddressed,
starting with Indonesian, the owner's home market.

Translation alone would not have helped. A language toggle that swaps
strings at the same URL gives a search engine nothing to index: one URL,
one indexed page, no way to signal which audience it serves. The SEO
payoff requires *distinct URLs per language*, linked by `hreflang`.

## Decision

### URLs

English keeps the canonical, unprefixed URL space (`/tool/merge-pdf`);
every other locale lives under its code (`/id/tool/merge-pdf`). Existing
links keep working, and the primary language keeps the shortest URLs.

Routing uses a single `#[nest("/:lang")]` block, so **adding a language
costs zero new routes** — four localized variants cover every locale
that will ever exist. `Locale::from_str` accepts only known language
codes, which is what stops `/tool/...` being parsed as locale "tool";
that is load-bearing enough to have its own test in both Rust and
Playwright. English is deliberately *not* parseable, so `/en/...` can
never become a duplicate of the canonical URL and split its ranking.

### Strings

UI strings are keyed by their **English text**, gettext-style:
`t(loc, "Choose a PDF to edit")`. Call sites stay readable, and a
missing translation degrades to English instead of to a raw key. The
trade-off is that editing an English string silently orphans its
translation; `catalog_is_sane` guards the table's shape (no duplicates,
no empty or identical entries) and the UI tests assert the strings that
matter.

Tool names, taglines, and the full SEO copy (title, description, all
118 FAQ pairs) live in per-locale tables. A test fails if any tool's
Indonesian title equals its English one, because a page that silently
serves English body copy under `<html lang="id">` ranks worse than
either language done properly.

### SEO structure

`seo-gen` writes every page in every locale: 82 pages for 2 locales.
Each carries `<html lang>`, a self-referencing canonical, and reciprocal
`hreflang` for every locale plus `x-default` → English. Every `hreflang`
href is byte-identical to that page's own canonical (including the
trailing slash on home), because Google ignores annotations that point
somewhere other than the canonical. The sitemap lists all locales.

## Consequences

- Adding a language is data, not code: a `Locale` variant plus rows in
  three tables. Routing, `hreflang`, the sitemap and the switcher all
  iterate `Locale::ALL`.
- The switcher is plain links to the mirrored URL, so a crawler can
  follow them and the URL always states the language. There is no
  automatic redirect based on `Accept-Language`: Google recommends
  against it, and it makes pages unreachable for crawlers.
- The nav ran out of horizontal room on phones once the switcher was
  added, so the brand wordmark now hides below 700px (was 380px) and the
  logo carries the brand alone.
- Fixed in passing: `home_body` in seo-gen listed categories by hand and
  had never been updated for Video, so the prerendered home page did not
  link any video tool. It now derives categories from the registry.

## The honest caveat

The Indonesian copy is a careful good-faith translation, but it has not
been reviewed by a native speaker. Google's guidance is explicitly
against publishing unreviewed machine-translated content at scale, and
awkward phrasing on a page whose purpose is to rank is worse than no
page. **A native review pass should happen before the Indonesian URLs
are submitted for indexing** — it is listed in `docs/MANUAL_QA.md`. The
structure (routing, hreflang, sitemap) is correct regardless; only the
prose needs the second pair of eyes.
