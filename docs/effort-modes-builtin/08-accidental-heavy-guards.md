# Accidental-Heavy guards (issue #13)

Product surface on sticky Expert/Heavy to prevent bill-shock and opaque waivers.

## Guards

| Guard | Behavior |
|-------|----------|
| **Sticky chrome** | Elevated mode always labels the status bar (`Expert` / `Heavy` / progress / Solo / Trivial). |
| **Resume notice** | Restoring sticky elevated mode sets chrome `· resumed` and injects a one-shot system reminder. |
| **First Heavy confirm** | First multi-agent Heavy team this session needs `--confirm` (or `GROK_HEAVY_AUTO_CONFIRM=1`). Chrome: `Heavy · confirm first team`. |
| **Trivial waiver** | Conservative short-circuit → chrome `Expert Trivial` / `Heavy Trivial` (not fake S of N). |
| **Solo** | `--solo` → chrome `… Solo`; no full team. |
| **Force team** | `--force-team` runs full N even if classifier would waive trivial (**does not** skip first-Heavy confirm). |

## Operator commands

```text
/heavy                         # sticky Heavy; first team still needs confirm
/heavy --confirm <task>        # unlock + run team (flags survive slash → turn inject)
/heavy --solo <task>           # single-leader this turn
/heavy --confirm --force-team fix typo  # unlock + force team despite trivial-looking text
/expert --force-team fix typo  # full Expert team despite trivial-looking text
/normal                        # clear sticky mode
```

Env (power users / CI): `GROK_HEAVY_AUTO_CONFIRM=1` skips the first-Heavy confirm gate.

**Note:** `--force-team` alone never unlocks a locked Heavy session — confirm first (or env).

## Non-goals

- Does not remove stickiness.
- Does not change fixed N=4/16.
- Does not auto-reset mode on trivial messages.

See `effort_mode.rs` (`WaiverReason`, `parse_effort_turn_flags`, `on_session_turn_start`).
