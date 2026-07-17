# Effort Reasoning Brains Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task after Phase 0 freeze confirmation.

**Goal:** Replace Expert/Heavy generic `specialist-i` / 4-angle cycling with a **library of 16 distinct reasoning protocols (“brains”)** so multi-agent runs produce **method deltas**, not cosplay personas. **Heavy uses all 16. Expert samples a random 4 of 16 per team run.** Config lives under the Powergrok config tree (`$GROK_HOME/effort-brains/`, i.e. `~/.powergrok/effort-brains` when running as powergrok).

**Architecture:** Keep existing sticky EffortMode + fixed-N ledger + join-all + post-synthesis execute gates. Swap only the **brief builder and spawn payload**: load catalog/rosters/brain files → build `SpecialistBrief` per slot (brain id, method prompt, contrarian_class, optional model) → existing `effort_team` fan-out. Synthesis injection becomes **disagreement-oriented** (agree / conflict / unique / residual). Defaults ship in-tree; user/project dirs override.

**Tech Stack:** Rust (`xai-grok-shell` effort path, `xai-grok-config` paths), TOML + Markdown brain files, existing subagent spawn (`effort_team` / session effort hooks), unit tests without live models.

**Parent docs:** `00-program-charter.md`, `01-tech-spec.md`, `02-architecture-map.md`, this plan extends Phase-style delivery for a **new feature slice** on top of builtin effort modes.

**Status:** Design approved for planning (2026-07-16/17). Implementation not started.

---

## 0. Frozen product decisions

| ID | Decision | Notes |
|----|----------|--------|
| F1 | **16 brains**, stable ids | Listed in §2 |
| F2 | **Heavy = all 16** in fixed catalog order | Default order §2.3 |
| F3 | **Expert = random 4 of 16**, without replacement, per **team run** | Re-sample only when `begin_team_run` / new non-trivial team starts; persist chosen ids on the run ledger |
| F4 | **No cosplay** | Brains are reasoning protocols (method / forbidden / artifact / delta_role), not characters |
| F5 | **Config root** | `$GROK_HOME/effort-brains/` (powergrok → `~/.powergrok/effort-brains`); project override: `<workspace>/.powergrok/effort-brains/` |
| F6 | **Precedence** | Built-in defaults ⊂ user `$GROK_HOME` ⊂ project `.powergrok` **merge-by-id** (higher wins for same brain id / roster keys) |
| F7 | **v1 diversity layers** | L1 protocol prompt + L2 artifact schema **required**; L3 tool policy optional later; L4 model override **config-optional, default off** |
| F8 | **Contrarian class** | `inversion`, `pre_mortem`, `red_team` (`contrarian_class = true`). Heavy full-team success still requires ≥1 successful contrarian-class specialist |
| F9 | **Expert sampling v1** | **Pure uniform random** of 4 (no multi-family constraint in v1) |
| F10 | **Reproducibility** | Log selected brain ids always; optional `GROK_EFFORT_BRAIN_SEED` (u64) for deterministic Expert draws in tests/debug |
| F11 | **DoD sizes unchanged** | Expert N=4, Heavy N=16; config must not silently change N without experimental flag (align Q9) |
| F12 | **Analytic-only inside N** | Unchanged; brains do not grant write tools inside the fixed team |
| F13 | **Invalid config** | Fail validation loud: do not start mandatory team with unknown ids / wrong Heavy length / missing contrarian class; surface error to user; do not silent-fallback to old angle cycle |
| F14 | **Brain file depth** | Short high-signal protocols (~40–80 lines body), not essays |

---

## 1. Problem / current state

| Surface | Today | After |
|---------|-------|--------|
| Roles | `specialist-i`, Heavy last `contrarian-i` | Brain **ids** (`first_principles`, …) |
| Diversity | 4 rotating topic angles | 16 **method protocols** |
| Expert | Always same 4 angles | **Random 4 brains** each team run |
| Heavy | 16 angle-cycled slots + last contrarian | **All 16 brains**; last default `red_team` |
| Config | Hardcoded in `effort_mode.rs` | Editable under `$GROK_HOME/effort-brains/` |
| Synthesis | Generic “synthesize reports” | Structured **disagreement map** by brain id |

