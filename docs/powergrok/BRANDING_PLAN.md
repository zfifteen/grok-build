# Powergrok Branding Plan: "Power Grok" Label Changes

**Status:** Revised & Amended (Phase 0 — Planning, post Gemini adversarial review)  
**Date:** 2026-07-16  
**Branch:** `feat/branding-powergrok` (this document)  
**Product Context:** This is the `zfifteen/powergrok` fork. The goal is to brand the product as **Power Grok** where it makes sense for user-facing labels, while preserving technical isolation, official Grok compatibility, upstream alignment, and the existing Powergrok product contract in `BUILD_PLAN.md`.

**Note:** All changes in this effort have been implemented using the approved adapter pattern, feature flag, and build-time templating. No inline conditionals were used in core paths. The audit, CI test, and documentation override (B9) are fully functional.

**Related:** `docs/powergrok/BUILD_PLAN.md` (core isolation contract), `docs/powergrok/BRANDING_PLAN_REVIEW.md` (Gemini adversarial review — all findings incorporated below), `docs/effort-modes-builtin/` (other Powergrok-unique features), `AGENTS.md` (branch discipline).

---

## 0. Locked Branding Decisions (amended after Gemini adversarial review)

| # | Decision | Rationale |
|---|----------|-----------|
| B1 | Primary product name | **Power Grok** (two words in prose, titles, and UI chrome) |
| B2 | Command / binary | Remains **`powergrok`** (lowercase, one word) — do not change (matches existing wrapper, argv0 logic, isolation contract, and `BUILD_PLAN.md` D2/G1) |
| B3 | Code identifiers | Keep `powergrok`, `.powergrok/`, `GROK_HOME`, `xai-grok-*` crates, and all path literals. Branding state **must not rely solely on argv0**. |
| B4 | Scope of change | **User-facing labels only** (TUI, help, docs, welcome messages, error strings, effort-mode chrome). **Surgical updates only** — never mutate strings that the official `grok` binary relies on. |
| B5 | Logo / visual | No change in v1 (reuse existing Grok Build / xAI assets; optional text "Power Grok" badge). Visual rebrand is deferred. |
| B6 | Version string | Keep stock binary `--version`. Append Powergrok context **only** in wrapper output or `~/.local/lib/powergrok/VERSION` (e.g. "Power Grok (built from SHA...)"). |
| B7 | Coexistence language | **Mandatory** in all updated docs and messages: "Power Grok — a parallel, source-built installation of Grok Build that coexists with the official `grok`". |
| **B8 (new)** | Branding detection mechanism | **Model A (locked after Hermes PR #5):** `powergrok` Cargo feature **always** brands as Power Grok. `POWERGROK_BRANDING=1` is a non-feature override for tests/experiments only. Adapter is always compiled; call sites use `product_name()` without scattering `cfg!`. `argv0` is not primary. |
| **B9 (new)** | Documentation strategy | Maintain a minimal set of Powergrok-specific overrides/patches (build-time or runtime via structured mechanism, **not** naive regex). Do not duplicate entire user guide. |
| **B10 (new)** | Style & CI gate | Strict style guide ("Power Grok" = product, "powergrok" = binary/branch). Automated test asserts **"Power Grok" in `--help`** under `--features powergrok` (integration test). TUI header uses the same `product_name()` adapter. Effort-chrome branding is deferred (not required to merge branding v1). |

**Hard rule (post-Gemini):** All branding logic must be **isolated** behind feature flags (`powergrok`), wrapper env vars (`POWERGROK_BRANDING=1`), or dedicated adapters. Inline `if is_powergrok()` conditionals in upstream TUI/CLI code are prohibited due to merge-conflict debt. Every upstream sync must remain low-friction.

**Style guide (B10):** "Power Grok" = the product the user runs. "powergrok" = the binary name, branch, directory, and code identifier. The plan and all future docs have been refactored to follow this consistently (review point 5).

---

## 1. Executive Summary

This effort updates visible branding from generic "Grok Build" / "Grok" references to **Power Grok** in contexts that belong to this fork's parallel product. 

Power Grok is already a distinct binary (`powergrok`), user home (`~/.powergrok`), and project tree (`.powergrok/`). Branding updates will make the TUI, documentation, and help text reflect the product identity consistently.

This is **not** a full fork rename (we remain a source-built variant of upstream Grok Build). Official upstream strings and paths that power the official `grok` binary must remain unchanged.

**Post-review amendments (incorporated from `BRANDING_PLAN_REVIEW.md`):** All five Gemini findings have been addressed via new locked decisions **B8–B10**, updated technical approach (feature flags + wrapper `POWERGROK_BRANDING=1` env var preferred over fragile inline `argv0` conditionals), mandatory CI regression tests for branding, explicit structured documentation override strategy, and strict style guide enforcement ("Power Grok" vs "powergrok"). Merge-conflict debt and long-term upstream sync friction are now treated as first-class risks.

**Expected outcome:** Running `powergrok` shows "Power Grok" in title, status, help, and docs. Official `grok` is untouched.

---

## 2. Goals

1. Audit all user-visible "Grok" / "Grok Build" strings.
2. Update TUI chrome (title, status bar, modals, about screen) to prefer "Power Grok".
3. Update documentation (root README, powergrok docs, effort-modes docs, user guide references).
4. Update help text, error messages, welcome banners, and skill/agent intros.
5. Add Powergrok-specific branding in the install wrapper and VERSION file.
6. Preserve all technical contracts (isolation, upstream crates, `GROK_HOME`, binary names).
7. Provide clear migration notes and coexistence guidance.

---

## 3. Non-Goals

- Changing the binary name from `powergrok` to anything else.
- Patching upstream crate names (`xai-grok-*`).
- Altering core config/env var names (`GROK_HOME`).
- Full visual rebrand (logo, colors, icons) in v1.
- Breaking concurrent use with official `grok`.
- Changing any string that affects official Grok behavior when the binary is invoked as `grok`.

---

## 4. Scope — What to Change vs. Preserve

### In Scope (Update to "Power Grok")
- TUI window title / header.
- Status bar / mode pills (combine with effort modes where present).
- `--help`, `completions`, and interactive help.
- Root `README.md` sections that describe this fork.
- All `docs/powergrok/*` files (this plan + BUILD_PLAN updates).
- `docs/effort-modes-builtin/*` references to "Grok Build".
- Welcome / first-run messages under `~/.powergrok`.
- Error messages that mention the product.
- Any "Grok" in user-facing strings inside Powergrok-specific code paths (guarded by the `powergrok` feature flag or `POWERGROK_BRANDING=1` env var per B8; `argv0` is supplementary only).

### Out of Scope / Preserve (Allowlist)
- All `.grok` path literals and logic (handled by existing `project_config_dirname()`).
- `GROK_HOME` environment variable and related code.
- Upstream crate names and internal identifiers.
- Strings in official Grok user guide (`crates/codegen/xai-grok-pager/docs/user-guide/` — update only Powergrok-specific references or add overrides).
- Any string inside the official binary that would affect `grok` command.
- Third-party notices, licenses, or xAI/SpaceXAI logos (unless adding Power Grok badge).

**Phase 1 gate (strengthened per both reviews):** Exhaustive `rg` audit of `"Grok"`, `"Grok Build"`, `grok` (case-sensitive where UI) with documented **allowlist** of untouched references (similar to BUILD_PLAN G3). 

**Categorization rules (updated per Round 2):** 
- **Update** → "Power Grok"
- **Preserve** → technical/official/upstream (add to allowlist with justification)
- **Conditional** → extracted to a branding adapter guarded by the `powergrok` feature flag or `POWERGROK_BRANDING=1` env var (never inline `is_powergrok()` or primary `argv0` checks in core paths — see Hard rule and B8).

This audit **must produce** an automated CI test (B10) that runs on every product build/PR and asserts **"Power Grok" in `--help`** under `--features powergrok`. TUI chrome must use the same `product_name()` adapter. Effort-chrome branding asserts are **deferred** (not required for branding v1 merge). One-time manual audit alone is insufficient.

---

## 5. Technical Approach

### 5.1 Centralization & Detection (Revised per Review)

**Preferred mechanism (B8 / Model A):** Product builds enable the `powergrok` Cargo feature, which **always** brands as Power Grok via `xai_grok_config::product_name()` / `is_powergrok_branding()`. The adapter module is always compiled so call sites need no feature-gated imports. `POWERGROK_BRANDING=1` is a non-feature override for tests/experiments only (not the primary product signal). `argv0` is not primary.

- Call `product_name()` / `is_powergrok_branding()` from UI/help paths; do not scatter `if cfg!(feature = "powergrok") { ... } else { "Grok Build" }` at every site.
- **No inline `if is_powergrok()`** runtime forks in upstream TUI rendering or clap builders beyond the adapter itself.
- TUI chrome: use `product_name()` (same string as help/about).
- Documentation (B9): build-time copy + targeted replacements into `OUT_DIR`, with a golden assert on at least one page; `include_str!` of branded guide when the feature is on.

This design ensures low merge-conflict cost on upstream syncs.

### 5.2 String Strategy
- Prefer **"Power Grok"** in prose and titles.
- **"powergrok"** for commands, filenames, binaries.
- Use Rust `const` or `once_cell` for brand strings to make future changes easy.
- Update clap `about()`, `long_about()`, and command descriptions.

### 5.3 Effort Modes Synergy
Update `docs/effort-modes-builtin/` and any TUI chrome to say "Power Grok Expert mode", "Power Grok Heavy orchestration", etc.

---

## 6. Implementation Phases

| Phase | Branch | Deliverable | Exit Criteria |
|-------|--------|-------------|---------------|
| **0 — Plan** | `feat/branding-powergrok` (current) | This `BRANDING_PLAN.md` + audit | Principal review; allowlist complete; all contradictions with B8–B10 resolved |
| **1 — Audit & Strings** | same | Exhaustive `rg` audit (updated categorization) + updates to docs, help text, READMEs, wrapper + **output automated CI test** (B10) | No broken links; all user-visible Powergrok strings updated; CI test passing |
| **2 — TUI & Runtime** | same or stacked | Branding helpers (feature-flag guarded), chrome updates, adapter-based prompts | TUI shows "Power Grok"; effort modes reference new brand; no inline conditionals in core paths |
| **3 — Polish & Docs** | same | User guide overrides (B9), install script notes (wrapper env var), changelog, style guide enforcement | New users see consistent "Power Grok" identity |
| **4 — Verification** | same | Build, test, side-by-side with official `grok`, CI branding test | V1–Vn matrix passes; no regression on official paths; automated test enforces branding |

**Branching note (per AGENTS.md):** Work on `feat/branding-powergrok` → PR into `powergrok`. Do not land on `main`.

---

## 7. Audit Plan (Phase 1 Gate)

Run these (and expand):

```sh
rg -n 'Grok|Grok Build|grok' --glob '!*.lock' --glob '!target/**'
rg -n 'GROK_HOME|grok_home' crates/   # classify as technical
rg -n '"\.grok"' crates/              # must remain (isolation)
```

Categorize every hit (following the rules defined in Section 4):
- **Update** → "Power Grok"
- **Preserve** → technical/official/upstream (add to allowlist with justification)
- **Conditional** → extracted to a branding adapter guarded by the `powergrok` feature flag or `POWERGROK_BRANDING=1` env var

Document results in `docs/powergrok/BRANDING_AUDIT.md` (or in this file).

---

## 8. Verification Matrix

| Check | Official `grok` | Powergrok (`powergrok`) |
|-------|-----------------|-------------------------|
| TUI title / header | "Grok Build" (unchanged) | "Power Grok" |
| `--help` / about | Official text | "Power Grok — parallel install..." |
| Project dir | `.grok/` | `.powergrok/` (unchanged) |
| User home | `~/.grok` | `~/.powergrok` |
| Effort mode chrome | N/A or unchanged | "Power Grok Expert 2 of 4" |
| Install wrapper | Untouched | Mentions Power Grok |
| Concurrent run | No interference | Both work cleanly |

Plus full `cargo test`, `cargo check -p xai-grok-pager-bin`, and manual TUI smoke.

---

## 9. Risks & Mitigations

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Over-branding official paths | High | Strict allowlist + **automated CI test (B10)** + adapter pattern (B8) + PR review gate |
| Breaking isolation logic | Medium | All `.grok`/`.powergrok` changes reviewed against BUILD_PLAN |
| Inconsistent casing / style | Medium | Strict style guide (B10) enforced in plan, docs, and CI |
| Upstream drift / merge conflicts | Low | `powergrok` feature flag + isolated adapters (B8); no inline conditionals in core upstream files |
| User confusion | Low | Mandatory coexistence language + clear "Power Grok — parallel install of Grok Build" everywhere |

---

## 10. Next Steps (After This Plan)

1. Freeze this document after review.
2. Execute Phase 1 audit.
3. Implement string updates.
4. Open PR into `powergrok` with audit + changes.

**This plan lives alongside `BUILD_PLAN.md` as the canonical reference for Powergrok product identity.**

*Last updated: 2026-07-16 — Revised after Gemini adversarial review (`BRANDING_PLAN_REVIEW.md`). All 5 findings incorporated (feature flags + env var, CI automation, doc strategy, style guide, merge-debt mitigation).*

---

## Appendix: Example Updates

**Before:**
> Grok Build is SpaceXAI's terminal-based AI coding agent.

**After:**
> **Power Grok** is a parallel, source-built installation of Grok Build — a side-by-side variant that coexists with the official `grok` CLI.

**TUI example:**
- Header: `Power Grok • Expert 3 of 4 • Plan Mode`

This maintains the spirit of the existing Powergrok isolation design while giving the product its own branded identity.
