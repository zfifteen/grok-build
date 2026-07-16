# Powergrok Branding Plan: "Power Grok" Label Changes

**Status:** Revised & Approved (Phase 0 — Planning, post self-review)  
**Date:** 2026-07-16  
**Branch:** `feat/branding-powergrok` (this document)  
**Product Context:** This is the `zfifteen/powergrok` fork. The goal is to brand the product as **Power Grok** where it makes sense for user-facing labels, while preserving technical isolation, official Grok compatibility, upstream alignment, and the existing Powergrok product contract in `BUILD_PLAN.md`.

**Related:** `docs/powergrok/BUILD_PLAN.md` (core isolation contract), `docs/effort-modes-builtin/` (other Powergrok-unique features), `AGENTS.md` (branch discipline).

---

## 0. Locked Branding Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| B1 | Primary product name | **Power Grok** (two words in prose/UI) |
| B2 | Command / binary | Remains **`powergrok`** (lowercase, one word) — do not change (matches existing wrapper, argv0 logic, and isolation) |
| B3 | Code identifiers | Keep `powergrok`, `.powergrok/`, `GROK_HOME` (when referring to the env var), `xai-grok-*` crate names (upstream) |
| B4 | Scope of change | **User-facing labels only**: TUI chrome, help text, READMEs, docs, welcome messages, version output notes, error messages, skill/agent descriptions. **Do not** touch official Grok paths, upstream strings that affect compatibility, or core engine identifiers. |
| B5 | Logo / visual | No change in v1 (reuse Grok Build / xAI assets or add simple "Power Grok" text badge). Future phase may add custom imagery. |
| B6 | Version string | Keep stock binary `--version`. Add optional Powergrok suffix in wrapper or VERSION file only (e.g. "Power Grok build from SHA..."). |
| B7 | Coexistence language | Always clarify "Power Grok (parallel install of Grok Build)" in docs to avoid confusion with official `grok`. |

**Hard rule (reinforced post-review):** All changes must be **guarded** (e.g. `if is_powergrok()`) or limited to Powergrok-specific files (`docs/powergrok/`, wrapper, effort-modes docs). Never edit core upstream user-guide files that official binaries load. Changes are **additive/conditional**, not destructive.

---

## 1. Executive Summary

This effort updates visible branding from generic "Grok Build" / "Grok" references to **Power Grok** in contexts that belong to this fork's parallel product. 

Power Grok is already a distinct binary (`powergrok`), user home (`~/.powergrok`), and project tree (`.powergrok/`). Branding updates will make the TUI, documentation, and help text reflect the product identity consistently.

This is **not** a full fork rename (we remain a source-built variant of upstream Grok Build). Official upstream strings and paths that power the official `grok` binary must remain unchanged.

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
- Any "Grok" in user-facing strings inside Powergrok-specific code paths (guarded by argv0 or `GROK_HOME`).

### Out of Scope / Preserve (Allowlist)
- All `.grok` path literals and logic (handled by existing `project_config_dirname()`).
- `GROK_HOME` environment variable and related code.
- Upstream crate names and internal identifiers.
- Strings in official Grok user guide (`crates/codegen/xai-grok-pager/docs/user-guide/` — update only Powergrok-specific references or add overrides).
- Any string inside the official binary that would affect `grok` command.
- Third-party notices, licenses, or xAI/SpaceXAI logos (unless adding Power Grok badge).

**Phase 1 gate:** Exhaustive `rg` audit of `"Grok"`, `"Grok Build"`, `grok` (case-sensitive where UI) with documented **allowlist** of untouched references (similar to BUILD_PLAN G3).

---

## 5. Technical Approach

### 5.1 Centralization (Recommended)
Introduce helpers where feasible (new or extend existing):

- In `xai-grok-config` or a new `powergrok-branding` leaf: `product_name()` / `product_brand()` that returns `"Power Grok"` when argv0 is `powergrok`.
- TUI chrome: extend `apply_effort_mode_chrome` style updates or add `apply_branding()`.
- Prompt injection / system reminders: conditional "You are Power Grok, a parallel install of Grok Build...".

Fallback to "Grok Build" for official paths.

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
| **0 — Plan** | `feat/branding-powergrok` (current) | This `BRANDING_PLAN.md` + audit | Principal review; allowlist complete |
| **1 — Audit & Strings** | same | Exhaustive `rg` + updates to docs, help text, READMEs, wrapper | No broken links; all user-visible Powergrok strings updated |
| **2 — TUI & Runtime** | same or stacked | Branding helpers, chrome updates, conditional prompts | TUI shows "Power Grok"; effort modes reference new brand |
| **3 — Polish & Docs** | same | User guide additions, install script notes, changelog | New users see consistent "Power Grok" identity |
| **4 — Verification** | same | Build, test, side-by-side with official `grok` | V1–Vn matrix passes; no regression on official paths |

**Branching note (per AGENTS.md):** Work on `feat/branding-powergrok` → PR into `powergrok`. Do not land on `main`.

---

## 7. Audit Plan (Phase 1 Gate)

Run these (and expand):

```sh
rg -n 'Grok|Grok Build|grok' --glob '!*.lock' --glob '!target/**'
rg -n 'GROK_HOME|grok_home' crates/   # classify as technical
rg -n '"\.grok"' crates/              # must remain (isolation)
```

Categorize every hit:
- **Update** → Power Grok
- **Preserve** → technical/official/upstream (add to allowlist with justification)
- **Conditional** → guarded by `is_powergrok()` or argv0 check

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
| Over-branding official paths | High | Strict allowlist + PR review gate (audit doc required) |
| Breaking isolation logic | Medium | All `.grok`/`.powergrok` changes reviewed against BUILD_PLAN |
| Inconsistent casing | Medium | Central brand helper + style guide in plan |
| Upstream drift | Low | Keep changes in Powergrok-specific modules; merge main → powergrok regularly |
| User confusion | Low | Clear "Power Grok is a side-by-side variant of Grok Build" everywhere |

---

## 10. Next Steps (After This Plan)

1. Freeze this document after review.
2. Execute Phase 1 audit.
3. Implement string updates.
4. Open PR into `powergrok` with audit + changes.

**This plan lives alongside `BUILD_PLAN.md` as the canonical reference for Powergrok product identity.**

*Last updated: 2026-07-16 — Initial draft on `feat/branding-powergrok`.*

---

## Appendix: Example Updates

**Before:**
> Grok Build is SpaceXAI's terminal-based AI coding agent.

**After:**
> **Power Grok** is a parallel, source-built installation of Grok Build — a side-by-side variant that coexists with the official `grok` CLI.

**TUI example:**
- Header: `Power Grok • Expert 3 of 4 • Plan Mode`

This maintains the spirit of the existing Powergrok isolation design while giving the product its own branded identity.
