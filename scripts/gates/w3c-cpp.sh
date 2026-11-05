#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The C++ conformance suite — Interpreter and AOT, 404 cases — which no local
# gate ran until this one existed.
#
# That absence has a measured cost. 233 of 234 C++ cases failed for three
# weeks while every gate in `scripts/gate --list` passed, because the suite
# lived only in CI and the CI lane could not turn red: the CLI returned 0 on a
# failing run, the step carried `continue-on-error`, and the report action was
# configured `fail_on_failure: false`. The gating is repaired, but a repaired
# lane still only speaks after a push. This is where it speaks before one.
#
# Two verdicts are read, not one: the exit status and the JUnit report, with a
# floor on the case count. `sce_gate_judge_w3c_report` holds the rules, since
# `w3c-wasm` judges the emscripten build of the same runner by them.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

sce_main_build_dir
BUILD_DIR="$SCE_MAIN_BUILD_DIR"

# ctest has no build dependency and neither does running the CLI directly, so
# the build is explicit: without it this gate would happily judge whatever
# binary was last produced, which is how a stale artifact reads as a pass.
sce_gate_step "building w3c_test_cli"
sce_gate_build "$BUILD_DIR" --target w3c_test_cli \
    || sce_gate_fail "w3c_test_cli build"

# The JUnit report is a scratch file for a local run and an artifact for the
# lane, which publishes it as a check run and deploys it to Pages. One
# variable covers both rather than the lane keeping its own copy of the
# command: unset, the report is written to a temporary directory and removed;
# set, it is written where the caller asked and left there.
if [[ -n "${SCE_W3C_REPORT_DIR:-}" ]]; then
    REPORT="$SCE_W3C_REPORT_DIR"
    mkdir -p "$REPORT"
    REPORT="$(cd "$REPORT" && pwd)"
else
    REPORT="$(mktemp -d)"
    sce_gate_on_exit "rm -rf '$REPORT'"
fi

# Run from the suite's own directory: the fixtures are resolved relative to it.
sce_gate_step "running the W3C conformance suite (Interpreter + AOT)"
status=0
( cd "$BUILD_DIR/tests" && SPDLOG_LEVEL="${SPDLOG_LEVEL:-off}" \
    ./w3c_test_cli --output "$REPORT/w3c_test_results.xml" ) >"$REPORT/w3c_test_output.log" 2>&1 || status=$?

# The floor is 400 against a current 404 (the helper says what each verdict
# guards against).
sce_gate_judge_w3c_report "$REPORT/w3c_test_results.xml" \
    "$REPORT/w3c_test_output.log" "$status" 400

sce_gate_step "$SCE_GATE_W3C_CASES W3C case(s) passed, $SCE_GATE_W3C_SKIPPED skipped"
