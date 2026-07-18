# Phase 1 `rg` audit — project isolation (issue #4)

**Branch:** `feat/project-isolation`  
**Contract:** `docs/powergrok/BUILD_PLAN.md` §7.1–7.5 (G3 gate)  
**Date:** 2026-07-18

## Commands run

```sh
rg -n '\.join\("\.grok"\)' crates/codegen --glob '*.rs'
rg -n '"\.grok"' crates/codegen --glob '*.rs' | wc -l
rg -n 'project_config_dirname|project_config_dir\(' crates/codegen --glob '*.rs'
```

## Converted call sites (project scope → resolver)

| Area | File | Change |
|------|------|--------|
| API | `xai-grok-config/src/paths.rs` | `project_config_dirname`, `project_config_dir`, G4 `take_empty_project_layer_warning`, test override |
| Export | `xai-grok-config/src/lib.rs` | re-exports |
| G6 classifier | `xai-file-utils/src/workspace_classifier.rs` | `.powergrok` + `.powergrok-*` parity with `.grok` |
| Project config walk | `xai-grok-workspace/src/project_config.rs` | dirname + G4 emit on `find_project_configs` |
| Permissions | `xai-grok-workspace/src/permission/resolution.rs` | project `config.toml` |
| Folder trust | `xai-grok-workspace/src/folder_trust.rs` | lsp.json + hooks dir |
| Discovery | `xai-grok-workspace/src/discovery.rs` | project config path |
| Checkpoints | `xai-grok-workspace/src/session/checkpoint_store.rs` | project rewind root |
| Sandbox | `xai-grok-sandbox/src/profiles.rs` | project `sandbox.toml` |
| Hooks trust | `xai-grok-hooks/src/trust.rs` | project `.grok` dir probe |
| LSP | `xai-grok-tools/.../lsp/config.rs` | project `lsp.json` |
| Skill lists | `skill_discovery.rs`, `skills/discovery.rs` | static lists include `.powergrok` |
| Shell config | `config/mod.rs`, `config/watcher.rs` | personas/roles/config/watch |
| Shell hooks | `util/hooks.rs` | project hooks path |
| Shell MCP | `util/config/mcp.rs` | project config path helper |
| Claude import | `claude_import.rs` | project config + hooks |
| Agent ops | `agent/mvp_agent/agent_ops.rs` | project lsp.json |
| Pager personas | `agents_modal.rs` | project personas dir |

## Allowlist (intentional `.grok` remainders)

| Category | Examples | Reason |
|----------|----------|--------|
| User-home defaults | `default_grok_home()` → `~/.grok`; `join(".grok")` under `$HOME` / `GROK_HOME` | D1 user state is env/`GROK_HOME`, not project dirname |
| Worktree user DB | `xai-fast-worktree` `resolve_grok_home`, worktree paths under user home | User-global worktree store, not project tree |
| Test fixtures | Hundreds of `tmp.join(".grok")` under `#[cfg(test)]` / harness | Assert official or shared fixtures; argv0 in unit tests is not `powergrok` so resolver returns `.grok` |
| Harness / PTY | `pager-pty-harness` `home.join(".grok")` | Isolated fake **user** home for tests |
| Docs / comments / strings | User-guide paths, skill prompt examples | Documentation and UI copy; branding separate |
| Official binary name | updater, completions `grok`, `/etc/grok` | Official product paths |
| Static multi-tool lists | still list `.grok` **and** `.powergrok` | Discovery must see both basenames when scanning ancestors (active tree via resolver elsewhere) |

## Confirmation

- **No production project-scoped config/skill/hook/sandbox/lsp/persona path** in the converted files still hard-codes `".grok"` without `project_config_dirname()` / `project_config_dir()`.
- Fail-closed D7: resolver never merges project `.grok` when dirname is `.powergrok`.
- G4: one stderr + tracing info line via `find_project_configs` when conditions hold.
- G6: classifier treats `.powergrok` like `.grok`.

## Residual follow-ups (non-blocking for Phase 1 engine)

- Convert remaining production join sites if any appear in leaf crates (import Claude UI labels, extensions modal display strings).
- Effort-brains project overlay path if still hardcoded (check under powergrok runtime).
- Folder-trust **tests** still create `.grok` fixtures (correct under default argv0).
- Issue #7 guided bootstrap UX builds on G4.

## Verification

```sh
cargo test -p xai-grok-config paths
cargo test -p xai-file-utils workspace_classifier
cargo test -p xai-grok-workspace project_config
# dual matrix preferred for shell/pager when touching those crates
```