**Code anchors (current):**

- `crates/codegen/xai-grok-shell/src/session/effort_mode.rs` — `EffortMode`, ledger, `build_specialist_briefs`, `specialist_angle`
- `crates/codegen/xai-grok-shell/src/session/effort_team.rs` — fan-out / join packaging
- `crates/codegen/xai-grok-shell/src/session/acp_session_impl/session_mode.rs` — `maybe_run_mandatory_effort_team`
- `crates/codegen/xai-grok-config/src/paths.rs` — `user_grok_home()` / `GROK_HOME`

---

## 2. The 16 brains (normative catalog)

### 2.1 Catalog table

| # | id | Family | Core move | Contrarian class | Delta role |
|---|-----|--------|-----------|------------------|------------|
| 0 | `first_principles` | Foundations | Rebuild from irreducible constraints | no | Regenerate solution space |
| 1 | `map_territory` | Foundations | Separate models/docs from reality | no | Expose representation error |
| 2 | `circle_of_competence` | Foundations | Mark known / guessed / out-of-domain | no | Bound overreach |
| 3 | `systems_loops` | Systems | Feedback, delays, coupling | no | Non-local / delayed effects |
| 4 | `theory_of_constraints` | Systems | Find and elevate the bottleneck | no | Collapse thrash to binding limit |
| 5 | `five_whys_root` | Systems | Causal chain to controllable root | no | Past surface symptoms |
| 6 | `second_order` | Consequences | “And then what?” multi-wave | no | Catch fix-creates-problem |
| 7 | `inversion` | Failure-first | Design by avoiding guaranteed loss | **yes** | Failure-first pressure |
| 8 | `pre_mortem` | Failure-first | Assume failure; narrate why | **yes** | Prospective autopsy |
| 9 | `scientific_method` | Evidence | Hypothesis + falsifiers | no | Empirical checkability |
| 10 | `bayesian_update` | Evidence | Prior → evidence → posterior | no | Explicit uncertainty |
| 11 | `fermi_estimate` | Evidence | Order-of-magnitude bounds | no | Scale / feasibility |
| 12 | `via_negativa` | Minimalism | Improve by removal | no | Complexity reduction |
| 13 | `ooda_tempo` | Tempo | Observe–Orient–Decide–Act latency | no | Speed under uncertainty |
| 14 | `steelman_dialectic` | Dialectic | Strongest fair counter-case | no | Synthesis-ready opposition |
| 15 | `red_team` | Adversarial | Attack as competent opponent | **yes** | Hard attack |

### 2.2 Shared specialist envelope (all brains)

Every specialist report must include:

```text
brain_id:
method_applied:
claims:
evidence:          # paths / symbols / commands
uncertainties:
disagreements_invited:
verdict:
```

Plus brain-specific sections defined in each brain file (failure catalog, loops, posteriors, estimates, attack paths, …).

### 2.3 Default Heavy order

Slots 0–15 = catalog table order above; slot 15 = `red_team`.

### 2.4 Expert sampling

```text
pool = all 16 ids
k = 4
selection = uniform without replacement
seed = GROK_EFFORT_BRAIN_SEED if set, else OS random
persist selection on EffortModeTracker team run / ledger rows
```

---

## 3. Config layout

### 3.1 Paths

```text
$GROK_HOME/effort-brains/           # user (powergrok: ~/.powergrok/effort-brains)
  catalog.toml
  rosters/
    expert.toml
    heavy.toml
  brains/
    first_principles.md
    ...
    red_team.md

<workspace>/.powergrok/effort-brains/   # project override (powergrok argv0 only)
  # same shape; merge-by-id over user + defaults
```

