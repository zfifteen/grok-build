# Powergrok Build & Install Plan

**Status:** Draft for peer review  
**Date:** 2026-07-15  
**Branch context:** `feat/effort-modes-builtin` (document-only deliverable for this plan)  
**Audience:** Engineers reviewing a side-by-side macOS install of a locally built Grok Build binary that coexists with the official `grok` install  
**Primary operator platform:** macOS (Apple Silicon and Intel both in scope; paths below use `$HOME` and are architecture-agnostic)

---

## 1. Executive summary

**Powergrok** is a **local, source-built** installation of this repository’s Grok Build CLI/TUI, installed under the command name `powergrok`, with a **private user state directory** so it never overwrites or shares on-disk identity with the official `grok` install.

The product already supports the hard isolation primitive:

| Mechanism | Role |
|-----------|------|
| Environment variable `GROK_HOME` | Overrides the entire user state root (config, auth, sessions, skills, plugins, locks, leader socket defaults, managed `bin/`, downloads, etc.) |
| Default when unset | `~/.grok` (canonicalized via `dunce::canonicalize` then `.join(".grok")`) |

Canonical implementation: `crates/codegen/xai-grok-config/src/paths.rs` (`grok_home()`, `default_grok_home()`, `user_grok_home()`, `grok_application()`).

**Recommended powergrok identity (v1):**

| Property | Value |
|----------|--------|
| Command on `PATH` | `powergrok` |
| Real binary location | `$HOME/.local/lib/powergrok/xai-grok-pager` (or versioned sibling) |
| Wrapper | `$HOME/.local/bin/powergrok` (sets `GROK_HOME`, `exec`s real binary) |
| User state root | `$HOME/.powergrok` (`GROK_HOME`) |
| Auto-update | **Disabled** in `$GROK_HOME/config.toml` (`[cli] auto_update = false`) |
| Official install | Untouched: `~/.local/bin/grok` → `~/.grok/bin/grok` (typical layout) |

**v1 strategy:** zero (or near-zero) product code changes. Isolation is achieved by **install layout + environment + config seed**, using production code paths already exercised by tests (for example `crates/codegen/xai-grok-pager/tests/grok_home_paths.rs`).

---

## 2. Goals

1. **Build** a release binary of `xai-grok-pager-bin` from this tree on the operator’s Mac.
2. **Install** that binary so the operator invokes it as `powergrok`.
3. **Isolate** all user-global state from the official install via `GROK_HOME=$HOME/.powergrok`.
4. **Preserve** the official `grok` binary, symlink chain, auth, sessions, and updater behavior.
5. **Disable** auto-update for the powergrok home so official release channels never replace the source-built binary under that home.
6. **Document** a repeatable rebuild → reinstall loop and a peer-reviewable verification matrix.
7. **Optionally** land a single install script under this repo so the procedure is one command and reviewable as code.

---

## 3. Scope of this plan

### 3.1 In scope (v1)

| Item | Detail |
|------|--------|
| macOS host build | `cargo build -p xai-grok-pager-bin --release` using workspace `rust-toolchain.toml` |
| Binary packaging | Rename/install artifact as powergrok-owned paths only |
| Wrapper | Shell launcher that always exports `GROK_HOME` before `exec` |
| Private home | Create `$HOME/.powergrok` and seed minimal `config.toml` |
| Official coexistence | Never write to `~/.local/bin/grok` or mutate `~/.grok/bin/*` |
| Rebuild workflow | Documented steps after code changes on this branch |
| Verification | Checklist proving path isolation and official install integrity |
| Peer-review artifacts | This document; optional install script in a follow-up commit |

### 3.2 Deferred (later milestones)

| Item | Rationale for deferral |
|------|------------------------|
| Second Cargo `[[bin]]` named `powergrok` | Nice for `ps` / argv0; isolation does not require it |
| Compile-time product branding (“Powergrok” in UI chrome) | Cosmetic; large surface area for little isolation value |
| Separate project-dir namespace (repo `.grok/`) | Product loads project config from workspace `.grok/`; shared by design today |
| Linux packaging / Homebrew formula | Operator request is macOS-first |
| Windows | Out of current operator need |
| Forked auth protocol or alternate API endpoints | Powergrok uses the same production endpoints as public builds (`xai-grok-env`) |
| Disabling telemetry by default | Orthogonal; can be set in seeded `config.toml` if product policy allows |
| Shipping powergrok via official `install.sh` | Would collide with product installer contracts; keep local |

