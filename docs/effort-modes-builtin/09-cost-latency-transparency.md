# Effort cost / latency transparency (issue #8)

Operator-visible **pre-flight**, **live elapsed**, and **post-run footer** for sticky Expert/Heavy multi-agent teams.

## Surfaces

| When | What |
|------|------|
| Before fan-out | System reminder: mode, **N**, approximate cost class (≈4× / ≈16×), join floor, abort via `/normal` |
| First Heavy | Still needs `--confirm` (issue #13); preflight restates the gate |
| While `Pursuing` | Chrome: `Expert 2 of 4 · 3m12s · brain_id` |
| Soft budget | Env `GROK_EFFORT_SOFT_BUDGET_N` — **warn only** if N exceeds (no crash) |
| After join / abort | One-line footer: specialists · elapsed · cost class (approx; not exact $) |

## Operator commands

```text
/expert …          # preflight on first non-trivial team begin
/heavy --confirm … # first Heavy unlock + preflight
/normal            # abort remaining → partial S of N (never full-team claim)
```

Env:
- `GROK_EFFORT_SOFT_BUDGET_N=<n>` — optional soft warn
- `GROK_HEAVY_AUTO_CONFIRM=1` — skip first-Heavy confirm (power users / CI)

## Non-goals

- Not a billing product or exact dollar meter.
- Does not change fixed N=4/16.
- Does not auto-downgrade Heavy.

Implementation: `effort_mode.rs` (`format_effort_preflight`, `team_elapsed_secs`, chrome `elapsed_secs`) + inject in `session_mode.rs`.
