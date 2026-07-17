# Adversarial Review: Powergrok Branding Plan (`BRANDING_PLAN.md`) - Round 2

**Date:** 2026-07-16
**Reviewer:** Antigravity Agent
**Status:** Completed
**Target:** `docs/powergrok/BRANDING_PLAN.md`

The plan has successfully incorporated the structural and strategic findings from the previous review (such as adopting feature flags, env vars, CI automation, and a strict style guide). 

However, this Round 2 review identifies **internal inconsistencies and leftover text** from the original draft that contradict the newly added rules. These remnants will cause developer confusion during implementation if not cleaned up.

---

## 1. Contradictory Instructions in Section 7 (Audit Plan)
**Reference:** Section 7 (Categorize every hit)
The newly established "Hard rule (post-Gemini)" explicitly prohibits inline `if is_powergrok()` conditionals. However, Section 7 still instructs developers to categorize string hits as:
> "Conditional → guarded by `is_powergrok()` or argv0 check"
* **The Risk:** A developer following the Phase 1 audit instructions will start writing the exact inline conditionals that the new architectural rules forbid. 
* **Recommendation:** Update this categorization step to reflect the new feature flag / adapter pattern. (e.g., "Conditional → extracted to a branding adapter guarded by `#[cfg(feature="powergrok")]`").

## 2. Outdated Scope Definitions
**Reference:** Section 4 (In Scope)
The "In Scope" section states:
> "Any "Grok" in user-facing strings inside Powergrok-specific code paths (guarded by argv0 or `GROK_HOME`)."
* **The Risk:** This ignores the newly added `B8` decision which mandates using `POWERGROK_BRANDING=1` and feature flags, stating that `argv0` is "supplementary only."
* **Recommendation:** Change this bullet point to refer to the new `POWERGROK_BRANDING` env var and compile-time flags rather than `argv0`.

## 3. Stale Mitigations in the Risk Table
**Reference:** Section 9 (Risks & Mitigations)
The risk table was not updated to reflect the new, stronger mitigations introduced in Section 0.
* **The Risk:** 
  * For **"Over-branding official paths"**, the mitigation still relies solely on "Strict allowlist + PR review gate", completely ignoring the new B10 requirement for an automated CI test.
  * For **"Upstream drift"**, the mitigation does not mention the `powergrok` feature flag or the adapter pattern, which are the actual technical solutions preventing merge-conflict debt.
* **Recommendation:** Update the mitigations column to prominently feature the CI test (B10) and feature flags/adapters (B8).

## 4. Missing CI Implementation in Phases
**Reference:** Section 6 (Implementation Phases)
While `B10` introduces a strict requirement for a CI gate, this requirement is missing from the actual work breakdown.
* **The Risk:** The CI test might be forgotten or deferred if it is not an explicit deliverable. 
* **Recommendation:** Add the creation of the CI test to the Exit Criteria of either Phase 1 or Phase 4. The current Phase 1 exit criteria only mention the `rg` audit.