### 3.3 Explicit dependencies on existing product behavior

Reviewers should treat these as **contracts** this plan relies on:

1. **`GROK_HOME` is process-lifetime / first-read via `OnceLock`** in `xai_grok_config::paths::grok_home`. The env var must be set **before** the process starts (wrapper/`env` at spawn). Mid-process mutation after first `grok_home()` call is ineffective.
2. **Sandbox writable roots** include grok home via `xai_grok_sandbox` → `xai_grok_config::grok_home()`, so private home remains writable under sandbox when enabled.
3. **Updater restart preference** uses `grok_application()` → `$GROK_HOME/bin/grok` when that path exists (`xai-grok-update`). With auto-update off and no managed symlink under powergrok home, restarts use `current_exe()`.
4. **Fish completion regeneration** can write `$HOME/.config/fish/completions/grok.fish` (user home, not only `GROK_HOME`). Auto-update off + no `grok update` under powergrok keeps this path quiet.
5. **System config** at `/etc/grok` (Unix) remains shared if present; personal Macs typically have none.

---

## 4. Background: official install topology (operator machine)

Observed / typical layout (confirm on each machine before install):

```text
~/.local/bin/grok                    → symlink → ~/.grok/bin/grok
~/.grok/bin/grok                     → symlink → versioned binary (e.g. grok-0.2.101)
~/.grok/bin/agent                    → related entry point (updater keeps in lockstep)
~/.grok/config.toml                  user config
~/.grok/auth.json                    credentials
~/.grok/sessions/                    session store
~/.grok/skills/, plugins/, ...       extensions
~/.grok/downloads/                   release artifacts (managed install)
```

Product docs (`crates/codegen/xai-grok-pager/docs/user-guide/05-configuration.md`):

| Variable | Meaning |
|----------|---------|
| `GROK_HOME` | Override config directory (default `~/.grok`) |

Build docs (repo root `README.md`):

| Command | Result |
|---------|--------|
| `cargo build -p xai-grok-pager-bin --release` | `target/release/xai-grok-pager` |
| Official ship name | `grok` |

Composition root: `crates/codegen/xai-grok-pager-bin` (`default-run` / `[[bin]]` name `xai-grok-pager`).

---

## 5. Target topology (powergrok)

### 5.1 Directory layout

```text
$HOME/.local/bin/powergrok                 # wrapper (mode 0755)
$HOME/.local/lib/powergrok/
  xai-grok-pager                           # release binary (mode 0755)
  xai-grok-pager.prev                      # optional previous binary for rollback
  VERSION                                  # optional plain-text git describe / cargo version

$HOME/.powergrok/                          # GROK_HOME
  config.toml                              # seeded; auto_update = false
  auth.json                                # created on first login (powergrok-only)
  sessions/                                # isolated
  skills/, plugins/, agents/, memory/, ...
  logs/
  completions/                             # optional, under GROK_HOME only
  bin/                                     # intentionally empty or unused in v1
  downloads/                               # leave unused; auto_update off
```

### 5.2 Wrapper contract

The wrapper is the **only** supported entry point for humans and scripts.

**Required behavior:**

1. Resolve `POWERGROK_HOME` defaulting to `$HOME/.powergrok` (optional operator override; maps to `GROK_HOME`).
2. `export GROK_HOME="$POWERGROK_HOME"` (or default).
3. Create `GROK_HOME` if missing (`mkdir -p`).
4. If `config.toml` is missing, write the seed file (see §7.3).
5. `exec` the real binary with `"$@"` unchanged.
6. Refuse to run if the real binary is missing (non-zero exit + clear message).
7. Refuse to install/run if the real binary path resolves to the official managed path (`$HOME/.grok/bin/grok` or same inode as `$(command -v grok)` after realpath) — hard guard against accidental overwrite.

**Recommended wrapper (POSIX shell, illustrative):**

