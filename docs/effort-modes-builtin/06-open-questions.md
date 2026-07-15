# 06 — Open Questions

**Status:** Phase 1 draft  
**Date:** 2026-07-15  
**Resolve before or during Phase 2 freeze**

---

## Q1 — Persist EffortMode across process resume?

**Context:** Research Spec 01 said session modes do not restore across restarts. Product **Plan** mode persists via `plan_mode.json`.

**Options:**

| Option | Pros | Cons |
|--------|------|------|
| **A. Persist** (recommended) | Matches plan UX; Heavy sessions resume cleanly | Diverges from early research prose |
| B. Session-lifetime only | Closer to original spec | Surprising loss on resume |

**Proposal:** A. Document the deliberate product choice.

---

## Q2 — Separate `EffortMode` vs extend `SessionMode`?

**Context:** `SessionMode` is `Default | Plan | Ask`.

**Proposal:** **Separate** `EffortMode`. Extending `SessionMode` couples plan write gates to effort depth and breaks orthogonality.

---

## Q3 — Empty `/expert` without model turn?

True builtins can set state with zero model involvement. Skills always needed a model read.

**Proposal:** Shell handles empty form fully (status line + mode set). Args form may still start a model turn for the task.

---

## Q4 — How hard is “block execute before synthesis”?

Mutating tools are widespread. Options:

1. Soft: policy + post-hoc detection  
2. Hard: gate write tools on SessionActor while `Pursuing` pre-synthesis  
3. Hybrid: hard gate only for Heavy  

**Proposal:** Soft in Phase 3; hard gate for both modes in Phase 4 if implementation cost is acceptable; else Heavy-first hard gate.

---

## Q5 — Feature flag?

**Proposal:** `effort_mode_builtins` (or config) default **on** after ship, **off** during early PRs, for rollback per migration doc.

---

## Q6 — ACP / headless surface in v1?

Research deferred headless flags.

**Proposal:** Interactive slash + session persist first. ACP effort id only if pager already needs a wire field; otherwise later.

---

## Q7 — Worktrees per specialist?

**Proposal:** Main workspace v1 (skills parity). Optional later under config.

---

## Q8 — Upstream contribution vs private fork

This clone is public `zfifteen/grok-build`. Contribution path (PR upstream vs fork features) is a **principal** decision; docs assume local product work first.

---

## Q9 — Team size constants vs config

**Proposal:** Hard defaults 4/16; config may clamp but not exceed without explicit experimental flag (avoids silent DoD drift).

---

## Q10 — Relationship to grok.com Expert/Heavy

Product should mirror **mental model** (4 vs 16 depth), not server multi-agent API. Confirm marketing/docs language avoids claiming identical backend.

---

## Decision log

| Date | Decision | Owner |
|------|----------|-------|
| 2026-07-15 | Open builtin program; Phase 1 docs in product repo | Principal + Grok |
| | | |
