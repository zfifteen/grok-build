#!/usr/bin/env bash
# Smoke tests for install-powergrok.sh (issue #6).
# Run from repo: ./scripts/tests/test-install-powergrok.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
INSTALL="${ROOT}/scripts/install-powergrok.sh"
WRAPPER_SRC="${ROOT}/scripts/powergrok.wrapper.sh"
PASS=0
FAIL=0

assert_eq() {
  local label="$1" got="$2" want="$3"
  if [[ "${got}" == "${want}" ]]; then
    echo "  PASS ${label}"
    PASS=$((PASS + 1))
  else
    echo "  FAIL ${label}: got='${got}' want='${want}'" >&2
    FAIL=$((FAIL + 1))
  fi
}

assert_file() {
  local label="$1" path="$2"
  if [[ -f "${path}" ]]; then
    echo "  PASS ${label}"
    PASS=$((PASS + 1))
  else
    echo "  FAIL ${label}: missing ${path}" >&2
    FAIL=$((FAIL + 1))
  fi
}

assert_exec() {
  local label="$1" path="$2"
  if [[ -x "${path}" ]]; then
    echo "  PASS ${label}"
    PASS=$((PASS + 1))
  else
    echo "  FAIL ${label}: not executable ${path}" >&2
    FAIL=$((FAIL + 1))
  fi
}

assert_contains() {
  local label="$1" file="$2" needle="$3"
  if grep -q -- "${needle}" "${file}"; then
    echo "  PASS ${label}"
    PASS=$((PASS + 1))
  else
    echo "  FAIL ${label}: '${needle}' not in ${file}" >&2
    FAIL=$((FAIL + 1))
  fi
}

test_usage_exits_zero() {
  echo "test_usage_exits_zero"
  local out
  out="$("${INSTALL}" --help)"
  assert_contains "help mentions --prefix" <(printf '%s\n' "${out}") "--prefix"
  assert_contains "help mentions powergrok" <(printf '%s\n' "${out}") "powergrok"
}

test_dry_run_no_writes() {
  echo "test_dry_run_no_writes"
  local tmp prefix home
  tmp="$(mktemp -d)"
  prefix="${tmp}/prefix"
  home="${tmp}/home"
  mkdir -p "${prefix}" "${home}"
  # dry-run with build would not write; with no-build needs artifact — use dry-run --build
  "${INSTALL}" --dry-run --build --prefix "${prefix}" --grok-home "${home}" --no-install-completions >/dev/null
  if [[ -e "${prefix}/bin/powergrok" || -e "${prefix}/lib/powergrok/powergrok" ]]; then
    echo "  FAIL dry-run wrote files under ${prefix}" >&2
    FAIL=$((FAIL + 1))
  else
    echo "  PASS dry-run wrote no install files"
    PASS=$((PASS + 1))
  fi
  rm -rf "${tmp}"
}

