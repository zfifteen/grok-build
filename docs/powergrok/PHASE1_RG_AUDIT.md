# Phase 1 `rg` audit — project isolation (issue #4)

**Branch:** `feat/project-isolation`  
**Contract:** `docs/powergrok/BUILD_PLAN.md` §7.1–7.5 (G3 gate)  
**Updated:** after PR #17 review (D7 loader fix)

## Commands run

```sh
rg -n '\.join\("\.grok"\)' crates/codegen --glob '*.rs'
rg -n 'project_config_dirname|project_config_dir\(' crates/codegen --glob '*.rs'
rg -n '"\.powergrok"' crates/codegen --glob '*.rs' | head -80
```

## Converted call sites (project scope → resolver)

| Area | File | Change |
|------|------|--------|
| API | `xai-grok-config/src/paths.rs` | dirname, dir, G4 pure+latch, test override |
| G6 classifier | `xai-file-utils/src/workspace_classifier.rs` | both names OK (skip lists only) |
| Project config | `xai-grok-workspace/src/project_config.rs` | dirname + G4 emit |
| Permissions / trust / LSP / sandbox / shell / checkpoints / personas | (see prior commit) | project joins via resolver |
| **Skill loaders (D7)** | `compat::skill_config_dirs`, `discover_skills_for_paths`, `skill_product_and_vendor_config_dirs` | **active dirname only** |
| **Rules loaders** | `compat::rules_dirs` | `{active}/rules` only |
| **Agent project dirs** | `discovery::project_agent_dirs_in` | `{active}/agents` + `.claude/agents` |
| **Plugin project dirs** | `plugins/discovery::project_plugin_dirs_in` | `{active}/plugins` + `.claude/plugins` |
| **Shell extensions** | `extensions/skills.rs` | product dirname only in local list |

## Allowlist (intentional remainders)

| Category | OK? | Reason |
|----------|-----|--------|
| **Classifiers** listing both `.grok` and `.powergrok` | Yes | G6 skip/parity only — not loaders |
| User-home `~/.grok` / `GROK_HOME` | Yes | D1 env home, not project dirname |
| Worktree user DB under home | Yes | User-global store |
| Test fixtures `tmp.join(".grok")` | Yes | Default argv0 → product `.grok`; fixtures match |
| Harness home.join(".grok") | Yes | Fake **user** home |
| Docs / comments / official binary | Yes | Non-project-scope |
| **Loaders listing both product basenames** | **No** | D7 violation — must not appear |

## Confirmation (post-review)

- Product skill/agent/plugin/rules **loaders** use **exactly one** basename from `project_config_dirname()`.
- Classifiers may list both names.
- Fail-closed D7: powergrok does not load project `.grok/skills|agents|plugins|rules`.
- G4: pure `should_emit_empty_project_layer_warning` + one-shot `take_…`.

## Kill checks (review)

| Check | Expected |
|-------|----------|
| override `.powergrok`, skills only under `.grok/skills` | no product skills loaded |
| override `.grok`, skills only under `.powergrok/skills` | no product skills loaded |
| classifier `.powergrok` path | not a project dir (G6) |
