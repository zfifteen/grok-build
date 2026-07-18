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
#   4. Refuse missing binary (127) and accidental official ~/.grok home (2),
#      even when the directory does not exist yet.

set -euo pipefail

# BEGIN_POWERGROK_INSTALL_DEFAULTS
# install-powergrok.sh replaces the next two assignments with absolute paths.
POWERGROK_LIB_DEFAULT="${HOME}/.local/lib/powergrok"
POWERGROK_HOME_DEFAULT="${HOME}/.powergrok"
# END_POWERGROK_INSTALL_DEFAULTS

: "${POWERGROK_LIB:=${POWERGROK_LIB_DEFAULT}}"
: "${POWERGROK_BIN:=${POWERGROK_LIB}/powergrok}"
REAL_BIN="${POWERGROK_BIN}"

canonical_path() {
  # Absolute + normalized; does not require the path to exist.
  python3 -c 'import os,sys; print(os.path.normpath(os.path.abspath(os.path.expanduser(sys.argv[1]))))' "$1"
}

resolve_grok_home() {
  if [[ -n "${POWERGROK_HOME:-}" ]]; then
    export GROK_HOME="${POWERGROK_HOME}"
  elif [[ -n "${GROK_HOME:-}" ]]; then
    export GROK_HOME
  else
    export GROK_HOME="${POWERGROK_HOME_DEFAULT}"
  fi
}

refuse_official_home_if_collision() {
  if [[ "${POWERGROK_ALLOW_OFFICIAL_HOME:-}" == "1" ]]; then
    return 0
  fi
  local official candidate
  official="$(canonical_path "${HOME}/.grok")"
  candidate="$(canonical_path "${GROK_HOME}")"
  if [[ "${candidate}" == "${official}" ]]; then
    echo "powergrok: GROK_HOME resolves to official ~/.grok; aborting" >&2
    echo "powergrok: set POWERGROK_HOME to a Power Grok home, or POWERGROK_ALLOW_OFFICIAL_HOME=1 only if intentional" >&2
    exit 2
  fi
  # If both exist, also compare physical paths (symlink → official).
  if [[ -e "${GROK_HOME}" && -e "${HOME}/.grok" ]]; then
    local home_phys official_phys
    if command -v realpath >/dev/null 2>&1; then
      home_phys="$(realpath "${GROK_HOME}" 2>/dev/null || true)"
      official_phys="$(realpath "${HOME}/.grok" 2>/dev/null || true)"
      if [[ -n "${home_phys}" && -n "${official_phys}" && "${home_phys}" == "${official_phys}" ]]; then
        echo "powergrok: GROK_HOME realpath equals official ~/.grok; aborting" >&2
        exit 2
      fi
    else
      home_phys="$(cd "${GROK_HOME}" && pwd -P 2>/dev/null)" || true
      official_phys="$(cd "${HOME}/.grok" && pwd -P 2>/dev/null)" || true
      if [[ -n "${home_phys}" && -n "${official_phys}" && "${home_phys}" == "${official_phys}" ]]; then
        echo "powergrok: GROK_HOME realpath equals official ~/.grok; aborting" >&2
        exit 2
      fi
    fi
  fi
}

ensure_home_and_seed_config() {
  mkdir -p "${GROK_HOME}"
  local config="${GROK_HOME}/config.toml"
  if [[ -f "${config}" ]]; then
    return 0
  fi
  cat >"${config}" <<'EOF'
# Powergrok — local source-built install.
[cli]
auto_update = false
EOF
}

require_real_binary() {
  if [[ -x "${REAL_BIN}" ]]; then
    return 0
  fi
  echo "powergrok: missing binary at ${REAL_BIN}" >&2
  echo "powergrok: build and install: ./scripts/install-powergrok.sh" >&2
  echo "powergrok: see docs/powergrok/BUILD_PLAN.md" >&2
  exit 127
}

refuse_if_binary_is_official_grok() {
  local grok_path
  grok_path="$(command -v grok 2>/dev/null || true)"
  if [[ -z "${grok_path}" || ! -e "${REAL_BIN}" || ! -e "${grok_path}" ]]; then
    return 0
  fi
  local real_phys grok_phys
  real_phys="$(cd "$(dirname "${REAL_BIN}")" && pwd -P)/$(basename "${REAL_BIN}")"
  if command -v realpath >/dev/null 2>&1; then
    real_phys="$(realpath "${REAL_BIN}" 2>/dev/null || echo "${real_phys}")"
    grok_phys="$(realpath "${grok_path}" 2>/dev/null || true)"
  else
    grok_phys="$(cd "$(dirname "${grok_path}")" && pwd -P)/$(basename "${grok_path}")"
  fi
  if [[ -n "${grok_phys}" && "${real_phys}" == "${grok_phys}" ]]; then
    echo "powergrok: real binary path equals official grok; aborting" >&2
    exit 2
  fi
}

main() {
  resolve_grok_home
  # Refuse official home BEFORE mkdir/seed (even if ~/.grok does not exist yet).
  refuse_official_home_if_collision
  require_real_binary
  refuse_if_binary_is_official_grok
  ensure_home_and_seed_config
  # Basename of REAL_BIN is "powergrok" → argv0 contract (G1). No exec -a.
  exec "${REAL_BIN}" "$@"
}

if [[ "${BASH_SOURCE[0]:-}" == "${0}" ]]; then
  main "$@"
fi
