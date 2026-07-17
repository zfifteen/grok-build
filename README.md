<div align="center">

# Power Grok (`powergrok`)

**Expert · Heavy · Normal** — session **effort modes** for Grok Build, in the terminal.

Power Grok brings the **mental model of [Grok on the web](https://grok.com)**
(light / deeper / maximum multi-agent depth) into a **source-built, parallel
CLI/TUI** that coexists with the official `grok` binary. Under Expert and Heavy,
specialists run **customizable reasoning profiles** (“brains”) you can edit —
not stock cosplay personas.

[Effort modes](#effort-modes-expert--heavy--normal) ·
[Reasoning brains](#reasoning-brains-customizable-profiles) ·
[Install](#install-side-by-side) ·
[Build](#building-from-source) ·
[Docs](#documentation) ·
[Layout](#repository-layout) ·
[License](#license)

| Slash | Depth | Specialists |
|-------|-------|-------------|
| **`/normal`** | Default | Single leader — no fixed team |
| **`/expert`** | High | **4** analytic specialists |
| **`/heavy`** | Maximum | **16** analytic specialists (≥1 contrarian) |

**Command:** `powergrok` · **State:** `~/.powergrok` · **Trunk:** `powergrok` branch ·
**Fork of:** [xai-org/grok-build](https://github.com/xai-org/grok-build)

</div>

---

## Effort modes: Expert · Heavy · Normal

This is the headline product difference.

On [Grok Web](https://grok.com), deeper effort means more deliberate multi-agent
work before you get an answer. Power Grok **mirrors that mental model** for the
local coding agent: sticky session modes, visible TUI chrome, and **hard
fixed-N gates** for non-trivial tasks — implemented as first-party shell
builtins, not “please remember a skill.”

| Mode | Builtin | Fixed successful specialists | When to use |
|------|---------|------------------------------|-------------|
| **Normal** | `/normal` | none required | Day-to-day coding, chat, lookups |
| **Expert** | `/expert` | **exactly 4** | Design, audits, tricky bugs — interactive team depth |
| **Heavy** | `/heavy` | **exactly 16**, ≥1 contrarian | Highest local multi-agent depth before you commit |

### How a turn works (Expert / Heavy)

1. You stay in **sticky** mode (empty `/expert` or `/heavy` sets it for the session).
2. On **non-trivial** user work, the shell **must** fan out a fixed team.
3. Specialists are **analytic-only** (read/search — no repo writes inside N).
4. **Join-all** → **leader synthesis** (structured disagreement, not a bland average).
5. **Execute** (writes) only **after** synthesis, outside the fixed N.
6. Replace caps, abort → partial report, and solo waiver are **ledger rules**, not prompt hope.

Local orchestration on your machine — not a dependency on the platform multi-agent
research API as the product path.

Program docs: [`docs/effort-modes-builtin/`](docs/effort-modes-builtin/).

---

## Reasoning brains (customizable profiles)

Expert and Heavy specialists are driven by a pool of **16 reasoning brains**:
versioned **protocols** (method, forbidden moves, artifact shape, delta role).
They are **profiles for subagents**, not character cosplay.

| Roster | Rule |
|--------|------|
| **Pool** | **16** built-in brains (shipped + seeded for edit) |
| **Expert** | Draws a **random 4** of 16 per team run (optional `GROK_EFFORT_BRAIN_SEED`) |
| **Heavy** | Runs **all 16** in fixed order (contrarian class required; `red_team` last by default) |
| **Synthesis** | Leader must surface consensus, conflicts, unique takes, residuals, and a decision |

### Built-in catalog (ids)

| Family | Brains |
|--------|--------|
| Foundations | `first_principles`, `map_territory`, `circle_of_competence` |
| Systems | `systems_loops`, `theory_of_constraints`, `five_whys_root` |
| Consequences | `second_order`, `inversion`, `pre_mortem` |
| Decision quality | `scientific_method`, `bayesian_update`, `fermi_estimate` |
| Adversarial / dialectic | `via_negativa`, `ooda_tempo`, `steelman_dialectic`, `red_team` |

### Customize the profiles

On first use, defaults seed under your Power Grok home. Edit freely; keep
frontmatter `id`s stable.

```text
$GROK_HOME/effort-brains/          # usually ~/.powergrok/effort-brains
  catalog.toml                     # list + flags (e.g. allow_model_overrides)
  rosters/expert.toml              # selection = random, k = 4
  rosters/heavy.toml               # fixed 16-slot order
  brains/*.md                      # protocol body + YAML frontmatter
  README.md
```

Optional **project** overlay: `<workspace>/.powergrok/effort-brains/` (merge-by-id).
Invalid config fails loudly — no silent fallback to generic specialists.

Optional per-brain **model** overrides when `allow_model_overrides = true` in
`catalog.toml`.

Plan: [`docs/effort-modes-builtin/07-reasoning-brains-implementation-plan.md`](docs/effort-modes-builtin/07-reasoning-brains-implementation-plan.md).

---

## Also: a parallel install of Grok Build

Under the modes sits a full Grok Build–family agent (TUI, tools, MCP, skills,
plan mode, headless, ACP). Power Grok is a **side-by-side product**, not a rename
of official `grok`:

| Concern | Official Grok Build | Power Grok |
|---------|---------------------|------------|
| Command | `grok` | **`powergrok`** |
| User state | `~/.grok` | **`~/.powergrok`** (`GROK_HOME`) |
| Project tree | `<repo>/.grok/` | **`<repo>/.powergrok/`** (when run as `powergrok`) |
| Auto-update | product default | **off** in Power Grok seed |
| Concurrent use | — | **Supported** with official `grok` |

Isolation contract: [`docs/powergrok/BUILD_PLAN.md`](docs/powergrok/BUILD_PLAN.md).  
Branding (“Power Grok” in prose, `powergrok` for binary/paths):
[`docs/powergrok/BRANDING_PLAN.md`](docs/powergrok/BRANDING_PLAN.md).

---

## Install (side-by-side)

Official released `grok` installers: [x.ai/cli](https://x.ai/cli).  
**Power Grok** builds from **this repo’s `powergrok` branch**.

| Piece | Path |
|-------|------|
| Wrapper on `PATH` | `~/.local/bin/powergrok` |
| Real binary | `~/.local/lib/powergrok/powergrok` |
| Version stamp | `~/.local/lib/powergrok/VERSION` |
| User home | `~/.powergrok` (`GROK_HOME`) |

```sh
git checkout powergrok
git pull origin powergrok

cargo build -p xai-grok-pager-bin --release --features powergrok

mkdir -p ~/.local/lib/powergrok ~/.local/bin ~/.powergrok
install -m 755 target/release/xai-grok-pager ~/.local/lib/powergrok/powergrok

cat > ~/.local/bin/powergrok <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
export GROK_HOME="${GROK_HOME:-$HOME/.powergrok}"
export POWERGROK_BRANDING="${POWERGROK_BRANDING:-1}"
exec "$HOME/.local/lib/powergrok/powergrok" "$@"
EOF
chmod +x ~/.local/bin/powergrok

powergrok --version
powergrok --help
```

Then in a session: `/expert` or `/heavy` for multi-agent depth; `/normal` to
return to single-leader defaults.

Auth still uses the product browser/account flow, under **Power Grok’s home**:
[`crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md`](crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md).

---

## Building from source

- **Rust** — [`rust-toolchain.toml`](rust-toolchain.toml) (rustup installs on first build)
- **protoc** — [`bin/protoc`](bin/protoc) or `PATH` / `$PROTOC`
- macOS and Linux supported; Windows best-effort

```sh
cargo check -p xai-grok-pager-bin --features powergrok
cargo run -p xai-grok-pager-bin --features powergrok
cargo build -p xai-grok-pager-bin --release --features powergrok
# → target/release/xai-grok-pager
```

Always pass **`--features powergrok`** for Power Grok branding and product line.

---

## Documentation

| Doc | What |
|-----|------|
| [`docs/effort-modes-builtin/`](docs/effort-modes-builtin/) | **Expert / Heavy / Normal** charter, tech spec, architecture, tests |
| [`docs/effort-modes-builtin/07-reasoning-brains-implementation-plan.md`](docs/effort-modes-builtin/07-reasoning-brains-implementation-plan.md) | **16 brains**, rosters, config, runtime |
| [`docs/powergrok/BUILD_PLAN.md`](docs/powergrok/BUILD_PLAN.md) | Isolation + install |
| [`docs/powergrok/BRANDING_PLAN.md`](docs/powergrok/BRANDING_PLAN.md) | “Power Grok” vs `powergrok` |
| [`AGENTS.md`](AGENTS.md) | Branch discipline (`main` intake vs `powergrok` trunk) |
| Pager user guide | [`crates/codegen/xai-grok-pager/docs/user-guide/`](crates/codegen/xai-grok-pager/docs/user-guide/) |
| Upstream docs | [docs.x.ai/build](https://docs.x.ai/build/overview) |

---

## Repository layout

| Path | Contents |
|------|----------|
| `crates/codegen/xai-grok-pager-bin` | Binary composition root (`xai-grok-pager`) |
| `crates/codegen/xai-grok-pager` | TUI + Power Grok chrome |
| `crates/codegen/xai-grok-shell` | Agent runtime; **effort modes + brains** under `session/` |
| `crates/codegen/xai-grok-tools` | Tools (terminal, edit, search, …) |
| `crates/codegen/xai-grok-workspace` | Filesystem, VCS, execution, checkpoints |
| `crates/codegen/xai-grok-config` | Paths, branding adapter, config |
| `docs/effort-modes-builtin/` | Effort mode + brains program docs |
| `docs/powergrok/` | Fork product contracts |
| `third_party/` | Vendored upstream source |

> [!IMPORTANT]
> Root `Cargo.toml` is **generated** — treat as read-only. Edit per-crate
> `Cargo.toml` files.

### Effort / brains code map

| Module | Role |
|--------|------|
| `session::effort_mode` | Sticky modes, ledgers, gates, chrome, team briefs |
| `session::effort_brains` | Catalog load/validate, select, prompt, seed export |
| `session::effort_team` | Shell-owned specialist spawn / join |

---

## Development

```sh
cargo check -p <crate> --features powergrok
cargo test -p xai-grok-shell --test effort_brains_load
cargo test -p xai-grok-config
cargo clippy -p <crate>
cargo fmt --all
```

---

## Branch model

| Branch | Role |
|--------|------|
| **`main`** | Upstream intake only (`xai-org/grok-build`) |
| **`powergrok`** | **Product trunk** — default branch; builds ship from here |
| **`feat/*`** | Feature work; PRs **into `powergrok`**, not product work into `main` |

Details: [`AGENTS.md`](AGENTS.md).

**Upstream engine:** [xai-org/grok-build](https://github.com/xai-org/grok-build) ·
**This product line:** [zfifteen/powergrok](https://github.com/zfifteen/powergrok)

---

## Contributing

Operator fork. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for inherited notices.
Power Grok features should target **`powergrok`** as the PR base.

---

## License

First-party code: **Apache License, Version 2.0** — [`LICENSE`](LICENSE).

Third-party / vendored code retains original licenses:

- [`THIRD-PARTY-NOTICES`](THIRD-PARTY-NOTICES)
- [`crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md`](crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md)
- [`third_party/NOTICE`](third_party/NOTICE)