Built-in defaults: ship under repo  
`crates/codegen/xai-grok-shell/src/session/effort_brains/defaults/`  
(or `docs/effort-modes-builtin/effort-brains-seed/` copied on first run — prefer **embed/load from crate defaults** so offline works).

### 3.2 `catalog.toml` schema (v1)

```toml
version = 1

[defaults]
allow_model_overrides = false
require_contrarian_on_heavy = true
expert_k = 4
# heavy_n fixed 16 / expert_k fixed 4 unless experimental

[[brains]]
id = "first_principles"
file = "brains/first_principles.md"
family = "foundations"
delta_role = "regenerate_solution_space"
contrarian_class = false
# model = "..."   # only if allow_model_overrides
```

### 3.3 `rosters/heavy.toml`

```toml
version = 1
mode = "heavy"
selection = "fixed"
slots = [
  "first_principles",
  "map_territory",
  "circle_of_competence",
  "systems_loops",
  "theory_of_constraints",
  "five_whys_root",
  "second_order",
  "inversion",
  "pre_mortem",
  "scientific_method",
  "bayesian_update",
  "fermi_estimate",
  "via_negativa",
  "ooda_tempo",
  "steelman_dialectic",
  "red_team",
]
```

Validation: `slots.len() == 16`, all ids in catalog, ≥1 `contrarian_class` among slots when `require_contrarian_on_heavy`.

### 3.4 `rosters/expert.toml`

```toml
version = 1
mode = "expert"
selection = "random"
k = 4
pool = "all"   # or explicit list of ids (must be length ≥ k)
```

Validation: `k == 4` (v1), pool resolves to ≥4 known ids.

### 3.5 Brain markdown

```markdown
---
id: first_principles
family: foundations
delta_role: regenerate_solution_space
contrarian_class: false
forbidden:
  - authority_as_proof
  - cargo_cult_patterns
  - premature_synthesis
artifact_sections:
  - atomic_truths
  - rebuilt_options
  - rejected_analogies
---

# Method
1. ...
# Stop rules
...
# Artifact details
...
```

Loader: parse YAML frontmatter + body; inject into specialist prompt wrapper.

### 3.6 Optional `config.toml` knobs

```toml
[effort.brains]
# enabled = true
# root = "effort-brains"   # relative to GROK_HOME
# seed = 42                # optional; env GROK_EFFORT_BRAIN_SEED wins
```

Keep minimal in v1; env seed is enough for tests.

---

## 4. Runtime design

### 4.1 Types (proposed)

```rust
// effort_brains/mod.rs (new)
pub struct BrainId(pub String);
pub struct BrainSpec {
    pub id: BrainId,
    pub family: String,
    pub delta_role: String,
    pub contrarian_class: bool,
    pub forbidden: Vec<String>,
    pub artifact_sections: Vec<String>,
    pub body_markdown: String,
    pub model: Option<String>,
}
pub struct BrainCatalog { /* id -> BrainSpec */ }
pub enum RosterSelection {
    Fixed { slots: Vec<BrainId> },
    Random { k: usize, pool: Vec<BrainId> },
}
pub struct EffortBrainConfig {
    pub catalog: BrainCatalog,
    pub expert: RosterSelection,
    pub heavy: RosterSelection,
    pub allow_model_overrides: bool,
    pub require_contrarian_on_heavy: bool,
}
```

Extend `SpecialistBrief`:

```rust
pub struct SpecialistBrief {
    pub slot: usize,
    pub role: String,           // brain id
    pub is_contrarian: bool,    // from brain.contrarian_class
    pub description: String,    // e.g. "expert brain first_principles (1/4)"
    pub prompt: String,
    pub brain_id: String,
    pub model_override: Option<String>,
}
```

### 4.2 Load algorithm

