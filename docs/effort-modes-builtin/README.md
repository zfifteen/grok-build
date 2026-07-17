# Effort Modes as Power Grok Builtins

**Program:** Promote Expert / Heavy / Normal from skills-first v1 into **first-party shell + TUI builtins**.  
**Phase 1 (this package):** documentation only — tech specs, architecture map, implementation plan, test plan, migration, open questions.  
**Status:** Phase 1 draft opened 2026-07-15 in product source (`zfifteen/powergrok` clone).  
**Product cwd:** this repository root.

**Branding note:** This document has been updated to refer to "Power Grok" per the branding plan. All technical references to the upstream project remain "Grok Build".

## Why this exists

Effort modes were designed as product session modes (sticky slash builtins, mode chrome, hard multi-agent gates). Without product source, we shipped **skills-first v1** (`/expert`, `/heavy`, `/normal` as policy skills under `~/.grok/skills/`). Spec history lives in:

`~/IdeaProjects/research/grok-build-effort-modes/`

Behavioral intent from that package remains law. **Delivery** for this program is product source in *this* tree.

## Phase plan (program-level)

| Phase | Deliverable | Gate |
|-------|-------------|------|
| **1 — Docs** | This directory: full tech specs + plans | Principal review / freeze |
| **2 — Scaffold** | `EffortMode` types, slash builtins, session persistence, chrome stubs | Compiles + unit tests green |
| **3 — Soft runtime** | Policy inject, roster guidance, soft caps, transparency hooks | Expert feels disciplined without hard gates |
| **4 — Hard runtime** | Join/success ledgers, fixed N=4/16, replace caps, abort FSM | Spec 10/13/14 gates enforceable in code |
| **5 — Skills coexistence** | Builtin wins name resolution; skills demote or become docs | No double-orchestration |
| **6 — Ship** | Changelog, user guide, optional skill deprecation notice | Release checklist |

Phase 1 does **not** change runtime behavior.

## Document index

| Doc | Purpose |
|-----|---------|
| [00-program-charter.md](./00-program-charter.md) | Goals, non-goals, authority, prior art |
| [01-tech-spec.md](./01-tech-spec.md) | Full product tech specification |
| [02-architecture-map.md](./02-architecture-map.md) | Real crates, files, and extension points |
| [03-implementation-plan.md](./03-implementation-plan.md) | Phased PR plan and ownership |
| [04-test-plan.md](./04-test-plan.md) | How we prove correctness in this codebase |
| [05-skills-coexistence-and-migration.md](./05-skills-coexistence-and-migration.md) | Skills v1 → builtins transition |
| [06-open-questions.md](./06-open-questions.md) | Decisions still open |
| [07-reasoning-brains-implementation-plan.md](./07-reasoning-brains-implementation-plan.md) | **16 reasoning brains** for Expert (random 4) / Heavy (all 16); config under `$GROK_HOME/effort-brains` |


## Prior art (external SoT for behavior)

**Portable (what the specs mean):** research design archive *Grok Build Effort Modes* — Specs 10 (behavioral), 11 (delivery constraint), 12 (skills-first v1), 13–14 (join/abort/replace/solo). That tree is **not vendored** in this repository.

**Operator workstation (Velocity Works layout, not portable to every clone):**

| Source | Role |
|--------|------|
| `IdeaProjects/research/grok-build-effort-modes/docs/tech-specs/` | Spec files 00–14 on principal machine |
| User skills `~/.grok/skills/{expert,heavy,normal}/` | Live skills-first v1 packages when installed |

External reviewers: treat absolute home/IdeaProjects paths as **agency-local SoT**, not required inputs to build this monorepo.

## Reading order

1. Charter → tech spec → architecture map  
2. Implementation plan → test plan  
3. Skills migration → open questions  

## Contact surface

Rocket.Chat channel **#grok-build** maps to this repository. Continuity of the program should be updated in agency state when phases complete.