test_install_uninstall_with_fake_artifact() {
  echo "test_install_uninstall_with_fake_artifact"
  local tmp prefix home artifact_dir fake
  tmp="$(mktemp -d)"
  prefix="${tmp}/prefix"
  home="${tmp}/home"
  # Point a temporary "repo" is hard; instead copy installer env by placing
  # fake artifact at real repo path only if missing — prefer isolated approach:
  # run installer with --no-build after planting fake at ROOT/target/release/.
  artifact_dir="${ROOT}/target/release"
  mkdir -p "${artifact_dir}"
  fake="${artifact_dir}/xai-grok-pager"
  local had_artifact=0
  local backup=""
  if [[ -e "${fake}" ]]; then
    had_artifact=1
    backup="$(mktemp)"
    cp -p "${fake}" "${backup}"
  fi
  cat >"${fake}" <<'EOF'
#!/usr/bin/env bash
# fake powergrok binary for installer tests
if [[ "${1:-}" == "--version" ]]; then
  echo "powergrok-fake 0.0.0-test"
  exit 0
fi
if [[ "${1:-}" == "completions" ]]; then
  echo "# fake completions for $2"
  exit 0
fi
if [[ "${1:-}" == "--help" ]]; then
  echo "Power Grok fake help"
  exit 0
fi
echo "fake-powergrok ok: $*"
EOF
  chmod +x "${fake}"

  "${INSTALL}" --no-build --prefix "${prefix}" --grok-home "${home}" \
    --install-completions >/dev/null

  assert_exec "wrapper installed" "${prefix}/bin/powergrok"
  assert_exec "real binary basename powergrok" "${prefix}/lib/powergrok/powergrok"
  assert_eq "real binary basename" "$(basename "${prefix}/lib/powergrok/powergrok")" "powergrok"
  assert_file "VERSION written" "${prefix}/lib/powergrok/VERSION"
  assert_contains "VERSION has git=" "${prefix}/lib/powergrok/VERSION" "git="
  assert_contains "VERSION has built_at=" "${prefix}/lib/powergrok/VERSION" "built_at="
  assert_contains "wrapper marker" "${prefix}/bin/powergrok" "# powergrok-wrapper"
  assert_contains "wrapper baked lib" "${prefix}/bin/powergrok" "POWERGROK_LIB_DEFAULT=\"${prefix}/lib/powergrok\""
  assert_contains "seed auto_update" "${home}/config.toml" "auto_update = false"
  assert_file "bash completions" "${home}/completions/bash/powergrok.bash"
  assert_file "zsh completions" "${home}/completions/zsh/_powergrok"

  # Wrapper exec works and exports home for child.
  local out
  out="$(env -u GROK_HOME -u POWERGROK_HOME "${prefix}/bin/powergrok" --version)"
  assert_contains "wrapper runs binary" <(printf '%s\n' "${out}") "powergrok-fake"

  # Seed must not clobber
  echo "# operator edit" >>"${home}/config.toml"
  "${INSTALL}" --no-build --prefix "${prefix}" --grok-home "${home}" --no-install-completions >/dev/null
  assert_contains "config not clobbered" "${home}/config.toml" "# operator edit"

  # Uninstall keeps home
  "${INSTALL}" --uninstall --prefix "${prefix}" --grok-home "${home}" >/dev/null
  if [[ -e "${prefix}/bin/powergrok" || -d "${prefix}/lib/powergrok" ]]; then
    echo "  FAIL uninstall left wrapper or lib" >&2
    FAIL=$((FAIL + 1))
  else
    echo "  PASS uninstall removed wrapper+lib"
    PASS=$((PASS + 1))
  fi
  assert_file "home kept without purge" "${home}/config.toml"

  # Restore / cleanup fake artifact
  if [[ "${had_artifact}" -eq 1 ]]; then
    cp -p "${backup}" "${fake}"
    rm -f "${backup}"
  else
    rm -f "${fake}"
  fi
  rm -rf "${tmp}"
}

test_purge_home() {
  echo "test_purge_home"
  local tmp prefix home fake artifact_dir had_artifact=0 backup=""
  tmp="$(mktemp -d)"
  prefix="${tmp}/prefix"
  home="${tmp}/home"
  artifact_dir="${ROOT}/target/release"
  mkdir -p "${artifact_dir}"
  fake="${artifact_dir}/xai-grok-pager"
  if [[ -e "${fake}" ]]; then
    had_artifact=1
    backup="$(mktemp)"
    cp -p "${fake}" "${backup}"
  fi
  printf '#!/bin/sh\necho ok\n' >"${fake}"
  chmod +x "${fake}"

  "${INSTALL}" --no-build --prefix "${prefix}" --grok-home "${home}" --no-install-completions >/dev/null
  "${INSTALL}" --uninstall --purge-home --prefix "${prefix}" --grok-home "${home}" >/dev/null
  if [[ -d "${home}" ]]; then
    echo "  FAIL purge-home left ${home}" >&2
    FAIL=$((FAIL + 1))
  else
    echo "  PASS purge-home removed home"
    PASS=$((PASS + 1))
  fi

  if [[ "${had_artifact}" -eq 1 ]]; then
    cp -p "${backup}" "${fake}"
    rm -f "${backup}"
  else
    rm -f "${fake}"
  fi
  rm -rf "${tmp}"
}

test_wrapper_source_has_marker() {
  echo "test_wrapper_source_has_marker"
  assert_contains "source marker" "${WRAPPER_SRC}" "# powergrok-wrapper"
  assert_contains "source defaults markers" "${WRAPPER_SRC}" "BEGIN_POWERGROK_INSTALL_DEFAULTS"
}

main() {
  [[ -x "${INSTALL}" ]] || { echo "missing ${INSTALL}" >&2; exit 1; }
  test_usage_exits_zero
  test_wrapper_source_has_marker
  test_dry_run_no_writes
  test_install_uninstall_with_fake_artifact
  test_purge_home
  echo
  echo "Results: ${PASS} passed, ${FAIL} failed"
  [[ "${FAIL}" -eq 0 ]]
}

main "$@"