1. Load **built-in** defaults into `EffortBrainConfig`.
2. If `$GROK_HOME/effort-brains/catalog.toml` exists → parse & **merge-by-id**.
3. If workspace `.powergrok/effort-brains/` exists (powergrok project dir) → merge-by-id again.
4. Validate rosters; on error return `EffortBrainError` (display to user; **no** team start).
5. Cache per session or reload on each team start (v1: load each `begin_team_run` is OK if cheap; else cache + mtime).

### 4.3 Brief builder (replaces `specialist_angle` path)

```text
build_specialist_briefs(mode, task, config, rng) -> Vec<SpecialistBrief>
  Normal -> []
  Expert -> sample 4 ids from expert roster policy; assign slots 0..3
  Heavy  -> fixed 16 ids; slots 0..15
  for each slot:
    load BrainSpec
    is_contrarian = spec.contrarian_class
    prompt = wrapper(mode, slot, n, task, spec)
```

**Prompt wrapper (normative skeleton):**

```text
You are specialist slot {i} of {n} on a **mandatory** {mode} effort-mode analytic team (join-all).
The shell launched you; do not spawn further subagents.

**Brain id:** {id}
**Family:** {family}
**Delta role:** {delta_role}
**Contrarian class:** {yes|no}

**Forbidden moves:** ...
**Required artifact sections:** common envelope + ...

# Brain protocol
{body_markdown}

# User task
{task}

Stay analytic and non-writing (read/search only).
Produce the required artifact. End with a short summary the lead can synthesize.
```

### 4.4 Ledger / gates

- `SpecialistLedgerRow.role` = brain id  
- `is_contrarian` = brain.contrarian_class  
- Heavy `can_claim_full_team` still requires N successes **and** ≥1 successful contrarian-class  
- Expert: no contrarian requirement (random 4 may or may not include one)

### 4.5 Tracker additions

- Store `selected_brain_ids: Vec<String>` on the active team run (for chrome + reports)  
- Status chrome examples: `Expert 2 of 4 · bayesian_update`, `Heavy 11 of 16 · red_team`

### 4.6 Synthesis package

Update `format_team_report_package` (or successor) to require leader structure:

```text
## Consensus (multi-brain)
## Conflicts (brain A vs brain B — do not paper over)
## Unique contributions (single-brain claims)
## Residuals / unknowns
## Decision (cite brain ids)
```

### 4.7 Spawn path

- `effort_team` continues shell-owned spawn  
- Pass brain prompt as specialist task text  
- If `allow_model_overrides && brain.model`: thread into subagent runtime overrides (feature-detect; if unsupported, log warning and continue)  
- v1 may ship model field in schema without wiring if spawn API is awkward — **Task phase marks optional**

### 4.8 First-run UX

- If user dir missing: **use built-ins** (no hard requirement to write disk)  
- Optional later: `powergrok` command or docs “export seed to ~/.powergrok/effort-brains”  
- v1 docs: how to copy seed and edit

---

## 5. Non-goals (v1)

1. Cosplay / historical character names as product surface  
2. Multi-family sampling constraints for Expert  
3. Two-wave Heavy (specialists reading peer reports)  
4. Peer chat between specialists  
5. Changing N away from 4/16  
6. Replacing Plan mode or permission modes  
7. Skills-first `/expert` path brain injection (builtins only)  
8. Mandatory multi-model spend  

---

## 6. Implementation tasks

> Each task is intentionally small. Prefer TDD. Commit after each green task.

### Task 1: Add module skeleton + error types

**Objective:** Create `effort_brains` module with types and empty load API.

**Files:**
- Create: `crates/codegen/xai-grok-shell/src/session/effort_brains/mod.rs`
- Create: `crates/codegen/xai-grok-shell/src/session/effort_brains/types.rs`
- Create: `crates/codegen/xai-grok-shell/src/session/effort_brains/error.rs`
- Modify: `crates/codegen/xai-grok-shell/src/session/mod.rs` (pub mod)

