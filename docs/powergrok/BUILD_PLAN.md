# Powergrok Build & Install Plan

**Status:** Draft for peer review — **operator decisions locked** (see §0)  
**Date:** 2026-07-15 (revised 2026-07-15 after clarifying Q&A)  
**Document branch (this file):** `feat/effort-modes-builtin`  
**Implementation branch (Phases 1+):** dedicated branch off `main` (e.g. `chore/powergrok-install`) — **do not** stack engine work on the effort-modes PR  
**Audience:** Engineers reviewing a side-by-side macOS install of a locally built Grok Build binary that coexists with the official `grok` install  
**Primary operator platform:** macOS (Apple Silicon and Intel both in scope; paths below use `$HOME` and are architecture-agnostic)

---

## 0. Locked operator decisions

Captured via sequential TUI menu review. These override earlier “recommended / open” wording in the first draft.

| # | Topic | Decision |
|---|--------|----------|
| D1 | User state root (`GROK_HOME`) | **`$HOME/.powergrok`** |
| D2 | On-disk binary layout | **Wrapper + lib binary**: `~/.local/bin/powergrok` → `exec` → `~/.local/lib/powergrok/xai-grok-pager` |
| D3 | Config seed (telemetry / feedback) | **Product defaults** for telemetry and feedback; seed **only** `[cli] auto_update = false` |
| D4 | Concurrent use | **Supported**: official `grok` and `powergrok` may run at the same time |
| D5 | Project config isolation | **Required in v1** (not deferred) |
| D6 | Project isolation mechanism | When **argv0 basename is `powergrok`**, use repo **`.powergrok/`** instead of **`.grok/`** for the entire project tree |
| D7 | Precedence if both trees exist | **`.powergrok/` only** — no read of project `.grok/`, no merge, no fallback |
| D8 | Completions (Phase 2) | Install **powergrok-named** completions **under `~/.powergrok` only** (not system fish `grok.fish`) |
| D9 | Version identity | **`$HOME/.local/lib/powergrok/VERSION` file** + stock binary `--version` (no engine version string patch) |
| D10 | Implementation landing | **Dedicated branch off `main`** for install script + project-dir engine change |

**Implication for v1 scope:** install layout alone is **insufficient**. v1 includes a **small, well-scoped product change**: resolve the project config directory name from process identity (`powergrok` → `.powergrok`).

---

## 1. Executive summary

**Powergrok** is a **local, source-built** installation of this repository’s Grok Build CLI/TUI that:

1. Is invoked as **`powergrok`** (never overwrites the official `grok` command).
2. Uses a **private user state directory** via `GROK_HOME=$HOME/.powergrok`.
3. Uses a **private project tree** via argv0-driven directory name **`.powergrok/`** at the workspace (entire parallel of project `.grok/`).
4. Keeps **auto-update off** so official release channels never replace the source-built binary under the powergrok home.
5. Supports **concurrent** sessions with official `grok`.

### Isolation primitives

| Layer | Official | Powergrok | Mechanism |
|-------|----------|-----------|-----------|
| Command | `grok` | `powergrok` | Distinct PATH entry + wrapper |
| User state | `~/.grok` | `~/.powergrok` | `GROK_HOME` (existing product contract) |
| Project tree | `<repo>/.grok/` | `<repo>/.powergrok/` | **New:** argv0 basename `powergrok` → project dir name `.powergrok` |
| Auto-update | product default | forced off in seed | `$GROK_HOME/config.toml` |

User-home implementation (existing): `crates/codegen/xai-grok-config/src/paths.rs` (`grok_home()`, `default_grok_home()`, `user_grok_home()`, `grok_application()`).

Project-dir implementation (required new work): centralize project directory **basename** (today many call sites hardcode `".grok"`) behind a single resolver, e.g. `project_config_dirname()` → `".grok"` | `".powergrok"`.

### Locked powergrok identity (v1)

| Property | Value |
|----------|--------|
| Command on `PATH` | `powergrok` |
| Real binary location | `$HOME/.local/lib/powergrok/xai-grok-pager` |
| Wrapper | `$HOME/.local/bin/powergrok` (sets `GROK_HOME`, `exec`s real binary; **argv0 seen by the process must be `powergrok`**) |
| User state root | `$HOME/.powergrok` (`GROK_HOME`) |
| Project tree | `<workspace>/.powergrok/` when running as `powergrok` |
| Auto-update | **Disabled** (`[cli] auto_update = false` only; telemetry/feedback unchanged) |
| Completions | Under `$GROK_HOME/completions/…` with command name `powergrok` |
| Version stamp | `$HOME/.local/lib/powergrok/VERSION` (git SHA, build time); `--version` stock |
| Official install | Untouched |

