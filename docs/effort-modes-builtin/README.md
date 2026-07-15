# Effort Modes as Grok Build Builtins

**Program:** Promote Expert / Heavy / Normal from skills-first v1 into **first-party shell + TUI builtins**.  
**Phase 1 (this package):** documentation only — tech specs, architecture map, implementation plan, test plan, migration, open questions.  
**Status:** Phase 1 draft opened 2026-07-15 in product source (`zfifteen/grok-build` clone).  
**Product cwd:** this repository root.

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

## Prior art (external SoT for behavior)

| Source | Role |
|--------|------|
| `research/grok-build-effort-modes/docs/tech-specs/10-requirements-refinement.md` | Locked behavioral requirements |
| Specs 13–14 | Join / completed / execute; abort, replace caps, solo |
| Spec 12 | Skills-first v1 ship law (historical delivery) |
| Spec 11 | Builtin-path dead-end analysis (now unblocked) |
| `~/.grok/skills/{expert,heavy,normal}/SKILL.md` | Live policy approximation |

## Reading order

1. Charter → tech spec → architecture map  
2. Implementation plan → test plan  
3. Skills migration → open questions  

## Contact surface

Rocket.Chat channel **#grok-build** maps to this repository. Continuity of the program should be updated in agency state when phases complete.
