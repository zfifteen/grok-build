#!/usr/bin/env bash
# Smoke tests for install-powergrok.sh (issue #6 + PR #16 review fixes).
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

assert_exit_nonzero() {
  local label="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    echo "  FAIL ${label}: expected non-zero exit" >&2
    FAIL=$((FAIL + 1))
  else
    echo "  PASS ${label}"
    PASS=$((PASS + 1))
  fi
}

make_fake_artifact() {
  local path="$1"
  cat >"${path}" <<'EOF'
#!/usr/bin/env bash
# fake powergrok binary for installer tests
# Record GROK_HOME for spy tests when FAKE_SPY_DIR is set.
if [[ -n "${FAKE_SPY_DIR:-}" ]]; then
  mkdir -p "${FAKE_SPY_DIR}"
  printf '%s\n' "GROK_HOME=${GROK_HOME-UNSET}" >>"${FAKE_SPY_DIR}/invocations.log"
  # Simulate a buggy engine only if GROK_HOME is unset: touch official home marker.
  if [[ -z "${GROK_HOME:-}" && -n "${HOME:-}" ]]; then
    mkdir -p "${HOME}/.grok"
    echo "leaked" >"${HOME}/.grok/LEAK_FROM_FAKE"
  fi
fi
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
  chmod +x "${path}"
}

test_usage_exits_zero() {
  echo "test_usage_exits_zero"
  local out
  out="$("${INSTALL}" --help)"
  assert_contains "help mentions --prefix" <(printf '%s\n' "${out}") "--prefix"
  assert_contains "help mentions powergrok" <(printf '%s\n' "${out}") "powergrok"
}

test_wrapper_source_has_marker() {
  echo "test_wrapper_source_has_marker"
  assert_contains "source marker" "${WRAPPER_SRC}" "# powergrok-wrapper"
  assert_contains "source defaults markers" "${WRAPPER_SRC}" "BEGIN_POWERGROK_INSTALL_DEFAULTS"
}

test_dry_run_no_writes() {
  echo "test_dry_run_no_writes"
  local tmp prefix home
  tmp="$(mktemp -d)"
  prefix="${tmp}/prefix"
  home="${tmp}/home"
  mkdir -p "${prefix}" "${home}"
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
  local tmp prefix home fake spy
  tmp="$(mktemp -d)"
  prefix="${tmp}/prefix"
  home="${tmp}/home"
  fake="${tmp}/fake-bin"
  spy="${tmp}/spy"
  make_fake_artifact "${fake}"

  FAKE_SPY_DIR="${spy}" POWERGROK_INSTALL_ARTIFACT="${fake}" \
    "${INSTALL}" --no-build --prefix "${prefix}" --grok-home "${home}" \
    --install-completions >/dev/null

  assert_exec "wrapper installed" "${prefix}/bin/powergrok"
  assert_exec "real binary basename powergrok" "${prefix}/lib/powergrok/powergrok"
  assert_eq "real binary basename" "$(basename "${prefix}/lib/powergrok/powergrok")" "powergrok"
  assert_file "VERSION written" "${prefix}/lib/powergrok/VERSION"
  assert_contains "VERSION has git=" "${prefix}/lib/powergrok/VERSION" "git="
  assert_contains "VERSION has built_at=" "${prefix}/lib/powergrok/VERSION" "built_at="
  assert_contains "wrapper marker" "${prefix}/bin/powergrok" "# powergrok-wrapper"
  # Absolute bake (prefix was absolute temp path)
  assert_contains "wrapper baked abs lib" "${prefix}/bin/powergrok" "POWERGROK_LIB_DEFAULT=\"${prefix}/lib/powergrok\""
  assert_contains "seed auto_update" "${home}/config.toml" "auto_update = false"
  assert_file "bash completions" "${home}/completions/bash/powergrok.bash"
  assert_file "zsh completions" "${home}/completions/zsh/_powergrok"

  # Installer must pass GROK_HOME on every binary invocation.
  if [[ -f "${spy}/invocations.log" ]] && grep -qv 'GROK_HOME=UNSET' "${spy}/invocations.log" \
    && grep -q "GROK_HOME=${home}" "${spy}/invocations.log"; then
    echo "  PASS installer sets GROK_HOME for binary runs"
    PASS=$((PASS + 1))
  else
    echo "  FAIL installer GROK_HOME spy log: $(cat "${spy}/invocations.log" 2>/dev/null || echo missing)" >&2
    FAIL=$((FAIL + 1))
  fi

  local out
  out="$(env -u GROK_HOME -u POWERGROK_HOME "${prefix}/bin/powergrok" --version)"
  assert_contains "wrapper runs binary" <(printf '%s\n' "${out}") "powergrok-fake"

  echo "# operator edit" >>"${home}/config.toml"
  POWERGROK_INSTALL_ARTIFACT="${fake}" \
    "${INSTALL}" --no-build --prefix "${prefix}" --grok-home "${home}" --no-install-completions >/dev/null
  assert_contains "config not clobbered" "${home}/config.toml" "# operator edit"

  POWERGROK_INSTALL_ARTIFACT="${fake}" \
    "${INSTALL}" --uninstall --prefix "${prefix}" --grok-home "${home}" >/dev/null
  if [[ -e "${prefix}/bin/powergrok" || -d "${prefix}/lib/powergrok" ]]; then
    echo "  FAIL uninstall left wrapper or lib" >&2
    FAIL=$((FAIL + 1))
  else
    echo "  PASS uninstall removed wrapper+lib"
    PASS=$((PASS + 1))
  fi
  assert_file "home kept without purge" "${home}/config.toml"

  rm -rf "${tmp}"
}

