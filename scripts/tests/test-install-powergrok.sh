#!/usr/bin/env bash
# Smoke / unit tests for install-powergrok.sh (issue #6).
# PHASE 1 SCAFFOLD — test names + expected assertions in comments only.
#
# Run (after Phase 3): scripts/tests/test-install-powergrok.sh
# Uses a temp PREFIX and --no-build with a fake artifact where possible.

set -euo pipefail

# test_usage_exits_zero
#   ./install-powergrok.sh --help → exit 0, mentions --prefix and powergrok

# test_refuse_grok_public_name
#   Internal: if PUBLIC_COMMAND forced to grok → die (or simulate via path check)

# test_dry_run_no_writes
#   --dry-run --no-build with missing artifact may still fail at assert —
#   dry-run before build should not create PREFIX files when artifact missing
#   OR dry-run with fake artifact creates nothing on disk

# test_uninstall_removes_wrapper_and_lib_only
#   Install into temp prefix with fake binary + wrapper; uninstall;
#   wrapper+lib gone; grok_home remains unless --purge-home

# test_seed_config_create_once
#   Missing config → seed auto_update=false; second run does not clobber edits

# test_wrapper_marker_present
#   Installed wrapper contains "# powergrok-wrapper"

# test_real_binary_basename_powergrok
#   Installed lib binary basename is powergrok

# test_version_file_written
#   VERSION contains git= or built_at=

main() {
  # PHASE 1: no tests executed yet
  echo "test-install-powergrok: scaffold only (Phase 1) — no assertions run" >&2
  exit 0
}

main "$@"
