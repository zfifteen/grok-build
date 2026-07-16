# Powergrok Build Plan Review

**Review Date:** 2026-07-15
**Target Document:** `docs/powergrok/BUILD_PLAN.md`
**Reviewer:** Antigravity AI (Peer Review)

## 1. Overall Assessment
The `BUILD_PLAN.md` is exceptionally well-structured and detailed. It correctly identifies the constraints of installing a source-built version of the CLI (`powergrok`) alongside an officially managed binary (`grok`). The strategy to achieve isolation via environmental overrides (`GROK_HOME`) rather than invasive code changes is robust and minimizes maintenance overhead. The risk analysis and mitigation strategies are comprehensive.

## 2. Strengths & Positive Observations
*   **Zero Product Code Changes (v1):** Leveraging existing environment variables (`GROK_HOME`) and configuration files (`config.toml`) is the right approach. It avoids forking the codebase or introducing `#cfg` flags for branding in v1.
*   **Wrapper Script Safety:** The proposed wrapper script contains excellent defensive programming, notably the check that aborts execution if `GROK_HOME` inadvertently resolves to the official `~/.grok` directory.
*   **Auto-update Mitigation:** Explicitly seeding `config.toml` with `auto_update = false` is critical and correctly implemented to prevent the official release channels from overwriting the locally compiled binary.
*   **Rollback Mechanism:** Keeping the previous binary as `xai-grok-pager.prev` during the manual/scripted install provides a simple and effective rollback path for developers.

## 3. Specific Findings & Technical Feedback

### 3.1 `OnceLock` and Environment Injection
The plan relies on `GROK_HOME` being evaluated early via a `OnceLock` (as described in §3.3). 
*   **Finding:** This is generally safe, provided that no library or crate initializes `grok_home()` during a static initialization phase *before* the wrapper's environment variables are fully processed (though the wrapper using `exec` guarantees the environment is passed to the spawned process).
*   **Recommendation:** A smoke test ensuring that invoking `powergrok` with a clean environment strictly uses the `~/.powergrok` path should be part of the automated CI/CD pipeline if `powergrok` becomes an officially supported developer workflow.

### 3.2 Seed Config Idempotency
*   **Finding:** §7.3 specifies that the install script will use a "create if missing" approach for `config.toml`. This is correct.
*   **Recommendation:** Ensure the wrapper script strictly follows `if [[ ! -f "$GROK_HOME/config.toml" ]]` and does not attempt to parse or merge configurations. The example bash wrapper provided in §5.2 implements this perfectly.

### 3.3 Fish Completions Caveat
*   **Finding:** §3.3 mentions that fish completion regeneration can write to `$HOME/.config/fish/completions/grok.fish`. 
*   **Recommendation:** Even with `auto_update = false`, if a user manually runs an update check or a command that regenerates completions, `powergrok` might overwrite the official `grok` completions. While minor, this could be documented as a known issue in the `README.md` (Phase 3).

## 4. Answers to Open Questions (§15)

Here are recommendations for the open questions posed in the document:

1.  **Home directory name:** 
    *Recommendation:* Agree with `~/.powergrok`. It directly mirrors the official `~/.grok` and clearly signals its purpose.
2.  **Lib vs bin for real binary:** 
    *Recommendation:* Agree with using the `lib` directory for the real binary and a wrapper in `bin`. This hard-enforces the `GROK_HOME` injection and prevents users from accidentally bypassing the isolation.
3.  **Branch strategy for Phase 2:** 
    *Recommendation:* A dedicated stacked PR or a `chore/powergrok-install` branch is highly recommended. It keeps the core product changes separate from developer tooling/installation scripts.
4.  **Telemetry / feedback defaults:** 
    *Recommendation:* Leave product defaults as-is unless there is a specific organizational policy against telemetry from developer builds. If developers are testing, their telemetry might be valuable (or noise, depending on the backend filtering). If it's noise, seed it to `false`.
5.  **Concurrent use:** 
    *Recommendation:* The filesystem isolation (separate leader sockets, sessions, auth) supports this. It is a highly desirable operator scenario to compare the official release against a local build side-by-side.
6.  **Project `.grok/` sharing:** 
    *Recommendation:* Acceptable for v1. Developers working in a repo generally want both binaries to respect the same project-level rules and skills.
7.  **Completions:** 
    *Recommendation:* Defer to Phase 2 or a later milestone (Milestone D). Generating `powergrok`-named completions requires clap overrides which might be out of scope for a quick v1 script.
8.  **Versioning UX:** 
    *Recommendation:* A simple `VERSION` file as proposed in §7.4 is sufficient for v1. Patching `--version` requires compile-time environment variable injection (`build.rs`), which violates the "zero product code changes" goal for v1.

## 5. Conclusion
The `BUILD_PLAN.md` is approved for Phase 1 execution from a technical and architectural standpoint. The isolation strategy is sound, and the edge cases are well-documented. Proceed with manual installation testing.