test_relative_prefix_bakes_absolute() {
  echo "test_relative_prefix_bakes_absolute"
  local tmp fake
  tmp="$(mktemp -d)"
  fake="${tmp}/fake-bin"
  make_fake_artifact "${fake}"
  (
    cd "${tmp}"
    POWERGROK_INSTALL_ARTIFACT="${fake}" \
      "${INSTALL}" --no-build --prefix "relpref" --grok-home "relhome" --no-install-completions >/dev/null
  )
  local baked want baked_path
  want="$(python3 -c 'import os,sys; print(os.path.realpath(os.path.abspath(sys.argv[1])))' "${tmp}/relpref/lib/powergrok")"
  baked="$(grep '^POWERGROK_LIB_DEFAULT=' "${tmp}/relpref/bin/powergrok" | head -1)"
  baked_path="${baked#POWERGROK_LIB_DEFAULT=\"}"
  baked_path="${baked_path%\"}"
  baked_path="$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "${baked_path}")"
  if [[ "${baked_path}" == "${want}" ]]; then
    echo "  PASS relative prefix baked absolute"
    PASS=$((PASS + 1))
  else
    echo "  FAIL relative bake: ${baked} → ${baked_path} want ${want}" >&2
    FAIL=$((FAIL + 1))
  fi
  rm -rf "${tmp}"
}

test_refuse_official_grok_home_install() {
  echo "test_refuse_official_grok_home_install"
  local tmp fake home_spy
  tmp="$(mktemp -d)"
  fake="${tmp}/fake-bin"
  make_fake_artifact "${fake}"
  home_spy="${tmp}/homespy"
  mkdir -p "${home_spy}"
  assert_exit_nonzero "installer refuses --grok-home=\$HOME/.grok" \
    env HOME="${home_spy}" POWERGROK_INSTALL_ARTIFACT="${fake}" \
    "${INSTALL}" --no-build --prefix "${tmp}/p" --grok-home "${home_spy}/.grok" --no-install-completions
  if [[ -e "${home_spy}/.grok/config.toml" ]]; then
    echo "  FAIL installer seeded official home" >&2
    FAIL=$((FAIL + 1))
  else
    echo "  PASS installer did not seed official home"
    PASS=$((PASS + 1))
  fi
  rm -rf "${tmp}"
}

