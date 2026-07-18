#!/usr/bin/env bash
# install-powergrok.sh — one-command Power Grok install (issue #6 / BUILD_PLAN §9)
#
# Primary operator path from a clean checkout of the powergrok branch:
#   ./scripts/install-powergrok.sh
#
# Non-goals (issue #6): isolation engine (#4), engine --version marketing patch,
# Windows/Homebrew packaging, force-push, touching official grok paths.
#
# PHASE 1 SCAFFOLD — function signatures + logic comments only.
# No side-effecting implementation yet. Phase 3 fills bodies one function at a time.

set -euo pipefail

# --- constants ----------------------------------------------------------------

readonly PUBLIC_COMMAND_NAME="powergrok"
readonly FORBIDDEN_PUBLIC_NAME="grok"
readonly CARGO_PACKAGE="xai-grok-pager-bin"
readonly CARGO_FEATURES="powergrok"
readonly ARTIFACT_REL="target/release/xai-grok-pager"
readonly WRAPPER_MARKER="# powergrok-wrapper"
readonly WRAPPER_SOURCE_REL="scripts/powergrok.wrapper.sh"
readonly SEED_CONFIG_HEADER="# Powergrok — local source-built install."

# --- option defaults (mutated by parse_args) ----------------------------------

DO_BUILD=1
DO_INSTALL_COMPLETIONS=1
DRY_RUN=0
UNINSTALL=0
PURGE_HOME=0
PREFIX="${HOME}/.local"
GROK_HOME_OPT="${HOME}/.powergrok"
REPO_ROOT=""

# --- helpers (signatures + intended logic) ------------------------------------

usage() {
  # Print §9.1 CLI help to stdout and exit 0.
  # Include: --build/--no-build, --prefix, --grok-home, --dry-run,
  # --uninstall, --purge-home, --install-completions / --no-install-completions,
  # -h/--help. Point at docs/powergrok/BUILD_PLAN.md for full contract.
  :
}

die() {
  # stderr: "install-powergrok: $*" ; exit 1
  :
}

log() {
  # stdout info line; prefix with "[dry-run] " when DRY_RUN=1
  :
}

run_or_dry() {
  # If DRY_RUN: log the command string and return 0.
  # Else: eval/execute "$@" (prefer array form).
  :
}

# Resolve absolute repo root containing Cargo.toml + scripts/install-powergrok.sh.
# Logic: start from BASH_SOURCE dir → parent until Cargo.toml + xai-grok-pager-bin
# found, or fail. Export REPO_ROOT.
resolve_repo_root() {
  # PHASE 1: describe only
  :
}

# Parse argv into DO_BUILD, PREFIX, GROK_HOME_OPT, DRY_RUN, UNINSTALL, etc.
# Unknown flags → usage + die. --purge-home alone without --uninstall → die.
parse_args() {
  # PHASE 1: describe only
  :
}

# Derived paths after PREFIX / GROK_HOME_OPT known.
# BIN_DIR=$PREFIX/bin
# LIB_DIR=$PREFIX/lib/powergrok
# WRAPPER_PATH=$BIN_DIR/powergrok
# REAL_BIN_PATH=$LIB_DIR/powergrok
# ARTIFACT=$REPO_ROOT/$ARTIFACT_REL
# WRAPPER_SRC=$REPO_ROOT/$WRAPPER_SOURCE_REL
derive_paths() {
  # PHASE 1: describe only
  :
}

# Hard-fail if public command basename would be "grok" or PREFIX/bin target is grok.
assert_public_name_is_powergrok() {
  # PHASE 1: describe only
  # Also fail if WRAPPER_PATH basename != powergrok.
  :
}

# Hard-fail if WRAPPER_PATH or REAL_BIN_PATH would overwrite official grok install
# when those paths resolve to the same realpath as $(command -v grok).
assert_does_not_clobber_official_grok() {
  # PHASE 1: describe only
  :
}

# Require rustc/cargo on PATH (and note rust-toolchain.toml). Clear stderr on miss.
require_rust_toolchain() {
  # PHASE 1: describe only
  :
}

# cargo build -p xai-grok-pager-bin --release --features powergrok from REPO_ROOT.
# Skip when DO_BUILD=0. On failure: die with build log hint.
build_release_artifact() {
  # PHASE 1: describe only
  :
}

# Assert ARTIFACT exists and is executable; else die with rebuild instructions.
assert_artifact_present() {
  # PHASE 1: describe only
  :
}

# mkdir -p LIB_DIR BIN_DIR GROK_HOME; backup existing REAL_BIN to powergrok.prev;
# install -m 755 ARTIFACT → REAL_BIN_PATH (basename powergrok).
install_real_binary() {
  # PHASE 1: describe only
  :
}

# Write LIB_DIR/VERSION with git SHA, branch (best-effort), built_at UTC,
# features=powergrok, and optional --version line from new binary.
write_version_file() {
  # PHASE 1: describe only
  :
}

# Install wrapper:
# Prefer copying WRAPPER_SRC and rewriting POWERGROK_LIB default to absolute LIB_DIR
# and ensuring GROK_HOME default matches GROK_HOME_OPT, OR generate wrapper from
# template matching BUILD_PLAN §5.2 with marker comment.
# chmod 755. Must exec "$LIB_DIR/powergrok" "$@" with no exec -a.
install_wrapper() {
  # PHASE 1: describe only
  :
}

# Create GROK_HOME/config.toml if missing with seed only (never overwrite).
seed_config_if_missing() {
  # PHASE 1: describe only
  :
}

# Best-effort: REAL_BIN completions bash/zsh into GROK_HOME/completions/...
# Never write system fish grok.fish. Failures warn, do not fail install.
install_completions_best_effort() {
  # PHASE 1: describe only
  :
}

# Print operator verify block (V1–V6, V9, V11 smoke) and PATH hint if needed.
print_verify_block() {
  # PHASE 1: describe only
  :
}

# Uninstall: remove wrapper only if it contains WRAPPER_MARKER; rm -rf LIB_DIR.
# --purge-home: rm -rf GROK_HOME_OPT. Never touch official grok or repo .powergrok/.
uninstall_powergrok() {
  # PHASE 1: describe only
  :
}

# Orchestrate install path (not uninstall).
do_install() {
  # PHASE 1 order:
  # resolve_repo_root; derive_paths
  # assert_public_name_is_powergrok
  # assert_does_not_clobber_official_grok
  # if DO_BUILD: require_rust_toolchain; build_release_artifact
  # assert_artifact_present
  # install_real_binary; write_version_file; install_wrapper
  # seed_config_if_missing
  # if DO_INSTALL_COMPLETIONS: install_completions_best_effort
  # print_verify_block
  :
}

main() {
  # parse_args "$@"; if UNINSTALL → uninstall_powergrok; else do_install
  :
}

if [[ "${BASH_SOURCE[0]:-}" == "${0}" ]]; then
  main "$@"
fi
