#!/usr/bin/env bash
# Power Grok / powergrok — safe upstream intake helper.
#
# Keeps product history on `powergrok`. Treats `main` as a disposable mirror
# of xai-org/grok-build. See docs/powergrok/UPSTREAM_INTAKE.md.
#
# Default is dry-run for mutating commands (prints the plan). Pass --apply
# to execute. Force-pushing origin/main requires --i-approve-force-main.
set -euo pipefail

ORIGIN_URL_EXPECTED="https://github.com/zfifteen/powergrok.git"
UPSTREAM_URL_EXPECTED="https://github.com/xai-org/grok-build.git"
PRODUCT_BRANCH="powergrok"
INTAKE_BRANCH="main"
UPSTREAM_REMOTE="upstream"
ORIGIN_REMOTE="origin"
UPSTREAM_REF="main"

usage() {
  cat <<'EOF'
Usage: bin/sync-upstream-intake.sh <command> [options]

Commands:
  status              Fetch + report lag (main / powergrok / upstream)
  backup              Create local backup/powergrok-<timestamp> at product tip
  ensure-remotes      Fix origin/upstream URLs if wrong (always applies)
  intake-main         Align local main to upstream/main (dry-run unless --apply)
  merge-powergrok     Merge main into powergrok (dry-run unless --apply)
  verify              Health checks after intake
  full                backup + intake-main + merge-powergrok (dry-run unless --apply)

Options:
  --apply                   Execute mutating git steps (intake-main / merge / full)
  --i-approve-force-main    Allow origin main --force-with-lease when needed
  --push-backup             After backup, also push backup branch to origin
  --allow-unrelated         Pass --allow-unrelated-histories on product merge
  --no-fetch                Skip network fetch (use existing remote-tracking refs)
  -h, --help                Show this help

Safety:
  - Never force-pushes powergrok.
  - Never force-pushes main without --i-approve-force-main.
  - Creates backup refs before intake-main / full when product tip exists.
EOF
}

log()  { printf '%s\n' "$*"; }
err()  { printf 'error: %s\n' "$*" >&2; }
die()  { err "$*"; exit 1; }

require_git_repo() {
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || die "not inside a git work tree"
  # Prefer repo root
  local root
  root="$(git rev-parse --show-toplevel)"
  cd "$root"
}

remote_url() {
  git remote get-url "$1" 2>/dev/null || true
}

normalize_url() {
  # Strip trailing .git and slashes for comparison
  local u="${1:-}"
  u="${u%.git}"
  u="${u%/}"
  printf '%s' "$u"
}

urls_match() {
  local a b
  a="$(normalize_url "$1")"
  b="$(normalize_url "$2")"
  [[ "$a" == "$b" ]]
}

ensure_clean_enough() {
  # Allow dirty tree for status/backup; block intake/merge if dirty.
  if [[ -n "$(git status --porcelain)" ]]; then
    die "working tree is dirty; commit/stash first (git status --porcelain not empty)"
  fi
}

cmd_ensure_remotes() {
  if ! git remote get-url "$ORIGIN_REMOTE" >/dev/null 2>&1; then
    git remote add "$ORIGIN_REMOTE" "$ORIGIN_URL_EXPECTED"
    log "added remote $ORIGIN_REMOTE -> $ORIGIN_URL_EXPECTED"
  elif ! urls_match "$(remote_url "$ORIGIN_REMOTE")" "$ORIGIN_URL_EXPECTED"; then
    log "fixing $ORIGIN_REMOTE URL: $(remote_url "$ORIGIN_REMOTE") -> $ORIGIN_URL_EXPECTED"
    git remote set-url "$ORIGIN_REMOTE" "$ORIGIN_URL_EXPECTED"
  fi

  if ! git remote get-url "$UPSTREAM_REMOTE" >/dev/null 2>&1; then
    git remote add "$UPSTREAM_REMOTE" "$UPSTREAM_URL_EXPECTED"
    log "added remote $UPSTREAM_REMOTE -> $UPSTREAM_URL_EXPECTED"
  elif ! urls_match "$(remote_url "$UPSTREAM_REMOTE")" "$UPSTREAM_URL_EXPECTED"; then
    log "fixing $UPSTREAM_REMOTE URL: $(remote_url "$UPSTREAM_REMOTE") -> $UPSTREAM_URL_EXPECTED"
    git remote set-url "$UPSTREAM_REMOTE" "$UPSTREAM_URL_EXPECTED"
  fi

  # Discourage accidental push to xAI
  git remote set-url --push "$UPSTREAM_REMOTE" DISABLE_PUSH_USE_ORIGIN_ONLY 2>/dev/null || true

  log "remotes:"
  git remote -v | sed 's/^/  /'
}