test_wrapper_refuses_official_home_absent() {
  echo "test_wrapper_refuses_official_home_absent"
  local tmp fake wrap rc
  tmp="$(mktemp -d)"
  fake="${tmp}/powergrok"
  make_fake_artifact "${fake}"
  # Minimal installed-style wrapper invoking refuse logic from source via env.
  # Use the real wrapper source with baked defaults.
  wrap="${tmp}/bin/powergrok"
  mkdir -p "${tmp}/bin" "${tmp}/lib/powergrok"
  cp "${fake}" "${tmp}/lib/powergrok/powergrok"
  chmod +x "${tmp}/lib/powergrok/powergrok"
  awk -v lib="${tmp}/lib/powergrok" -v home="${tmp}/.powergrok" '
    /# BEGIN_POWERGROK_INSTALL_DEFAULTS/ {
      print
      print "POWERGROK_LIB_DEFAULT=\"" lib "\""
      print "POWERGROK_HOME_DEFAULT=\"" home "\""
      in_block=1
      next
    }
    /# END_POWERGROK_INSTALL_DEFAULTS/ { in_block=0; print; next }
    in_block==1 { next }
    { print }
  ' "${WRAPPER_SRC}" >"${wrap}"
  chmod +x "${wrap}"

  set +e
  env -u POWERGROK_ALLOW_OFFICIAL_HOME HOME="${tmp}" GROK_HOME="${tmp}/.grok" "${wrap}" --version >/dev/null 2>&1
  rc=$?
  set -e
  if [[ "${rc}" -eq 2 ]]; then
    echo "  PASS wrapper refuses absent official GROK_HOME (exit 2)"
    PASS=$((PASS + 1))
  else
    echo "  FAIL wrapper rc=${rc}, expected 2" >&2
    FAIL=$((FAIL + 1))
  fi
  if [[ -e "${tmp}/.grok/config.toml" ]]; then
    echo "  FAIL wrapper seeded official home" >&2
    FAIL=$((FAIL + 1))
  else
    echo "  PASS wrapper did not create official home"
    PASS=$((PASS + 1))
  fi
  rm -rf "${tmp}"
}

test_install_does_not_write_official_home() {
  echo "test_install_does_not_write_official_home"
  local tmp fake home_spy prefix pg_home
  tmp="$(mktemp -d)"
  fake="${tmp}/fake-bin"
  make_fake_artifact "${fake}"
  home_spy="${tmp}/homespy"
  prefix="${tmp}/prefix"
  pg_home="${tmp}/pg_home"
  mkdir -p "${home_spy}"

  FAKE_SPY_DIR="${tmp}/spy" HOME="${home_spy}" POWERGROK_INSTALL_ARTIFACT="${fake}" \
    "${INSTALL}" --no-build --prefix "${prefix}" --grok-home "${pg_home}" --install-completions >/dev/null

  if [[ -e "${home_spy}/.grok" ]]; then
    echo "  FAIL install created ${home_spy}/.grok" >&2
    FAIL=$((FAIL + 1))
  else
    echo "  PASS install did not create official ~/.grok under spy HOME"
    PASS=$((PASS + 1))
  fi
  rm -rf "${tmp}"
}

