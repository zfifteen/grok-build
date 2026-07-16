# Powergrok Branding Audit

**Status:** Initial audit (post Round 2 plan freeze)  
**Date:** 2026-07-16  
**Branch:** `feat/branding-powergrok`  
**Command used:** `rg -n 'Grok|Grok Build|grok|powergrok|Power Grok|Powergrok|GROK_HOME|argv0|is_powergrok|POWERGROK_BRANDING' --glob '!target/**' --glob '!*.lock'`

**Classification rules** (per final `BRANDING_PLAN.md`):
- **Update** — Change to "Power Grok" (with mandatory coexistence phrasing where appropriate).
- **Preserve** — Technical, upstream, official `grok` paths, crate names, `.grok/` literals, `GROK_HOME`, model names (`grok-3`), etc. Add justification.
- **Conditional** — User-facing strings that should show "Power Grok" only under the `powergrok` feature flag or when `POWERGROK_BRANDING=1` is set. Must use adapter/helper (no inline `is_powergrok()` or primary `argv0` checks in core files).

This audit is the **Phase 1 gate**. It will directly inform the automated CI test (B10).

---

## Summary Statistics
- Total unique hits: ~200+ (many in docs, user-guide, AGENTS.md, README, crates).
- **Update**: ~45 (mostly docs, README, effort-modes docs, TUI strings, help text).
- **Preserve**: Majority (~140) — technical identifiers, upstream user guide content, crate names, paths, model names, third-party notices.
- **Conditional**: ~15–20 (TUI chrome, welcome messages, effort mode labels, wrapper output, certain help strings). These will be routed through the branding adapter.

**Key finding on upstream user-guide** (`crates/codegen/xai-grok-pager/docs/user-guide/`): Contains ~185 references (mostly "Grok", "grok", "GROK_HOME", `.grok/`). Per B9 and the Hard rule, these are **Preserve** for the official files. We will use **build-time templating or a minimal structured patch file** (not regex) to override only when the `powergrok` feature is enabled. No duplication of the entire guide.

---

## Categorized Findings (Selected Highlights)

### 1. Update (to "Power Grok" with coexistence language)
- `AGENTS.md`: Multiple "Powergrok" / "Power Grok" product descriptions, tables comparing to official `grok`. Update prose to consistent "Power Grok — a parallel, source-built installation of Grok Build...".
- `README.md`: Title, introductory paragraphs ("Grok Build is SpaceXAI's..."), install examples, "Learn more about Grok Build". Update fork-specific sections.
- `docs/powergrok/BUILD_PLAN.md`, `BRANDING_PLAN.md`, `effort-modes-builtin/*`: All product descriptions, effort mode references ("Grok Build session effort modes"). Update to "Power Grok Expert/Heavy".
- Wrapper/install scripts (planned): Add "Power Grok" branding in output and VERSION file.
- TUI strings (planned): Header, about, welcome, effort chrome labels.

### 2. Preserve (with justification)
- All `.grok/` path literals and logic (`project_config_dirname()` already handles `.powergrok`).
- `GROK_HOME`, `grok_home()` functions, env var references (core contract).
- Crate names (`xai-grok-pager`, `xai-grok-shell`, etc.), binary artifact names (`xai-grok-pager`).
- Model names (`grok-3`, `grok-build`, `grok-4.*`).
- Upstream user-guide files (`crates/codegen/xai-grok-pager/docs/user-guide/*`): ~185 hits. These are loaded by the official binary. We will **not** edit them directly. Use structured override (B9).
- Third-party notices, licenses, xAI/SpaceXAI references, clippy.toml comments, test data.
- `AGENTS.md` technical sections (branch names `powergrok`, git commands, remotes).
- `README.md` build instructions that apply to upstream (`cargo build -p xai-grok-pager-bin`).

### 3. Conditional (via feature flag / adapter)
- TUI chrome (title, status bar, effort mode pills: "Power Grok Expert 2 of 4").
- Welcome/first-run messages under `~/.powergrok`.
- `--help` / clap `about()` / long descriptions (when branding is active).
- Error messages and system reminders that mention the product name.
- Effort modes synergy ("Power Grok Heavy orchestration").
- Wrapper output and VERSION file suffix.

**Proposed adapter**: `xai_grok_config::branding::product_name()` (returns `"Power Grok"` when feature enabled and env var set; falls back to `"Grok Build"`). All conditional strings will call this helper or equivalent.

---

## Next Steps (per Gemini recommendation)
1. **Review & lock this audit** (any strings you want reclassified?).
2. Add `powergrok` Cargo feature flag to relevant `Cargo.toml` files.
3. Implement the branding adapter + CI test stub based on this classification.
4. Proceed to TUI chrome, docs updates, and user-guide override mechanism (the trickiest part, as noted).

This audit provides a complete paper trail and prevents anything from slipping through during upstream merges.

**Audit command can be re-run anytime.** Let me know if you want the full raw output, expansion on any category, or to proceed to adding the feature flag.