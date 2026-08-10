#!/usr/bin/env bash
# Operator-facing shortcut: pull xAI (xai-org/grok-build) into this fork's main.
#
# Under the hood this calls bin/sync-upstream-intake.sh (full safety model:
# remotes check, dry-run default, backup before rewrite, no force on powergrok).
#
# Examples:
#   ./bin/update-from-xai.sh
#       Fetch + print lag (dry-run plan for main).
#
#   ./bin/update-from-xai.sh --apply
#       Align local main to upstream/main and push origin/main.
#       Does not merge into powergrok.
#
#   ./bin/update-from-xai.sh --full --apply
#       backup → intake main → merge main into powergrok.
#       You still push powergrok yourself if the merge is clean:
#         git push origin powergrok
#
#   ./bin/update-from-xai.sh --merge-product --apply
#       Only merge already-updated main into powergrok.
#
# Force-push origin/main (root republish) still requires the explicit flag:
#   ./bin/update-from-xai.sh --apply --i-approve-force-main
#
# Docs: docs/powergrok/UPSTREAM_INTAKE.md
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INTAKE="$ROOT/bin/sync-upstream-intake.sh"

[[ -x "$INTAKE" || -f "$INTAKE" ]] || {
  printf 'error: missing %s\n' "$INTAKE" >&2
  exit 1
}

APPLY=0
FULL=0
MERGE_PRODUCT=0
STATUS_ONLY=0
PASSTHRU=()

usage() {
  cat <<'EOF'
Usage: bin/update-from-xai.sh [options]

Default (no flags): status report + dry-run plan for aligning main to xAI.

Options:
  --apply                   Execute (otherwise dry-run / status only)
  --full                    After main intake, also merge main → powergrok
  --merge-product           Only merge main → powergrok (skip main intake)
  --status                  Force status-only (same as no flags)
  --push-backup             Push backup/powergrok-* to origin when created
  --i-approve-force-main    Allow origin main --force-with-lease when required
  --allow-unrelated         Allow unrelated-histories product merge
  --no-fetch                Skip network fetch
  -h, --help                Show this help

Typical daily path (fork main only):
  ./bin/update-from-xai.sh              # look
  ./bin/update-from-xai.sh --apply      # do it

Product fold-in after main is current:
  ./bin/update-from-xai.sh --full --apply
  # or, if main was already updated:
  ./bin/update-from-xai.sh --merge-product --apply
  git push origin powergrok
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --apply) APPLY=1; shift ;;
    --full) FULL=1; shift ;;
    --merge-product) MERGE_PRODUCT=1; shift ;;
    --status) STATUS_ONLY=1; shift ;;
    --push-backup|--i-approve-force-main|--allow-unrelated|--no-fetch)
      PASSTHRU+=("$1"); shift ;;
    *)
      printf 'error: unknown arg: %s (see --help)\n' "$1" >&2
      exit 1
      ;;
  esac
done

if [[ "$FULL" == "1" && "$MERGE_PRODUCT" == "1" ]]; then
  printf 'error: use either --full or --merge-product, not both\n' >&2
  exit 1
fi

run_intake() {
  # shellcheck disable=SC2086
  bash "$INTAKE" "$@" ${PASSTHRU[@]+"${PASSTHRU[@]}"}
}

if [[ "$STATUS_ONLY" == "1" || ( "$APPLY" == "0" && "$FULL" == "0" && "$MERGE_PRODUCT" == "0" ) ]]; then
  run_intake status
  if [[ "$APPLY" == "0" && "$FULL" == "0" && "$MERGE_PRODUCT" == "0" ]]; then
    printf '\n'
    printf 'dry-run tip: to update fork main from xAI, re-run:\n'
    printf '  %s --apply\n' "$0"
  fi
  exit 0
fi

if [[ "$MERGE_PRODUCT" == "1" ]]; then
  if [[ "$APPLY" == "1" ]]; then
    run_intake merge-powergrok --apply
  else
    run_intake merge-powergrok
  fi
  exit $?
fi

if [[ "$FULL" == "1" ]]; then
  if [[ "$APPLY" == "1" ]]; then
    run_intake full --apply
  else
    run_intake full
  fi
  exit $?
fi

# Default apply path: main only (what you asked for as the self-serve script).
if [[ "$APPLY" == "1" ]]; then
  run_intake intake-main --apply
else
  run_intake intake-main
fi