**Step 1:** Types `BrainId`, `BrainSpec`, `RosterSelection`, `EffortBrainConfig`, `EffortBrainError`  
**Step 2:** `load_effort_brain_config() -> Result<EffortBrainConfig, EffortBrainError>` stub returning `Err(NotImplemented)` or built-in empty  
**Step 3:** Unit test module compiles  
**Step 4:** Commit `feat(effort-brains): scaffold types and module`

---

### Task 2: Built-in default catalog (data only)

**Objective:** Embed default catalog.toml + 16 short brain markdown stubs + heavy/expert rosters.

**Files:**
- Create: `crates/codegen/xai-grok-shell/src/session/effort_brains/defaults/catalog.toml`
- Create: `crates/codegen/xai-grok-shell/src/session/effort_brains/defaults/rosters/{expert,heavy}.toml`
- Create: `crates/codegen/xai-grok-shell/src/session/effort_brains/defaults/brains/*.md` (16 files)
- Create: `crates/codegen/xai-grok-shell/src/session/effort_brains/defaults/mod.rs` (`include_str!` or `rust-embed` — prefer `include_str!` for no new dep)

**Content rule:** Each brain body states Method / Forbidden / Artifact / Stop rules (F14 short).

**Step 1:** Write all 16 brains + catalog + rosters  
**Step 2:** Test: defaults parse counts = 16 brains, heavy slots = 16, expert k = 4  
**Step 3:** Commit `feat(effort-brains): ship built-in 16-brain defaults`

---

### Task 3: Parse + validate

**Objective:** Parse TOML/MD and validate roster invariants.

**Files:**
- Create: `crates/codegen/xai-grok-shell/src/session/effort_brains/load.rs`
- Create: `crates/codegen/xai-grok-shell/src/session/effort_brains/validate.rs`
- Test: `effort_brains` unit tests

**Rules:**
- catalog version == 1  
- every roster id ∈ catalog  
- heavy fixed len 16; ≥1 contrarian_class if required  
- expert random k=4; pool size ≥ k  
- unknown frontmatter id vs filename mismatch → error  

**Step 1:** Failing tests for valid defaults + invalid heavy length + unknown id  
**Step 2:** Implement parse/validate  
**Step 3:** Green + commit `feat(effort-brains): parse and validate catalog/rosters`

---

### Task 4: Merge layers (defaults / user / project)

**Objective:** Implement precedence F6.

**Files:**
- Modify: `load.rs`
- Use: `xai_grok_config::paths::user_grok_home`
- Project path: resolve `.powergrok/effort-brains` when project config dirname is powergrok (reuse existing project dir helper if any; else `cwd.join(".powergrok/effort-brains")` gated by product basename / existing project_config_dirname)

**Step 1:** Tests with temp dirs for user+project override of one brain body  
**Step 2:** Implement merge-by-id  
**Step 3:** Commit `feat(effort-brains): merge defaults, user, and project layers`

---

### Task 5: Expert random sampling + seed

**Objective:** `select_brain_ids(mode, config, rng) -> Vec<BrainId>`.

**Files:**
- Create: `effort_brains/select.rs`
- Env: `GROK_EFFORT_BRAIN_SEED`

**Step 1:** Tests: Heavy returns fixed 16; Expert returns 4 unique; same seed → same 4; different seed usually differs  
**Step 2:** Implement  
**Step 3:** Commit `feat(effort-brains): expert random-4 and heavy fixed selection`

---

### Task 6: Prompt builder

**Objective:** `render_specialist_prompt(mode, slot, n, task, spec) -> String`.

**Files:**
- Create: `effort_brains/prompt.rs`
- Test snapshot or contains assertions for brain id, forbidden, task echo, non-writing clause

**Step 1:** Failing tests  
**Step 2:** Implement wrapper §4.3  
**Step 3:** Commit `feat(effort-brains): specialist prompt renderer`

---

### Task 7: Wire `build_specialist_briefs` to brains

**Objective:** Replace angle cycle with brain-backed briefs.

