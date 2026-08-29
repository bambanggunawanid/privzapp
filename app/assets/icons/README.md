# Tool tile icons

One unique 64x64 SVG per tool — all 31 of them (14 PDF, 13 image, 2
archive, 2 security). Gold/orange + electric-blue accents on a rounded
tile, no external assets.

The tile gradient is the tool's **category** colour, hue-matched to the
`.cat-*` tints in `app/assets/main.css` so the grid reads by group:

| Category | Tile gradient       | matches       |
| -------- | ------------------- | ------------- |
| PDF      | `#52304D` `#3A293F` | `.cat-pdf`    |
| Image    | `#24455C` `#1B3242` | `.cat-image`  |
| Archive  | `#4C3919` `#362713` | `.cat-archive`|
| Security | `#40325E` `#2E2545` | `.cat-security`|

Keep new tiles in that range: every accent colour used in the set stays
above 3:1 contrast (WCAG 1.4.11, graphical objects) against both stops.
The colour is baked into each file on purpose — the icons are loaded via
`<img src>`, so CSS in the host page cannot reach inside them.

Wiring: `app/src/icons.rs` maps tool slugs to these files; every render
site (home grid, all-tools menu, tool page header, related-tools links)
falls back to the registry's emoji when a slug has no SVG here.

Adding an icon for a new tool: name the file exactly after the tool's
slug in `crates/pz-core` (`compress-img.svg`, `zip-files.svg`, …), drop
it in this folder, and add the slug to the match in `app/src/icons.rs`.
Keep them self-contained (no external refs — the CSP forbids them) and
give internal gradient/clip ids unique-per-file-or-not: files are
referenced via `<img src>`, each SVG is its own document, so duplicate
ids across files never collide.
