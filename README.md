<div align="center">

# Power Grok (`powergrok`)

**Power Grok** is a **source-built, parallel install** of [Grok Build](https://github.com/xai-org/grok-build)
that coexists with the official `grok` CLI. It is the product line of this fork
([zfifteen/powergrok](https://github.com/zfifteen/powergrok)): same agent runtime
family, **different product identity**, private state, and fork-first session modes.

[What makes this fork different](#what-makes-this-fork-different) ·
[Install (side-by-side)](#install-side-by-side) ·
[Building from source](#building-from-source) ·
[Documentation](#documentation) ·
[Repository layout](#repository-layout) ·
[Development](#development) ·
[Branch model](#branch-model) ·
[License](#license)

**Command:** `powergrok` · **User state:** `~/.powergrok` · **Trunk:** `powergrok` branch

</div>

---

## What makes this fork different

Upstream Grok Build is a terminal AI coding agent: full-screen TUI, codebase
understanding, edits, shell, web search, long-running tasks, headless/CI, and
ACP for editors. Power Grok keeps that substrate and adds a **product layer**
you will not get from a stock install alone.

### 1. Parallel product, not a rename

| Concern | Official Grok Build | Power Grok |
|---------|---------------------|------------|
| Command | `grok` | **`powergrok`** |
| User state | `~/.grok` | **`~/.powergrok`** (`GROK_HOME`) |
| Project tree | `<repo>/.grok/` | **`<repo>/.powergrok/`** (when run as `powergrok`) |
| Auto-update | product default | **off** in Power Grok seed (source-built stays put) |
| Concurrent use | — | **Supported** alongside official `grok` |

Power Grok does **not** overwrite the official binary. Full isolation contract:
[`docs/powergrok/BUILD_PLAN.md`](docs/powergrok/BUILD_PLAN.md). Branding rules
(“Power Grok” in prose, `powergrok` for binary/paths):
[`docs/powergrok/BRANDING_PLAN.md`](docs/powergrok/BRANDING_PLAN.md).

### 2. Builtin effort modes — Expert, Heavy, Normal

Session **effort modes** are first-party shell builtins (`/expert`, `/heavy`,
`/normal`): sticky mode, TUI chrome, and hard local multi-agent gates for
non-trivial work.

| Mode | Fixed successful specialists | Role |
|------|------------------------------|------|
| **Normal** | none required | Default single-leader Power Grok |
| **Expert** | **exactly 4** | High-quality interactive team depth |
| **Heavy** | **exactly 16**, ≥1 contrarian | Maximum local multi-agent depth |

What that means in practice:

- **Shell-mandatory fan-out** for non-trivial Expert/Heavy turns — not “hope the
  model remembers a skill.”
- **Analytic-only fixed team** (read/search specialists; no repo writes inside N).
- **Join-all → leader synthesis → optional post-N execute** after synthesis.
- **Replace caps, abort → partial report, solo waiver** — enforceable ledger
  semantics, not soft prompt theater.
- **Not** the platform multi-agent research API as the product path; this is
  **local** orchestration on your machine.

Program docs: [`docs/effort-modes-builtin/`](docs/effort-modes-builtin/).

### 3. Sixteen reasoning brains (not cosplay personas)

Expert and Heavy specialists are not generic `specialist-0..N` fillers. They run
**versioned reasoning protocols** — method, forbidden moves, artifact shape, and
a deliberate **delta role** so the team disagrees productively.

| Roster rule | Behavior |
|-------------|----------|
| **Pool** | **16** built-in brains |
| **Expert** | **Random 4** of 16 (without replacement per team run; seedable via `GROK_EFFORT_BRAIN_SEED`) |
| **Heavy** | **All 16** in fixed order (≥1 `contrarian_class`; last slot defaults to `red_team`) |
| **Leader synthesis** | Disagreement-oriented package (consensus / conflicts / unique / residuals / decision) |

Catalog families (built-in ids):

| Family | Brains |
|--------|--------|
| Foundations | `first_principles`, `map_territory`, `circle_of_competence` |
| Systems | `systems_loops`, `theory_of_constraints`, `five_whys_root` |
| Consequences | `second_order`, `inversion`, `pre_mortem` |
| Decision quality | `scientific_method`, `bayesian_update`, `fermi_estimate` |
| Adversarial / dialectic | `via_negativa`, `ooda_tempo`, `steelman_dialectic`, `red_team` |

**Editable config** (seeded on first use from product defaults):

```text
$GROK_HOME/effort-brains/
  catalog.toml
  rosters/{expert,heavy}.toml
  brains/*.md
  README.md
```

Optional project overlay: `<workspace>/.powergrok/effort-brains/`. Merge-by-id;
invalid config fails loudly — no silent drop back to angle cosplay.

Implementation plan: [`docs/effort-modes-builtin/07-reasoning-brains-implementation-plan.md`](docs/effort-modes-builtin/07-reasoning-brains-implementation-plan.md).

### 4. Same agent, clearer product surface

You still get the Grok Build capabilities (TUI, tools, MCP, skills, plan mode,
headless, ACP). Power Grok adds **named isolation**, **Power Grok chrome**, and
**effort/brain orchestration** so deep work is a **mode you enter**, not a
prompt you hope sticks.

---

## Install (side-by-side)

Official released `grok` installers remain at [x.ai/cli](https://x.ai/cli) for
the stock product. **Power Grok** is built from **this repo’s `powergrok`
branch** and installed beside it.

Typical layout (see build plan for the full contract):

| Piece | Path |
|-------|------|
| Wrapper on `PATH` | `~/.local/bin/powergrok` |
| Real binary | `~/.local/lib/powergrok/powergrok` |
| Version stamp | `~/.local/lib/powergrok/VERSION` |
| User home | `~/.powergrok` (`GROK_HOME`) |

```sh
# From a clean checkout of this repo (product trunk):
git checkout powergrok
git pull origin powergrok

cargo build -p xai-grok-pager-bin --release --features powergrok

mkdir -p ~/.local/lib/powergrok ~/.local/bin ~/.powergrok
install -m 755 target/release/xai-grok-pager ~/.local/lib/powergrok/powergrok

# Wrapper: set GROK_HOME, branding, exec real binary
cat > ~/.local/bin/powergrok <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
export GROK_HOME="${GROK_HOME:-$HOME/.powergrok}"
export POWERGROK_BRANDING="${POWERGROK_BRANDING:-1}"
exec "$HOME/.local/lib/powergrok/powergrok" "$@"
EOF
chmod +x ~/.local/bin/powergrok

powergrok --version
powergrok --help   # should identify Power Grok under the powergrok feature
```

First launch still uses the product auth flow (browser / account) under
**Power Grok’s home**, not the official `~/.grok` tree. Authentication guide
(upstream pager docs still apply technically):
[`crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md`](crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md).

---

## Building from source

Requirements:

- **Rust** — pinned by [`rust-toolchain.toml`](rust-toolchain.toml); `rustup`
  installs it on first build.
- **protoc** — [`bin/protoc`](bin/protoc) (dotslash) or `protoc` on `PATH` /
  `$PROTOC`.
- macOS and Linux are supported build hosts; Windows is best-effort from this
  tree.

```sh
# Fast check
cargo check -p xai-grok-pager-bin --features powergrok

# Dev run (TUI)
cargo run -p xai-grok-pager-bin --features powergrok

# Release artifact (install as powergrok — see above)
cargo build -p xai-grok-pager-bin --release --features powergrok
# → target/release/xai-grok-pager
```

Always pass **`--features powergrok`** for product branding and the Power Grok
build line. Without it you get closer to stock Grok Build labeling.

---

## Documentation

| Doc | What |
|-----|------|
| [`docs/powergrok/BUILD_PLAN.md`](docs/powergrok/BUILD_PLAN.md) | Isolation, install, argv0 / `.powergrok/` contract |
| [`docs/powergrok/BRANDING_PLAN.md`](docs/powergrok/BRANDING_PLAN.md) | “Power Grok” vs `powergrok` style and feature rules |
| [`docs/effort-modes-builtin/`](docs/effort-modes-builtin/) | Expert / Heavy / Normal program (charter → tech spec → plans) |
| [`docs/effort-modes-builtin/07-reasoning-brains-implementation-plan.md`](docs/effort-modes-builtin/07-reasoning-brains-implementation-plan.md) | 16 brains, rosters, config, runtime wiring |
| [`AGENTS.md`](AGENTS.md) | Fork branch discipline (`main` intake vs `powergrok` trunk) |
| Pager user guide | [`crates/codegen/xai-grok-pager/docs/user-guide/`](crates/codegen/xai-grok-pager/docs/user-guide/) — shortcuts, slash commands, MCP, skills, headless, sandbox, … |
| Upstream product docs | [docs.x.ai/build](https://docs.x.ai/build/overview) (stock Grok Build) |

Upstream marketing / binary install: [x.ai/cli](https://x.ai/cli).

---

## Repository layout

| Path | Contents |
|------|----------|
| `crates/codegen/xai-grok-pager-bin` | Composition-root package; builds `xai-grok-pager` |
| `crates/codegen/xai-grok-pager` | TUI: scrollback, prompt, modals, rendering, Power Grok chrome |
| `crates/codegen/xai-grok-shell` | Agent runtime; **effort modes + reasoning brains** live under `session/` |
| `crates/codegen/xai-grok-tools` | Tools (terminal, file edit, search, …) |
| `crates/codegen/xai-grok-workspace` | Host filesystem, VCS, execution, checkpoints |
| `crates/codegen/xai-grok-config` | Paths, branding adapter (`product_name()`), config |
| `docs/powergrok/` | Fork product contracts (build, branding, reviews) |
| `docs/effort-modes-builtin/` | Effort mode + brains program documentation |
| `crates/common/`, `crates/build/`, `prod/mc/` | Shared leaf crates |
| `third_party/` | Vendored upstream source (e.g. Mermaid stack) |

> [!IMPORTANT]
> The root `Cargo.toml` (workspace members, dependency versions, lints,
> profiles) is **generated** — treat it as read-only. Prefer editing per-crate
> `Cargo.toml` files.

### Effort / brains code map (quick)

| Module | Role |
|--------|------|
| `xai-grok-shell::session::effort_mode` | Sticky modes, ledgers, gates, chrome, team briefs |
| `xai-grok-shell::session::effort_brains` | Catalog load/validate, select, prompt, seed export |
| `xai-grok-shell::session::effort_team` | Shell-owned specialist spawn / join |

---

## Development

```sh
cargo check -p <crate> --features powergrok   # target specific crates; full workspace is slow
cargo test -p xai-grok-shell --test effort_brains_load
cargo test -p xai-grok-config
cargo clippy -p <crate>
cargo fmt --all
```

Lint/format config: `clippy.toml` and `rustfmt.toml` at the repo root.

---

## Branch model

This repository is a **fork**, not an upstream mirror alone:

| Branch | Role |
|--------|------|
| **`main`** | Upstream intake only (`xai-org/grok-build`) |
| **`powergrok`** | **Product trunk** and GitHub default — builds and docs ship from here |
| **`feat/*`** | Feature work; open PRs **into `powergrok`**, never product work into `main` |

Details: [`AGENTS.md`](AGENTS.md).

---

## Relationship to upstream

- **Upstream source of truth for Grok Build engine:** [xai-org/grok-build](https://github.com/xai-org/grok-build)
- **This fork’s product line:** [zfifteen/powergrok](https://github.com/zfifteen/powergrok) on branch **`powergrok`**
- First-party Power Grok work (effort modes, brains, isolation, branding) lands
  on **`powergrok`**. Upstream sync enters via **`main`**, then merges forward.

---

## Contributing

This is a personal / operator fork. Contribution policy may differ from
upstream; see [`CONTRIBUTING.md`](CONTRIBUTING.md) for any inherited notices.
Feature work intended for Power Grok should target **`powergrok`** as the PR base.

---

## License

First-party code in this repository is licensed under the **Apache License,
Version 2.0** — see [`LICENSE`](LICENSE).

Third-party and vendored code remains under its original licenses. See:

- [`THIRD-PARTY-NOTICES`](THIRD-PARTY-NOTICES) — crates.io / git dependencies,
  bundled UI themes, and in-tree source ports
- [`crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md`](crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md)
- [`third_party/NOTICE`](third_party/NOTICE) — vendored Mermaid-stack index
