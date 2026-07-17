**Implements the complete Power Grok branding effort** as defined in `docs/powergrok/BRANDING_PLAN.md` (post two rounds of Gemini adversarial review) and the exhaustive audit in `docs/powergrok/BRANDING_AUDIT.md`.

### Summary of Changes

This PR introduces **"Power Grok"** as the official user-facing product name for this fork (`zfifteen/powergrok`) while maintaining perfect upstream compatibility, isolation, and zero merge-conflict debt on future `main` → `powergrok` syncs.

#### Key Technical Decisions (B1–B10 from the plan)

- **B8 – Branding adapter** (`crates/codegen/xai-grok-config/src/branding.rs`):
  - New module behind the `powergrok` Cargo feature.
  - `product_name()` and `is_powergrok_branding()` using `OnceLock`.
  - **Primary signal**: `POWERGROK_BRANDING=1` environment variable (set by install wrapper).
  - **Secondary**: compile-time `cfg!(feature = "powergrok")`.
  - **Strict rule enforced**: No inline `if is_powergrok_branding()` or primary `argv0` checks in core TUI/CLI paths. All branding routes through this adapter.

- **B9 – Documentation strategy** (`crates/codegen/xai-grok-pager/build.rs` + `src/docs.rs`):
  - Build-time templating of the user guide when the `powergrok` feature is enabled.
  - Copies `docs/user-guide/*.md` to `OUT_DIR`, performs targeted safe replacements, and exposes via `POWERGROK_USER_GUIDE_DIR` env var.
  - `extract_user_guide_docs()` and `guide!` macro updated to load branded content under the feature flag.
  - Avoids duplicating the entire guide or using runtime regex.

- **B10 – CI/Style gate** (`crates/codegen/xai-grok-pager-bin/src/main.rs`):
  - Integration test `branding_help_contains_power_grok()` that builds with `--features powergrok` and asserts `"Power Grok"` appears in `--help` output.
  - Additional test in `tests/branding_integration.rs`.
  - Strict style guide: **"Power Grok"** = product name in prose/UI; **`powergrok`** = binary name, branch, crate identifiers, paths.

#### Changes Made

**Core branding:**
- Added `powergrok = []` feature to `xai-grok-config`, `xai-grok-pager`, and `xai-grok-pager-bin` (propagated correctly).
- `xai_grok_config::branding` module with full test coverage (including `serial_test` for env var).
- Updated TUI title management (`crates/codegen/xai-grok-pager/src/notifications/title.rs`, `mod.rs`) to use `product_name()` behind `cfg!(feature = "powergrok")`.
- Updated clap `about()` and startup banner in `main.rs`.

**Documentation & user experience:**
- Updated `docs/powergrok/*`, `AGENTS.md`, `README.md`, and all `docs/effort-modes-builtin/*.md`.
- Build-time user guide override (B9) so `powergrok` shows branded content while official `grok` remains untouched.
- Coexistence language added everywhere: *"Power Grok — a parallel, source-built installation of Grok Build that coexists with the official `grok`"*.

**Verification:**
- `cargo check --features powergrok`
- `cargo test --features powergrok` (including the new B10 integration test)
- Manual verification of TUI title, `--help`, effort modes, and docs loading.
- All upstream sync paths and official `grok` behavior preserved.
- No inline conditionals in hot paths (adapter pattern strictly followed per Gemini review).

#### Related

- `docs/powergrok/BRANDING_PLAN.md` (full locked decisions, risk table, phase gates)
- `docs/powergrok/BRANDING_AUDIT.md` (~200 hits classified; upstream user-guide preserved via B9)
- `docs/powergrok/BRANDING_PLAN_REVIEW.md` (both Gemini adversarial reviews fully addressed)
- `docs/powergrok/BUILD_PLAN.md` (isolation contract unchanged)

This completes the branding initiative. Future upstream merges into `powergrok` remain clean.

---

**Commits authored by:** Grok Build Agent <agent@powergrok.dev>

*(Auth and push restored by Hermes per the postmortem. All auth rules now strictly observed.)*