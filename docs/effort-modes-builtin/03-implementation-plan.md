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

- [x] Add `EffortMode { Normal, Expert, Heavy }` (shell-local in `session/effort_mode.rs`).  
- [x] Unit tests: parse/display, team_size, requires_contrarian.

### PR-2.2 — `EffortModeTracker`

- [x] New `session/effort_mode.rs` pure FSM: mode set/clear, pursuit, snapshot.  
- [x] SessionActor ownership.  
- [x] **Persist/restore:** `effort_mode.json` via `PersistenceMsg::EffortModeState` + spawn restore.  
- [x] Unit + integration tests for transitions + snapshot round-trip.

### PR-2.3 — Slash builtins

- [x] Register `/expert` `/heavy` `/normal` in `BUILTIN_COMMANDS` behind `BuiltinGate::EffortMode` / `GROK_EFFORT_MODE_BUILTINS`.  
- [x] `BuiltinAction::SetEffortMode`; parse `--solo` in resolve.  
- [x] Dispatch: mode-only → `ok_end_turn`; args form → sticky mode + prompt.  
- [x] Resolve tests (in-module) for names, `--solo`, skill shadow, gate-off.

### PR-2.4 — Chrome stub

- [x] Telemetry span `session.effort_mode_toggled` + structured log on apply.  
- [ ] Dedicated pager pill Event (follow-up).

**Phase 2 exit:** builtins exist when flag on; sticky mode **persists on resume** (Q1=A); flag-off restores skill slash; soft reminder on elevated turns.

---

## 4. Phase 3 — Soft orchestration

### PR-3.1 — Leader policy injection

- [x] On turn start under Expert/Heavy, inject structured effort policy via `inject_effort_mode_reminders` (same system-reminder path as plan mode).  
- [x] Plan-active: non-writing constraint in policy + hard `PlanBlocksExecute`.

### PR-3.2 — Triviality + solo

- [x] `is_trivial_task` helper + tests.  
- [x] Solo waiver on tracker (`--solo` parse + `set_mode(..., solo)`).

### PR-3.3 — Soft roster guidance

- [x] N=4 / N=16 + contrarian guidance in `policy_reminder` text.  
- [ ] Soft max-parallel knobs (product optional; deferred).

### PR-3.4 — Transparency

- [x] `progress_label` S of N for partial/abort UX.  
- [ ] Subagent inspect UI link (chrome follow-up).

**Phase 3 exit:** shell-owned mode + soft policy + hard execute gate; team spawn still model-obedient.

---

## 5. Phase 4 — Hard runtime (true product Heavy/Expert)

### PR-4.1 — Ledger + join service

- [x] Record every specialist slot (`SpecialistLedgerRow`).  
- [x] Join-all success criteria pure (`can_claim_full_team`); re-wait ≤1 cap fields.  
- [x] Integration tests with fake task_ids (no live N).

### PR-4.2 — Replace caps + hard-stop

- [x] Replace budgets + hard-stop continue one extra wave.  
- [x] Under-count finalize forbidden (`UnderCount` / `MissingContrarian`).

### PR-4.3 — Heavy contrarian invariant

- [x] Detect contrarian among successes; fail closed without it.

### PR-4.4 — Execute ordering gate

- [x] Block `AccessKind::Edit` until synthesis (and under plan+effort).  
- [x] Post-N implementer registration (`outside_n`).

### PR-4.5 — Abort / mid-flight normal

- [x] `abort_team` → PartialReport; `/normal` → `clear_to_normal`.

**Phase 4 exit (updated):** B3–B9 green via pure FSM + integration tests.  
**Live mandatory fan-out:** shell-owned `maybe_run_mandatory_effort_team` (Expert N=4 / Heavy N=16) spawns + join-all before the leader model turn; team spawn is no longer model-obedient soft policy.

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