do_fetch() {
  if [[ "$NO_FETCH" == "1" ]]; then
    log "skip fetch (--no-fetch)"
    return 0
  fi
  log "fetching $UPSTREAM_REMOTE $UPSTREAM_REF and $ORIGIN_REMOTE …"
  # Force-update the remote-tracking ref so a stale/locked upstream/main
  # cannot leave status/intake reading an old SHA after a successful network fetch.
  if ! git fetch "$UPSTREAM_REMOTE" "+${UPSTREAM_REF}:refs/remotes/${UPSTREAM_REMOTE}/${UPSTREAM_REF}"; then
    log "warn: forced tracking update failed; falling back to plain fetch"
    git fetch "$UPSTREAM_REMOTE" "$UPSTREAM_REF"
  fi
  git fetch "$ORIGIN_REMOTE" --prune
}

# Canonical CLI version lives in xai-grok-version (lockstepped with shell).
version_line_from_file() {
  local path="$1"
  if [[ -f "$path" ]]; then
    rg -N --no-line-number '^version\s*=' "$path" | head -1 || true
  fi
}

version_line_from_ref() {
  local ref="$1"
  local path="$2"
  if git cat-file -e "${ref}:${path}" 2>/dev/null; then
    git show "${ref}:${path}" | rg -N --no-line-number '^version\s*=' | head -1 || true
  fi
}

report_cli_versions() {
  local local_line up_line
  local_line="$(version_line_from_file crates/codegen/xai-grok-version/Cargo.toml)"
  if [[ -z "$local_line" ]]; then
    local_line="$(version_line_from_file crates/codegen/xai-grok-shell/Cargo.toml)"
  fi
  up_line="$(version_line_from_ref "$UPSTREAM_REMOTE/$UPSTREAM_REF" crates/codegen/xai-grok-version/Cargo.toml)"
  if [[ -z "$up_line" ]]; then
    up_line="$(version_line_from_ref "$UPSTREAM_REMOTE/$UPSTREAM_REF" crates/codegen/xai-grok-shell/Cargo.toml)"
  fi
  if [[ -n "$local_line" || -n "$up_line" ]]; then
    log ""
    log "local CLI version (xai-grok-version):    ${local_line:-missing}"
    log "upstream CLI version (xai-grok-version): ${up_line:-missing}"
  fi
}

short() {
  git rev-parse --short "$1" 2>/dev/null || printf 'missing'
}

full_sha() {
  git rev-parse "$1" 2>/dev/null || printf ''
}

has_merge_base() {
  git merge-base "$1" "$2" >/dev/null 2>&1
}