test_purge_home_refuses_official() {
  echo "test_purge_home_refuses_official"
  local tmp fake home_spy
  tmp="$(mktemp -d)"
  fake="${tmp}/fake-bin"
  make_fake_artifact "${fake}"
  home_spy="${tmp}/homespy"
  mkdir -p "${home_spy}/.grok" "${tmp}/prefix/lib/powergrok" "${tmp}/prefix/bin"
  echo x >"${home_spy}/.grok/keep"
  # plant a marker wrapper so uninstall path can run
  cat >"${tmp}/prefix/bin/powergrok" <<'EOF'
#!/usr/bin/env bash
# powergrok-wrapper — marker for uninstall detection
exit 0
EOF
  chmod +x "${tmp}/prefix/bin/powergrok"

  assert_exit_nonzero "purge-home refuses official" \
    env HOME="${home_spy}" \
    "${INSTALL}" --uninstall --purge-home --prefix "${tmp}/prefix" --grok-home "${home_spy}/.grok"
  assert_file "official home kept after refused purge" "${home_spy}/.grok/keep"
  rm -rf "${tmp}"
}

test_purge_home_ok() {
  echo "test_purge_home_ok"
  local tmp fake
  tmp="$(mktemp -d)"
  fake="${tmp}/fake-bin"
  make_fake_artifact "${fake}"
  POWERGROK_INSTALL_ARTIFACT="${fake}" \
    "${INSTALL}" --no-build --prefix "${tmp}/prefix" --grok-home "${tmp}/home" --no-install-completions >/dev/null
  POWERGROK_INSTALL_ARTIFACT="${fake}" \
    "${INSTALL}" --uninstall --purge-home --prefix "${tmp}/prefix" --grok-home "${tmp}/home" >/dev/null
  if [[ -d "${tmp}/home" ]]; then
    echo "  FAIL purge-home left home" >&2
    FAIL=$((FAIL + 1))
  else
    echo "  PASS purge-home removed home"
    PASS=$((PASS + 1))
  fi
  rm -rf "${tmp}"
}

test_upgrade_creates_prev_and_status() {
  echo "test_upgrade_creates_prev_and_status"
  local tmp fake1 fake2 out
  tmp="$(mktemp -d)"
  fake1="${tmp}/fake1"
  fake2="${tmp}/fake2"
  make_fake_artifact "${fake1}"
  echo '#!/usr/bin/env bash
echo "fake2 version"
if [[ "${1:-}" == "completions" ]]; then exit 0; fi
exit 0' >"${fake2}"
  chmod +x "${fake2}"

  POWERGROK_INSTALL_ARTIFACT="${fake1}" \
    "${INSTALL}" --no-build --prefix "${tmp}/prefix" --grok-home "${tmp}/home" --no-install-completions >/dev/null
  assert_file "VERSION after first install" "${tmp}/prefix/lib/powergrok/VERSION"
  assert_file "real binary v1" "${tmp}/prefix/lib/powergrok/powergrok"
  grep -q '^git=' "${tmp}/prefix/lib/powergrok/VERSION" || {
    echo "  FAIL VERSION missing git=" >&2
    FAIL=$((FAIL + 1))
  }
  # Second install upgrades and backs up prev
  POWERGROK_INSTALL_ARTIFACT="${fake2}" \
    "${INSTALL}" --no-build --prefix "${tmp}/prefix" --grok-home "${tmp}/home" --no-install-completions >/dev/null
  assert_file "prev binary after upgrade" "${tmp}/prefix/lib/powergrok/powergrok.prev"
  assert_file "VERSION after upgrade" "${tmp}/prefix/lib/powergrok/VERSION"
  grep -q 'auto_update = false' "${tmp}/home/config.toml" || {
    echo "  FAIL auto_update not false after upgrade" >&2
    FAIL=$((FAIL + 1))
  }
  out="$("${INSTALL}" --status --prefix "${tmp}/prefix" --grok-home "${tmp}/home" 2>&1)" || true
  echo "${out}" | grep -q 'Power Grok install status' || {
    echo "  FAIL status header missing: ${out}" >&2
    FAIL=$((FAIL + 1))
  }
  echo "${out}" | grep -q 'auto_update' || {
    echo "  FAIL status missing auto_update" >&2
    FAIL=$((FAIL + 1))
  }
  echo "  PASS upgrade prev + status + auto_update"
  PASS=$((PASS + 1))
  rm -rf "${tmp}"
}

