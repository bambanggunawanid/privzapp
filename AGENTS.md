# Agent guide (any coding agent)

Canonical, always-current agent instructions live in **[CLAUDE.md](CLAUDE.md)**
— ground rules, commands, gotchas. Read that first; this file is the map.

## Orientation in 30 seconds

- Product promise: **files never leave the device.** No network in engine
  crates, ever. UI copy about privacy is a contract, not marketing.
- Architecture: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
  Decisions & rationale: [docs/adr/](docs/adr/).
  What's next & why: [docs/ROADMAP.md](docs/ROADMAP.md).
- One-command verification: `./scripts/verify.sh` (fmt, clippy -D warnings,
  tests, wasm32 check). CI runs the same script.
- Definition of done includes docs, in the same commit:
  [docs/CONTINUOUS_DOCUMENTATION.md](docs/CONTINUOUS_DOCUMENTATION.md).

## Task playbooks (Claude Code loads these as skills)

Stored in `.claude/skills/`, readable as plain markdown by any agent:

- `add-tool/` — the end-to-end recipe for a new file tool (registry →
  engine → dispatch → UI → tests → docs).
- `verify/` — run and interpret the verification battery.
- `release/` — production web bundle + PWA sanity checks + version bumps.
- `continuous-docs/` — the per-change documentation checklist.

Specialist reviewers in `.claude/agents/`: `privacy-reviewer` (run before
releases and on any telemetry/dependency/UI-copy change) and `wasm-guard`
(run on dependency or engine changes).

## Optional plugins (Claude Code)

The repo is self-sufficient, but two additions play well with it:

```bash
# Community skills library (brainstorming, TDD, debugging workflows)
claude plugin marketplace add obra/superpowers-marketplace
claude plugin install superpowers@superpowers-marketplace
```

Project permissions for the common read/build/test commands are already
allowlisted in `.claude/settings.json`, so agents run the verify loop
without permission prompts.

## Hard "ask first" lines

- Any field added to `pz_telemetry::Event` (privacy contract).
- Anything that sends bytes anywhere (rejected by design).
- Reducing PBKDF2 rounds or changing the `.pzv` format (ADR-0003).
- Landing Web-Worker/PWA behavior changes without a manual browser test
  (this container is headless — say what needs hand-testing).