**Wrapper argv0 note:** The process that implements project-dir detection must see basename `powergrok`. Prefer:

```sh
exec -a powergrok "$REAL_BIN" "$@"
```

or install/copy the binary as a file literally named `powergrok` and `exec` that path. A wrapper that `exec`s `xai-grok-pager` **without** argv0 rewriting will **fail D6** unless an alternate signal (env flag) is added—see §6.3.

---

## 2. Goals

1. **Build** a release binary of `xai-grok-pager-bin` from this tree on the operator’s Mac.
2. **Install** that binary so the operator invokes it as `powergrok` with correct argv0.
3. **Isolate user-global state** via `GROK_HOME=$HOME/.powergrok`.
4. **Isolate project state** via `.powergrok/` for the full project tree when argv0 is `powergrok`.
5. **Preserve** the official `grok` binary, symlink chain, auth, sessions, updater, and project `.grok/` behavior.
6. **Disable** auto-update for the powergrok home only.
7. **Support concurrent** official + powergrok sessions.
8. **Document** rebuild → reinstall and a verification matrix covering both homes and both project trees.
9. **Land** install script + project-dir engine change on a **dedicated branch off `main`**.

---

## 3. Scope of this plan

### 3.1 In scope (v1)

| Item | Detail |
|------|--------|
| macOS host build | `cargo build -p xai-grok-pager-bin --release` (`rust-toolchain.toml` → 1.92.0) |
| Binary packaging | Install to `~/.local/lib/powergrok/`; wrapper to `~/.local/bin/powergrok` |
| Wrapper | Exports `GROK_HOME`; preserves/forces argv0 `powergrok`; create-if-missing config seed |
| Private user home | `$HOME/.powergrok` |
| Project dir engine change | Single resolver for project basename; argv0 `powergrok` → `.powergrok`; **no** fallback to `.grok` |
| Entire project tree under new name | `config.toml`, `skills/`, `plugins/`, `agents/`, `hooks/`, `lsp.json`, `sandbox.toml`, `personas/`, and any other project-scoped `.grok/*` peers |
| Completions | Generate/install under `~/.powergrok/completions/` for command name `powergrok` |
| VERSION stamp | Written at install/rebuild time |
| Official coexistence | Never write `~/.local/bin/grok` or mutate `~/.grok/bin/*` |
| Concurrent use | Verification includes both processes alive |
| Install script | `scripts/install-powergrok.sh` (+ wrapper source) |
| Tests | Unit/integration for project dirname resolver; smoke that official still uses `.grok` |

### 3.2 Deferred (later milestones)

| Item | Rationale |
|------|-----------|
| Second Cargo `[[bin]]` named `powergrok` | Optional; argv0 can be set via `exec -a` or install rename without a second bin target |
| Compile-time UI branding (“Powergrok” chrome) | Cosmetic |
| Patching `--version` string | Rejected for v1 (D9) |
| Merge/fallback project layers | Rejected for v1 (D7) |
| Linux packaging / Homebrew formula | macOS-first |
| Windows | Out of current operator need |
| Forked auth / alternate API endpoints | Same production endpoints (`xai-grok-env`) |
| Seeding telemetry/feedback off | Rejected for v1 (D3) |
| Shipping via official `install.sh` | Keep local / scripted only |
| Env-only project override (`GROK_PROJECT_DIRNAME`) | Optional escape hatch—**may** be added as implementation aid if argv0 is fragile; not a substitute for D6 |

### 3.3 Explicit dependencies on existing product behavior