```sh
#!/usr/bin/env bash
set -euo pipefail

POWERGROK_LIB="${POWERGROK_LIB:-$HOME/.local/lib/powergrok}"
REAL_BIN="${POWERGROK_BIN:-$POWERGROK_LIB/xai-grok-pager}"
export GROK_HOME="${POWERGROK_HOME:-$GROK_HOME:-$HOME/.powergrok}"

if [[ ! -x "$REAL_BIN" ]]; then
  echo "powergrok: missing binary at $REAL_BIN" >&2
  echo "powergrok: build with: cargo build -p xai-grok-pager-bin --release" >&2
  exit 127
fi

# Safety: never point GROK_HOME at the official home by accident unless forced.
if [[ "${POWERGROK_ALLOW_OFFICIAL_HOME:-}" != "1" ]]; then
  official="${HOME}/.grok"
  if [[ "$(cd "$GROK_HOME" 2>/dev/null && pwd -P)" == "$(cd "$official" 2>/dev/null && pwd -P)" ]]; then
    echo "powergrok: GROK_HOME resolves to official ~/.grok; aborting" >&2
    echo "powergrok: set POWERGROK_ALLOW_OFFICIAL_HOME=1 only if intentional" >&2
    exit 2
  fi
fi

mkdir -p "$GROK_HOME"
if [[ ! -f "$GROK_HOME/config.toml" ]]; then
  cat >"$GROK_HOME/config.toml" <<'EOF'
# Powergrok local install — source-built binary; do not enable auto_update.
[cli]
auto_update = false
EOF
fi

exec "$REAL_BIN" "$@"
```

### 5.3 Environment variables (operator surface)

| Variable | Default | Purpose |
|----------|---------|---------|
| `GROK_HOME` | Set by wrapper to `$HOME/.powergrok` | Product state root |
| `POWERGROK_HOME` | `$HOME/.powergrok` | Wrapper-only alias before export to `GROK_HOME` |
| `POWERGROK_LIB` | `$HOME/.local/lib/powergrok` | Directory containing real binary |
| `POWERGROK_BIN` | `$POWERGROK_LIB/xai-grok-pager` | Explicit binary path override |
| `POWERGROK_ALLOW_OFFICIAL_HOME` | unset | Escape hatch for debugging only |

Operators may still pass extra product env vars (`RUST_LOG`, `GROK_TELEMETRY_ENABLED`, etc.) through the wrapper unchanged.

---

## 6. Isolation analysis (review matrix)

### 6.1 Isolated when `GROK_HOME` is private

These live under `grok_home()` / `user_grok_home()` and therefore under `~/.powergrok` when set:

| Asset | Typical relative path |
|-------|------------------------|
| Main config | `config.toml` |
| Managed overlay | `managed_config.toml` |
| Requirements cache | `requirements.toml` |
| Auth | `auth.json` (+ lock) |
| Sessions | `sessions/{encoded-cwd}/…` |
| Memory | `memory/` |
| Skills / plugins / agents | `skills/`, `plugins/`, `agents/` |
| Pager UI prefs | `pager.toml` |
| MCP credentials | `mcp_credentials.json` |
| Leader IPC (defaults) | `leader.sock`, `leader.lock`, `leader.log` (when using default home-based paths) |
| Active sessions registry | `active_sessions.json` (+ lock) |
| Logs / traces / memtrace | `logs/`, related subdirs |
| Crash dumps | under home as configured by crash handler wiring |
| Worktree DB / fast-worktree state | home-relative open_default paths |
| Campaigns / hooks disabled list | home-relative |
| Completions written under home | `completions/bash/…`, `completions/zsh/…` |

**Implication:** Official and powergrok can run concurrently with separate auth, session history, skills installs, and leader sockets (default paths).

### 6.2 Shared by design (document for reviewers)

