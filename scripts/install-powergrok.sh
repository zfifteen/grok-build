#!/usr/bin/env bash
# install-powergrok.sh — one-command Power Grok install (issue #6 / BUILD_PLAN §9)
#
# Primary operator path from a clean checkout of the powergrok branch:
#   ./scripts/install-powergrok.sh
#
# Non-goals: isolation engine (#4), engine --version marketing patch,
# Windows/Homebrew packaging, force-push, touching official grok paths.

set -euo pipefail

readonly PUBLIC_COMMAND_NAME="powergrok"
readonly FORBIDDEN_PUBLIC_NAME="grok"
readonly CARGO_PACKAGE="xai-grok-pager-bin"
readonly CARGO_FEATURES="powergrok"
readonly ARTIFACT_REL="target/release/xai-grok-pager"
readonly WRAPPER_MARKER="# powergrok-wrapper"
readonly WRAPPER_SOURCE_REL="scripts/powergrok.wrapper.sh"

DO_BUILD=1
DO_INSTALL_COMPLETIONS=1
DRY_RUN=0
UNINSTALL=0
PURGE_HOME=0
SHOW_STATUS=0
DO_ROLLBACK=0
CHECK_FRESHNESS=0
PREFIX="${HOME}/.local"
GROK_HOME_OPT="${HOME}/.powergrok"
REPO_ROOT=""

BIN_DIR=""
LIB_DIR=""
WRAPPER_PATH=""
REAL_BIN_PATH=""
ARTIFACT=""
WRAPPER_SRC=""
VERSION_PATH=""
PREV_BIN_PATH=""
OFFICIAL_HOME=""

usage() {
  cat <<'EOF'
Usage: scripts/install-powergrok.sh [options]

One-command Power Grok install + source-build lifecycle (side-by-side with official grok).

Install / upgrade (default):
  ./scripts/install-powergrok.sh
  # Re-run after `git pull origin powergrok` to upgrade; keeps wrapper/home;
  # backs up prior binary to powergrok.prev; rewrites VERSION.

Lifecycle:
  --status                Show install identity (VERSION, paths, auto_update) and exit
  --check-freshness       With --status: best-effort compare HEAD vs origin/powergrok
                          (advisory only — never auto-downloads or installs)
  --rollback              Restore lib binary from powergrok.prev (no cargo, no official updater)
  --build / --no-build    Build release artifact before install (default: build)
  --prefix DIR            Install prefix (default: $HOME/.local); absolute
  --grok-home DIR         Power Grok home / GROK_HOME (default: $HOME/.powergrok)
  --dry-run               Print actions; do not write or build
  --uninstall             Remove wrapper + lib/powergrok (not repo project data)
  --purge-home            With --uninstall, also remove --grok-home
  --install-completions / --no-install-completions
  -h, --help              Show this help

Layout after install:
  $prefix/bin/powergrok              wrapper (sets GROK_HOME, execs named binary)
  $prefix/lib/powergrok/powergrok    real binary (basename = argv0)
  $prefix/lib/powergrok/powergrok.prev  previous binary after upgrade (rollback source)
  $prefix/lib/powergrok/VERSION      git SHA + build time
  $grok-home/config.toml             seeded once: auto_update = false

Upgrade runbook: docs/powergrok/LIFECYCLE.md
Contract:        docs/powergrok/BUILD_PLAN.md §5, §8, §9
Issues:          #6 install, #14 lifecycle
EOF
}

die() {
  echo "install-powergrok: $*" >&2
  exit 1
}

log() {
  if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "[dry-run] $*"
  else
    echo "install-powergrok: $*"
  fi
}

run_or_dry() {
  if [[ "${DRY_RUN}" -eq 1 ]]; then
    # shellcheck disable=SC2145
    log "would run: $*"
    return 0
  fi
  "$@"
}

# Absolute, normalized path (does not require the path to exist).
canonical_path() {
  local input="$1"
  python3 -c 'import os,sys; print(os.path.normpath(os.path.abspath(os.path.expanduser(sys.argv[1]))))' "${input}"
}

escape_double_quotes() {
  # Escape \ and " for inclusion inside double-quoted shell assignments.
  python3 -c 'import sys; s=sys.argv[1]; print(s.replace("\\", "\\\\").replace("\"", "\\\""))' "$1"
}

