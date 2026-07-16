# Effort Modes Builtin — Verification

**Date:** 2026-07-16

## Commands

| Step | Result |
|------|--------|
| `cargo test --test effort_mode_gates --test effort_slash_launch` | **25/25 pass** (21 gates + 4 slash) |
| `cargo check -p xai-grok-pager -p xai-grok-shell` | **green** |

## Full TUI chrome (shell-driven)

| Acceptance | Wiring |
|------------|--------|
| Sticky Expert/Heavy indicator | `SessionUpdate::EffortModeUpdated` → pager `effort_chrome` status chip |
| Live S of N | `format_effort_chrome_label` → e.g. `Expert 2 of 4`, `Heavy 3 of 16` |
| Partial / Waived | `Expert Partial 3 of 4`, `Heavy Waived`; `/normal` clears label |
| Shell state, not model prose | `emit_effort_chrome_update` on slash, turn-start, specialist finish, abort, normal |

Emit sites: `session_mode.rs` (`emit_effort_chrome_update`).  
Pager: `session_notification.rs` `apply_effort_mode_chrome` → `agent_view` status segment `"effort"`.

## Production path (mandatory team)

| Concern | Production wiring |
|---------|-------------------|
| Bind spawn → ledger slot | `updates.rs` SubagentSpawned → `effort_bind_spawned_subagent` |
| Record outcome + claim | SubagentFinished → `effort_on_subagent_finished` → finalize |
| Turn-end finalize | `turn.rs` `handle_turn_end` → `effort_on_turn_end` |
| Abort unlock | `on_session_user_cancel` → `abort_team` + PartialReport |
| Mandatory fan-out | `maybe_run_mandatory_effort_team` (Expert N=4 / Heavy N=16) |

## Test highlights

- `effort_chrome_labels_mode_progress_partial_waived_and_normal_clears`
- `effort_chrome_wire_clears_on_normal`
- `production_join_path_finalizes_and_unlocks_execute`
- `execute_blocked_until_production_finalize`
- `hard_stop_continue_one_wave_per_grant_then_hard_stop_again`