| Asset | Behavior | Operator impact |
|-------|----------|-----------------|
| Repo project config | `.grok/` under the git workspace | Both commands load the same project MCP/skills/hooks/permissions |
| System managed config | `/etc/grok` if present | Enterprise Macs only |
| Claude managed-settings probe | fixed OS paths in `paths.rs` | Read-only compat; shared |
| Production API endpoints | `xai-grok-env` production URLs | Same backend product |
| Fish completions path | `$HOME/.config/fish/completions/grok.fish` on update regen | Keep auto-update off |
| Display / product name | Still “Grok Build” in most UI | Branding only |
| Keychain / OS credential stores | Only if product uses them outside `auth.json` | Verify on first login; v1 assumes file auth under `GROK_HOME` as primary |

### 6.3 Collision risks and mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Install overwrites `~/.local/bin/grok` | Critical | Install only `powergrok` name; script aborts if target is `grok` |
| Wrapper forgets `GROK_HOME` | Critical | Single wrapper; never `exec` bare binary from docs |
| Auto-update replaces source binary under `$GROK_HOME/bin/grok` | High | Seed `auto_update = false`; do not create managed `bin/grok` symlink in v1 |
| `OnceLock` caches wrong home if env set after import | Medium | Env only at process start |
| Operator copies skills via symlink to `~/.grok/skills` | Low (intentional) | Document share-vs-isolate choice; default no symlinks |
| Completions clobber fish `grok.fish` | Low | No update; optional powergrok-named completion only under `~/.powergrok` |
| Running both TUIs in same repo | Low | Shared project `.grok/`; separate user homes still fine |
| Disk usage (two full binaries) | Low | ~100–150+ MB release binary; document |

---

## 7. Build procedure (macOS)

### 7.1 Prerequisites

| Requirement | Notes |
|-------------|--------|
| Xcode CLT / linker | Standard macOS Rust host |
| `rustup` | Toolchain file pins channel **1.92.0** (`rust-toolchain.toml`) |
| `protoc` resolution | Repo provides `bin/protoc` (dotslash) or `$PROTOC` / `PATH` |
| Disk / RAM | Full release build of pager-bin is large; expect multi-minute first compile |
| Network | First build may fetch crates; runtime auth needs network to xAI services |

Confirm:

```sh
cd /path/to/grok-build
rustc --version    # expect 1.92.0 after rustup reads rust-toolchain.toml
cargo --version
```

### 7.2 Build commands

**Fast validation (optional):**

```sh
cargo check -p xai-grok-pager-bin
```

**Release binary (required for install):**

```sh
cargo build -p xai-grok-pager-bin --release
```

**Artifact path:**

```text
target/release/xai-grok-pager
```

**Optional metadata capture:**

```sh
./target/release/xai-grok-pager --version || true
git rev-parse --short HEAD > /tmp/powergrok-git-rev.txt
```

Notes for reviewers:

- Package features default include `jemalloc` and `sandbox-enforce` on Unix (`xai-grok-pager-bin` `Cargo.toml`).
- Full workspace `cargo build` is discouraged for iteration; target the pager-bin package only (README guidance).
- Binary name on disk remains `xai-grok-pager` until an optional Cargo rename milestone.

### 7.3 Seed config (authoritative content)

File: `$HOME/.powergrok/config.toml`

```toml
# Powergrok — local source-built install.
# Managed by the powergrok install/wrapper process for first-run only;
# subsequent edits by the operator are preserved (wrapper must not overwrite).

[cli]
auto_update = false
```

**Invariant:** After first create, reinstall **must not** clobber operator edits to `config.toml`. Install script uses “create if missing” only.

Optional future seeds (peer review may accept or reject):

```toml
# Example only — not required for isolation:
# [telemetry]
# ... product-specific keys if documented ...
```

### 7.4 Install steps (manual, v1)

Run after a successful release build:

```sh
REPO="$(pwd)"   # repository root
LIB="$HOME/.local/lib/powergrok"
BIN_DIR="$HOME/.local/bin"
HOME_PG="$HOME/.powergrok"

mkdir -p "$LIB" "$BIN_DIR" "$HOME_PG"

# Preserve previous binary for one-step rollback
if [[ -f "$LIB/xai-grok-pager" ]]; then
  cp -p "$LIB/xai-grok-pager" "$LIB/xai-grok-pager.prev"
fi

install -m 755 "$REPO/target/release/xai-grok-pager" "$LIB/xai-grok-pager"

# Write VERSION stamp
{
  echo "git=$(git -C "$REPO" rev-parse HEAD)"
  echo "built_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  "$LIB/xai-grok-pager" --version 2>/dev/null || true
} > "$LIB/VERSION"

# Seed config if absent
if [[ ! -f "$HOME_PG/config.toml" ]]; then
  cat >"$HOME_PG/config.toml" <<'EOF'
# Powergrok local install — source-built binary; do not enable auto_update.
[cli]
auto_update = false
EOF
fi

# Install wrapper (see §5.2); example:
install -m 755 /path/to/powergrok-wrapper.sh "$BIN_DIR/powergrok"
```