**Files:**
- Modify: `effort_mode.rs` (`build_specialist_briefs` signature may take `&EffortBrainConfig` + `&mut impl Rng` **or** load inside with injectable config for tests)
- Prefer: `build_specialist_briefs_with(mode, task, &config, rng)`
- Keep thin wrapper `build_specialist_briefs` for tests using built-ins + seed 0

**Step 1:** Update unit tests in `effort_mode.rs` that currently assert angle strings / contrarian last role  
**Step 2:** Expert briefs: 4 unique brain ids; Heavy: 16; last default red_team contrarian  
**Step 3:** Remove `specialist_angle` (or keep dead code free — delete)  
**Step 4:** Commit `feat(effort-brains): drive SpecialistBrief from brain catalog`

---

### Task 8: Tracker / ledger persistence of selection

**Objective:** Store selected brain ids on team run; chrome label includes brain id.

**Files:**
- Modify: `effort_mode.rs` (`EffortModeTracker`, snapshot/status label helpers)
- Modify: any TUI status consumer if needed (grep `status_label` / effort chrome)

**Step 1:** Tests for status string containing brain id when pursuing  
**Step 2:** Implement  
**Step 3:** Commit `feat(effort-brains): ledger and chrome use brain ids`

---

### Task 9: `effort_team` + mandatory run integration

**Objective:** Production path loads config, selects brains, spawns with new prompts.

**Files:**
- Modify: `effort_team.rs`
- Modify: `session_mode.rs` (`maybe_run_mandatory_effort_team`)
- On config error: user-visible message; do not claim full team; do not fall back to angles

**Step 1:** Integration-style unit test with temp GROK_HOME if feasible; else mock config inject  
**Step 2:** Wire load + select + build + existing spawn  
**Step 3:** Commit `feat(effort-brains): wire mandatory effort team to brain config`

---

### Task 10: Synthesis disagreement package

**Objective:** Leader injection structures multi-brain conflict.

**Files:**
- Modify: `format_team_report_package` in `effort_mode.rs` (or `effort_brains/synthesize.rs`)

**Step 1:** Test package contains Consensus / Conflicts / Unique / Residuals / Decision headings and brain ids  
**Step 2:** Implement  
**Step 3:** Commit `feat(effort-brains): disagreement-oriented team report package`

---

### Task 11: Optional model override hook (thin)

**Objective:** Plumb `model` field if spawn path supports it; otherwise document deferred.

**Files:**
- Modify: spawn request construction in `effort_team.rs` / subagent request types

**Gate:** Only if existing runtime overrides already support model — do not invent new model routing in this task.  
**Step:** Spike → implement or write `DEFERRED.md` note in this folder.  
**Commit:** `feat(effort-brains): optional model override plumbing` or `docs: defer brain model overrides`

---

### Task 12: Docs + seed export instructions

**Objective:** Operator-facing docs.

**Files:**
- Create: `docs/effort-modes-builtin/08-reasoning-brains.md` (user/product spec: 16 brains, edit guide, Expert random, Heavy full)
- Modify: `docs/effort-modes-builtin/README.md` (index)
- Modify: `docs/effort-modes-builtin/01-tech-spec.md` — short pointer to brains (do not fork entire spec unless needed)
- Modify: `docs/effort-modes-builtin/06-open-questions.md` — add Q10 brains resolved summary

**Commit:** `docs(effort-brains): user guide and index`

---

### Task 13: Test plan alignment

**Objective:** Extend automated tests list.

**Files:**
- Modify: `docs/effort-modes-builtin/04-test-plan.md` with T-Brain-* cases  
- Ensure unit tests cover:
  - T-Brain-1 defaults validate  
  - T-Brain-2 expert k=4 unique  
  - T-Brain-3 heavy 16 + contrarian class present  
  - T-Brain-4 seed reproducibility  
  - T-Brain-5 project override wins  
  - T-Brain-6 invalid roster errors  
  - T-Brain-7 brief prompt contains protocol markers  
  - T-Brain-8 no write encouragement in prompt  
  - T-Brain-9 synthesis package structure  