cmd_status() {
  cmd_ensure_remotes
  do_fetch

  local up_sha main_sha prod_sha origin_main origin_prod
  up_sha="$(full_sha "$UPSTREAM_REMOTE/$UPSTREAM_REF")"
  main_sha="$(full_sha "$INTAKE_BRANCH" || true)"
  prod_sha="$(full_sha "$PRODUCT_BRANCH" || true)"
  origin_main="$(full_sha "$ORIGIN_REMOTE/$INTAKE_BRANCH" || true)"
  origin_prod="$(full_sha "$ORIGIN_REMOTE/$PRODUCT_BRANCH" || true)"

  log "=== intake status ==="
  log "upstream ($UPSTREAM_REMOTE/$UPSTREAM_REF): $(short "$UPSTREAM_REMOTE/$UPSTREAM_REF")  ${up_sha}"
  log "local main:                              $(short "$INTAKE_BRANCH")  ${main_sha:-missing}"
  log "origin/main:                             $(short "$ORIGIN_REMOTE/$INTAKE_BRANCH")  ${origin_main:-missing}"
  log "local powergrok:                         $(short "$PRODUCT_BRANCH")  ${prod_sha:-missing}"
  log "origin/powergrok:                        $(short "$ORIGIN_REMOTE/$PRODUCT_BRANCH")  ${origin_prod:-missing}"
  log ""

  if [[ -n "$up_sha" && -n "$main_sha" ]]; then
    if [[ "$up_sha" == "$main_sha" ]]; then
      log "main == upstream/main  (intake mirror current)"
    else
      log "main != upstream/main  (intake lag)"
      if has_merge_base "$INTAKE_BRANCH" "$UPSTREAM_REMOTE/$UPSTREAM_REF"; then
        log "  merge-base: $(short "$(git merge-base "$INTAKE_BRANCH" "$UPSTREAM_REMOTE/$UPSTREAM_REF")")"
        log "  commits on upstream not in main:"
        git log --oneline "$INTAKE_BRANCH..$UPSTREAM_REMOTE/$UPSTREAM_REF" | sed 's/^/    /' || true
      else
        log "  NO merge-base (unrelated / republished root). First-time intake needs force main + --allow-unrelated product merge."
        log "  See docs/powergrok/UPSTREAM_INTAKE.md § First-time / root-republish"
      fi
    fi
  fi

  if [[ -n "$main_sha" && -n "$prod_sha" ]]; then
    if git merge-base --is-ancestor "$INTAKE_BRANCH" "$PRODUCT_BRANCH" 2>/dev/null; then
      log "main is ancestor of powergrok  (product has absorbed current main)"
    else
      log "main is NOT fully absorbed into powergrok  (run merge-powergrok after intake-main)"
      if has_merge_base "$INTAKE_BRANCH" "$PRODUCT_BRANCH"; then
        log "  commits on main not in powergrok:"
        git log --oneline "$PRODUCT_BRANCH..$INTAKE_BRANCH" | sed 's/^/    /' || true
      else
        log "  NO merge-base between main and powergrok (root jump pending)"
      fi
    fi
  fi

  report_cli_versions

  log ""
  log "working tree: $(if [[ -z "$(git status --porcelain)" ]]; then echo clean; else echo DIRTY; fi)"
  log ""
  log "operator shortcuts:"
  log "  update fork main only:  ./bin/update-from-xai.sh --apply"
  log "  main + merge powergrok: ./bin/update-from-xai.sh --full --apply"
  log "  status only:            ./bin/update-from-xai.sh"
}

cmd_backup() {
  cmd_ensure_remotes
  git show-ref --verify --quiet "refs/heads/$PRODUCT_BRANCH" || die "missing branch $PRODUCT_BRANCH"
  local stamp name
  stamp="$(date +%Y%m%d-%H%M%S)"
  name="backup/${PRODUCT_BRANCH}-${stamp}"
  git branch "$name" "$PRODUCT_BRANCH"
  log "created local backup ref $name @ $(short "$PRODUCT_BRANCH") ($(full_sha "$PRODUCT_BRANCH"))"
  if [[ "$PUSH_BACKUP" == "1" ]]; then
    log "pushing $name to $ORIGIN_REMOTE …"
    git push -u "$ORIGIN_REMOTE" "$name"
  else
    log "tip: pass --push-backup to also store this on origin"
  fi
  printf '%s\n' "$name"
}