**PATH requirement:** `$HOME/.local/bin` must appear on `PATH` (typical for official grok). Verify:

```sh
command -v powergrok
command -v grok
```

### 7.5 First-run procedure

1. `powergrok --version` (or help) — confirms wrapper + binary.
2. `powergrok` — TUI; complete auth if prompted.
3. Confirm files:
   - `$HOME/.powergrok/auth.json` created or updated  
   - `$HOME/.grok/auth.json` mtime unchanged  
4. `grok --version` still works and still resolves under `~/.grok`.

### 7.6 Rebuild loop (day-to-day)

```sh
cd /path/to/grok-build
cargo build -p xai-grok-pager-bin --release
install -m 755 target/release/xai-grok-pager "$HOME/.local/lib/powergrok/xai-grok-pager"
# wrapper unchanged
powergrok --version
```

Optional: keep `xai-grok-pager.prev` as in §7.4.

---

## 8. Proposed install script (v1.1 deliverable)

**Path (proposed):** `scripts/install-powergrok.sh`  
**Language:** `bash`, `set -euo pipefail`  
**Idempotent:** Yes for reinstall (binary replace + version stamp; config create-if-missing)

### 8.1 CLI interface

```text
Usage: scripts/install-powergrok.sh [options]

Options:
  --build              Run cargo release build before install (default: on if artifact missing)
  --no-build           Install existing target/release/xai-grok-pager only
  --prefix DIR         Default: $HOME/.local
  --grok-home DIR      Default: $HOME/.powergrok
  --dry-run            Print actions only
  --uninstall          Remove wrapper + lib dir; leave ~/.powergrok unless --purge-home
  --purge-home         With --uninstall, also remove POWERGROK_HOME
  -h, --help
```

### 8.2 Script algorithm

1. Resolve repo root (script location or `git rev-parse --show-toplevel`).
2. Assert host is macOS or Linux (warn on others).
3. Assert `cargo` available when build required.
4. If build: `cargo build -p xai-grok-pager-bin --release` with `CARGO_TERM_COLOR=always` optional.
5. Assert artifact exists and is executable.
6. **Hard fail** if install destination for the public name would be `grok`.
7. Install binary to `$prefix/lib/powergrok/xai-grok-pager` with prev backup.
8. Write embedded wrapper to `$prefix/bin/powergrok` (or copy from `scripts/powergrok.wrapper.sh`).
9. Seed config if missing.
10. Print verification commands and resolved paths.
11. Exit 0.

### 8.3 Uninstall algorithm

1. Remove `$prefix/bin/powergrok` if it is a powergrok wrapper (grep marker comment `# powergrok-wrapper`).
2. Remove `$prefix/lib/powergrok/` tree.
3. Leave `$HOME/.powergrok` unless `--purge-home` (auth/sessions are operator data).

### 8.4 Script non-goals (for script review)

- No call to `https://x.ai/cli/install.sh`
- No modification of `~/.grok/**`
- No `npm` / `brew` publish
- No `sudo`

---

## 9. Optional code-change milestones (after v1)

Only if peer review demands stronger product identity. Ordered by cost/benefit.

### Milestone B — Cargo binary alias

| Change | Location |
|--------|----------|
| Add `[[bin]] name = "powergrok"` with same `path = "src/main.rs"` **or** install rename only | `xai-grok-pager-bin/Cargo.toml` |

**Benefit:** `argv0` / process list shows `powergrok`.  
**Cost:** Extra artifact; CI/time; still requires `GROK_HOME` for isolation.

### Milestone C — Default home from argv0 (risky)

