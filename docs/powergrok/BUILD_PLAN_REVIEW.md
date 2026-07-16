# Powergrok Build Plan Review

**Review Date:** 2026-07-15  
**Target Document:** `docs/powergrok/BUILD_PLAN.md`  
**Reviewer:** Antigravity AI (Peer Review)

## Supersession notice

This review was written against the **first draft** of `BUILD_PLAN.md` (install-only / near-zero engine change).

**Operator decisions locked later the same day** (see `BUILD_PLAN.md` §0) **change the v1 scope**, in particular:

| First-draft assumption in this review | Locked decision |
|---------------------------------------|-----------------|
| “Zero product code changes (v1)” | **Invalid for v1** — project isolation is required via argv0 `powergrok` → repo `.powergrok/` (D5–D7) |
| Shared project `.grok/` is fine | **Rejected** — full project tree under `.powergrok/` only; no fallback/merge |
| Open questions §15 still open | **Mostly closed** in `BUILD_PLAN.md` §0 |

**Use `BUILD_PLAN.md` (post-decision revision) as the source of truth.**  
A fresh peer review should re-assess the **engine** work: `project_config_dirname()`, call-site conversion, argv0/`exec -a` contract, and verification V11–V14.

The original findings below are retained for history only.

---

## 1. Overall Assessment (first draft)

The first-draft `BUILD_PLAN.md` was exceptionally well-structured and detailed. It correctly identified the constraints of installing a source-built version of the CLI (`powergrok`) alongside an officially managed binary (`grok`). The strategy to achieve **user-home** isolation via `GROK_HOME` rather than invasive branding forks remains robust. Risk analysis and mitigation for install layout remain useful for Phases 2–3.

## 2. Strengths still valid after decisions

* **Wrapper script safety** — abort if `GROK_HOME` resolves to official `~/.grok`.
* **Auto-update mitigation** — seed `auto_update = false` only; create-if-missing.
* **Rollback** — `xai-grok-pager.prev` and `VERSION` stamp.
* **`OnceLock` / env-at-exec** — still the correct model for user home.
* **Lib + wrapper layout** — locked as D2.

## 3. First-draft findings (historical)

### 3.1 `OnceLock` and environment injection

Safe when the wrapper `exec`s with env set. Smoke tests for `~/.powergrok` remain required.

### 3.2 Seed config idempotency

Create-if-missing only; do not merge on reinstall.

### 3.3 Fish completions caveat

Still relevant: keep powergrok completions under `~/.powergrok/completions/` (locked D8); avoid writing `~/.config/fish/completions/grok.fish`.

## 4. First-draft answers to open questions (historical)

Superseded by `BUILD_PLAN.md` §0. For the record, first-draft reviewer leanings matched many later locks (`~/.powergrok`, lib+wrapper, dedicated branch) but **underestimated** project isolation (now mandatory engine work).

---

*Re-review requested against the post-decision plan before Phase 1 implementation.*