1. **`GROK_HOME` is process-lifetime / first-read via `OnceLock`** in `xai_grok_config::paths::grok_home`. Must be set **before** process start.
2. **Sandbox writable roots** include user grok home via `xai_grok_sandbox` → `xai_grok_config::grok_home()`.
3. **Updater restart** prefers `$GROK_HOME/bin/grok` when present. Keep auto-update off; do not create managed `bin/grok` under powergrok home in v1.
4. **Fish completion regen** can write `$HOME/.config/fish/completions/grok.fish` on official update paths—powergrok must not use that path (D8).
5. **System config** `/etc/grok` remains shared if present (rare on personal Macs).
6. **Many call sites hardcode `".grok"`** for project trees (config load, skills discovery, hooks, sandbox profiles, agents modal, etc.). v1 **must** funnel these through one resolver or isolation is incomplete.

### 3.4 Known hard-coded project `.grok` surfaces (implementation inventory seed)

Non-exhaustive; Phase 1 engine work must `rg` and convert product call sites:

| Area | Example paths |
|------|----------------|
| Sandbox profiles | `workspace.join(".grok").join("sandbox.toml")` (`xai-grok-sandbox`) |
| Skills discovery | `SKILL_CONFIG_DIRS` / `".grok"` in tools skill discovery |
| Hooks trust | `dir.join(".grok")` (`xai-grok-hooks`) |
| Agents / personas UI | `cwd.join(".grok").join("personas")` (pager) |
| Extensions modal | path windows matching `".grok"` |
| Config loader project layers | project `.grok/config.toml` resolution |
| Workspace classifier | treats `.grok` as special dir name (may need `.powergrok` peer) |

Official `grok` must continue to resolve **only** `.grok` for project scope.

---

## 4. Background: official install topology (operator machine)

Typical layout:

```text
~/.local/bin/grok                    → symlink → ~/.grok/bin/grok
~/.grok/bin/grok                     → symlink → versioned binary (e.g. grok-0.2.101)
~/.grok/bin/agent                    → related entry point
~/.grok/config.toml                  user config
~/.grok/auth.json                    credentials
~/.grok/sessions/                    session store
~/.grok/skills/, plugins/, ...       extensions
~/.grok/downloads/                   release artifacts
```

Product docs: `GROK_HOME` overrides user config directory (default `~/.grok`) — `crates/codegen/xai-grok-pager/docs/user-guide/05-configuration.md`.

Build: `cargo build -p xai-grok-pager-bin --release` → `target/release/xai-grok-pager` (shipped officially as `grok`).

Composition root: `crates/codegen/xai-grok-pager-bin`.

---

## 5. Target topology (powergrok)

### 5.1 Directory layout

```text
$HOME/.local/bin/powergrok                 # wrapper (mode 0755); argv0 strategy per §5.2
$HOME/.local/lib/powergrok/
  xai-grok-pager                           # release binary (mode 0755)
  xai-grok-pager.prev                      # optional previous binary for rollback
  VERSION                                  # git SHA + built_at (+ optional --version text)

$HOME/.powergrok/                          # GROK_HOME
  config.toml                              # seeded once: auto_update = false only
  auth.json                                # first login (powergrok-only)
  sessions/
  skills/, plugins/, agents/, memory/, ...
  logs/
  completions/
    bash/powergrok.bash                    # Phase 2
    zsh/_powergrok                         # Phase 2
  bin/                                     # intentionally unused in v1
  downloads/                               # unused; auto_update off

<repo>/.powergrok/                         # project tree when running as powergrok
  config.toml
  skills/, plugins/, agents/, hooks/, ...
  lsp.json, sandbox.toml, personas/, ...

<repo>/.grok/                              # official project tree only (powergrok ignores)
```

### 5.2 Wrapper contract

**Required behavior:**

1. Default `POWERGROK_HOME` / `GROK_HOME` to `$HOME/.powergrok`.
2. `export GROK_HOME=…` before `exec`.
3. `mkdir -p "$GROK_HOME"`.
4. If `config.toml` missing, write seed (§7.3) — **never overwrite** existing config.
5. Ensure the **executed process argv0 basename is `powergrok`** (D6):
   - **Preferred:** `exec -a powergrok "$REAL_BIN" "$@"` (bash), or  
   - **Alternative:** install a second copy/symlink named `powergrok` in `POWERGROK_LIB` and `exec` that path.
6. Refuse to run if real binary missing (exit 127).
7. Refuse if `GROK_HOME` resolves to official `~/.grok` unless `POWERGROK_ALLOW_OFFICIAL_HOME=1`.
8. Optional: refuse if real binary realpath equals official `grok` realpath.

**Illustrative wrapper:**

