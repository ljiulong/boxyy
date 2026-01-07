#!/usr/bin/env bash

set -u

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_TARGET="boxy-cli"
OFFLINE_MODE="${BOXY_TEST_OFFLINE:-0}"
FAST_MODE="${BOXY_TEST_FAST:-0}"

PASS=0
FAIL=0
SKIP=0

TMP_DIR=""

cleanup() {
  if [[ -n "${TMP_DIR}" && -d "${TMP_DIR}" ]]; then
    rm -rf "${TMP_DIR}"
  fi
}
trap cleanup EXIT

log() {
  printf '%s\n' "$*"
}

record_pass() {
  PASS=$((PASS + 1))
  log "[PASS] $1"
}

record_fail() {
  FAIL=$((FAIL + 1))
  log "[FAIL] $1"
  if [[ -n "$2" ]]; then
    log "       $2"
  fi
}

record_skip() {
  SKIP=$((SKIP + 1))
  log "[SKIP] $1"
  if [[ -n "$2" ]]; then
    log "       $2"
  fi
}

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

run_cmd() {
  local name="$1"
  local cmd="$2"
  local out_file err_file
  out_file="${TMP_DIR}/${name}.out"
  err_file="${TMP_DIR}/${name}.err"

  if eval "$cmd" >"${out_file}" 2>"${err_file}"; then
    record_pass "$name"
    return 0
  else
    record_fail "$name" "command failed: $cmd"
    return 1
  fi
}

run_json() {
  local name="$1"
  local cmd="$2"

  if ! have_cmd jq; then
    record_skip "$name" "jq not found, skipping JSON validation"
    return 2
  fi

  if eval "$cmd" | jq . >/dev/null 2>&1; then
    record_pass "$name"
    return 0
  else
    record_fail "$name" "invalid JSON or command failed: $cmd"
    return 1
  fi
}

main() {
  TMP_DIR="$(mktemp -d)"

  log "== Boxy CLI quick test =="
  log "root: ${ROOT_DIR}"
  log ""

  if ! have_cmd cargo; then
    record_fail "check cargo" "cargo not found in PATH"
    exit 1
  fi

  log "-- build"
  if ! run_cmd "cargo build -p ${BIN_TARGET}" "cd \"${ROOT_DIR}\" && cargo build -p ${BIN_TARGET}"; then
    log ""
    log "Build failed. Aborting remaining tests."
    exit 1
  fi

  log ""
  log "-- basic"
  run_cmd "help" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- --help"
  run_cmd "version" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- --version"

  log ""
  log "-- scan/list"
  run_cmd "scan" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- scan"
  run_json "scan json" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- scan --json"
  run_cmd "list" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- list"

  log ""
  log "-- manager-specific"
  if have_cmd brew; then
    run_cmd "list brew" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- list --manager brew"
    run_cmd "info brew git" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- info git --manager brew"
  else
    record_skip "brew tests" "brew not found"
  fi

  if have_cmd npm; then
    run_cmd "list npm" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- list --manager npm"
    if [[ "${OFFLINE_MODE}" == "1" || "${FAST_MODE}" == "1" ]]; then
      record_skip "npm info/search" "offline/fast mode enabled"
    else
      run_cmd "info npm" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- info npm --manager npm"
      run_cmd "search npm" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- search vscode --manager npm"
    fi
  else
    record_skip "npm tests" "npm not found"
  fi

  log ""
  log "-- json output"
  run_json "list json" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- list --json"
  if have_cmd npm; then
    run_json "info json" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- info npm --json"
    if [[ "${OFFLINE_MODE}" == "1" || "${FAST_MODE}" == "1" ]]; then
      record_skip "search json" "offline/fast mode enabled"
    else
      run_json "search json" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- search code --json"
    fi
  else
    record_skip "npm json tests" "npm not found"
  fi
  if [[ "${OFFLINE_MODE}" == "1" || "${FAST_MODE}" == "1" ]]; then
    record_skip "outdated json" "offline/fast mode enabled"
  else
    run_json "outdated json" "cd \"${ROOT_DIR}\" && cargo run -p ${BIN_TARGET} -- outdated --json"
  fi

  log ""
  log "== Summary =="
  log "PASS: ${PASS}  FAIL: ${FAIL}  SKIP: ${SKIP}"

  log ""
  log "== Suggestions =="
  if (( FAIL == 0 )); then
    log "- Looks good. Consider running full tests: cargo test"
  else
    if ! have_cmd jq; then
      log "- Install jq to validate JSON output: brew install jq"
    fi
    if [[ "${OFFLINE_MODE}" != "1" && "${FAST_MODE}" != "1" ]]; then
      log "- If network-dependent commands hang, rerun with BOXY_TEST_OFFLINE=1"
    fi
    log "- Rerun failing commands with RUST_LOG=debug for more detail"
    log "- If manager-specific tests failed, ensure the manager is installed and in PATH"
    log "- If build failed, run cargo check and cargo clippy -- -D warnings"
  fi
}

main "$@"
