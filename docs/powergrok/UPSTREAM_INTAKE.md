# Upstream intake — keep Power Grok product work, absorb xAI daily

This is the **operator/agent runbook** for integrating [xai-org/grok-build](https://github.com/xai-org/grok-build) into this fork without losing `powergrok` product commits (branding, Expert/Heavy/brains, README, docs).

Canonical branch rules live in repo-root [`AGENTS.md`](../../AGENTS.md).  
Automation entrypoint: [`bin/sync-upstream-intake.sh`](../../bin/sync-upstream-intake.sh).

---

## Goals

1. **Never delete product history** on `powergrok` as part of intake.
2. Keep **`main` as a pure mirror** of xAI `main` (no product commits).
3. Bring xAI changes into product via **`main` → `powergrok` merge** (merge commit preferred).
4. Support **daily** monorepo syncs from xAI with a repeatable checklist.
5. Survive **root republishes** (xAI sometimes re-publishes with a new “Publish harness…” SHA that shares no merge-base with an older local `main`).

---

## Non-goals / hard bans

| Ban | Why |
|-----|-----|
| Land product work on `main` | `main` is intake-only; product work is invisible to the next reset |
| `git reset --hard` / force-push **`powergrok`** for intake | Rewrites product tip; loses or rewrites SHAs open PRs depend on |
| Rebase `powergrok` onto `main` without an explicit backup + operator OK | Same class of rewrite risk |
| Fix remotes by pointing `upstream` at the fork | You stop seeing true xAI updates (this happened once; fixed 2026-07-17) |
| Thrash `gh auth`, flip remotes to SSH, rewrite global `user.*` | Separate from intake; see Hermes `git-https-credentials` |

---

## Mental model

```text
xai-org/grok-build  main     (true upstream; often daily “Synced from monorepo”)
        │
        │  fetch + hard-align (main is disposable mirror)
        ▼
origin/main                  (intake only — may force-with-lease after root jump)
        │
        │  merge commit (product history preserved as first parent)
        ▼
origin/powergrok             (product trunk — never force for intake)
        ▲
        │  feature PRs
   feat/*
```

**What “don’t lose my changes” means here**

- Your commits stay reachable from `powergrok` (first parent after merges).
- Before any risky step, a **local backup ref** points at the pre-intake tip.
- Optional: push the backup branch to `origin` so it survives machine loss.

---

## Remotes (must be true)

| Remote | URL | Role |
|--------|-----|------|
| `origin` | `https://github.com/zfifteen/powergrok.git` | Fork push/pull |
| `upstream` | `https://github.com/xai-org/grok-build.git` | **Only** xAI source for `main` |

Heal if wrong:

```sh
git remote set-url origin https://github.com/zfifteen/powergrok.git
git remote set-url upstream https://github.com/xai-org/grok-build.git
# optional: refuse accidental push to xAI
git remote set-url --push upstream DISABLE_PUSH_USE_ORIGIN_ONLY
git fetch upstream main
git fetch origin
```

Do **not** use a dual-remote layout where both names point at the fork and call that “synced with xAI.”

---

## Daily rhythm (happy path — after roots are aligned)

Once `main` and `upstream/main` share history and `powergrok` has already merged that line:

```sh
# From repo root, clean tree preferred
./bin/sync-upstream-intake.sh status          # lag report
./bin/sync-upstream-intake.sh backup          # local safety tip
./bin/sync-upstream-intake.sh intake-main --apply
./bin/sync-upstream-intake.sh merge-powergrok --apply
# resolve conflicts if any, then:
./bin/sync-upstream-intake.sh verify
```

Or one shot (still refuses force without flags):

```sh
./bin/sync-upstream-intake.sh full --apply
```

### Manual equivalent

```sh
git fetch upstream main
git fetch origin

# 1) Safety
git branch "backup/powergrok-$(date +%Y%m%d-%H%M%S)" powergrok

# 2) Mirror xAI onto local main (no product commits allowed on main)
git checkout main
git reset --hard upstream/main
git push origin main
# only if rejected because tip moved / root jump — needs operator OK:
# git push origin main --force-with-lease

# 3) Product absorbs intake (preserves powergrok commits)
git checkout powergrok
git merge main -m "chore(intake): merge upstream $(git rev-parse --short main) into powergrok"
# fix conflicts → git add -A && git merge --continue
git push origin powergrok
```

**Daily force rules**

- Force-push **`main`**: only when histories diverged or root republished; always `--force-with-lease`; always after backup.
- Force-push **`powergrok`**: **not part of intake.** Only operator-approved recovery.

---

## First-time / root-republish (no merge-base)

Symptom:

```text
fatal: refusing to merge unrelated histories
# or
git merge-base main upstream/main   # empty
```

xAI re-published “Publish harness…” under a new SHA (`c68e39f…`) while this fork’s old `main` (`b189869…`) is a different root. Product commits on `powergrok` still parent from the old root — they are **not** deleted by fixing `main`.

### Safe procedure (merge, keep all SHAs)

```sh
# A. Immortalize product tip
git branch backup/powergrok-pre-intake-$(date +%Y%m%d) powergrok
git push -u origin backup/powergrok-pre-intake-$(date +%Y%m%d)   # recommended

# B. Align intake mirror to xAI (rewrites main only)
git fetch upstream main
git checkout main
git reset --hard upstream/main
git push origin main --force-with-lease   # requires operator approval

# C. Merge into product WITHOUT rewriting product commits
git checkout powergrok
git merge main --allow-unrelated-histories \
  -m "chore(intake): merge upstream root $(git rev-parse --short main) into powergrok (unrelated histories)"
# resolve conflicts carefully (see hotspots below)
git push origin powergrok
```

After this **one** unrelated merge, later days use the happy path (no `--allow-unrelated-histories`).

### Alternate (cleaner history, rewrites product SHAs — only if you accept force-push of powergrok)

```sh
git branch backup/powergrok-pre-rebase-$(date +%Y%m%d) powergrok
OLD_MAIN=$(git rev-parse main)          # old intake tip, e.g. b189869
git checkout main && git reset --hard upstream/main
git checkout powergrok
git rebase --onto main "$OLD_MAIN"      # replays product-only commits
# fix conflicts per commit; then force-with-lease powergrok ONLY with operator OK
```

Prefer the **merge** path for “I don’t want to lose changes” and for open PR stability.

---

## Conflict hotspots (Power Grok product)

Expect conflicts where product work and xAI both touch:

| Area | Paths (typical) | Prefer |
|------|-----------------|--------|
| Branding | `xai-grok-config` branding adapter, welcome, clap about, notifications, pager-bin | Keep `product_name()` / `powergrok` feature behavior |
| Effort modes / brains | `session/effort_*`, slash registration, spawn/team hooks | Keep product modules; re-wire call sites if upstream moved neighbors |
| Session plumbing | `persistence.rs`, jsonl storage, `slash_commands.rs`, acp spawn/tool_calls/tests | Integrate **both**: upstream durability/auth + product effort hooks |
| README | root `README.md` | Keep **Power Grok** product face (modes-first, brains, hero); re-apply after taking any useful upstream install notes into Reference |
| Versions / lock | `Cargo.toml`, `Cargo.lock`, shell version | Prefer upstream version lineage after merge; re-run `cargo` generate lock as needed |

**Principle:** upstream wins on shared harness security/correctness; product wins on branding and effort builtins; merge both when a file is dual-purpose.

### Post-merge verify (minimum)

```sh
./bin/sync-upstream-intake.sh verify
# plus product gates when tree builds:
cargo test -p xai-grok-shell --test effort_brains_load
cargo test -p xai-grok-shell --test effort_mode_gates
cargo check -p xai-grok-pager-bin
cargo check -p xai-grok-pager-bin --features powergrok
# hooks SSRF partial mitigation should exist after modern upstream:
rg -n 'Policy::none' crates/codegen/xai-grok-hooks/src/runner/http.rs
# welcome still branded under feature:
rg -n 'product_name' crates/codegen/xai-grok-pager/src/views/welcome/mod.rs
```

---

## Agent rules for intake sessions

1. Run **`status`** first; report lag (SHAs, commit subjects, version if present).
2. Create **`backup/powergrok-…`** before any reset/merge.
3. Never force-push `powergrok` for intake.
4. Never force-push `main` without quoting the exact command and waiting for operator approval (script flag `--i-approve-force-main`).
5. Prefer **merge commits** into `powergrok` over rebase for routine intake.
6. Do not commit product features while mid-conflict on an intake merge; finish intake or abort.
7. After success, leave `status` green: `origin/main == upstream/main`, and `main` is ancestor of `powergrok` (or documented open merge).

---

## Recovery if something goes wrong

```sh
# Return product tip to pre-intake backup (local)
git checkout powergrok
git reset --hard backup/powergrok-pre-intake-YYYYMMDD

# If backup was pushed:
git fetch origin
git reset --hard origin/backup/powergrok-pre-intake-YYYYMMDD
# push recovery needs operator OK if origin/powergrok already moved
```

Abort an in-progress merge:

```sh
git merge --abort
```

---

## Cadence recommendation

| When | Action |
|------|--------|
| Daily (or when you sit down to ship) | `./bin/sync-upstream-intake.sh status` |
| Upstream tip moved | `backup` → `intake-main --apply` → `merge-powergrok --apply` → `verify` |
| Large security dump (SSRF, sandbox, auth) | Same day merge; prioritize conflict resolution on hooks/sandbox |
| Mid-feature on `feat/*` | Finish or park the feature PR; rebase `feat/*` onto updated `powergrok` **after** intake lands |

---

## Related

- [`AGENTS.md`](../../AGENTS.md) — branch model
- [`BUILD_PLAN.md`](BUILD_PLAN.md) — install / isolation product contract
- [`BRANDING_PLAN.md`](BRANDING_PLAN.md) — keep branding behind adapters on every sync
- Hermes skill `grok-build` — fork identity + SSRF provenance notes

*Last updated: 2026-07-17 — first-class daily intake + root-republish safety.*