| Idea | Derive default home as `~/.powergrok` when `argv0` basename is `powergrok`, even without env |
|------|----------------------------------------------------------------------------------------------|
| Risk | Surprises if someone renames binary; tests must cover; `OnceLock` init order |
| Recommendation | **Reject for v1**; wrapper + env is explicit and already tested |

### Milestone D — Completions named powergrok

Generate shell completions under `$GROK_HOME/completions/` with command name `powergrok` if clap supports bin name override. Wire zsh `fpath` docs for operators.

### Milestone E — UI badge

Optional status-bar suffix `powergrok` / `source` when `GROK_HOME` ≠ default. Pure UX.

---

## 10. Verification plan (acceptance criteria)

### 10.1 Automated / scripted checks

| ID | Check | Pass criteria |
|----|-------|---------------|
| V1 | `command -v powergrok` | Path under `$HOME/.local/bin/powergrok` |
| V2 | `command -v grok` | Still official path; realpath under `~/.grok` or prior install |
| V3 | `readlink` / `realpath` of both | Distinct inodes |
| V4 | `GROK_HOME` for powergrok | Wrapper exports `$HOME/.powergrok` |
| V5 | Official env | `env -u GROK_HOME grok --version` still works |
| V6 | Config seed | `$HOME/.powergrok/config.toml` contains `auto_update = false` |
| V7 | Home isolation probe | `powergrok` creates a marker file only under `~/.powergrok` (e.g. touch via session start) |
| V8 | Official mtime | `stat -f %m ~/.grok/auth.json` unchanged across powergrok first login (if auth already existed) |
| V9 | Binary version | `powergrok --version` runs without error |
| V10 | Concurrent safety | Start powergrok; confirm `~/.powergrok/active_sessions.json` (or equivalent) updates; `~/.grok/active_sessions.json` stable |

### 10.2 Manual TUI checks

| ID | Check |
|----|-------|
| M1 | Auth browser flow completes under powergrok |
| M2 | New session appears only under `~/.powergrok/sessions/` |
| M3 | Slash `/config` or docs paths display `$GROK_HOME` abbreviation when overridden (product already has display helpers) |
| M4 | Official `grok` session list unchanged |
| M5 | Project skills in repo `.grok/skills` still visible to both (expected shared behavior) |

### 10.3 Regression against product tests (optional CI)

Existing coverage for `GROK_HOME`:

- `crates/codegen/xai-grok-pager/tests/grok_home_paths.rs`
- Harness isolation in `xai-grok-test-support` and PTY e2e (`GROK_HOME` set per temp home)

No new unit tests are **required** for a pure install-script v1; if Milestone C lands, add targeted path tests.

### 10.4 Suggested one-liner smoke (post-install)

```sh
powergrok --help >/dev/null
test -x "$HOME/.local/lib/powergrok/xai-grok-pager"
test -f "$HOME/.powergrok/config.toml"
grep -q 'auto_update = false' "$HOME/.powergrok/config.toml"
test "$(command -v grok)" != "$(command -v powergrok)"
echo "powergrok smoke OK"
```

---

## 11. Security & privacy notes

1. **Credentials:** `auth.json` under `~/.powergrok` is separate. Treat with same file permissions as official (`0600` typical). Do not sync this directory to public git.
2. **Secrets in config:** Same rules as official; never commit `~/.powergrok`.
3. **Wrapper `exec`:** Prefer `exec` to avoid zombie shells and to keep signal handling on the real process.
4. **No privilege escalation:** Install under user prefix only.
5. **Supply chain:** Powergrok binary is **whatever this checkout builds**. Pin git SHA in `$HOME/.local/lib/powergrok/VERSION` for audit.
6. **Auto-update off:** Prevents remote installer content from replacing the local artifact inside powergrok’s home; official install continues to update itself independently.
7. **Sandbox:** Private home must remain writable; product already treats grok home as writable. Do not place `GROK_HOME` on a read-only volume.

---

## 12. Operational runbooks

### 12.1 Rollback binary

```sh
LIB="$HOME/.local/lib/powergrok"
cp -p "$LIB/xai-grok-pager.prev" "$LIB/xai-grok-pager"
powergrok --version
```

