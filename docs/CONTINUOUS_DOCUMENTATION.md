# Continuous documentation — spec & plan

Documentation is part of the change, not an afterthought. This file defines
what "documented" means for PrivZapp, which docs exist, who they serve, and
how drift is caught mechanically. It applies to humans and coding agents
alike.

## The rule

> **A change is done when the code, its tests, and its documentation land in
> the same commit.** If the docs didn't need to change, that's a fact you
> verified, not one you assumed.

"Documentation" for a given change is the union of:

| You changed…                          | You must update…                                   |
|---------------------------------------|----------------------------------------------------|
| The tool registry (`pz-core::TOOLS`)  | README tools table *(test-enforced)*, CHANGELOG    |
| Any user-visible behavior             | CHANGELOG `[Unreleased]`                           |
| Crate structure, data flow, platforms | `docs/ARCHITECTURE.md`                             |
| A hard-to-reverse design decision     | New ADR in `docs/adr/` (next number, statused)     |
| Build/verify workflow, gotchas        | `CLAUDE.md` (agents) and/or README (humans)        |
| Public engine APIs                    | Rustdoc on the item (`cargo doc` must stay clean)  |
| Roadmap reality (done/blocked/cut)    | README roadmap checkboxes + `docs/ROADMAP.md`      |

## Doc map (who reads what)

- **README.md** — users and first-time contributors. Product promise, tool
  list, quickstart, roadmap checkboxes.
- **CHANGELOG.md** — users upgrading. Keep-a-Changelog format; every
  user-visible change gets a line in `[Unreleased]` when it merges.
- **docs/ARCHITECTURE.md** — contributors. The crate graph, invariants, and
  data flow; updated when the shape of the system changes.
- **docs/adr/** — future maintainers wondering "why is it like this?".
  Numbered, immutable once Accepted; supersede rather than edit.
- **docs/ROADMAP.md** — planners. What's next, what's blocked and why,
  including designs for approved-but-deferred work.
- **CLAUDE.md / AGENTS.md** — coding agents. Ground rules, commands,
  gotchas. Update when a new gotcha is discovered or a rule changes.
- **Rustdoc** — engine API consumers. Every public engine item carries a
  doc comment stating its contract (inputs, outputs, failure modes).

## Mechanical enforcement (drift is a test failure)

1. `crates/pz-core/tests/docs_sync.rs`:
   - every registered tool name must appear in README.md;
   - CHANGELOG.md must have an `[Unreleased]` (or current-version) section;
   - the privacy promise copy must survive edits.
2. `scripts/verify.sh` runs the full battery (fmt, clippy `-D warnings`,
   tests incl. docs-sync, wasm check) — CI runs the same script, so a doc
   drift blocks merge exactly like a failing unit test.
3. The `continuous-docs` skill (`.claude/skills/continuous-docs/`) gives
   agents the per-change checklist; the `add-tool` skill bakes doc updates
   into the tool recipe so the common case can't skip them.

## Plan (how this grows)

- **Now (v0.1)**: the above — registry-driven README check, changelog
  discipline, ADRs for every irreversible decision, agent skills.
- **Next**: generate the README tools table *from* the registry (a tiny
  `cargo xtask` printing markdown; test compares output instead of
  substring-matching names). Add `#![deny(missing_docs)]` to pz-core once
  existing items are fully documented.
- **Later**: publish rustdoc + these docs as a static site alongside the
  app; a public "what we collect" page generated from the
  `pz_telemetry::Event` schema so the privacy contract is user-auditable.
