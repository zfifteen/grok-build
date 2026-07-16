# 00 — Program Charter: Builtin Effort Modes

**Status:** Phase 1 draft  
**Date:** 2026-07-15  
**Authority:** Principal direction in #grok-build — reopen original product implementation goal; Phase 1 = documentation.

---

## 1. Mission

Ship **Expert**, **Heavy**, and **Normal** as **first-party Grok Build session effort modes**: shell builtins, sticky session state, TUI chrome, and (for non-trivial work) **hard local multi-agent orchestration** with fixed team sizes.

| Mode | Fixed successful specialists | Notes |
|------|------------------------------|--------|
| **Normal** | none required | Default Grok Build |
| **Expert** | **exactly 4** | High quality, interactive team depth |
| **Heavy** | **exactly 16**, ≥1 contrarian | Maximum local multi-agent depth |

Mechanism: local `spawn_subagent` fan-out → join-all → leader synthesis → optional post-N execute.  
**Not** the platform multi-agent research API as the product path.

---

## 2. Goals

1. **Real builtins** — `/expert`, `/heavy`, `/normal` win shell name resolution the same way other `BuiltinCommand` entries do (not skill markdown alone).
2. **Sticky session state** — empty slash sets mode without requiring a model “policy read”; mode survives turns and compact per product rules (process restart: see open questions; plan mode *does* persist today).
3. **Orthogonal composition** with Plan mode (`SessionMode::Plan`) and permission / always-approve modes. Effort mode must not force always-approve.
4. **Hard fixed-N gates** for non-trivial work (4 / 16), with Spec 13–14 join, replace caps, abort → partial, and solo waiver semantics.
5. **Analytic-only fixed team** — no repo writes inside the N specialists; execute after synthesis, outside N.
6. **Maximum transparency** — ledgers and specialist outcomes inspectable in TUI / session tools.
7. **Honest migration** from skills-first v1 without double-running skill policy and builtin runtime.

---

## 3. Non-goals (Phase 1–4)

1. Pixel-perfect grok.com Heavy UI.
2. Product flow that *depends* on the platform multi-agent research API.
3. Replacing Plan mode or permission modes.
4. Forcing multi-agent on trivial messages (typos, pure lookups).
5. Default worktree isolation for every specialist (v1 default: main workspace; optional later).
6. Headless `--effort-mode` / ACP effort surface as a Phase 2 blocker (document, implement later unless cheap).
7. Deleting user skills on day one of builtin ship (coexistence window required).

---

## 4. Success criteria (program)

Phase 1 succeeds when:

- Full tech specs, architecture map, implementation plan, test plan, migration plan, and open questions exist under `docs/effort-modes-builtin/`.
- Specs cite **real** product crates and patterns (especially Plan mode and slash builtins).
- Principal can freeze Phase 1 and authorize Phase 2 scaffolding.

Program success (later phases):

- `/expert` / `/heavy` / `/normal` are builtins in `BUILTIN_COMMANDS`.
- Non-trivial Expert/Heavy turns cannot finalize as “full team” with under-count success.
- Normal mode leaves effort runtime idle (regression suite).
- Skills no longer double-orchestrate when builtins are present.

---

## 5. Prior art and conflict rules

| Layer | Wins on |
|-------|---------|
| Spec 10 (+13/14 skill ops intent) | Behavioral contract (team sizes, join, execute order, solo, abort) |
| **This package** | Product-source delivery design once Phase 1 freezes |
| Spec 12 skills | Historical v1 ship; demoted when builtins ship |
| Live skill text | Best-effort approximation until builtins land |

When product architecture conflicts with research prose, **record the decision in [06-open-questions.md](./06-open-questions.md)** and update the tech spec; do not silently diverge.

---

## 6. Analogues in this codebase

| Existing feature | Why it matters |
|------------------|----------------|
| `PlanModeTracker` (`session/plan_mode.rs`) | Pure state machine + persistence + mid-turn transitions — **primary pattern for EffortMode** |
| `GoalTracker` (`session/goal_tracker.rs`) | Second pure SessionActor twin — history/cap/wire-hardening patterns for ledgers |
| ACP `SessionMode` (`Default` / `Plan` / `Ask`) | Prompt/plan dimension — keep **orthogonal** to EffortMode |
| `BUILTIN_COMMANDS` (`session/slash_commands.rs`) | Registration, gates, `BuiltinAction` resolution; flag-off must omit effort names |
| Skill slash resolution (`InvokeSkill`) | Current skill path; loses when builtins registered; reclaims when flag unregisters names |
| Builtin subagents (`explore`, `general-purpose`, `plan`) | Roster building blocks for fixed teams |
| Subagent spawn/wait tools | Join contract implementation surface |

---

## 7. Constraints

- **Restricted channel wakes** may limit heavy shell during docs/impl; prefer work in this repo tree.
- Do not dump secrets into docs.
- Affirmative product prose in user-facing guides; specs may use precise normative language.