**Commit:** `test(effort-brains): document and cover brain test matrix`

---

### Task 14: Manual verification on powergrok install

**Objective:** Prove UX on real binary.

**Steps:**
1. `cargo build -p xai-grok-pager-bin --release --features powergrok`  
2. Install to `~/.local/lib/powergrok/powergrok` (existing wrapper)  
3. Session: `/heavy` on non-trivial task → chrome shows brain ids; 16 slots  
4. Session: `/expert` twice on non-trivial tasks → different brain sets (unless seed fixed)  
5. Break user catalog deliberately → clear error, no silent angle fallback  
6. `/normal` clears elevated mode  

**Commit:** none required; note results in PR description.

---

## 7. File touch map (summary)

| Area | Path |
|------|------|
| New module | `crates/codegen/xai-grok-shell/src/session/effort_brains/**` |
| Briefs / tracker | `.../session/effort_mode.rs` |
| Fan-out | `.../session/effort_team.rs` |
| Session hook | `.../session/acp_session_impl/session_mode.rs` |
| Paths | `xai-grok-config` paths (read-only use) |
| Docs | `docs/effort-modes-builtin/07-*.md` (this), `08-*.md`, README, 01/04/06 updates |

---

## 8. Risks & mitigations

| Risk | Mitigation |
|------|------------|
| Prompt-only diversity still one model | L2 artifacts + disagreement synthesis; L4 later |
| Latency at N=16 | Unchanged parallelism; brains don't add serial waves in v1 |
| User misconfig | Validate hard; error message lists missing ids |
| Merge surprises | Document F6; tests for override |
| Drift from 4/16 DoD | Validate k/n; experimental flag only to change |
| Skills double path | Builtins only; skills demoted when builtins on |
| Large binary from 16 md embeds | Acceptable; keep brains short |

---

## 9. Success criteria

1. Built-in 16 brains load with zero user config.  
2. Heavy non-trivial run briefs reference **all 16 ids** exactly once.  
3. Expert non-trivial run briefs reference **exactly 4 distinct** catalog ids; reseedable.  
4. No remaining dependency on `specialist_angle` topic cycle for mandatory teams.  
5. Heavy full-team claim still requires contrarian-class success.  
6. User can edit `$GROK_HOME/effort-brains/brains/*.md` and see prompt changes on next team run.  
7. Project `.powergrok/effort-brains` overrides user by id.  
8. Unit tests T-Brain-1..9 green without network/live model.  
9. Docs explain edit workflow and no-cosplay rule.

---

## 10. PR slicing recommendation

| PR | Scope |
|----|--------|
| PR-A | Tasks 1–3 (scaffold, defaults, validate) |
| PR-B | Tasks 4–7 (merge, select, prompt, wire briefs) |
| PR-C | Tasks 8–10 (tracker, session wire, synthesis) |
| PR-D | Tasks 11–13 (optional model, docs, test plan) |
| Manual | Task 14 on powergrok binary |

Base branch: effort-capable product branch (e.g. `feat/branding-powergrok` only if needed for install; prefer branch from effort-modes worktree / `powergrok` trunk — **do not block on branding**). Suggested branch name: `feat/effort-reasoning-brains`.

---

## 11. Open items deliberately deferred

- Multi-family Expert sampling  
- Two-phase Heavy (peer-aware meta brains)  
- GUI editor for brains  
- Automatic seed export to `~/.powergrok` on first launch  
- Headless `--effort-brains-dir`  
- Upstream contribution packaging  

---

## 12. Next immediate actions

1. Principal: confirm F1–F14 still frozen (especially Expert pure-random and contrarian_class set).  
2. Implement PR-A (Tasks 1–3) under global 4-phase code authoring discipline.  
3. After PR-A green, proceed PR-B without expanding scope to L4 models.

---

*End of implementation plan.*
