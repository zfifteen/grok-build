<div align="center">

# Power Grok

**Expert · Heavy · Normal** — session effort modes for a coding agent that
actually sticks with them.

![Power Grok — Normal · Expert · Heavy effort modes](docs/powergrok/assets/readme-hero.jpg)

[What it adds](#what-power-grok-adds) ·
[How modes work](#how-expert-heavy-and-normal-work) ·
[Brains](#reasoning-brains-your-subagent-profiles) ·
[Try it](#try-it-in-a-session) ·
[Install](#install-side-by-side) ·
[Reference](#reference)

**Command:** `powergrok` ·
**Trunk:** [`powergrok`](https://github.com/zfifteen/powergrok/tree/powergrok) ·
**Fork of:** [xai-org/grok-build](https://github.com/xai-org/grok-build)

</div>

---

## What Power Grok adds

If you use [Grok on the web](https://grok.com), you already know the idea:
sometimes you want a quick answer; sometimes you want the model to **work harder**
before it speaks. Deeper modes feel like a team thinking, not a single reply.

**Power Grok** brings that **same depth idea** (light / deeper / max team) into the
terminal — as **local shell modes**, not a clone of the web UI or a dependency
on the platform multi-agent research API.

It’s a **source-built fork** of [Grok Build](https://github.com/xai-org/grok-build):
same agent family (TUI, tools, edits, shell, MCP, plan mode, headless, ACP), plus
**first-class effort modes** and **customizable specialist profiles** (“brains”).
It installs **next to** the official `grok` CLI; it doesn’t replace it.

On the **`powergrok` trunk**, Expert / Heavy / Normal and the 16-brain roster are
product features, not skill markdown you hope the model remembers.

In short: **web-style effort depth, local by design.**

Need the binary first? Jump to [Install](#install-side-by-side).

---

## How Expert, Heavy, and Normal work

Type a slash command. The mode **sticks** for the session. The chrome tells you
where you are. For **non-trivial** work under Expert or Heavy, the shell
**enforces** a fixed team size instead of hoping the model “remembers the policy.”
Trivial turns (quick lookups, small fixes) stay single-leader even in elevated modes.

| You type | Feel | What happens |
|----------|------|----------------|
| **`/normal`** | Everyday agent | One leader. Fast. Default coding and chat. |
| **`/expert`** | “Think harder with me” | **4** analytic specialists, then a synthesis. Usual sweet spot. |
| **`/heavy`** | “Leave no angle unturned” | **16** analytic specialists (≥1 contrarian), then synthesis. Thorough — and **costly / slower**. |

### What “analytic specialists” means

Under Expert and Heavy, the fixed team is there to **read, search, argue, and
write structured reports** — not to rewrite your repo mid-deliberation.
Writes wait until **after** the leader has synthesized the team’s work.

Depth first; execution second.

### A typical Expert/Heavy turn

1. You’re already in `/expert` or `/heavy` (sticky — set it once).
2. You ask for something that actually needs depth (design, audit, hard bug).
3. Power Grok spawns the fixed team **locally** on your machine.
4. Everyone finishes (or you abort / take a partial report under the rules).
5. The leader gets a **disagreement-oriented** package: where the brains agreed,
   where they conflicted, what only one of them saw, what’s still open.
6. **Then** you can execute — implement, patch, commit — with eyes open.

No “skill theater.” The gates live in the shell.

More detail: [`docs/effort-modes-builtin/`](docs/effort-modes-builtin/).

---

## Reasoning brains: your subagent profiles

The specialists aren’t blank slots labeled `specialist-3`.

They run **brains** — short, versioned **reasoning protocols**: how to think,
what moves are forbidden, what artifact to produce, and what *kind* of delta
they’re responsible for. Not cosplay personas. Not job-title roleplay.

### The simple rules

- There is a pool of **16** built-in brains.
- **Expert** picks a **random 4** of those 16 for each team run (variety without
  always spinning the full roster; optional seed: `GROK_EFFORT_BRAIN_SEED`).
- **Heavy** runs **all 16**, in a fixed order, with at least one deliberately
  contrarian profile in the mix.
- You can **edit** the pool. That’s the point.

### Built-ins (ids you’ll see in chrome and reports)

| Family | Brains |
|--------|--------|
| Foundations | `first_principles`, `map_territory`, `circle_of_competence` |
| Systems | `systems_loops`, `theory_of_constraints`, `five_whys_root` |
| Consequences | `second_order`, `inversion`, `pre_mortem` |
| Decision quality | `scientific_method`, `bayesian_update`, `fermi_estimate` |
| Adversarial / dialectic | `via_negativa`, `ooda_tempo`, `steelman_dialectic`, `red_team` |

### Make them yours

On first use, Power Grok seeds editable copies under your home:

```text
~/.powergrok/effort-brains/
  catalog.toml
  rosters/expert.toml      # random 4
  rosters/heavy.toml       # all 16, ordered
  brains/*.md              # the actual protocols
```

Want a project-specific set? Drop an overlay at
`<repo>/.powergrok/effort-brains/`. Merge is by id; broken config fails **loud**
instead of quietly turning into generic specialists.

Optional: per-brain model overrides when you set `allow_model_overrides = true`
in `catalog.toml`.

Full plan: [`docs/effort-modes-builtin/07-reasoning-brains-implementation-plan.md`](docs/effort-modes-builtin/07-reasoning-brains-implementation-plan.md).

---

## Try it in a session

After [install](#install-side-by-side) puts `powergrok` on your `PATH` (and you’ve
completed product auth under `~/.powergrok`):

```text
powergrok
/expert          # sticky
# … ask for a real design review or audit …

/heavy           # full 16 — expect more time and cost
/normal          # back to single-leader day-to-day
```

You’ll see mode progress in chrome (e.g. `Expert 2 of 4 · bayesian_update`).
That’s the team working — not a wallpaper label.

---

## Side-by-side with official `grok`

| Concern | Official | Power Grok |
|---------|----------|------------|
| Command | `grok` | **`powergrok`** |
| User state | `~/.grok` | **`~/.powergrok`** (`GROK_HOME`) |
| Project tree | `<repo>/.grok/` | **`<repo>/.powergrok/`** (when run as `powergrok`) |
| Concurrent use | — | **Supported** |
| Auto-update | product default | **off** in Power Grok seed (source-built stays put) |

Full isolation contract: [`docs/powergrok/BUILD_PLAN.md`](docs/powergrok/BUILD_PLAN.md).  
Branding (“Power Grok” in prose, `powergrok` for binary/paths):
[`docs/powergrok/BRANDING_PLAN.md`](docs/powergrok/BRANDING_PLAN.md).

---

## Reference

Install paths, build flags, layout, and ops. The product story is above; this
half is the conventional README.

### Install (side-by-side)

Official released `grok` installers: [x.ai/cli](https://x.ai/cli).  
**Power Grok** builds from **this repo’s `powergrok` branch**.

| Piece | Path |
|-------|------|
| Wrapper on `PATH` | `~/.local/bin/powergrok` |
| Real binary | `~/.local/lib/powergrok/powergrok` |
| Version stamp | `~/.local/lib/powergrok/VERSION` |
| User home | `~/.powergrok` (`GROK_HOME`) |

```sh
git clone https://github.com/zfifteen/powergrok.git
cd powergrok
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

# Ensure ~/.local/bin is on PATH, then:
powergrok --version
powergrok --help
```

First launch opens the product browser/account flow under **Power Grok’s home**
(not official `~/.grok`). See
[`crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md`](crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md).

Longer install / isolation notes: [`docs/powergrok/BUILD_PLAN.md`](docs/powergrok/BUILD_PLAN.md).

### Building from source

**Requirements**

- **Rust** — pinned by [`rust-toolchain.toml`](rust-toolchain.toml); `rustup` installs on first build
- **protoc** — [`bin/protoc`](bin/protoc) (dotslash) or `protoc` on `PATH` / `$PROTOC`
- macOS and Linux supported; Windows best-effort from this tree

```sh
cargo check -p xai-grok-pager-bin --features powergrok
cargo run -p xai-grok-pager-bin --features powergrok
cargo build -p xai-grok-pager-bin --release --features powergrok
# → target/release/xai-grok-pager
```

Always pass **`--features powergrok`** for Power Grok branding and the product build line.

### Documentation

| Doc | What |
|-----|------|
| [`docs/effort-modes-builtin/`](docs/effort-modes-builtin/) | Expert / Heavy / Normal program (charter → tech spec → plans) |
| [`docs/effort-modes-builtin/07-reasoning-brains-implementation-plan.md`](docs/effort-modes-builtin/07-reasoning-brains-implementation-plan.md) | 16 brains, rosters, config, runtime |
| [`docs/powergrok/BUILD_PLAN.md`](docs/powergrok/BUILD_PLAN.md) | Isolation + install contract |
| [`docs/powergrok/BRANDING_PLAN.md`](docs/powergrok/BRANDING_PLAN.md) | “Power Grok” (prose) vs `powergrok` (binary/paths) |
| [`AGENTS.md`](AGENTS.md) | Branch discipline (`main` intake vs `powergrok` trunk) |
| Pager user guide | [`crates/codegen/xai-grok-pager/docs/user-guide/`](crates/codegen/xai-grok-pager/docs/user-guide/) |
| Upstream docs | [docs.x.ai/build](https://docs.x.ai/build/overview) |

### Repository layout

| Path | Contents |
|------|----------|
| `crates/codegen/xai-grok-pager-bin` | Binary composition root (`xai-grok-pager`) |
| `crates/codegen/xai-grok-pager` | TUI + Power Grok chrome |
| `crates/codegen/xai-grok-shell` | Agent runtime; **effort modes + brains** under `session/` |
| `crates/codegen/xai-grok-tools` | Tools (terminal, edit, search, …) |
| `crates/codegen/xai-grok-workspace` | Filesystem, VCS, execution, checkpoints |
| `crates/codegen/xai-grok-config` | Paths, branding adapter, config |
| `docs/effort-modes-builtin/` | Effort mode + brains program docs |
| `docs/powergrok/` | Fork product contracts + README assets |
| `third_party/` | Vendored upstream source |

> [!IMPORTANT]
> Root `Cargo.toml` is **generated** — treat as read-only. Prefer editing per-crate `Cargo.toml` files.

#### Effort / brains code map

| Module | Role |
|--------|------|
| `session::effort_mode` | Sticky modes, ledgers, gates, chrome, team briefs |
| `session::effort_brains` | Catalog load/validate, select, prompt, seed export |
| `session::effort_team` | Shell-owned specialist spawn / join |

### Development

```sh
cargo check -p <crate> --features powergrok
cargo test -p xai-grok-shell --test effort_brains_load
cargo test -p xai-grok-config
cargo clippy -p <crate>
cargo fmt --all
```

### Branch model

| Branch | Role |
|--------|------|
| **`main`** | Upstream intake only (`xai-org/grok-build`) |
| **`powergrok`** | **Product trunk** — GitHub default; builds ship from here |
| **`feat/*`** | Feature work; open PRs **into `powergrok`**, not product work into `main` |

Details: [`AGENTS.md`](AGENTS.md).

### Power-user knobs (optional)

| Knob | Purpose |
|------|---------|
| `GROK_HOME` | Defaults to `~/.powergrok` via wrapper |
| `POWERGROK_BRANDING=1` | Non-feature branding override (wrapper sets this) |
| `GROK_EFFORT_BRAIN_SEED` | Deterministic Expert brain draw (tests / repro) |

### Contributing

Operator fork. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for inherited notices.  
Power Grok features should target **`powergrok`** as the PR base.

### License

First-party code: **Apache License, Version 2.0** — [`LICENSE`](LICENSE).

Third-party / vendored code retains original licenses:

- [`THIRD-PARTY-NOTICES`](THIRD-PARTY-NOTICES)
- [`crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md`](crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md)
- [`third_party/NOTICE`](third_party/NOTICE)
