# Power Grok — source-build lifecycle

**Issue:** [#14](https://github.com/zfifteen/powergrok/issues/14)  
**Contract:** `docs/powergrok/BUILD_PLAN.md` §5 topology, §8 rebuild, §9 install, D3 `auto_update = false`, D9 VERSION  
**Branch:** product trunk `powergrok` (never force-push for intake)

## Why this exists

Power Grok seeds **`[cli] auto_update = false`** so the official release channel cannot replace a source-built binary under `~/.powergrok`. Freshness is **operator-controlled**: pull trunk, re-run the installer, optional rollback from `powergrok.prev`.

`powergrok.prev` is a **single slot**: each upgrade overwrites the previous backup (a third upgrade drops the oldest retained binary). Rollback may also write `powergrok.before-rollback` (forensics copy of the binary left behind) under the same lib dir.

## Happy path (upgrade)

```sh
cd /path/to/powergrok
git checkout powergrok
git pull origin powergrok

./scripts/install-powergrok.sh
# rebuilds release (unless --no-build), installs named binary,
# backs up prior binary → $prefix/lib/powergrok/powergrok.prev,
# rewrites VERSION (git SHA + built_at), refreshes wrapper bake,
# leaves GROK_HOME / auto_update = false untouched if already seeded
```

Verify:

```sh
./scripts/install-powergrok.sh --status
# or in-session: /install-status

cat ~/.local/lib/powergrok/VERSION
grep auto_update ~/.powergrok/config.toml   # still false
command -v grok                             # official path unchanged
```

## Status and freshness (advisory only)

```sh
./scripts/install-powergrok.sh --status
./scripts/install-powergrok.sh --status --check-freshness
# --check-freshness may `git fetch origin powergrok` then report
# commits_behind_origin_powergrok. It NEVER downloads or installs bits.
```

In-product (AlwaysOn builtin):

```text
/install-status
# alias: /powergrok-status
```

Shows product name, project dirname, `GROK_HOME`, VERSION fields when the process is the installed named binary (sibling `VERSION` file), auto_update probe, and upgrade/rollback hints.

## Rollback

After at least one upgrade that created `powergrok.prev`:

```sh
./scripts/install-powergrok.sh --rollback
# restores lib binary from powergrok.prev
# writes VERSION with state=rolled-back and git=(unknown) (not a commit id)
# may write powergrok.before-rollback (the binary that was replaced)
# does not touch wrapper, GROK_HOME, or official grok
# does not invoke cargo or official updater
```

Then re-run a normal install when ready for a proper SHA stamp again.

### Freshness labels

`--status --check-freshness` reports **checkout** lag (`checkout_commits_behind_origin_powergrok` = repo HEAD vs `origin/powergrok`). That is **not** “installed binary vs origin.” Compare `installed_VERSION_git` separately. Fetch may update remote-tracking refs only; it never installs bits.

## Uninstall

```sh
./scripts/install-powergrok.sh --uninstall              # wrapper + lib only
./scripts/install-powergrok.sh --uninstall --purge-home # also remove GROK_HOME
# never removes repo .powergrok/ project trees
# never touches official grok / ~/.grok
```

## Layout reminder

| Path | Role |
|------|------|
| `$prefix/bin/powergrok` | PATH wrapper |
| `$prefix/lib/powergrok/powergrok` | real binary (argv0) |
| `$prefix/lib/powergrok/powergrok.prev` | previous binary after upgrade (single slot) |
| `$prefix/lib/powergrok/powergrok.before-rollback` | optional forensics copy written by `--rollback` |
| `$prefix/lib/powergrok/VERSION` | git SHA, branch, built_at, features (or `state=rolled-back`) |
| `$GROK_HOME/config.toml` | seed: `auto_update = false` |

Default prefix: `~/.local`. Default home: `~/.powergrok`.

## Non-goals

- Do **not** turn auto-update on under `~/.powergrok`
- Do **not** use official download/updater as the primary upgrade path
- Do **not** overwrite official `grok`
- Do **not** force-push `powergrok` as part of UX
- Freshness is never a force-install

## Related

- Install issue #6 / `scripts/install-powergrok.sh`
- Isolation #4 / project tree D7
- Bootstrap #7 / `/bootstrap-project`