test_rollback_restores_prev() {
  echo "test_rollback_restores_prev"
  local tmp fake1 fake2 body
  tmp="$(mktemp -d)"
  fake1="${tmp}/fake1"
  fake2="${tmp}/fake2"
  echo '#!/usr/bin/env bash
echo V1
exit 0' >"${fake1}"
  chmod +x "${fake1}"
  echo '#!/usr/bin/env bash
echo V2
exit 0' >"${fake2}"
  chmod +x "${fake2}"

  POWERGROK_INSTALL_ARTIFACT="${fake1}" \
    "${INSTALL}" --no-build --prefix "${tmp}/prefix" --grok-home "${tmp}/home" --no-install-completions >/dev/null
  POWERGROK_INSTALL_ARTIFACT="${fake2}" \
    "${INSTALL}" --no-build --prefix "${tmp}/prefix" --grok-home "${tmp}/home" --no-install-completions >/dev/null
  body="$("${tmp}/prefix/lib/powergrok/powergrok" 2>&1 || true)"
  [[ "${body}" == *V2* ]] || {
    echo "  FAIL expected V2 before rollback got ${body}" >&2
    FAIL=$((FAIL + 1))
  }
  "${INSTALL}" --rollback --prefix "${tmp}/prefix" --grok-home "${tmp}/home" >/dev/null
  body="$("${tmp}/prefix/lib/powergrok/powergrok" 2>&1 || true)"
  [[ "${body}" == *V1* ]] || {
    echo "  FAIL expected V1 after rollback got ${body}" >&2
    FAIL=$((FAIL + 1))
  }
  grep -q 'rollback-from-prev' "${tmp}/prefix/lib/powergrok/VERSION" || {
    echo "  FAIL VERSION not stamped for rollback" >&2
    FAIL=$((FAIL + 1))
  }
  # official grok path not required; home still has auto_update false
  grep -q 'auto_update = false' "${tmp}/home/config.toml"
  echo "  PASS rollback restores prev and stamps VERSION"
  PASS=$((PASS + 1))
  rm -rf "${tmp}"
}

test_freshness_is_advisory() {
  echo "test_freshness_is_advisory"
  local tmp out
  tmp="$(mktemp -d)"
  make_fake_artifact "${tmp}/fake"
  POWERGROK_INSTALL_ARTIFACT="${tmp}/fake" \
    "${INSTALL}" --no-build --prefix "${tmp}/prefix" --grok-home "${tmp}/home" --no-install-completions >/dev/null
  out="$("${INSTALL}" --status --check-freshness --prefix "${tmp}/prefix" --grok-home "${tmp}/home" --dry-run 2>&1)" || true
  echo "${out}" | grep -qi 'advisory\|Freshness\|never auto' || {
    echo "  FAIL freshness output missing advisory language: ${out}" >&2
    FAIL=$((FAIL + 1))
  }
  # dry-run freshness must not remove install
  assert_file "install survives freshness dry-run" "${tmp}/prefix/lib/powergrok/powergrok"
  echo "  PASS freshness advisory"
  PASS=$((PASS + 1))
  rm -rf "${tmp}"
}

main() {
  [[ -x "${INSTALL}" ]] || { echo "missing ${INSTALL}" >&2; exit 1; }
  test_usage_exits_zero
  test_wrapper_source_has_marker
  test_dry_run_no_writes
  test_install_uninstall_with_fake_artifact
  test_relative_prefix_bakes_absolute
  test_refuse_official_grok_home_install
  test_wrapper_refuses_official_home_absent
  test_install_does_not_write_official_home
  test_purge_home_refuses_official
  test_purge_home_ok
  test_upgrade_creates_prev_and_status
  test_rollback_restores_prev
  test_freshness_is_advisory
  echo
  echo "Results: ${PASS} passed, ${FAIL} failed"
  [[ "${FAIL}" -eq 0 ]]
}

main "$@"