cmd_intake_main() {
  cmd_ensure_remotes
  do_fetch
  ensure_clean_enough

  local up_sha main_sha
  up_sha="$(full_sha "$UPSTREAM_REMOTE/$UPSTREAM_REF")"
  [[ -n "$up_sha" ]] || die "missing $UPSTREAM_REMOTE/$UPSTREAM_REF after fetch"
  main_sha="$(full_sha "$INTAKE_BRANCH" || true)"

  if [[ "$main_sha" == "$up_sha" ]]; then
    log "main already at upstream/main ($(short "$up_sha")). nothing to do."
    return 0
  fi

  local need_force=0
  if [[ -n "$main_sha" ]] && ! has_merge_base "$INTAKE_BRANCH" "$UPSTREAM_REMOTE/$UPSTREAM_REF"; then
    need_force=1
    log "unrelated histories: aligning main will rewrite intake branch tip"
  elif [[ -n "$(full_sha "$ORIGIN_REMOTE/$INTAKE_BRANCH" || true)" ]]; then
    # If origin/main is not ancestor of new tip, push will need force
    if ! git merge-base --is-ancestor "$ORIGIN_REMOTE/$INTAKE_BRANCH" "$UPSTREAM_REMOTE/$UPSTREAM_REF" 2>/dev/null; then
      need_force=1
    fi
  fi

  log "plan:"
  log "  git checkout $INTAKE_BRANCH"
  log "  git reset --hard $UPSTREAM_REMOTE/$UPSTREAM_REF   # -> $(short "$UPSTREAM_REMOTE/$UPSTREAM_REF")"
  if [[ "$need_force" == "1" ]]; then
    log "  git push $ORIGIN_REMOTE $INTAKE_BRANCH --force-with-lease"
  else
    log "  git push $ORIGIN_REMOTE $INTAKE_BRANCH"
  fi

  if [[ "$APPLY" != "1" ]]; then
    log "dry-run only. re-run with --apply to execute."
    if [[ "$need_force" == "1" ]]; then
      log "force push will also need --i-approve-force-main"
    fi
    return 0
  fi

  if [[ "$need_force" == "1" && "$APPROVE_FORCE_MAIN" != "1" ]]; then
    die "origin/main update needs --force-with-lease; re-run with --apply --i-approve-force-main after operator approval"
  fi

  # Backup product tip before rewriting intake (always)
  cmd_backup >/dev/null

  local current
  current="$(git branch --show-current || true)"
  git checkout "$INTAKE_BRANCH"
  git reset --hard "$UPSTREAM_REMOTE/$UPSTREAM_REF"
  if [[ "$need_force" == "1" ]]; then
    git push "$ORIGIN_REMOTE" "$INTAKE_BRANCH" --force-with-lease
  else
    git push "$ORIGIN_REMOTE" "$INTAKE_BRANCH"
  fi
  log "main now $(short "$INTAKE_BRANCH") == upstream/main"
  report_cli_versions
  if git merge-base --is-ancestor "$INTAKE_BRANCH" "$PRODUCT_BRANCH" 2>/dev/null; then
    log "product branch $PRODUCT_BRANCH already contains this main tip."
  else
    log "next (optional): fold main into product → ./bin/update-from-xai.sh --merge-product --apply"
    log "  or: ./bin/sync-upstream-intake.sh merge-powergrok --apply"
  fi
  if [[ -n "$current" && "$current" != "$INTAKE_BRANCH" ]]; then
    git checkout "$current" || true
  fi
}

cmd_merge_powergrok() {
  cmd_ensure_remotes
  do_fetch
  ensure_clean_enough

  git show-ref --verify --quiet "refs/heads/$PRODUCT_BRANCH" || die "missing $PRODUCT_BRANCH"
  git show-ref --verify --quiet "refs/heads/$INTAKE_BRANCH" || die "missing $INTAKE_BRANCH"

  if git merge-base --is-ancestor "$INTAKE_BRANCH" "$PRODUCT_BRANCH" 2>/dev/null; then
    log "main already fully merged into powergrok. nothing to do."
    return 0
  fi

  local allow=()
  local msg
  msg="chore(intake): merge upstream $(git rev-parse --short "$INTAKE_BRANCH") into powergrok"

  if ! has_merge_base "$INTAKE_BRANCH" "$PRODUCT_BRANCH"; then
    if [[ "$ALLOW_UNRELATED" != "1" ]]; then
      die "no merge-base between main and powergrok; re-run with --allow-unrelated (see UPSTREAM_INTAKE.md)"
    fi
    allow=(--allow-unrelated-histories)
    msg="chore(intake): merge upstream root $(git rev-parse --short "$INTAKE_BRANCH") into powergrok (unrelated histories)"
  fi

  log "plan:"
  log "  git checkout $PRODUCT_BRANCH"
  log "  git merge $INTAKE_BRANCH ${allow[*]:-} -m \"$msg\""
  log "  # resolve conflicts if needed, then: git push $ORIGIN_REMOTE $PRODUCT_BRANCH"

  if [[ "$APPLY" != "1" ]]; then
    log "dry-run only. re-run with --apply to execute."
    return 0
  fi

  cmd_backup >/dev/null
  git checkout "$PRODUCT_BRANCH"
  if git merge "$INTAKE_BRANCH" ${allow[@]+"${allow[@]}"} -m "$msg"; then
    log "merge completed cleanly at $(short "$PRODUCT_BRANCH")"
    log "push when ready: git push $ORIGIN_REMOTE $PRODUCT_BRANCH"
  else
    err "merge stopped with conflicts. fix files, then:"
    err "  git add -A && git merge --continue"
    err "or abort: git merge --abort"
    err "backup refs still point at pre-merge tip (see git branch | rg backup/powergrok)"
    exit 2
  fi
}

