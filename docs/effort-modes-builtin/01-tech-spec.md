# 01 — Tech Spec: Builtin Effort Modes

**Status:** Phase 1 draft  
**Date:** 2026-07-15  
**Parents:** [00-program-charter.md](./00-program-charter.md), research Specs 10/13/14  
**Normative for product delivery after Phase 1 freeze**

---

## 1. Summary

Add a session-scoped **`EffortMode`** orthogonal to plan/permission modes:

```text
EffortMode ∈ { Normal, Expert, Heavy }
```

User control:

```text
/expert [task…]
/heavy  [task…]
/normal
```

- Empty form → set sticky mode (shell-handled; no skill policy required).
- Args form → set mode (if needed) **and** enqueue the task as a normal user prompt under that mode.
- Non-trivial work under Expert/Heavy runs a **fixed analytic team** of size **N=4** / **N=16**, join-all, synthesize, then optional execute outside N.

---

## 2. Concepts

### 2.1 EffortMode (session policy)

Controls:

- Specialist team size target and contrarian requirement  
- Policy overlay / system reminders for the leader  
- Soft guidance on reasoning effort (if product supports it without model lock-in)  
- Runtime gates (hard in Phase 4)

Does **not** control:

- Always-approve / yolo  
- Plan mode write gates  
- Model picker catalog (grok.com “modes” in `chat_modes.rs` are unrelated)

### 2.2 Orthogonality matrix

| Dimension | Type | Owner today |
|-----------|------|-------------|
| Plan / Ask / Default | `SessionMode` | `session_mode.rs`, `PlanModeTracker` |
| Permission / yolo | session flag | `SetYolo` / permissions |
| **Effort** | **`EffortMode` (new)** | **this feature** |

Valid combinations include: Heavy + Plan (read-only team + leader), Expert + always-approve (user choice; document risk), Normal + Plan.

When Plan is active: fixed team and leader stay **non-writing** (align with existing plan write gates). Skip EXECUTE until plan exit unless user explicitly continues outside plan.

### 2.3 Pursuit state (per turn / per team run)

| State | Meaning |
|-------|---------|
| `Idle` | No team run |
| `Pursuing` | Filling to N successes under replace caps |
| `Aborting` | User cancelled; freeze ledger |
| `PartialReport` | Delivering S&lt;N after abort or hard-stop + user choice |
| `Waived` | Solo / trivial / subagents disabled |

---

## 3. User interface

### 3.1 Slash commands (builtins)

Register in `BUILTIN_COMMANDS` (same module as `compact`, `always-approve`, …):

| Name | Aliases | Args | Action |
|------|---------|------|--------|
| `expert` | — | optional task | `SetEffortMode { Expert }` + optional prompt |
| `heavy` | — | optional task | `SetEffortMode { Heavy }` + optional prompt |
| `normal` | — | none / ignored | `SetEffortMode { Normal }`; cancel in-flight team |

**Name resolution:** builtins resolve **before** skill slash tokens so product wins over `~/.grok/skills/{expert,heavy,normal}`.

**Gates:** `BuiltinGate::AlwaysOn` for all three (or a dedicated gate if subagents missing → still allow mode set, warn on team run).

### 3.2 TUI chrome

Minimum Phase 3+:

- Effort mode pill / status segment (Expert / Heavy / Normal)  
- Optional team progress (S of N) while pursuing  
- Clear indication of Partial / Waived

Mirror plan-mode update patterns (`enqueue_current_mode_update` analogue or dedicated event).

### 3.3 Solo waiver

Allow single-agent Expert/Heavy only if:

- args include `--solo`, or  
- user explicitly forbids subagents / requests solo  

Strip leading `--solo` from task text. Natural language from the **model** is not a waiver.

### 3.4 Trivial short-circuit

Solo without full team only for conservative trivial classes: typos, renames, one-line obvious fixes, pure factual lookup with no judgment.

Hard research, architecture, audits, multi-file work: **never** trivial.

---

## 4. Runtime pipeline (non-trivial, `Pursuing`)

1. **Orient** — constraints, goal, risks (leader).  
2. **Decompose** — N distinct analytic briefs (role tags alone insufficient).  
3. **Fan-out** — spawn N local subagents (`background: true`); prefer explore / non-editing capability modes.  
4. **Join** — wait_all semantics; timeout ≠ success; re-wait ≤1 per slot.  
5. **Ledger** — slot, role, task_id, status, counts_toward_N, rewaits, replaces.  
6. **Replace** — max 1 replace/slot, max 2 replace waves/turn; one-for-one only.  
7. **Contrarian (Heavy)** — ≥1 successful contrarian; if missing, vacate a non-contrarian slot and replace (never N+1).  
8. **Hard-stop** if still short → show ledger; offer `--solo` / `continue` / `/normal`.  
9. **Synthesize** — only success rows; residual risks; cite slots.  
10. **Execute** — leader or one post-N implementer **outside** N, only if code requested and Plan allows.  
11. **Verify** — after execute when code changed.