```sh
#!/usr/bin/env bash
# powergrok-wrapper — marker for uninstall detection
set -euo pipefail

POWERGROK_LIB="${POWERGROK_LIB:-$HOME/.local/lib/powergrok}"
REAL_BIN="${POWERGROK_BIN:-$POWERGROK_LIB/xai-grok-pager}"
export GROK_HOME="${POWERGROK_HOME:-${GROK_HOME:-$HOME/.powergrok}}"

if [[ ! -x "$REAL_BIN" ]]; then
  echo "powergrok: missing binary at $REAL_BIN" >&2
  exit 127
fi

if [[ "${POWERGROK_ALLOW_OFFICIAL_HOME:-}" != "1" ]]; then
  official="${HOME}/.grok"
  if [[ -d "$GROK_HOME" && -d "$official" ]] \
    && [[ "$(cd "$GROK_HOME" && pwd -P)" == "$(cd "$official" && pwd -P)" ]]; then
    echo "powergrok: GROK_HOME resolves to official ~/.grok; aborting" >&2
    exit 2
  fi
fi

mkdir -p "$GROK_HOME"
if [[ ! -f "$GROK_HOME/config.toml" ]]; then
  cat >"$GROK_HOME/config.toml" <<'EOF'
# Powergrok local install — source-built binary.
[cli]
auto_update = false
EOF
fi

# Force argv0 so project-dir resolver selects .powergrok/
exec -a powergrok "$REAL_BIN" "$@"
```

### 5.3 Environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `GROK_HOME` | `$HOME/.powergrok` (via wrapper) | User state root |
| `POWERGROK_HOME` | `$HOME/.powergrok` | Wrapper alias → `GROK_HOME` |
| `POWERGROK_LIB` | `$HOME/.local/lib/powergrok` | Real binary directory |
| `POWERGROK_BIN` | `$POWERGROK_LIB/xai-grok-pager` | Real binary path |
| `POWERGROK_ALLOW_OFFICIAL_HOME` | unset | Debug escape hatch |

---

## 6. Isolation analysis

### 6.1 Isolated user state (`GROK_HOME=~/.powergrok`)

Config, auth, sessions, memory, user skills/plugins/agents, pager.toml, MCP credentials, leader defaults, active_sessions, logs, worktree DB open_default, campaigns, etc. — all under `grok_home()`.

### 6.2 Isolated project state (argv0 → `.powergrok/`)

**Full tree** (D6, entire-tree decision):

| Official path | Powergrok path |
|---------------|----------------|
| `<ws>/.grok/config.toml` | `<ws>/.powergrok/config.toml` |
| `<ws>/.grok/skills/` | `<ws>/.powergrok/skills/` |
| `<ws>/.grok/plugins/` | `<ws>/.powergrok/plugins/` |
| `<ws>/.grok/agents/` | `<ws>/.powergrok/agents/` |
| `<ws>/.grok/hooks/` | `<ws>/.powergrok/hooks/` |
| `<ws>/.grok/lsp.json` | `<ws>/.powergrok/lsp.json` |
| `<ws>/.grok/sandbox.toml` | `<ws>/.powergrok/sandbox.toml` |
| `<ws>/.grok/personas/` (and peers) | `<ws>/.powergrok/personas/` (and peers) |

**Precedence (D7):** If both `.grok/` and `.powergrok/` exist, powergrok uses **only** `.powergrok/`. Missing `.powergrok/` means **empty** project layer for powergrok (operator creates it). Official `grok` never reads `.powergrok/`.

### 6.3 Shared surfaces (still shared)

| Asset | Notes |
|-------|--------|
| Production API endpoints | Same backend |
| System `/etc/grok` | If present |
| Claude managed-settings OS paths | Read-only compat probes |
| UI product name “Grok Build” | Branding unchanged in v1 |
| Workspace source files | Same git tree; only config namespaces split |

### 6.4 Collision risks and mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Overwrite `~/.local/bin/grok` | Critical | Install only `powergrok`; script hard-fail |
| Forgot `GROK_HOME` | Critical | Wrapper-only PATH entry |
| argv0 not `powergrok` → still uses `.grok/` | Critical | `exec -a powergrok` or named binary; test in verify matrix |
| Incomplete call-site conversion | High | Central resolver + `rg` gate in PR checklist |
| Auto-update re-enabled | High | Seed + docs; create-if-missing only |
| Concurrent leader/socket clash | Medium | Separate `GROK_HOME` defaults |
| Operator expects merge with `.grok/` | Medium | Document D7; empty until `.powergrok/` created |
| Workspace classifier ignores `.powergrok` | Medium | Teach classifier both names if needed |

