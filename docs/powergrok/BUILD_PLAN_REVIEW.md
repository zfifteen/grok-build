# Powergrok Build Plan Review (Revised)

**Review Date:** 2026-07-15
**Target Document:** `docs/powergrok/BUILD_PLAN.md`
**Reviewer:** Antigravity AI (Peer Review)

## 1. Executive Summary
This revised `BUILD_PLAN.md` provides a much stronger and clearer path forward by locking in critical operator decisions (§0). The decision to enforce strict project-level isolation (`.powergrok/` vs `.grok/`) in v1 is the most significant change. While this increases the v1 scope by requiring product code changes (engine refactoring), it provides a vastly superior concurrency model and completely eliminates the risk of cross-contamination between the official build and the local build in the same workspace.

The plan is well thought out, and the risks introduced by the `argv0` strategy are accurately identified and mitigated.

## 2. Assessment of Locked Decisions (D1-D10)
*   **D1/D2/D3 (Home, Layout, Telemetry):** Approved. Keeping telemetry to product defaults while forcing `auto_update = false` is the right balance. The lib + wrapper layout is solid.
*   **D4/D5/D6/D7 (Project Isolation & Concurrency):** Approved, but this is the riskiest part of the plan. Using `argv0` basename to dynamically alter the project config directory (`.grok/` vs `.powergrok/`) with *no fallback* is a strict and clean isolation boundary. It correctly forces the operator to explicitly manage their `powergrok` workspace state without leaking into the official state.
*   **D8/D9/D10 (Completions, Version, Branching):** Approved. Confining completions to `~/.powergrok` prevents system pollution. Using a dedicated branch off `main` for Phase 1 and 2 is mandatory given the engine changes required.

## 3. Technical Feedback & Risk Assessment

### 3.1 `argv0` Project Isolation Strategy (Engine Refactor)
*   **Finding:** The plan to replace all hardcoded `".grok"` instances with a dynamic `project_config_dirname()` based on `argv0` is conceptually sound. 
*   **Risk:** `exec -a` is bash-specific. If operators use `zsh` or another shell for their wrappers, `argv0` manipulation can be fragile.
*   **Recommendation:** To ensure `argv0` is consistently passed as `powergrok` regardless of the wrapper's shell, consider skipping `exec -a` and instead just installing (or symlinking) the binary into `$POWERGROK_LIB` under the exact name `powergrok` instead of `xai-grok-pager`. Then the wrapper simply `exec "$POWERGROK_LIB/powergrok" "$@"`. This provides 100% shell-agnostic `argv0` propagation.

### 3.2 Completeness of Call-Site Conversion
*   **Finding:** As noted in §3.4 and §16 (R3), missing a hardcoded `".grok"` will cause a partial leak.
*   **Recommendation:** The PR that implements Phase 1 must include an exhaustive `rg "\.grok"` audit in the PR description, explicitly detailing which instances were converted and which were intentionally left as `.grok` (e.g., user-home paths or legacy compatibility).

### 3.3 Empty Project Layer UX
*   **Finding:** D7 dictates no fallback. If a user runs `powergrok` in a repo with only `.grok/`, they get an empty project layer.
*   **Recommendation:** This is correct for isolation, but UX will suffer. The CLI/TUI should ideally print a one-time informational warning (perhaps in the status bar or during startup) when it detects a `.grok/` directory but no `.powergrok/` directory, pointing the user to the migration step in §7.2.

## 4. Answers to Remaining Open Questions (§15)

1.  **Canonical crate:** `xai-grok-config` is the most logical home. If it causes dependency cycles with lower-level utility crates, a tiny leaf crate (e.g., `xai-grok-paths`) should be created to hold just the `OnceLock` and path resolution logic.
2.  **Completeness bar:** The merge gate *must* be an exhaustive `rg '\"\.grok\"'` audit. Any missed project-scoped path completely breaks the isolation contract established in D7. A documented allowlist for non-project paths is required.
3.  **`exec -a` portability:** As recommended in §3.1, avoid `exec -a` entirely. Install/symlink the binary as `powergrok` in the lib directory and `exec` it directly. This solves the bash-only portability issue elegantly.
4.  **Workspace classifier:** Yes, the workspace root classifier must treat `.powergrok` identically to `.grok`. Otherwise, tools relying on workspace root detection will fail when running under `powergrok`.
5.  **Git hygiene:** Strongly recommend advising operators to add `.powergrok/` to their global `~/.gitignore`. Since powergrok is a local developer tool, its project state shouldn't pollute shared repository history.

## 5. Conclusion
The revised `BUILD_PLAN.md` is approved for Phase 1 implementation. The move to strict project isolation via `argv0` is a substantial improvement, provided the `argv0` propagation and code auditing are executed meticulously. Proceed with the engine changes on a dedicated branch off `main`.

---

## 6. Plan author disposition (post-review)

All technical feedback in §3 and answers in §4 were accepted and written into `BUILD_PLAN.md` as **§0.1 (G1–G7)** and related sections:

| Review item | Plan ID | Where applied |
|-------------|---------|---------------|
| Named lib binary (avoid `exec -a`) | G1, G2 | §5.1–5.2, §8.4–8.5, Appendix A/B |
| Exhaustive `rg` + allowlist merge gate | G3 | §7.4, Phase 1 exit criteria |
| Empty project layer warning | G4 | §7.2, V16, M6 |
| Canonical crate `xai-grok-config` | G5 | §7.1 |
| Classifier parity for `.powergrok` | G6 | §7.5, V17 |
| Git hygiene advice | G7 | §12.6 |

See `BUILD_PLAN.md` Appendix D for the full disposition table.