### 4.1 Successful specialist (counts toward N)

- Local subagent (or one-for-one replacement)  
- Terminal success  
- Non-empty task-relevant report  
- One slot; not the leader  

Does **not** count: failed, cancelled child, timeout without report, empty “ok”, thin duplicates.

### 4.2 Abort

User stop / “don’t wait for the rest”:

1. → `Aborting`  
2. Cancel running specialists  
3. Freeze ledger (no replace)  
4. → `PartialReport` allowed with S&lt;N  
5. Mode sticky remains Expert/Heavy until `/normal`

### 4.3 Mid-flight `/normal`

Clear effort mode **and** cancel team (same spirit as Normal skill Spec 14). Abandoned findings may inform Normal work only if labeled non-complete.

---

## 5. Persistence

**Recommended (align with PlanModeTracker):**

- Persist `EffortMode` (+ optional last ledger summary) under session directory, e.g. `effort_mode.json`.  
- Restore on session resume.  

**Research Spec 01 note:** original design said “mode does not restore across process restarts.” Product already persists plan mode. **Proposal:** persist EffortMode across resume (better UX); document in open questions if principal prefers session-lifetime-only.

Compact: keep mode; optionally inject compact-safe synthesis summary for in-flight work.

---

## 6. Configuration

v1 profiles (optional config, defaults hard-coded):

| Key | Expert default | Heavy default |
|-----|----------------|---------------|
| `team_size` | 4 | 16 |
| `require_contrarian` | false (prefer 1) | true |
| `max_rewait_per_slot` | 1 | 1 |
| `max_replace_per_slot` | 1 | 1 |
| `max_replace_waves` | 2 | 2 |
| `join_timeout_ms` | ≥300000 | ≥300000 |

Config validation clamps illegal values. Soft cost budgets warn only in early phases.

---

## 7. API / type sketch (product)

```rust
// New module: session/effort_mode.rs (parallel to plan_mode.rs)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffortMode {
    Normal,
    Expert,
    Heavy,
}

impl EffortMode {
    pub fn team_size(self) -> Option<usize> {
        match self {
            Self::Normal => None,
            Self::Expert => Some(4),
            Self::Heavy => Some(16),
        }
    }
    pub fn requires_contrarian(self) -> bool {
        matches!(self, Self::Heavy)
    }
}

pub struct EffortModeTracker { /* mode, pursuit, ledger, ... */ }
```

Slash:

```rust
BuiltinAction::SetEffortMode {
    mode: EffortMode,
    task: Option<String>, // remaining args after flags
    solo: bool,
}
```

Telemetry events analogous to `PlanModeToggled`.

---

## 8. Security and safety

1. Effort mode **never** enables always-approve.  
2. Specialist briefs are treated as untrusted for shell expansion; provenance in ledger.  
3. Plan mode write restrictions apply to the whole team.  
4. Subagents disabled → set mode allowed; team run becomes Waived with user-visible reason.  
5. No secrets in effort mode snapshots or docs.

---

## 9. Out of scope for first builtin ship

- Platform multi-agent API bridge as default  
- Dynamic team size beyond 4/16  
- Worktree-per-specialist mandatory  
- Headless flag parity (nice-to-have if cheap)  

---

## 10. Acceptance (product)

| ID | Criterion |
|----|-----------|
| B1 | `/expert` `/heavy` `/normal` appear as builtins; beat skills on name |
| B2 | Empty `/expert` sets sticky Expert without skill file |
| B3 | Expert non-trivial full-team finalize requires 4 successful ledger rows |
| B4 | Heavy non-trivial full-team finalize requires 16 successes + ≥1 contrarian |
| B5 | Under-count cannot claim full team while `Pursuing` |
| B6 | Abort → partial labeled S/N |
| B7 | Execute only after synthesis; writers outside N |
| B8 | Plan + Heavy stays non-writing until plan allows execute |
| B9 | `/normal` mid-flight cancels team and clears mode |
| B10 | Normal mode: effort runtime idle (tests) |

---

## 11. References

- Architecture: [02-architecture-map.md](./02-architecture-map.md)  
- Research Spec 10, 13, 14  
- Plan mode: `crates/codegen/xai-grok-shell/src/session/plan_mode.rs`  
- Slash: `crates/codegen/xai-grok-shell/src/session/slash_commands.rs`