---

## 7. Engine design: project config dirname (v1)

### 7.1 API sketch (illustrative)

Place in a low-level crate already depended on by config, tools, hooks, sandbox (likely `xai-grok-config` or a tiny shared util):

```rust
/// Basename of the per-workspace config directory (".grok" or ".powergrok").
pub fn project_config_dirname() -> &'static str {
    // OnceLock: first call wins for process lifetime (mirrors grok_home discipline).
}

pub fn project_config_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(project_config_dirname())
}
```

**Resolution rules (locked):**

1. If `std::env::args_os().next()` basename (file name of argv0) equals `powergrok` or `powergrok.exe` → `".powergrok"`.
2. Else → `".grok"`.
3. **No** env override required for v1; optional `POWERGROK_PROJECT_DIRNAME` only if peer review demands test hooks (must default off for production).

**Tests:**

| Case | argv0 | Expected dirname |
|------|-------|------------------|
| Official | `grok` / `xai-grok-pager` | `.grok` |
| Powergrok | `powergrok` | `.powergrok` |
| Both trees on disk | `powergrok` | reads only `.powergrok` |
| Missing `.powergrok` | `powergrok` | no project config (empty layer) |
| Official with `.powergrok` present | `grok` | ignores `.powergrok`, uses `.grok` |

### 7.2 Migration note for operators

First powergrok session in a repo with only `.grok/`:

- Project MCP/skills/hooks from `.grok/` are **invisible** to powergrok (D7).
- Operator copies or re-creates under `.powergrok/` as needed:

```sh
# Optional one-time bootstrap (operator-driven, not automatic)
cp -R .grok .powergrok
```

Automatic copy is **out of scope** (would blur isolation).

### 7.3 Display helpers

User-facing paths that today say `.grok/` should use the resolver when describing project scope (config help, MCP add --project, etc.) so powergrok users see `.powergrok/`.

---

## 8. Build procedure (macOS)

### 8.1 Prerequisites

| Requirement | Notes |
|-------------|--------|
| Xcode CLT / linker | Standard macOS Rust host |
| `rustup` | Channel **1.92.0** from `rust-toolchain.toml` |
| `protoc` | Repo `bin/protoc` (dotslash) or `$PROTOC` / `PATH` |
| Disk / time | Large release build; multi-minute cold compile |

### 8.2 Build commands

```sh
cargo check -p xai-grok-pager-bin          # optional fast validation
cargo build -p xai-grok-pager-bin --release
# artifact: target/release/xai-grok-pager
```

Target package only (avoid full workspace build).

### 8.3 Seed config (authoritative)

`$HOME/.powergrok/config.toml` — create if missing **only**:

```toml
# Powergrok — local source-built install.
[cli]
auto_update = false
```

No telemetry/feedback keys (D3).

### 8.4 Install steps (manual)

```sh
REPO="$(pwd)"
LIB="$HOME/.local/lib/powergrok"
BIN_DIR="$HOME/.local/bin"
HOME_PG="$HOME/.powergrok"

mkdir -p "$LIB" "$BIN_DIR" "$HOME_PG"

if [[ -f "$LIB/xai-grok-pager" ]]; then
  cp -p "$LIB/xai-grok-pager" "$LIB/xai-grok-pager.prev"
fi

install -m 755 "$REPO/target/release/xai-grok-pager" "$LIB/xai-grok-pager"

{
  echo "git=$(git -C "$REPO" rev-parse HEAD)"
  echo "built_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  "$LIB/xai-grok-pager" --version 2>/dev/null || true
} > "$LIB/VERSION"

# seed config if absent — see §8.3
# install wrapper with exec -a powergrok — see §5.2
```

### 8.5 Rebuild loop

```sh
cargo build -p xai-grok-pager-bin --release
install -m 755 target/release/xai-grok-pager "$HOME/.local/lib/powergrok/xai-grok-pager"
# refresh VERSION stamp
powergrok --version   # stock product string
cat "$HOME/.local/lib/powergrok/VERSION"
```

