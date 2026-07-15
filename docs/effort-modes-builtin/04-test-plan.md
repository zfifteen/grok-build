# 04 — Test Plan: Builtin Effort Modes

**Status:** Phase 1 draft  
**Date:** 2026-07-15  
**Parents:** [01-tech-spec.md](./01-tech-spec.md), research Spec 09 (adapted)  
**Codebase patterns:** `plan_mode.rs` unit tests, `acp_session_tests/plan_mode_*`, shell slash tests

---

## 1. Purpose

Prove builtin Effort Modes meet acceptance B1–B10 with **deterministic** tests in this monorepo. Prefer mocked subagents and pure FSM tests over live model runs.

---

## 2. Levels

| Level | What | CI cadence |
|-------|------|------------|
| **Unit** | `EffortMode` math, tracker FSM, triviality, `--solo` parse, success criteria pure functions | Every PR |
| **Component** | Slash resolve, session persist/restore, plan+effort matrix on SessionActor with fakes | Every PR |
| **Integration** | Mock N subagent completions → ledger → synthesis gate → execute allow | Every PR (Phase 4+) |
| **E2E headless** | Stub model tool scripts for Expert 4 / Heavy 16 happy paths | Nightly / pre-release |
| **Manual TUI** | Pill, progress S/N, abort UX, skill collision | Phase demos |
| **Regression** | Normal mode idle; existing plan/permission suites green | Every PR |

---

## 3. Principles

1. Map every case to acceptance ID (B*) or research ID (A*/Spec 14 B*).  
2. **Test the runtime, not model intelligence** — inject tool results.  
3. **No live 16-agent fan-out in PR CI** — use fakes; optional gated soak.  
4. Fail closed on under-count finalize and always-approve non-coupling.  
5. Keep Normal mode as control group.

---

## 4. Case catalog (minimum)

### 4.1 Mode & slash (B1–B2, B9)

| ID | Case | Expect |
|----|------|--------|
| T-S1 | Resolve `/expert`, `/heavy`, `/normal` as builtins | BuiltinAction set mode |
| T-S2 | Skill files present with same names | Builtin still wins |
| T-S3 | Empty `/expert` | Mode=Expert; sticky |
| T-S4 | `/expert fix CI` | Mode=Expert; task queued |
| T-S5 | `/expert --solo task` | Solo waiver; task stripped |
| T-S6 | `/normal` while pursuing | Mode Normal; team cancelled; partial ok |
| T-S7 | Resume session | EffortMode restored from snapshot (if persist chosen) |

### 4.2 Tracker FSM

| ID | Case | Expect |
|----|------|--------|
| T-F1 | Normal → Expert → Heavy → Normal | Legal transitions |
| T-F2 | Pursuing → Aborting → PartialReport | Freeze ledger |
| T-F3 | Replace caps exhausted | Hard-stop state |
| T-F4 | Mid-turn plan enter under Heavy | Non-writing flags |

### 4.3 Team gates (B3–B7)

| ID | Case | Expect |
|----|------|--------|
| T-G1 | Expert 4 successes | Full-team finalize allowed |
| T-G2 | Expert 3 successes pursuing | Finalize blocked / hard-stop path |
| T-G3 | Heavy 16 + 0 contrarian | Repair or hard-stop; no claim complete |
| T-G4 | Heavy 16 + ≥1 contrarian | Full-team ok |
| T-G5 | Timeout without report | Not success; re-wait then fail/replace |
| T-G6 | Empty “ok” report | Not success |
| T-G7 | Execute tool before synthesis | Denied or deferred (Phase 4) |
| T-G8 | Post-N implementer | Labeled outside N; does not inflate success count |

### 4.4 Orthogonality (B8, safety)

| ID | Case | Expect |
|----|------|--------|
| T-O1 | Heavy + Plan active | No write tools in team/leader until plan allows |
| T-O2 | Expert + always-approve on | Mode set does not toggle yolo; yolo remains user-owned |
| T-O3 | Subagents disabled | Mode set ok; team Waived with reason |

### 4.5 Normal regression (B10)

| ID | Case | Expect |
|----|------|--------|
| T-N1 | Default session | Effort runtime idle; no team hooks |
| T-N2 | Existing compact/plan/yolo tests | Still pass |

### 4.6 Abort / partial (B6)

| ID | Case | Expect |
|----|------|--------|
| T-A1 | User abort at 5/16 | Partial 5/16; no further replaces |
| T-A2 | Continue after hard-stop | One extra replace wave only |

---

## 5. Fixtures

- Fake `task_id` handles with controllable terminal states.  
- Temp session directory for `effort_mode.json`.  
- Preloaded skill dirs named expert/heavy to test collision.  
- Plan mode tracker in Active for matrix tests.

---

## 6. CI robustness

1. No network requirement for PR tests.  
2. Bound join waits with fake clocks or instant completions.  
3. Cap concurrent fake agents; do not spawn 16 OS processes.  
4. Snapshot stable strings for user-visible hard-stop copy.

---

## 7. Manual TUI checklist (phase demos)

- [ ] Pill shows Expert / Heavy / Normal  
- [ ] Progress S of N while running  
- [ ] Abort copy is clear  
- [ ] Specialist transcripts reachable  
- [ ] `/normal` clears chrome  
- [ ] Skills autocomplete does not override builtins  

---

## 8. Exit criteria by phase

| Phase | Must pass |
|-------|-----------|
| 2 Scaffold | T-S*, T-F1, T-N*, T-O2 subset |
| 3 Soft | + policy inject smoke (may be lighter) |
| 4 Hard | T-G*, T-A*, T-O1, T-O3 |
| Ship | All automated + manual checklist |

---

## 9. Mapping to research Spec 09

Research Spec 09 assumed more of a greenfield EffortRuntime and multi-agent API bridge. This plan **keeps** gate/acceptance spirit and **drops** CI dependence on research-bridge cases until those become product goals. Prefer IDs T-* here; cite research A* where policy text still applies.