is_official_home_path() {
  # True if path canonically equals $HOME/.grok (even if neither exists yet).
  local candidate="$1"
  local official_c candidate_c
  official_c="$(canonical_path "${HOME}/.grok")"
  candidate_c="$(canonical_path "${candidate}")"
  [[ "${candidate_c}" == "${official_c}" ]]
}

assert_safe_install_path() {
  local label="$1" path="$2"
  [[ -n "${path}" ]] || die "${label} is empty"
  [[ "${path}" != "/" ]] || die "${label} must not be filesystem root"
  # Refuse bare relative leftovers (canonical_path should already absolute them).
  [[ "${path}" == /* ]] || die "${label} must be absolute after normalization (got: ${path})"
}

resolve_repo_root() {
  local script_dir root candidate
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  candidate="$(cd "${script_dir}/.." && pwd -P)"
  if [[ -f "${candidate}/Cargo.toml" && -d "${candidate}/crates/codegen/xai-grok-pager-bin" ]]; then
    REPO_ROOT="${candidate}"
    return 0
  fi
  root="${candidate}"
  while [[ "${root}" != "/" ]]; do
    if [[ -f "${root}/Cargo.toml" && -d "${root}/crates/codegen/xai-grok-pager-bin" ]]; then
      REPO_ROOT="${root}"
      return 0
    fi
    root="$(cd "${root}/.." && pwd -P)"
  done
  die "could not find powergrok repo root (Cargo.toml + xai-grok-pager-bin) from ${script_dir}"
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h|--help)
        usage
        exit 0
        ;;
      --build)
        DO_BUILD=1
        shift
        ;;
      --no-build)
        DO_BUILD=0
        shift
        ;;
      --prefix)
        [[ $# -ge 2 ]] || die "--prefix requires a directory"
        PREFIX="$2"
        shift 2
        ;;
      --grok-home)
        [[ $# -ge 2 ]] || die "--grok-home requires a directory"
        GROK_HOME_OPT="$2"
        shift 2
        ;;
      --dry-run)
        DRY_RUN=1
        shift
        ;;
      --uninstall)
        UNINSTALL=1
        shift
        ;;
      --status)
        SHOW_STATUS=1
        shift
        ;;
      --rollback)
        DO_ROLLBACK=1
        shift
        ;;
      --check-freshness)
        CHECK_FRESHNESS=1
        shift
        ;;
      --purge-home)
        PURGE_HOME=1
        shift
        ;;
      --install-completions)
        DO_INSTALL_COMPLETIONS=1
        shift
        ;;
      --no-install-completions)
        DO_INSTALL_COMPLETIONS=0
        shift
        ;;
      *)
        die "unknown option: $1 (try --help)"
        ;;
    esac
  done

  if [[ "${PURGE_HOME}" -eq 1 && "${UNINSTALL}" -ne 1 ]]; then
    die "--purge-home requires --uninstall"
  fi
  if [[ "${CHECK_FRESHNESS}" -eq 1 && "${SHOW_STATUS}" -ne 1 ]]; then
    # Allow standalone freshness as status+freshness.
    SHOW_STATUS=1
  fi
  local modes=0
  [[ "${UNINSTALL}" -eq 1 ]] && modes=$((modes + 1))
  [[ "${SHOW_STATUS}" -eq 1 ]] && modes=$((modes + 1))
  [[ "${DO_ROLLBACK}" -eq 1 ]] && modes=$((modes + 1))
  if [[ "${modes}" -gt 1 ]]; then
    die "use only one of --status / --rollback / --uninstall (with optional install flags)"
  fi
}

derive_paths() {
  # Expand ~ and force absolute/normalized paths so baked wrapper defaults
  # do not depend on the caller's cwd (Codex + Hermes review).
  PREFIX="$(canonical_path "${PREFIX}")"
  GROK_HOME_OPT="$(canonical_path "${GROK_HOME_OPT}")"
  OFFICIAL_HOME="$(canonical_path "${HOME}/.grok")"

  assert_safe_install_path "--prefix" "${PREFIX}"
  assert_safe_install_path "--grok-home" "${GROK_HOME_OPT}"

  BIN_DIR="${PREFIX}/bin"
  LIB_DIR="${PREFIX}/lib/powergrok"
  WRAPPER_PATH="${BIN_DIR}/${PUBLIC_COMMAND_NAME}"
  REAL_BIN_PATH="${LIB_DIR}/${PUBLIC_COMMAND_NAME}"
  VERSION_PATH="${LIB_DIR}/VERSION"
  PREV_BIN_PATH="${LIB_DIR}/powergrok.prev"
  # Test override: POWERGROK_INSTALL_ARTIFACT=/abs/path/to/fake-bin
  if [[ -n "${POWERGROK_INSTALL_ARTIFACT:-}" ]]; then
    ARTIFACT="$(canonical_path "${POWERGROK_INSTALL_ARTIFACT}")"
  else
    ARTIFACT="${REPO_ROOT}/${ARTIFACT_REL}"
  fi
  WRAPPER_SRC="${REPO_ROOT}/${WRAPPER_SOURCE_REL}"
}

assert_public_name_is_powergrok() {
  if [[ "${PUBLIC_COMMAND_NAME}" == "${FORBIDDEN_PUBLIC_NAME}" ]]; then
    die "refusing to install public command name '${FORBIDDEN_PUBLIC_NAME}'"
  fi
  local base
  base="$(basename "${WRAPPER_PATH}")"
  if [[ "${base}" != "${PUBLIC_COMMAND_NAME}" ]]; then
    die "wrapper basename must be '${PUBLIC_COMMAND_NAME}', got '${base}'"
  fi
  base="$(basename "${REAL_BIN_PATH}")"
  if [[ "${base}" != "${PUBLIC_COMMAND_NAME}" ]]; then
    die "real binary basename must be '${PUBLIC_COMMAND_NAME}', got '${base}'"
  fi
}

_paths_collide() {
  # Return 0 if two existing paths are the same physical file.
  local a="$1" b="$2"
  [[ -e "${a}" && -e "${b}" ]] || return 1
  if command -v realpath >/dev/null 2>&1; then
    [[ "$(realpath "${a}")" == "$(realpath "${b}")" ]]
    return
  fi
  local ia ib
  ia="$(ls -di "${a}" | awk '{print $1}')"
  ib="$(ls -di "${b}" | awk '{print $1}')"
  [[ -n "${ia}" && "${ia}" == "${ib}" ]]
}

assert_does_not_clobber_official_grok() {
  local grok_path
  grok_path="$(command -v grok 2>/dev/null || true)"
  if [[ -n "${grok_path}" ]]; then
    if _paths_collide "${WRAPPER_PATH}" "${grok_path}"; then
      die "install would overwrite official grok at ${grok_path}"
    fi
    if _paths_collide "${REAL_BIN_PATH}" "${grok_path}"; then
      die "real binary path collides with official grok at ${grok_path}"
    fi
    if [[ "$(basename "${grok_path}")" == "${FORBIDDEN_PUBLIC_NAME}" \
       && "${WRAPPER_PATH}" == "${grok_path}" ]]; then
      die "refusing to replace official grok PATH entry"
    fi
  fi
}

assert_grok_home_is_not_official() {
  if [[ "${POWERGROK_ALLOW_OFFICIAL_HOME:-}" == "1" ]]; then
    log "warning: POWERGROK_ALLOW_OFFICIAL_HOME=1 — allowing official home path"
    return 0
  fi
  if is_official_home_path "${GROK_HOME_OPT}"; then
    die "--grok-home resolves to official ~/.grok (${GROK_HOME_OPT}); refuse (set POWERGROK_ALLOW_OFFICIAL_HOME=1 only if intentional)"
  fi
}

require_rust_toolchain() {
  command -v cargo >/dev/null 2>&1 || die "cargo not found; install rustup (see rust-toolchain.toml)"
  command -v rustc >/dev/null 2>&1 || die "rustc not found; install rustup (see rust-toolchain.toml)"
}

build_release_artifact() {
  if [[ "${DO_BUILD}" -ne 1 ]]; then
    log "skipping build (--no-build)"
    return 0
  fi
  if [[ -n "${POWERGROK_INSTALL_ARTIFACT:-}" ]]; then
    log "skipping build (POWERGROK_INSTALL_ARTIFACT override)"
    return 0
  fi
  require_rust_toolchain
  log "building ${CARGO_PACKAGE} --release --features ${CARGO_FEATURES}"
  if [[ "${DRY_RUN}" -eq 1 ]]; then
    log "would run: cargo build -p ${CARGO_PACKAGE} --release --features ${CARGO_FEATURES} (cwd=${REPO_ROOT})"
    return 0
  fi
  (
    cd "${REPO_ROOT}"
    cargo build -p "${CARGO_PACKAGE}" --release --features "${CARGO_FEATURES}"
  ) || die "cargo build failed"
}

assert_artifact_present() {
  if [[ "${DRY_RUN}" -eq 1 && "${DO_BUILD}" -eq 1 && -z "${POWERGROK_INSTALL_ARTIFACT:-}" ]]; then
    log "skipping artifact check in dry-run with build (artifact would be produced)"
    return 0
  fi
  if [[ ! -f "${ARTIFACT}" ]]; then
    die "missing artifact ${ARTIFACT}; run without --no-build or build first"
  fi
  if [[ ! -x "${ARTIFACT}" ]]; then
    log "warning: artifact exists but is not executable yet: ${ARTIFACT}"
  fi
}

install_real_binary() {
  log "install real binary → ${REAL_BIN_PATH}"
  if [[ "${DRY_RUN}" -eq 1 ]]; then
    log "would mkdir -p ${LIB_DIR} ${BIN_DIR}"
    log "would backup existing binary to ${PREV_BIN_PATH} if present"
    log "would install -m 755 ${ARTIFACT} ${REAL_BIN_PATH}"
    return 0
  fi
  mkdir -p "${LIB_DIR}" "${BIN_DIR}"
  if [[ -e "${REAL_BIN_PATH}" ]]; then
    cp -p "${REAL_BIN_PATH}" "${PREV_BIN_PATH}"
    log "backed up previous binary to ${PREV_BIN_PATH}"
  fi
  install -m 755 "${ARTIFACT}" "${REAL_BIN_PATH}"
  if [[ "$(basename "${REAL_BIN_PATH}")" != "${PUBLIC_COMMAND_NAME}" ]]; then
    die "internal error: installed basename is not ${PUBLIC_COMMAND_NAME}"
  fi
}

# Run the real binary under Power Grok home so the engine never touches ~/.grok.
run_installed_binary_isolated() {
  env -u POWERGROK_HOME \
    GROK_HOME="${GROK_HOME_OPT}" \
    HOME="${HOME}" \
    PATH="${PATH}" \
    "${REAL_BIN_PATH}" "$@"
}

write_version_file() {
  log "write VERSION → ${VERSION_PATH}"
  if [[ "${DRY_RUN}" -eq 1 ]]; then
    log "would write git SHA, built_at, features to ${VERSION_PATH}"
    return 0
  fi
  local git_sha git_branch built_at version_line
  git_sha="$(git -C "${REPO_ROOT}" rev-parse HEAD 2>/dev/null || echo "unknown")"
  git_branch="$(git -C "${REPO_ROOT}" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")"
  built_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  version_line=""
  if [[ -x "${REAL_BIN_PATH}" ]]; then
    # Critical: always set GROK_HOME so --version cannot initialize ~/.grok.
    version_line="$(run_installed_binary_isolated --version 2>/dev/null | head -n 1 || true)"
  fi
  {
    echo "git=${git_sha}"
    echo "branch=${git_branch}"
    echo "built_at=${built_at}"
    echo "features=${CARGO_FEATURES}"
    echo "artifact=${ARTIFACT_REL}"
    if [[ -n "${version_line}" ]]; then
      echo "version_line=${version_line}"
    fi
  } >"${VERSION_PATH}"
}

install_wrapper() {
  log "install wrapper → ${WRAPPER_PATH}"
  [[ -f "${WRAPPER_SRC}" ]] || die "missing wrapper source ${WRAPPER_SRC}"

  if [[ "${DRY_RUN}" -eq 1 ]]; then
    log "would install wrapper from ${WRAPPER_SRC} with POWERGROK_LIB=${LIB_DIR} default home=${GROK_HOME_OPT}"
    return 0
  fi

  mkdir -p "${BIN_DIR}"
  local tmp lib_esc home_esc
  tmp="$(mktemp)"
  lib_esc="$(escape_double_quotes "${LIB_DIR}")"
  home_esc="$(escape_double_quotes "${GROK_HOME_OPT}")"

  # Bake absolute defaults; quote-escape paths so awk -v cannot break on ".
  awk -v lib="${lib_esc}" -v home="${home_esc}" '
    BEGIN { in_block = 0 }
    /# BEGIN_POWERGROK_INSTALL_DEFAULTS/ {
      print
      print "# install-powergrok.sh baked absolute defaults for this prefix."
      print "POWERGROK_LIB_DEFAULT=\"" lib "\""
      print "POWERGROK_HOME_DEFAULT=\"" home "\""
      in_block = 1
      next
    }
    /# END_POWERGROK_INSTALL_DEFAULTS/ {
      in_block = 0
      print
      next
    }
    in_block == 1 { next }
    { print }
  ' "${WRAPPER_SRC}" >"${tmp}"

  if ! grep -q "${WRAPPER_MARKER}" "${tmp}"; then
    rm -f "${tmp}"
    die "wrapper source missing uninstall marker '${WRAPPER_MARKER}'"
  fi
  if ! grep -Fq "POWERGROK_LIB_DEFAULT=\"${LIB_DIR}\"" "${tmp}" \
    && ! grep -Fq "POWERGROK_LIB_DEFAULT=\"${lib_esc}\"" "${tmp}"; then
    rm -f "${tmp}"
    die "wrapper lib default bake failed"
  fi
  if ! grep -Fq "POWERGROK_HOME_DEFAULT=\"${GROK_HOME_OPT}\"" "${tmp}" \
    && ! grep -Fq "POWERGROK_HOME_DEFAULT=\"${home_esc}\"" "${tmp}"; then
    rm -f "${tmp}"
    die "wrapper home default bake failed"
  fi

  install -m 755 "${tmp}" "${WRAPPER_PATH}"
  rm -f "${tmp}"

  if ! grep -q "${WRAPPER_MARKER}" "${WRAPPER_PATH}"; then
    die "installed wrapper missing uninstall marker"
  fi
}

seed_config_if_missing() {
  local config="${GROK_HOME_OPT}/config.toml"
  log "seed config if missing → ${config}"
  if [[ "${DRY_RUN}" -eq 1 ]]; then
    log "would mkdir -p ${GROK_HOME_OPT} and write seed only if ${config} absent"
    return 0
  fi
  mkdir -p "${GROK_HOME_OPT}"
  if [[ -f "${config}" ]]; then
    log "config already exists; leaving untouched"
    return 0
  fi
  cat >"${config}" <<'EOF'
# Powergrok — local source-built install.
[cli]
auto_update = false
EOF
}

install_completions_best_effort() {
  if [[ "${DO_INSTALL_COMPLETIONS}" -ne 1 ]]; then
    log "skipping completions (--no-install-completions)"
    return 0
  fi
  local bash_dir zsh_dir bash_out zsh_out
  bash_dir="${GROK_HOME_OPT}/completions/bash"
  zsh_dir="${GROK_HOME_OPT}/completions/zsh"
  bash_out="${bash_dir}/powergrok.bash"
  zsh_out="${zsh_dir}/_powergrok"

  log "generate completions under ${GROK_HOME_OPT}/completions (best-effort)"
  if [[ "${DRY_RUN}" -eq 1 ]]; then
    log "would run: GROK_HOME=${GROK_HOME_OPT} ${REAL_BIN_PATH} completions bash > ${bash_out}"
    log "would run: GROK_HOME=${GROK_HOME_OPT} ${REAL_BIN_PATH} completions zsh > ${zsh_out}"
    return 0
  fi

  if [[ ! -x "${REAL_BIN_PATH}" ]]; then
    log "warning: binary not executable; skipping completions"
    return 0
  fi

  mkdir -p "${bash_dir}" "${zsh_dir}"
  # Critical: always set GROK_HOME so completions cannot initialize ~/.grok.
  if run_installed_binary_isolated completions bash >"${bash_out}" 2>/dev/null; then
    log "wrote ${bash_out}"
  else
    log "warning: completions bash failed (ignored)"
    rm -f "${bash_out}"
  fi
  if run_installed_binary_isolated completions zsh >"${zsh_out}" 2>/dev/null; then
    log "wrote ${zsh_out}"
  else
    log "warning: completions zsh failed (ignored)"
    rm -f "${zsh_out}"
  fi
}

print_verify_block() {
  cat <<EOF

=== Power Grok install — verify ===
Wrapper:     ${WRAPPER_PATH}
Real binary: ${REAL_BIN_PATH}
VERSION:     ${VERSION_PATH}
GROK_HOME:   ${GROK_HOME_OPT}

Smoke (also V1–V6, V9, V11):
  command -v powergrok
  test -x "${REAL_BIN_PATH}"
  test -f "${VERSION_PATH}" && cat "${VERSION_PATH}"
  grep -q 'auto_update = false' "${GROK_HOME_OPT}/config.toml"
  powergrok --help >/dev/null
  powergrok --version
  test "\$(command -v grok 2>/dev/null || true)" != "\$(command -v powergrok)"
  # argv0: real binary basename
  basename "${REAL_BIN_PATH}"   # expect: powergrok
  # install must not touch official home (when present):
  #   ls -ld ~/.grok  # mtime should be unchanged by install

EOF
  case ":${PATH}:" in
    *":${BIN_DIR}:"*) ;;
    *)
      cat <<EOF
PATH does not include ${BIN_DIR}. Add:
  export PATH="${BIN_DIR}:\$PATH"
EOF
      ;;
  esac

  cat <<EOF
Official grok was not modified.
Rebuild later:  ./scripts/install-powergrok.sh
Uninstall:      ./scripts/install-powergrok.sh --uninstall
Contract:       docs/powergrok/BUILD_PLAN.md
EOF
}

uninstall_powergrok() {
  derive_paths
  assert_safe_install_path "--prefix" "${PREFIX}"
  assert_safe_install_path "--grok-home" "${GROK_HOME_OPT}"

  log "uninstall wrapper=${WRAPPER_PATH} lib=${LIB_DIR}"

  if [[ -e "${WRAPPER_PATH}" ]]; then
    if grep -q "${WRAPPER_MARKER}" "${WRAPPER_PATH}" 2>/dev/null; then
      run_or_dry rm -f "${WRAPPER_PATH}"
      log "removed wrapper ${WRAPPER_PATH}"
    else
      die "refusing to remove ${WRAPPER_PATH}: missing '${WRAPPER_MARKER}' marker (not our wrapper)"
    fi
  else
    log "wrapper not present: ${WRAPPER_PATH}"
  fi

  if [[ -d "${LIB_DIR}" ]]; then
    # Extra safety: never rm -rf something that isn't .../lib/powergrok
    if [[ "$(basename "${LIB_DIR}")" != "powergrok" ]]; then
      die "refusing to remove unexpected lib dir: ${LIB_DIR}"
    fi
    run_or_dry rm -rf "${LIB_DIR}"
    log "removed ${LIB_DIR}"
  else
    log "lib dir not present: ${LIB_DIR}"
  fi

  if [[ "${PURGE_HOME}" -eq 1 ]]; then
    log "purge home ${GROK_HOME_OPT}"
    if is_official_home_path "${GROK_HOME_OPT}"; then
      die "refusing to purge official ~/.grok (path=${GROK_HOME_OPT})"
    fi
    # Also refuse if path is a symlink resolving to official home.
    if [[ -e "${GROK_HOME_OPT}" ]] && command -v realpath >/dev/null 2>&1; then
      if [[ "$(realpath "${GROK_HOME_OPT}")" == "$(realpath "${HOME}/.grok" 2>/dev/null || true)" ]]; then
        die "refusing to purge path that realpath-equals official ~/.grok"
      fi
    fi
    if [[ -d "${GROK_HOME_OPT}" ]]; then
      run_or_dry rm -rf "${GROK_HOME_OPT}"
    fi
  else
    log "left home intact: ${GROK_HOME_OPT} (use --purge-home to remove)"
  fi

  log "uninstall complete (repo project .powergrok/ untouched; official grok untouched)"
}

print_status() {
  derive_paths
  assert_safe_install_path "--prefix" "${PREFIX}"
  assert_safe_install_path "--grok-home" "${GROK_HOME_OPT}"

  echo "=== Power Grok install status ==="
  echo "command:      ${PUBLIC_COMMAND_NAME}"
  echo "wrapper:      ${WRAPPER_PATH}$([[ -x "${WRAPPER_PATH}" ]] && echo " (present)" || echo " (missing)")"
  echo "real_binary:  ${REAL_BIN_PATH}$([[ -x "${REAL_BIN_PATH}" ]] && echo " (present)" || echo " (missing)")"
  echo "prev_binary:  ${PREV_BIN_PATH}$([[ -e "${PREV_BIN_PATH}" ]] && echo " (present — rollback available)" || echo " (none)")"
  echo "VERSION_file: ${VERSION_PATH}"
  if [[ -f "${VERSION_PATH}" ]]; then
    sed 's/^/  /' "${VERSION_PATH}"
  else
    echo "  (missing — run ./scripts/install-powergrok.sh)"
  fi
  echo "GROK_HOME:    ${GROK_HOME_OPT}"
  local config="${GROK_HOME_OPT}/config.toml"
  if [[ -f "${config}" ]]; then
    if grep -Eq '^[[:space:]]*auto_update[[:space:]]*=[[:space:]]*false' "${config}"; then
      echo "auto_update:  false (seed intact)"
    elif grep -Eq '^[[:space:]]*auto_update[[:space:]]*=' "${config}"; then
      echo "auto_update:  $(grep -E '^[[:space:]]*auto_update[[:space:]]*=' "${config}" | head -1 | sed 's/^[[:space:]]*//')"
      echo "  warning: expected auto_update = false for source-built Power Grok"
    else
      echo "auto_update:  (not set in config.toml)"
    fi
  else
    echo "auto_update:  (no config.toml yet)"
  fi
  echo "project_dir:  .powergrok when argv0 is powergrok (D7 isolation)"
  echo "official_grok: $(command -v grok 2>/dev/null || echo '(not on PATH — OK)')"
  echo
  echo "Upgrade:   git pull origin powergrok && ./scripts/install-powergrok.sh"
  echo "Rollback:  ./scripts/install-powergrok.sh --rollback"
  echo "Runbook:   docs/powergrok/LIFECYCLE.md"

  if [[ "${CHECK_FRESHNESS}" -eq 1 ]]; then
    echo
    echo "=== Freshness (advisory only — never auto-installs) ==="
    if [[ -z "${REPO_ROOT}" ]] || [[ ! -d "${REPO_ROOT}/.git" ]]; then
      soft_resolve_repo_root || true
    fi
    if [[ -z "${REPO_ROOT}" ]] || [[ ! -d "${REPO_ROOT}/.git" ]]; then
      echo "repo: not found near installer; skip remote compare"
      return 0
    fi
    local head remote ahead
    head="$(git -C "${REPO_ROOT}" rev-parse --short HEAD 2>/dev/null || echo unknown)"
    echo "repo: ${REPO_ROOT}"
    echo "HEAD: ${head}"
    if git -C "${REPO_ROOT}" rev-parse --verify origin/powergrok >/dev/null 2>&1; then
      # Best-effort fetch; ignore network failure.
      if [[ "${DRY_RUN}" -eq 1 ]]; then
        echo "[dry-run] would: git fetch origin powergrok"
      else
        git -C "${REPO_ROOT}" fetch origin powergrok --quiet 2>/dev/null \
          || echo "fetch: skipped or failed (using existing origin/powergrok tip)"
      fi
      remote="$(git -C "${REPO_ROOT}" rev-parse --short origin/powergrok 2>/dev/null || echo unknown)"
      echo "origin/powergrok: ${remote}"
      ahead="$(git -C "${REPO_ROOT}" rev-list --count HEAD..origin/powergrok 2>/dev/null || echo "?")"
      echo "commits_behind_origin_powergrok: ${ahead}"
      if [[ "${ahead}" != "0" && "${ahead}" != "?" ]]; then
        echo "advice: git pull origin powergrok && ./scripts/install-powergrok.sh"
        echo "(no automatic download or install performed)"
      else
        echo "advice: checkout appears current with origin/powergrok (rebuild still optional)"
      fi
    else
      echo "origin/powergrok: not available (fetch remotes first)"
    fi
    if [[ -f "${VERSION_PATH}" ]]; then
      local inst_sha
      inst_sha="$(grep -E '^git=' "${VERSION_PATH}" | head -1 | cut -d= -f2- || true)"
      echo "installed_VERSION_git: ${inst_sha:-unknown}"
      if [[ -n "${inst_sha}" && "${inst_sha}" != "unknown" ]]; then
        local full_head
        full_head="$(git -C "${REPO_ROOT}" rev-parse HEAD 2>/dev/null || true)"
        if [[ -n "${full_head}" && "${inst_sha}" != "${full_head}" ]]; then
          echo "note: installed binary SHA differs from repo HEAD — reinstall to refresh"
        fi
      fi
    fi
  fi
}

rollback_powergrok() {
  derive_paths
  assert_safe_install_path "--prefix" "${PREFIX}"
  assert_public_name_is_powergrok
  assert_does_not_clobber_official_grok

  if [[ ! -e "${PREV_BIN_PATH}" ]]; then
    die "no previous binary at ${PREV_BIN_PATH}; nothing to roll back"
  fi
  if [[ ! -f "${PREV_BIN_PATH}" ]]; then
    die "previous binary path is not a file: ${PREV_BIN_PATH}"
  fi

  log "rollback: restore ${PREV_BIN_PATH} → ${REAL_BIN_PATH}"
  if [[ "${DRY_RUN}" -eq 1 ]]; then
    log "would install -m 755 ${PREV_BIN_PATH} ${REAL_BIN_PATH}"
    log "would rewrite VERSION note for rollback"
    return 0
  fi

  mkdir -p "${LIB_DIR}"
  # Keep a one-shot copy of the binary we are leaving (optional forensics).
  if [[ -e "${REAL_BIN_PATH}" ]]; then
    cp -p "${REAL_BIN_PATH}" "${LIB_DIR}/powergrok.before-rollback" || true
  fi
  install -m 755 "${PREV_BIN_PATH}" "${REAL_BIN_PATH}"
  {
    echo "git=rollback-from-prev"
    echo "branch=n/a"
    echo "built_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "features=${CARGO_FEATURES}"
    echo "artifact=powergrok.prev"
    echo "note=restored from ${PREV_BIN_PATH}; re-run installer after git pull for a proper VERSION"
  } >"${VERSION_PATH}"
  log "rollback complete; wrapper and GROK_HOME untouched; official grok untouched"
  echo "Restored: ${REAL_BIN_PATH}"
  echo "From:     ${PREV_BIN_PATH}"
  echo "VERSION:  ${VERSION_PATH} (rollback stamp)"
}

do_install() {
  resolve_repo_root
  derive_paths
  assert_public_name_is_powergrok
  assert_does_not_clobber_official_grok
  assert_grok_home_is_not_official

  log "repo=${REPO_ROOT}"
  log "prefix=${PREFIX} grok-home=${GROK_HOME_OPT}"

  build_release_artifact
  assert_artifact_present
  install_real_binary
  # Seed home before any binary invocation so GROK_HOME dir exists for engine.
  seed_config_if_missing
  write_version_file
  install_wrapper
  install_completions_best_effort
  print_verify_block
}

main() {
  parse_args "$@"
  if [[ "${UNINSTALL}" -eq 1 ]]; then
    REPO_ROOT="$(pwd)"
    uninstall_powergrok
  elif [[ "${SHOW_STATUS}" -eq 1 ]]; then
    soft_resolve_repo_root || true
    print_status
  elif [[ "${DO_ROLLBACK}" -eq 1 ]]; then
    REPO_ROOT="$(pwd)"
    rollback_powergrok
  else
    do_install
  fi
}

# Like resolve_repo_root but returns 1 instead of dying (status/freshness).
soft_resolve_repo_root() {
  local script_dir root candidate
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  candidate="$(cd "${script_dir}/.." && pwd -P)"
  if [[ -f "${candidate}/Cargo.toml" && -d "${candidate}/crates/codegen/xai-grok-pager-bin" ]]; then
    REPO_ROOT="${candidate}"
    return 0
  fi
  root="${candidate}"
  while [[ "${root}" != "/" ]]; do
    if [[ -f "${root}/Cargo.toml" && -d "${root}/crates/codegen/xai-grok-pager-bin" ]]; then
      REPO_ROOT="${root}"
      return 0
    fi
    root="$(cd "${root}/.." && pwd -P)"
  done
  REPO_ROOT=""
  return 1
}

if [[ "${BASH_SOURCE[0]:-}" == "${0}" ]]; then
  main "$@"
fi
