# AGENTS.md — zfifteen/powergrok (Powergrok)

This repository is the **zfifteen** fork of [xai-org/grok-build](https://github.com/xai-org/grok-build), published as **[zfifteen/powergrok](https://github.com/zfifteen/powergrok)**.  
It is the home of **Powergrok**: a source-built, side-by-side install of Grok Build that coexists with the official `grok` CLI.

Agents working in this repo **must** follow the branch and release flow below.  
Product design for Powergrok isolation and install lives in [`docs/powergrok/BUILD_PLAN.md`](docs/powergrok/BUILD_PLAN.md).

---

## 1. Branch model (mandatory)

| Branch | Role | Merge into it from |
|--------|------|--------------------|
| **`main`** | **Upstream intake only.** Tracks `xai-org/grok-build` `main`. | Upstream only (fetch + fast-forward or reset to match upstream). |
| **`powergrok`** | **Product trunk and GitHub default branch.** Official Powergrok builds ship from here. | (1) `main` after upstream sync, (2) feature PRs. |
| **`feat/*` / topic branches** | Feature work, docs, product changes. | Open PRs **into `powergrok`**, not into `main`. |

### Flow (canonical)

```text
xai-org/grok-build (upstream)
        │
        │  fetch / sync
        ▼
      main                 ← upstream mirror; not the product default
        │
        │  merge when ready
        ▼
    powergrok              ← product default; official builds
        ▲
        │  merge PRs
   feat / topic branches
```

### Hard rules for agents

1. **Default branch for work and PRs is `powergrok`.**  
   Base new branches on `powergrok`. Target PRs at `powergrok`.
2. **Do not land product work on `main`.**  
   `main` exists to stay aligned with upstream history.
3. **Do not open PRs against `main` for Powergrok features, docs, or install scripts.**  
   Use `powergrok` as the PR base.
4. **After syncing upstream into `main`, merge `main` → `powergrok`** (merge commit or PR) so the product line receives upstream.  
   Prefer regular merges over rewriting shared product history.
5. **Official Powergrok builds** always come from the **`powergrok`** tip (or tags cut from it).  
   Build/install procedure: see `docs/powergrok/BUILD_PLAN.md`.
6. **Force-push to `main` or `powergrok` only with explicit operator approval.**  
   History rewrites on those branches close or break open PRs and confuse GitHub compare views.
7. **Keep feature branches based on a common ancestor with `powergrok`.**  
   If `main`/`powergrok` were reset to a new upstream root, rebase topic branches onto `powergrok` before opening or updating a PR.

---

## 2. Remotes

| Remote | URL (expected) | Purpose |
|--------|----------------|---------|
| `origin` | `https://github.com/zfifteen/powergrok.git` | This fork (product + intake) |
| `upstream` | `https://github.com/xai-org/grok-build.git` | Upstream source of truth for `main` |

**Hard rule:** `upstream` must point at **xai-org/grok-build**, never at this fork. A dual-remote-to-fork layout silently hides true xAI updates.

Heal / bootstrap:

```sh
git remote set-url origin https://github.com/zfifteen/powergrok.git
git remote set-url upstream https://github.com/xai-org/grok-build.git
# optional: refuse accidental push to xAI
git remote set-url --push upstream DISABLE_PUSH_USE_ORIGIN_ONLY
# or:
./bin/sync-upstream-intake.sh ensure-remotes
```

---

## 3. Sync upstream without losing `powergrok` work

**Full runbook (daily + first-time root republish, conflict hotspots, recovery):**  
**[`docs/powergrok/UPSTREAM_INTAKE.md`](docs/powergrok/UPSTREAM_INTAKE.md)**  
**Script:** [`bin/sync-upstream-intake.sh`](bin/sync-upstream-intake.sh)

### Safety invariants (mandatory)

1. **Product commits live only on `powergrok` (and feature branches).** Never put branding / effort / product docs on `main` — the next intake `reset --hard` drops them from `main` by design.
2. **Intake never force-pushes `powergrok`.** Product tip moves only by merge commit (or feature PR merges).
3. **Before any intake that rewrites `main` or merges into product, create a backup ref** (`backup/powergrok-<timestamp>`). Prefer also `git push -u origin backup/…`.
4. **Force-push `main` only with explicit operator approval** (`--force-with-lease`), typically when xAI republished a new open-source root (no merge-base with old `main`).
5. Prefer **merge** of `main` → `powergrok` over rebasing `powergrok` onto `main` so product SHAs stay stable for open PRs.

### Daily happy path (after roots are aligned)

```sh
./bin/sync-upstream-intake.sh status
./bin/sync-upstream-intake.sh full --apply
# if only origin/main needs a non-fast-forward update after operator OK:
# ./bin/sync-upstream-intake.sh intake-main --apply --i-approve-force-main
git push origin powergrok   # after a clean product merge
./bin/sync-upstream-intake.sh verify
```

### First-time / unrelated histories (root republish)

When `git merge-base main upstream/main` is empty:

```sh
./bin/sync-upstream-intake.sh backup --push-backup
./bin/sync-upstream-intake.sh intake-main --apply --i-approve-force-main   # operator OK required
./bin/sync-upstream-intake.sh merge-powergrok --apply --allow-unrelated
# resolve conflicts → git push origin powergrok
```

Manual skeleton (same semantics):

```sh
git fetch upstream main && git fetch origin
git branch "backup/powergrok-$(date +%Y%m%d-%H%M%S)" powergrok

git checkout main
git reset --hard upstream/main
git push origin main   # or --force-with-lease with operator OK

git checkout powergrok
git merge main -m "chore(intake): merge upstream into powergrok"
# if no merge-base: add --allow-unrelated-histories once
git push origin powergrok
```

Verify:

```sh
./bin/sync-upstream-intake.sh verify
# origin/main SHA == upstream/main SHA
# main is ancestor of powergrok (or documented open merge)
```

---

## 4. Feature work recipe

```sh
git fetch origin
git checkout powergrok
git pull origin powergrok

git checkout -b feat/<short-name>
# ... commits ...

git push -u origin HEAD
gh pr create --base powergrok --head feat/<short-name>
```

If GitHub reports **unrelated histories** between the topic branch and `powergrok`:

```sh
# Replay commits after the dead root onto current powergrok
git fetch origin powergrok
git rebase --onto origin/powergrok <old-root-sha> feat/<short-name>
git push --force-with-lease
```

Do **not** open a PR from `main` into `powergrok` merely to “sync” when tips already match.

---

## 5. Powergrok product identity (pointer)

Powergrok is a **parallel install**, not a rename of the official binary on the operator’s machine:

| Concern | Official | Powergrok |
|---------|----------|-----------|
| Command | `grok` | `powergrok` |
| User state | `~/.grok` | `~/.powergrok` (`GROK_HOME`) |
| Project tree | `<repo>/.grok/` | `<repo>/.powergrok/` (argv0 / named binary; see build plan) |
| Build line | upstream releases | **this repo’s `powergrok` branch** |

Full plan, locked decisions, and peer-review amendments:  
**[`docs/powergrok/BUILD_PLAN.md`](docs/powergrok/BUILD_PLAN.md)**

---

## 6. What “done” looks like for git health

- [ ] `upstream` remote URL is `https://github.com/xai-org/grok-build.git` (not the fork).
- [ ] `origin/main` SHA equals `upstream/main` SHA (or documented lag with a plan to sync).
- [ ] GitHub **default branch** is `powergrok`.
- [ ] Open product PRs use **base = `powergrok`**.
- [ ] Topic branches share a merge-base with `powergrok` (GitHub “compare” works without “unrelated histories”).
- [ ] Force-pushes to `main` only with operator approval; **`powergrok` not force-pushed for intake**.
- [ ] A recent `backup/powergrok-*` ref exists before any root-jump intake.
- [ ] `./bin/sync-upstream-intake.sh verify` passes after an intake cycle.

---

## 7. Scope of this file

This `AGENTS.md` owns **fork workflow and Powergrok branch discipline** for this checkout.  
Daily/root-republish **upstream intake** details: [`docs/powergrok/UPSTREAM_INTAKE.md`](docs/powergrok/UPSTREAM_INTAKE.md).  
Upstream Grok Build coding style, crate layout, and product behavior remain as documented in the tree (README, crate docs, user guide). When implementing Powergrok isolation/install, treat `docs/powergrok/BUILD_PLAN.md` as the product contract.

*Last updated: 2026-07-17 — dual-branch model + safe daily upstream intake (backup, no force on powergrok).*
