# 03 — Implementation Plan

**Status:** Phase 1 draft  
**Date:** 2026-07-15  
**Depends on:** [01-tech-spec.md](./01-tech-spec.md), [02-architecture-map.md](./02-architecture-map.md)

---

## 1. Strategy

Ship in **thin vertical slices** that always compile and test green. Prefer PlanModeTracker patterns. Keep skills working until builtins own name resolution and hard gates.

```text
Docs (Phase 1) → Types+slash+persist → Soft policy → Hard runtime → Migration → Ship
```

---

## 2. Phase 1 — Documentation (CURRENT)

**Owner:** Grok operator / principal review in #grok-build  

Deliverables: this directory.

Exit:

- [x] Charter, tech spec, architecture map, implementation plan, test plan, migration, open questions  
- [ ] Principal freeze / revision cycle  

**No code changes required to exit Phase 1.**

---

## 3. Phase 2 — Scaffold (product types + slash + persistence)

### PR-2.1 — Shared `EffortMode` enum

- Add `EffortMode { Normal, Expert, Heavy }` (shell-local first, or shared tools type if pager needs wire id).  
- Unit tests: parse/display, team_size, requires_contrarian.

### PR-2.2 — `EffortModeTracker`

- New `session/effort_mode.rs` pure FSM: mode set/clear, pursuit stubs, snapshot.  
- SessionActor ownership + `effort_mode.json` persist/restore (mirror plan).  
- Unit tests for transitions.

### PR-2.3 — Slash builtins

- Register `/expert` `/heavy` `/normal` in `BUILTIN_COMMANDS`.  
- `BuiltinAction::SetEffortMode`.  
- Dispatch: update tracker; empty form → no model turn if product pattern allows (else minimal system notice); args form → enqueue prompt.  
- Tests: resolve names, `--solo` parse, skill name collision preference.

### PR-2.4 — Chrome stub

- Surface current EffortMode in session info / minimal pill hook.  
- Telemetry `EffortModeToggled`.

**Phase 2 exit:** builtins exist; sticky mode persists on resume; no hard multi-agent yet (optional soft reminder only).

---

## 4. Phase 3 — Soft orchestration

### PR-3.1 — Leader policy injection

- On turn start under Expert/Heavy, inject structured effort policy (N, contrarian, execute-after-synthesis, join_all).  
- Plan-active: inject non-writing constraint.

### PR-3.2 — Triviality + solo

- Conservative trivial short-circuit helper + tests.  
- Solo waiver flags on tracker for the turn.

### PR-3.3 — Soft roster guidance

- Suggested 4- and 16-role briefs in policy text (from skill roster tables).  
- Soft max-parallel guidance if product has concurrency knobs.

### PR-3.4 — Transparency

- Encourage / structure ledger output in leader synthesis template.  
- Link to subagent inspect UI.

**Phase 3 exit:** behavior ≈ high-quality skills, but shell-owned mode + chrome; still largely model-obedient.

---

## 5. Phase 4 — Hard runtime (true product Heavy/Expert)

### PR-4.1 — Ledger + join service

- Record every specialist slot.  
- Join-all with re-wait ≤1; success definition from tech spec.  
- Unit tests with fake task handles.

### PR-4.2 — Replace caps + hard-stop

- Enforce replace budgets; hard-stop UX (message to user).  
- Forbidden: silent under-count full-team finalize.

### PR-4.3 — Heavy contrarian invariant

- Detect contrarian success; repair via slot replace.

### PR-4.4 — Execute ordering gate

- Block or warn leader file-mutating tools until synthesis complete for the team run (design carefully vs normal tools).  
- Post-N implementer labeled outside N.

### PR-4.5 — Abort / mid-flight normal

- Wire user cancel and `/normal` to abort FSM + partial report path.

**Phase 4 exit:** B3–B9 acceptance IDs green in automated tests with mocked subagents.

---

## 6. Phase 5 — Skills coexistence

See [05-skills-coexistence-and-migration.md](./05-skills-coexistence-and-migration.md).

- Builtin name win verified.  
- Install script / skill README points to builtins.  
- Optional: ship thin skill stubs that say “builtin owns this mode.”  
- Agency/PGS docs update (`AGENTS.md` references remain valid as slash UX).

---

## 7. Phase 6 — Ship

- Changelog entry in shell changelogs style.  
- User guide section (slash + modes).  
- Manual TUI demo checklist.  
- Release owner sign-off.

---

## 8. Risk register

| Risk | Mitigation |
|------|------------|
| Model ignores soft policy | Phase 4 hard gates |
| Name collision with skills | Builtin-first resolve + migration docs |
| Plan + Effort write races | Explicit matrix tests |
| 16-agent CI cost/time | Mock children; never live 16 in PR CI |
| Overloading `SessionMode` | Separate `EffortMode` dimension |
| Scope creep (API bridge, worktrees) | Park in open questions |

---

## 9. Suggested ownership

| Slice | Owner suggestion |
|-------|------------------|
| Tracker + slash | Shell engineer / Grok in-repo |
| TUI pill | Pager |
| Hard join/ledger | Shell + tools |
| Docs / agency continuity | Grok operator |
| PGS contract text | PGS maintainers after ship |

---

## 10. Immediate next action after Phase 1 freeze

Open **PR-2.1 + PR-2.2** on a feature branch:

```text
feat/effort-mode-tracker
```

Implement pure `EffortMode` + `EffortModeTracker` with unit tests only; no slash yet if PR size prefers split.
