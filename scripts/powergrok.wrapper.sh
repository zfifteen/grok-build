#!/usr/bin/env bash
# powergrok-wrapper — marker for uninstall detection
#
# Installed to $prefix/bin/powergrok by scripts/install-powergrok.sh.
# Contract: docs/powergrok/BUILD_PLAN.md §5.2 / GitHub issue #6.
#
# Responsibilities:
#   1. Export GROK_HOME (default $HOME/.powergrok) before exec.
#   2. Ensure home dir + seed config.toml if missing (auto_update = false only).
#   3. exec the real binary whose basename is powergrok (G1 argv0; no exec -a).
#   4. Refuse missing binary (127) and accidental official ~/.grok home (2).
#
# PHASE 1 SCAFFOLD — signatures + logic comments only. No executable body yet.
# Implementation lands in Phase 3.

set -euo pipefail

# --- defaults (overridable by install-time rewrite or env) --------------------

# POWERGROK_LIB: directory containing the real named binary.
# Logic: default $HOME/.local/lib/powergrok; install script may bake absolute path.
: "${POWERGROK_LIB:=${HOME}/.local/lib/powergrok}"

# REAL_BIN / POWERGROK_BIN: path to the real binary file named "powergrok".
# Logic: basename MUST be powergrok so process argv0 supports project isolation.
: "${POWERGROK_BIN:=${POWERGROK_LIB}/powergrok}"
REAL_BIN="${POWERGROK_BIN}"

# GROK_HOME resolution:
#   POWERGROK_HOME wins if set, else existing GROK_HOME, else $HOME/.powergrok.
# Logic: export after resolve so child process inherits Power Grok home only.
resolve_grok_home() {
  # PHASE 1: describe only
  # 1. If POWERGROK_HOME is non-empty → use it.
  # 2. Else if GROK_HOME is non-empty → use it (caller override).
  # 3. Else → $HOME/.powergrok.
  # 4. export GROK_HOME to the resolved value.
  :
}

# Refuse GROK_HOME that realpath-equals official ~/.grok unless escape hatch set.
# Logic: compare physical paths of GROK_HOME and $HOME/.grok when both exist.
# Exit 2 with clear stderr if equal and POWERGROK_ALLOW_OFFICIAL_HOME != 1.
# Missing dirs: skip comparison (no false positive).
refuse_official_home_if_collision() {
  # PHASE 1: describe only
  :
}

# Ensure $GROK_HOME exists; create seed config.toml only if absent.
# Seed content (authoritative BUILD_PLAN §8.3):
#   # Powergrok — local source-built install.
#   [cli]
#   auto_update = false
# Never overwrite an existing config.toml. No telemetry keys.
ensure_home_and_seed_config() {
  # PHASE 1: describe only
  :
}

# Validate real binary is present and executable; exit 127 with rebuild hint.
require_real_binary() {
  # PHASE 1: describe only
  # If ! -x "$REAL_BIN": stderr two lines (missing path + rebuild pointer), exit 127.
  :
}

# Optional: refuse if REAL_BIN realpath equals official grok realpath.
# Logic: if command -v grok and both resolve, compare; exit 2 on match.
# Best-effort; skip if grok missing or realpath unavailable.
refuse_if_binary_is_official_grok() {
  # PHASE 1: describe only
  :
}

# Main entry: resolve home → guards → seed → exec REAL_BIN "$@".
# Never returns on success (exec replaces process). No exec -a.
main() {
  # PHASE 1: describe only
  # resolve_grok_home
  # require_real_binary
  # refuse_official_home_if_collision
  # refuse_if_binary_is_official_grok  (optional best-effort)
  # ensure_home_and_seed_config
  # exec "$REAL_BIN" "$@"
  :
}

# When this file is executed (not sourced), run main.
if [[ "${BASH_SOURCE[0]:-}" == "${0}" ]]; then
  main "$@"
fi
