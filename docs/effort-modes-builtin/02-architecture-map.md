# 02 — Architecture Map (Product Source)

**Status:** Phase 1 draft  
**Date:** 2026-07-15  
**Repo:** this tree (`zfifteen/grok-build`)

Maps Effort Mode work onto **existing** crates. Paths are relative to repo root.

---

## 1. High-level placement

```text
User slash  →  shell BuiltinCommand  →  BuiltinAction::SetEffortMode
                 ↓
          EffortModeTracker (session)
                 ↓
     prompt inject / team run hooks
                 ↓
     tool runtime: spawn_subagent + wait
                 ↓
     join ledger + synthesis (leader turn)
                 ↓
     optional execute (leader / post-N)
                 ↓
     TUI: pill, progress, transcripts
```

---

## 2. Primary crates

| Crate | Role for Effort Modes |
|-------|------------------------|
| `crates/codegen/xai-grok-shell` | Slash builtins, session actor, plan mode twin, ACP session |
| `crates/codegen/xai-grok-tools` | Tools, skills, session mode types, subagent execution |
| `crates/codegen/xai-grok-tools-api` | Shared slash wording helpers (pattern for canonical strings) |
| `crates/common/xai-tool-types` | Builtin subagent catalog (`explore`, `general-purpose`, `plan`) |
| `crates/codegen/xai-grok-subagent-resolution` | How subagent types resolve |
| `crates/codegen/xai-chat-state` | Conversation / command surfaces if events land here |
| `crates/codegen/xai-grok-pager` / `xai-grok-pager-render` | TUI chrome, mode display |
| `crates/codegen/xai-grok-telemetry` | Mode toggle / team lifecycle events |
| `crates/codegen/xai-grok-config` + `xai-grok-config-types` | Optional profile config |
| `crates/codegen/xai-grok-agent` | Leader prompts / templates if policy injection lives here |

---

## 3. Concrete extension points

### 3.1 Slash registration

**File:** `crates/codegen/xai-grok-shell/src/session/slash_commands.rs`

- Add `BuiltinCommand` entries: `expert`, `heavy`, `normal`.  
- Extend `BuiltinAction` with `SetEffortMode { … }`.  
- Implement `command_name` / `args_provided` arms.  
- Confirm advertising order and skill collision: builtins must resolve first (verify in resolve path next to `InvokeSkill`).

**Related:** `session/commands.rs`, `session/acp_session_impl/slash_exec.rs` — dispatch handlers for new action.

### 3.2 Session state machine (primary design twin)

**File:** `crates/codegen/xai-grok-shell/src/session/plan_mode.rs`

Pattern to copy:

- Pure tracker, no I/O  
- Explicit state enum + transitions  
- Snapshot struct for disk  
- Mid-turn enter/exit  
- SessionActor owns tracker behind mutex  

**New file (proposed):** `crates/codegen/xai-grok-shell/src/session/effort_mode.rs`

**Wire-up:** SessionActor fields + `handle_*` in `session/acp_session_impl/` (see `session_mode.rs` for plan toggle).

### 3.3 Plan vs Effort dimensions

| File | Notes |
|------|--------|
| `crates/codegen/xai-grok-tools/src/types/session_mode.rs` | `SessionMode::{Default,Plan,Ask}` — **do not overload** with Expert/Heavy |
| `session/acp_session_impl/session_mode.rs` | Plan toggle + prompt mode |

Effort is a **sibling** dimension. If ACP needs an effort id later, introduce a separate id type or session update channel rather than stuffing Expert into `SessionMode`.

### 3.4 Subagent catalog and spawn

| Surface | Path |
|---------|------|
| Builtin subagents | `crates/common/xai-tool-types/src/task.rs` (`builtin_subagent_by_name`) |
| Resolution | `crates/codegen/xai-grok-subagent-resolution` |
| Spawn / wait tools | `xai-grok-tools` implementations (task tools) |

Hard gates (Phase 4) may need:

- Interception or post-hoc validation of spawn counts under EffortMode  
- Join helper that enforces `wait_all` + success criteria  
- Prefer product-owned `EffortRuntime` over relying solely on model obedience  

### 3.5 Skills path (coexistence)

Skill discovery / slash:

- `xai-grok-tools` skills implementation  
- Shell resolve: `InvokeSkill` in `slash_commands.rs`  

When builtins ship, skill packages of the same name must not double-apply policy. See [05-skills-coexistence-and-migration.md](./05-skills-coexistence-and-migration.md).

### 3.6 TUI / pager

- Mode pill patterns for plan / permission — locate parallel widgets under `xai-grok-pager` / `pager-render`.  
- Subagent transcript inspectability already exists; Effort UI should surface **ledger** + progress S/N.

### 3.7 Telemetry

- Pattern: `PlanModeToggled` in shell + telemetry events crate.  
- Add: `EffortModeToggled`, `EffortTeamStarted`, `EffortTeamCompleted { s, n, partial }`.

---

## 4. Suggested module layout

```text
xai-grok-shell/src/session/
  effort_mode.rs          # tracker + snapshot (new)
  plan_mode.rs            # existing twin
  slash_commands.rs       # register builtins
  acp_session_impl/
    effort_mode.rs        # handle SetEffortMode, inject, team hooks (new)
    session_mode.rs       # plan remains here
    slash_exec.rs         # dispatch

xai-grok-tools/src/types/
  effort_mode.rs          # shared EffortMode enum if pager needs it (new)
  session_mode.rs         # unchanged Plan/Ask/Default
```

Optional later:

```text
xai-grok-shell/src/session/effort_runtime/
  ledger.rs
  join.rs
  replace.rs
  triviality.rs
```

---

## 5. Dependency rules

1. `EffortModeTracker` stays pure (like plan) for unit tests.  
2. Runtime spawn/join may live in shell or tools; avoid circular deps — prefer shell orchestration calling tool APIs the agent already has.  
3. Pager depends on wire types only (shared enum), not shell internals.  
4. Config types crate for profile structs if config grows.

---

## 6. Build / test locations

| Kind | Where |
|------|--------|
| Unit (tracker FSM) | `effort_mode.rs` `#[cfg(test)]` or shell tests |
| Slash resolve | shell unit tests next to slash_commands |
| Plan+Effort matrix | `acp_session_tests/` patterns (`plan_mode_*_tests.rs`) |
| Subagent join | tools/shell integration tests with mocked children |
| E2E | pager harness / shell tests as existing e2e style |

---

## 7. Inventory notes from clone HEAD

- Open-source publish commit: `c1b5909` (“Publish harness and TUI open-source”).  
- No existing `expert`/`heavy` builtin entries in `BUILTIN_COMMANDS` at Phase 1 open.  
- Plan mode is the strongest first-party sticky-mode implementation to mirror.
