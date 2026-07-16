# Effort Modes Builtin — Verification (post-synthesize wiring)

**Date:** 2026-07-16  
**SCRATCH:** `/var/folders/k_/spz3zlj566sc4qh29g0tk6jh0000gn/T/grok-goal-e648e7148bdc/implementer`

## Commands

| Step | Result |
|------|--------|
| `cargo test -p xai-grok-shell --test effort_mode_gates --test effort_slash_launch` | **20/20 pass** → `effort-gates.log` |
| `cargo check -p xai-grok-shell --lib` | **green** |

## Production path (skeptic gaps closed)

| Concern | Production wiring |
|---------|-------------------|
| Bind spawn → ledger slot | `updates.rs` SubagentSpawned → `effort_bind_spawned_subagent` → `assign_next_pending_task` |
| Record outcome + claim | `updates.rs` SubagentFinished → `effort_on_subagent_finished` → `on_session_specialist_finished` → `try_finalize_synthesis` → `mark_synthesis_complete` |
| Turn-end finalize | `turn.rs` `handle_turn_end` → `effort_on_turn_end` → `on_session_turn_end` |
| Abort unlock | `on_session_user_cancel` → `abort_team` + `mark_partial_synthesis` |
| Hard-stop continue | Per-grant one wave; after join still short → hard-stop again (§4.4) |

## Test highlights

- `production_join_path_finalizes_and_unlocks_execute` — full join path unlocks writes  
- `execute_blocked_until_production_finalize` — mid-team blocked; 4th success unlocks  
- `hard_stop_continue_one_wave_per_grant_then_hard_stop_again` — multi-cycle continue  
- Under-count claim still denied without synthesis  