### 12.2 Reset powergrok state only

```sh
# Destructive to powergrok sessions/auth only
rm -rf "$HOME/.powergrok"
# next powergrok launch re-seeds config.toml via wrapper
```

Official `~/.grok` untouched.

### 12.3 Full removal

```sh
rm -f "$HOME/.local/bin/powergrok"
rm -rf "$HOME/.local/lib/powergrok"
rm -rf "$HOME/.powergrok"   # optional data purge
```

### 12.4 Debugging “wrong home”

```sh
# Should print powergrok home after any command that initializes config:
GROK_HOME="$HOME/.powergrok" "$HOME/.local/lib/powergrok/xai-grok-pager" --help
# Confirm wrapper:
bash -x "$(command -v powergrok)" --version 2>&1 | head
```

### 12.5 Sharing skills intentionally

```sh
# Example: read-only share of official user skills into powergrok
ln -s "$HOME/.grok/skills" "$HOME/.powergrok/skills"
```

Document this as an **operator choice**. Default install creates no symlinks.

---

## 13. Implementation phases (for execution after approval)

| Phase | Deliverable | Exit criteria |
|-------|-------------|---------------|
| **0 — Review** | This `BUILD_PLAN.md` merged / approved | Peer sign-off on isolation model |
| **1 — Manual install** | Operator builds + installs per §7 | V1–V10 smoke pass on one Mac |
| **2 — Script** | `scripts/install-powergrok.sh` + wrapper source | Dry-run + real install on clean paths |
| **3 — Docs pointer** | Short link from repo docs or powergrok README | New contributor can find plan + script |
| **4 — Optional milestones** | B/D/E only if requested | Separate RFCs |

**Phase 0 is this document.** Phases 1–2 are implementation work **after** peer review.

---

## 14. File checklist (expected tree after Phase 2)

```text
docs/powergrok/
  BUILD_PLAN.md                 # this document
  README.md                     # optional short operator guide (Phase 3)

scripts/
  install-powergrok.sh          # Phase 2
  powergrok.wrapper.sh          # Phase 2 (source for installed wrapper)
```

No changes to `crates/**` required for Phases 0–2.

---

## 15. Open questions for peer review

Please answer explicitly in review comments:

1. **Home directory name:** `~/.powergrok` vs `~/.grok-power` vs `$XDG_CONFIG_HOME/powergrok`?  
   *Recommendation: `~/.powergrok` (mirrors `~/.grok`, easy to type, obvious in `ls -a`).*

2. **Lib vs bin for real binary:** `~/.local/lib/powergrok/xai-grok-pager` + thin wrapper vs single fat binary named `powergrok` that requires env?  
   *Recommendation: lib + wrapper so `GROK_HOME` cannot be forgotten.*

3. **Should Phase 2 land on this feature branch or a dedicated `chore/powergrok-install` branch?**  
   *Recommendation: dedicated small branch off main or stacked PR for review clarity.*

4. **Telemetry / feedback defaults** for source builds — leave product defaults or seed off?

5. **Is concurrent use of official + powergrok a supported operator scenario, or sequential only?**  
   *Plan assumes concurrent is supported at filesystem level.*

6. **Project `.grok/` sharing** — acceptable, or do reviewers want a follow-up design for project-home override?

7. **Completions:** install zsh/bash completions for the name `powergrok` in Phase 2?

8. **Versioning UX:** should `--version` be patched to include `powergrok` / git SHA, or is `VERSION` file enough?

---

## 16. Risks register

| ID | Risk | Likelihood | Impact | Mitigation |
|----|------|------------|--------|------------|
| R1 | Operator runs raw `target/release/xai-grok-pager` without `GROK_HOME` | Medium | Pollutes official home | Docs + wrapper-only PATH entry; education |
| R2 | Future product change weakens `GROK_HOME` | Low | Isolation break | Pin behavior via smoke tests in Phase 2; re-read `paths.rs` on upgrades |
| R3 | Auto-update re-enabled by operator | Medium | Official binary pulled into powergrok home | Config comment + README warning |
| R4 | Build fails (protoc / toolchain) | Medium | Blocks install | Document prereqs; use repo `bin/protoc` |
| R5 | Disk full / huge target/ | Medium | Failed build | `cargo clean` guidance; only build pager-bin |
| R6 | PATH order: another `powergrok` | Low | Wrong binary | `command -v -a powergrok` in verify |
| R7 | Official install script run after powergrok | Low | Should still only touch official paths | Keep names distinct |
| R8 | Peer confusion with effort-modes work on same branch | Medium | Review noise | Keep this doc self-contained; prefer separate PR for scripts |

