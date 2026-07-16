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

If `upstream` is missing:

```sh
git remote add upstream https://github.com/xai-org/grok-build.git
```

---

## 3. Sync upstream into `main` (operator / agent recipe)

```sh
git fetch upstream main
git fetch origin main

# main must match upstream tip (identical history)
git checkout main
git reset --hard upstream/main
git push origin main
# If histories diverged (different roots), force-push is required and needs
# explicit operator approval: git push origin main --force-with-lease
```

Then bring product current:

```sh
git checkout powergrok
git merge main          # or open a PR: main → powergrok
git push origin powergrok
```

Verify:

```sh
# should be identical
git rev-parse origin/main upstream/main
# product may be ahead of main after local merges; main should not be ahead of powergrok without a pending merge
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

- [ ] `origin/main` SHA equals `upstream/main` SHA (or documented lag with a plan to sync).
- [ ] GitHub **default branch** is `powergrok`.
- [ ] Open product PRs use **base = `powergrok`**.
- [ ] Topic branches share a merge-base with `powergrok` (GitHub “compare” works without “unrelated histories”).
- [ ] Force-pushes to `main` / `powergrok` only happened with operator approval and a recovery note if PRs were affected.

---

## 7. Scope of this file

This `AGENTS.md` owns **fork workflow and Powergrok branch discipline** for this checkout.  
Upstream Grok Build coding style, crate layout, and product behavior remain as documented in the tree (README, crate docs, user guide). When implementing Powergrok isolation/install, treat `docs/powergrok/BUILD_PLAN.md` as the product contract.

*Last updated: 2026-07-15 — dual-branch model (`main` = upstream intake, `powergrok` = product trunk).*