### 8.6 Completions (Phase 2 install)

After binary install, best-effort:

```sh
# Pseudocode — exact CLI subcommand must match product (`completions <shell>`)
"$REAL_BIN" completions bash > "$HOME/.powergrok/completions/bash/powergrok.bash"
"$REAL_BIN" completions zsh  > "$HOME/.powergrok/completions/zsh/_powergrok"
```

If clap embeds bin name from argv0, run under `exec -a powergrok` so completion scripts say `powergrok`. Document operator `fpath` / `source` lines in Phase 3 README. **Do not** write `~/.config/fish/completions/grok.fish`.

---

## 9. Install script (Phase 2 deliverable)

**Path:** `scripts/install-powergrok.sh`  
**Wrapper source:** `scripts/powergrok.wrapper.sh`  
**Branch:** off `main` (D10)

### 9.1 CLI

```text
Usage: scripts/install-powergrok.sh [options]

  --build / --no-build
  --prefix DIR          default: $HOME/.local
  --grok-home DIR       default: $HOME/.powergrok
  --dry-run
  --uninstall
  --purge-home          with --uninstall, remove POWERGROK_HOME
  --install-completions generate into $GROK_HOME/completions (default: on)
  -h, --help
```

### 9.2 Algorithm

1. Resolve repo root.
2. Build if needed (`xai-grok-pager-bin` release).
3. Assert artifact exists.
4. Hard-fail if public name target is `grok`.
5. Install binary + VERSION + prev backup.
6. Install wrapper with argv0 forcing.
7. Seed config if missing.
8. Optionally generate completions under `GROK_HOME` only.
9. Print verify commands (including project-dir smoke).

### 9.3 Uninstall

Remove wrapper (marker comment `# powergrok-wrapper`) + `lib/powergrok/`. Leave `~/.powergrok` unless `--purge-home`. Leave repo `.powergrok/` (project data).

---

## 10. Verification plan (acceptance criteria)

### 10.1 Scripted checks

| ID | Check | Pass criteria |
|----|-------|---------------|
| V1 | `command -v powergrok` | `…/bin/powergrok` |
| V2 | `command -v grok` | Official path unchanged |
| V3 | Distinct binaries | Different realpaths/inodes |
| V4 | Wrapper exports home | `GROK_HOME` → `$HOME/.powergrok` |
| V5 | Official without override | `env -u GROK_HOME grok --version` works |
| V6 | Seed | `auto_update = false` present; no forced telemetry keys |
| V7 | User isolation | Powergrok creates/updates only under `~/.powergrok` |
| V8 | Official auth mtime | Unchanged across powergrok login |
| V9 | VERSION file | Exists; stock `--version` still runs |
| V10 | Concurrent | Both processes; separate `active_sessions` homes |
| V11 | argv0 | Process list / internal diagnostic shows basename `powergrok` |
| V12 | Project dir powergrok | Creates/reads `<repo>/.powergrok/` only |
| V13 | Project dir official | Creates/reads `<repo>/.grok/` only |
| V14 | Dual trees | With both present, powergrok ignores `.grok/` project config |
| V15 | Completions | Files under `~/.powergrok/completions/` only; no new `grok.fish` from powergrok install |

### 10.2 Manual TUI checks

| ID | Check |
|----|-------|
| M1 | Auth under powergrok → `~/.powergrok/auth.json` |
| M2 | Sessions under `~/.powergrok/sessions/` |
| M3 | Project skill only in `.powergrok/skills` visible to powergrok |
| M4 | Same skill only in `.grok/skills` visible to official `grok`, **not** powergrok |
| M5 | Concurrent TUI sessions stable |

### 10.3 Automated tests (engine PR)

- Unit tests for `project_config_dirname()` argv0 matrix.
- At least one integration test that config load with argv0 `powergrok` does not open project `.grok/config.toml`.
- Official default path regression: argv0 non-powergrok still `.grok`.

### 10.4 Smoke one-liner

```sh
powergrok --help >/dev/null
test -x "$HOME/.local/lib/powergrok/xai-grok-pager"
test -f "$HOME/.local/lib/powergrok/VERSION"
grep -q 'auto_update = false' "$HOME/.powergrok/config.toml"
test "$(command -v grok)" != "$(command -v powergrok)"
echo "powergrok smoke OK"
```

