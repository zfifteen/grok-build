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

usage() {
  cat <<'EOF'
Usage: scripts/install-powergrok.sh [options]

One-command Power Grok install (side-by-side with official grok).

Options:
  --build                 Build release artifact before install (default)
  --no-build              Skip cargo build; require existing target/release/xai-grok-pager
  --prefix DIR            Install prefix (default: $HOME/.local)
  --grok-home DIR         Power Grok user home / GROK_HOME (default: $HOME/.powergrok)
  --dry-run               Print actions; do not write or build
  --uninstall             Remove wrapper + lib/powergrok (not repo project data)
  --purge-home            With --uninstall, also remove --grok-home directory
  --install-completions   Generate completions under GROK_HOME/completions (default)
  --no-install-completions
  -h, --help              Show this help

Layout after install:
  $prefix/bin/powergrok              wrapper (sets GROK_HOME, execs named binary)
  $prefix/lib/powergrok/powergrok    real binary (basename = argv0)
  $prefix/lib/powergrok/VERSION      git SHA + build time
  $grok-home/config.toml             seeded once: auto_update = false

Contract: docs/powergrok/BUILD_PLAN.md §5, §8, §9
Issue:    https://github.com/zfifteen/powergrok/issues/6
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
}

derive_paths() {
  # Expand leading ~ if user passed it literally.
  PREFIX="${PREFIX/#\~/${HOME}}"
  GROK_HOME_OPT="${GROK_HOME_OPT/#\~/${HOME}}"

  BIN_DIR="${PREFIX}/bin"
  LIB_DIR="${PREFIX}/lib/powergrok"
  WRAPPER_PATH="${BIN_DIR}/${PUBLIC_COMMAND_NAME}"
  REAL_BIN_PATH="${LIB_DIR}/${PUBLIC_COMMAND_NAME}"
  VERSION_PATH="${LIB_DIR}/VERSION"
  PREV_BIN_PATH="${LIB_DIR}/powergrok.prev"
  ARTIFACT="${REPO_ROOT}/${ARTIFACT_REL}"
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
  # Fallback: device+inode
  local ia ib
  ia="$(ls -di "${a}" | awk '{print $1}')"
  ib="$(ls -di "${b}" | awk '{print $1}')"
  [[ -n "${ia}" && "${ia}" == "${ib}" ]]
}

assert_does_not_clobber_official_grok() {
  local grok_path
  grok_path="$(command -v grok 2>/dev/null || true)"
  if [[ -z "${grok_path}" ]]; then
    return 0
  fi
  if _paths_collide "${WRAPPER_PATH}" "${grok_path}"; then
    die "install would overwrite official grok at ${grok_path}"
  fi
  if _paths_collide "${REAL_BIN_PATH}" "${grok_path}"; then
    die "real binary path collides with official grok at ${grok_path}"
  fi
  # Also refuse if wrapper target name is literally the grok path basename wrongly.
  if [[ "$(basename "${grok_path}")" == "${FORBIDDEN_PUBLIC_NAME}" \
     && "${WRAPPER_PATH}" == "${grok_path}" ]]; then
    die "refusing to replace official grok PATH entry"
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
  if [[ "${DRY_RUN}" -eq 1 && "${DO_BUILD}" -eq 1 ]]; then
    log "skipping artifact check in dry-run with build (artifact would be produced)"
    return 0
  fi
  if [[ ! -f "${ARTIFACT}" ]]; then
    die "missing artifact ${ARTIFACT}; run without --no-build or build first"
  fi
  if [[ ! -x "${ARTIFACT}" ]]; then
    # cargo artifacts are usually +x; still accept non-x and chmod on install
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
    version_line="$("${REAL_BIN_PATH}" --version 2>/dev/null | head -n 1 || true)"
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
  # Copy repo wrapper; rewrite the two default assignments between markers.
  local tmp
  tmp="$(mktemp)"
  awk -v lib="${LIB_DIR}" -v home="${GROK_HOME_OPT}" '
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
  if ! grep -q "POWERGROK_LIB_DEFAULT=\"${LIB_DIR}\"" "${tmp}"; then
    rm -f "${tmp}"
    die "wrapper lib default bake failed"
  fi
  if ! grep -q "POWERGROK_HOME_DEFAULT=\"${GROK_HOME_OPT}\"" "${tmp}"; then
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
    log "would run: ${REAL_BIN_PATH} completions bash > ${bash_out}"
    log "would run: ${REAL_BIN_PATH} completions zsh > ${zsh_out}"
    return 0
  fi

  if [[ ! -x "${REAL_BIN_PATH}" ]]; then
    log "warning: binary not executable; skipping completions"
    return 0
  fi

  mkdir -p "${bash_dir}" "${zsh_dir}"
  if "${REAL_BIN_PATH}" completions bash >"${bash_out}" 2>/dev/null; then
    log "wrote ${bash_out}"
  else
    log "warning: completions bash failed (ignored)"
    rm -f "${bash_out}"
  fi
  if "${REAL_BIN_PATH}" completions zsh >"${zsh_out}" 2>/dev/null; then
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
    run_or_dry rm -rf "${LIB_DIR}"
    log "removed ${LIB_DIR}"
  else
    log "lib dir not present: ${LIB_DIR}"
  fi

  if [[ "${PURGE_HOME}" -eq 1 ]]; then
    log "purge home ${GROK_HOME_OPT}"
    if [[ "${GROK_HOME_OPT}" == "${HOME}/.grok" ]]; then
      die "refusing to purge official ~/.grok"
    fi
    if [[ -d "${GROK_HOME_OPT}" ]]; then
      run_or_dry rm -rf "${GROK_HOME_OPT}"
    fi
  else
    log "left home intact: ${GROK_HOME_OPT} (use --purge-home to remove)"
  fi

  log "uninstall complete (repo project .powergrok/ untouched; official grok untouched)"
}

do_install() {
  resolve_repo_root
  derive_paths
  assert_public_name_is_powergrok
  assert_does_not_clobber_official_grok

  log "repo=${REPO_ROOT}"
  log "prefix=${PREFIX} grok-home=${GROK_HOME_OPT}"

  build_release_artifact
  assert_artifact_present
  install_real_binary
  write_version_file
  install_wrapper
  seed_config_if_missing
  install_completions_best_effort
  print_verify_block
}

main() {
  parse_args "$@"
  if [[ "${UNINSTALL}" -eq 1 ]]; then
    # Paths come from --prefix / --grok-home; repo root not required.
    REPO_ROOT="${REPO_ROOT:-$(pwd)}"
    uninstall_powergrok
  else
    do_install
  fi
}

if [[ "${BASH_SOURCE[0]:-}" == "${0}" ]]; then
  main "$@"
fi