cmd_verify() {
  cmd_ensure_remotes
  do_fetch

  local ok=1
  local up main prod
  up="$(full_sha "$UPSTREAM_REMOTE/$UPSTREAM_REF")"
  main="$(full_sha "$INTAKE_BRANCH" || true)"
  prod="$(full_sha "$PRODUCT_BRANCH" || true)"

  if [[ "$main" == "$up" ]]; then
    log "OK  main == upstream/main ($(short "$main"))"
  else
    log "FAIL main != upstream/main"
    ok=0
  fi

  if [[ -n "$main" && -n "$prod" ]] && git merge-base --is-ancestor "$INTAKE_BRANCH" "$PRODUCT_BRANCH" 2>/dev/null; then
    log "OK  main is ancestor of powergrok"
  else
    log "FAIL main not fully in powergrok (pending merge or root jump)"
    ok=0
  fi

  if urls_match "$(remote_url "$UPSTREAM_REMOTE")" "$UPSTREAM_URL_EXPECTED"; then
    log "OK  upstream remote URL"
  else
    log "FAIL upstream remote URL is $(remote_url "$UPSTREAM_REMOTE")"
    ok=0
  fi

  if urls_match "$(remote_url "$ORIGIN_REMOTE")" "$ORIGIN_URL_EXPECTED"; then
    log "OK  origin remote URL"
  else
    log "FAIL origin remote URL is $(remote_url "$ORIGIN_REMOTE")"
    ok=0
  fi

  if [[ -f crates/codegen/xai-grok-hooks/src/runner/http.rs ]]; then
    if rg -q 'Policy::none' crates/codegen/xai-grok-hooks/src/runner/http.rs; then
      log "OK  hooks runner has Policy::none (upstream SSRF redirect mitigation present)"
    else
      log "WARN hooks runner lacks Policy::none (likely still on pre-0.2.102 tree)"
    fi
  fi

  if [[ "$ok" == "1" ]]; then
    log "verify: PASS"
    return 0
  fi
  log "verify: FAIL"
  return 1
}

cmd_full() {
  if [[ "$APPLY" == "1" ]]; then
    cmd_backup
    cmd_intake_main
    # After root jump, auto-enable unrelated if needed
    if ! has_merge_base "$INTAKE_BRANCH" "$PRODUCT_BRANCH" 2>/dev/null; then
      ALLOW_UNRELATED=1
      log "auto-enabling --allow-unrelated for product merge (no merge-base)"
    fi
    cmd_merge_powergrok
    cmd_verify || true
  else
    log "full dry-run: would backup, intake-main, merge-powergrok"
    cmd_status
    log ""
    log "re-run: $0 full --apply"
    log "if status reported NO merge-base, also pass --i-approve-force-main (and expect unrelated product merge)"
  fi
}

# --- arg parse ---
APPLY=0
APPROVE_FORCE_MAIN=0
PUSH_BACKUP=0
ALLOW_UNRELATED=0
NO_FETCH=0
CMD=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --apply) APPLY=1; shift ;;
    --i-approve-force-main) APPROVE_FORCE_MAIN=1; shift ;;
    --push-backup) PUSH_BACKUP=1; shift ;;
    --allow-unrelated) ALLOW_UNRELATED=1; shift ;;
    --no-fetch) NO_FETCH=1; shift ;;
    status|backup|ensure-remotes|intake-main|merge-powergrok|verify|full)
      CMD="$1"; shift ;;
    *)
      die "unknown arg: $1 (see --help)"
      ;;
  esac
done

[[ -n "$CMD" ]] || { usage; exit 1; }

require_git_repo

case "$CMD" in
  status) cmd_status ;;
  backup) cmd_backup ;;
  ensure-remotes) cmd_ensure_remotes ;;
  intake-main) cmd_intake_main ;;
  merge-powergrok) cmd_merge_powergrok ;;
  verify) cmd_verify ;;
  full) cmd_full ;;
esac