---

## 11. Security & privacy

1. Separate `auth.json` under `~/.powergrok`; same permission hygiene as official.
2. Never commit `~/.powergrok` or repo secrets under `.powergrok/`.
3. Wrapper uses `exec` / `exec -a`.
4. User-prefix install only (no `sudo`).
5. Pin git SHA in `VERSION` for audit of source-built binary.
6. Auto-update off prevents channel binaries from replacing local artifact in powergrok home.
7. Empty project layer when `.powergrok/` missing is intentional (fail closed for project isolation).

---

## 12. Operational runbooks

### 12.1 Rollback binary

```sh
cp -p "$HOME/.local/lib/powergrok/xai-grok-pager.prev" \
      "$HOME/.local/lib/powergrok/xai-grok-pager"
```

### 12.2 Reset powergrok user state

```sh
rm -rf "$HOME/.powergrok"   # re-seed on next launch
```

### 12.3 Reset powergrok project state (one repo)

```sh
rm -rf /path/to/repo/.powergrok
```

### 12.4 Full removal

```sh
rm -f "$HOME/.local/bin/powergrok"
rm -rf "$HOME/.local/lib/powergrok"
rm -rf "$HOME/.powergrok"          # optional
# repo .powergrok/ left for operator to delete per project
```

### 12.5 Bootstrap project tree from official (optional)

```sh
cp -R .grok .powergrok   # only when operator wants a starting copy
```

---

## 13. Implementation phases (post-approval)

| Phase | Branch | Deliverable | Exit criteria |
|-------|--------|-------------|---------------|
| **0 — Plan** | current docs branch OK | This `BUILD_PLAN.md` | Peer + operator decisions locked (§0) |
| **1 — Engine** | **off `main`** (D10) | `project_config_dirname()` + call-site conversion + tests | V12–V14 + unit tests green; official `.grok` unchanged |
| **2 — Install** | same feature branch as Phase 1 or stacked | `scripts/install-powergrok.sh`, wrapper, completions under home | V1–V11, V15; dry-run + real install |
| **3 — Docs** | same | Short `docs/powergrok/README.md` operator guide | New contributor can install and verify |
| **4 — Optional** | later | Cargo `[[bin]]` name, UI badge | Separate RFC |

**Ordering:** Phase 1 **before** relying on project isolation in daily use. Phase 2 can prototype wrapper against main **before** Phase 1 merges if only user-home isolation is needed temporarily—but **v1 definition of done requires Phase 1**.

---

## 14. Expected tree after Phases 1–3

```text
docs/powergrok/
  BUILD_PLAN.md
  README.md

scripts/
  install-powergrok.sh
  powergrok.wrapper.sh

crates/codegen/xai-grok-config/   # or agreed crate
  src/… project_config_dirname …
# plus call-site updates across config/tools/hooks/sandbox/pager
```

---

## 15. Open questions (remaining)

Most prior open questions are **closed in §0**. Remaining for peer engineers:

1. **Canonical crate** for `project_config_dirname()` — `xai-grok-config` vs smaller leaf to avoid cycles?
2. **Completeness bar** — is “all `rg '\"\\.grok\"'` product call sites for project paths” the merge gate, or a documented allowlist of intentional user-home-only strings?
3. **`exec -a` portability** — bash-on-macOS is fine; document zsh `ARGV0` / symlink fallback if operators use a non-bash wrapper.
4. **Workspace classifier / ignore lists** — should `.powergrok` be treated identically to `.grok` for “not a project dir” heuristics?
5. **Git hygiene** — recommend adding `.powergrok/` to a global gitignore template, or leave untracked risk to operators?

---

## 16. Risks register

| ID | Risk | Likelihood | Impact | Mitigation |
|----|------|------------|--------|------------|
| R1 | Raw binary run without wrapper | Medium | Pollutes `~/.grok` | PATH only installs wrapper |
| R2 | argv0 not rewritten | High without care | Project isolation fails closed to `.grok` | `exec -a` + V11/V12 |
| R3 | Missed hard-coded `.grok` call site | High | Partial leak of project config | Central API + rg gate + tests |
| R4 | Auto-update re-enabled | Medium | Channel binary in powergrok home | Docs + seed |
| R5 | Operator surprise at empty project layer | Medium | “MCP missing” | README + optional copy recipe |
| R6 | Concurrent use bugs outside home | Low | Session confusion | V10; separate homes |
| R7 | Review noise if mixed with effort-modes | Medium | Slow merge | D10 dedicated branch |
| R8 | Build/toolchain failures | Medium | Blocked install | Document protoc + 1.92.0 |