---

## 17. Success definition (definition of done for the program)

Powergrok is **done for v1** when:

1. Operator can run **`powergrok`** from a normal shell after a source build.
2. **`GROK_HOME`** for that process is **`$HOME/.powergrok`**.
3. Official **`grok`** continues to use **`~/.grok`** with unchanged binary path.
4. **`[cli] auto_update = false`** is present in the powergrok config seed.
5. Verification matrix §10.1 items V1–V10 pass on the operator’s Mac.
6. This plan (and later the install script) is reviewable in git history.

---

## 18. References (code & docs)

| Reference | Path / note |
|-----------|-------------|
| User home resolution | `crates/codegen/xai-grok-config/src/paths.rs` |
| Config load order | `crates/codegen/xai-grok-config/src/lib.rs` module docs |
| `GROK_HOME` tests | `crates/codegen/xai-grok-pager/tests/grok_home_paths.rs` |
| User guide env table | `crates/codegen/xai-grok-pager/docs/user-guide/05-configuration.md` |
| Headless env | `crates/codegen/xai-grok-pager/docs/user-guide/14-headless-mode.md` |
| Binary package | `crates/codegen/xai-grok-pager-bin/Cargo.toml` |
| Build instructions | repo root `README.md` |
| Toolchain pin | `rust-toolchain.toml` (`1.92.0`) |
| Auto-update / restart | `crates/codegen/xai-grok-update/src/auto_update.rs` (`grok_application`, completions paths) |
| Sandbox home writability | `crates/codegen/xai-grok-sandbox/src/paths.rs` |
| Production endpoints | `crates/codegen/xai-grok-env/src/lib.rs` |

---

## 19. Appendix A — Side-by-side path map

| Concern | Official | Powergrok |
|---------|----------|-----------|
| Command | `grok` | `powergrok` |
| Wrapper / shim | often `~/.local/bin/grok` → managed bin | `~/.local/bin/powergrok` → wrapper → lib binary |
| State root | `~/.grok` | `~/.powergrok` |
| Auth | `~/.grok/auth.json` | `~/.powergrok/auth.json` |
| Config | `~/.grok/config.toml` | `~/.powergrok/config.toml` |
| Sessions | `~/.grok/sessions/` | `~/.powergrok/sessions/` |
| Managed release bin | `~/.grok/bin/grok` | unused in v1 |
| Project config | `<repo>/.grok/` | **same** `<repo>/.grok/` |
| Auto-update | product default (often on) | forced off in seed |

---

## 20. Appendix B — Decision record (proposed)

| Decision | Choice | Status |
|----------|--------|--------|
| Isolation mechanism | `GROK_HOME` + wrapper | Proposed |
| Command name | `powergrok` | Proposed |
| State directory | `~/.powergrok` | Proposed |
| Auto-update | disabled | Proposed (required for source builds) |
| Code changes in engine | none for v1 | Proposed |
| Install automation | shell script Phase 2 | Proposed |
| Project `.grok` | shared | Accepted product behavior |

---

## 21. Appendix C — Reviewer checklist (copy/paste)

```text
[ ] Isolation model understood (GROK_HOME OnceLock + wrapper)
[ ] Official install paths are never write targets
[ ] auto_update=false is mandatory and create-if-missing only
[ ] Shared project .grok/ called out as intentional
[ ] Fish completion / updater caveats acknowledged
[ ] Verification matrix sufficient for merge of Phase 2 script
[ ] Open questions §15 answered or parked with owners
[ ] Approve Phase 0 only / Approve through Phase 2 / Request changes
```

---

*End of plan. Implementation of Phase 1+ starts only after peer review disposition.*
