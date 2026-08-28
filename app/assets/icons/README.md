# Tool tile icons

PrivZapp PDF Tools — 14 unique 64x64 SVG icons. Gold/orange +
electric-blue accents, plum rounded tile, no external assets.

Wiring: `app/src/icons.rs` maps tool slugs to these files; every render
site (home grid, all-tools menu, tool page header, related-tools links)
falls back to the registry's emoji when a slug has no SVG here.

Adding icons for more tools: name the file exactly after the tool's
slug in `crates/pz-core` (`compress-img.svg`, `zip-files.svg`, …), drop
it in this folder, and add the slug to the match in `app/src/icons.rs`.
Keep them self-contained (no external refs — the CSP forbids them) and
give internal gradient/clip ids unique-per-file-or-not: files are
referenced via `<img src>`, each SVG is its own document, so duplicate
ids across files never collide.