---

## 17. Definition of done (v1)

1. `powergrok` on PATH via wrapper + lib binary (D2).
2. Process `GROK_HOME` is `$HOME/.powergrok` (D1).
3. Process project tree is `<ws>/.powergrok/` when argv0 is `powergrok` (D5–D7).
4. Official `grok` still uses `~/.grok` and `<ws>/.grok/` exclusively for those layers.
5. Seed config has `auto_update = false` only (D3).
6. Completions (if installed) live under `~/.powergrok/completions/` as `powergrok` (D8).
7. `VERSION` file present; stock `--version` (D9).
8. Concurrent official + powergrok verified (D4).
9. Implementation on dedicated branch off `main` (D10).
10. Verification §10.1 V1–V15 pass on operator Mac.

---

## 18. References

| Reference | Path |
|-----------|------|
| User home resolution | `crates/codegen/xai-grok-config/src/paths.rs` |
| Config load order | `crates/codegen/xai-grok-config/src/lib.rs` |
| `GROK_HOME` tests | `crates/codegen/xai-grok-pager/tests/grok_home_paths.rs` |
| User guide | `crates/codegen/xai-grok-pager/docs/user-guide/05-configuration.md` |
| Binary package | `crates/codegen/xai-grok-pager-bin/Cargo.toml` |
| Build instructions | repo root `README.md` |
| Toolchain | `rust-toolchain.toml` (`1.92.0`) |
| Auto-update | `crates/codegen/xai-grok-update/src/auto_update.rs` |
| Sandbox home | `crates/codegen/xai-grok-sandbox/src/paths.rs` |
| Endpoints | `crates/codegen/xai-grok-env/src/lib.rs` |

---

## 19. Appendix A — Side-by-side path map

| Concern | Official | Powergrok |
|---------|----------|-----------|
| Command | `grok` | `powergrok` |
| Real binary | `~/.grok/bin/grok` (managed) | `~/.local/lib/powergrok/xai-grok-pager` |
| User state | `~/.grok` | `~/.powergrok` |
| Project tree | `<repo>/.grok/` | `<repo>/.powergrok/` only |
| Auth | `~/.grok/auth.json` | `~/.powergrok/auth.json` |
| Auto-update | product default | forced off |
| Completions | product/updater paths | `~/.powergrok/completions/*powergrok*` |
| Version audit | release channel | `…/lib/powergrok/VERSION` |

---

## 20. Appendix B — Decision record

| Decision | Choice | Status |
|----------|--------|--------|
| User home | `~/.powergrok` | **Locked D1** |
| Layout | Wrapper + lib | **Locked D2** |
| Telemetry seed | Product defaults | **Locked D3** |
| Concurrent | Supported | **Locked D4** |
| Project isolation | Required v1 | **Locked D5** |
| Mechanism | argv0 `powergrok` → `.powergrok/` full tree | **Locked D6** |
| Dual-tree precedence | `.powergrok/` only | **Locked D7** |
| Completions | Under `~/.powergrok`, name powergrok | **Locked D8** |
| Version UX | VERSION file + stock `--version` | **Locked D9** |
| Landing branch | Off `main`, dedicated | **Locked D10** |
| Engine changes | Required for project dir | **Locked** |

---

## 21. Appendix C — Reviewer checklist

```text
[ ] §0 decisions understood (esp. D5–D7 project isolation)
[ ] Official paths never write targets
[ ] auto_update=false seed only; telemetry untouched
[ ] argv0 strategy (exec -a / named binary) is mandatory
[ ] Central project_config_dirname design acceptable
[ ] No fallback/merge with project .grok/ for powergrok
[ ] Completions confined to ~/.powergrok
[ ] Implementation branch off main (not effort-modes stack)
[ ] Verification V11–V14 cover project isolation
[ ] Approve Phase 0 / request changes to engine design
```

---

*End of plan. Phase 1+ implementation starts on a dedicated branch off `main` after peer review disposition.*
