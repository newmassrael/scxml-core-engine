#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
# Two verdicts are read, not one. The exit status is the primary gate, and the
# XML is checked independently — a suite that reports failures while exiting 0
# is the exact regression this lane already shipped once, and a gate that
# trusted the exit status alone would not see it come back. The floor on the
# case count covers the other direction: a suite that registers nothing exits
# 0 with an empty report, which reads as success in both channels.

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

xml="$REPORT/w3c_test_results.xml"
[[ -f "$xml" ]] || {
    tail -n 30 "$REPORT/w3c_test_output.log" >&2
    sce_gate_fail "the suite wrote no report — it died before finishing"
}

# One `<testsuites>` header carries the totals; the per-engine `<testsuite>`
# elements repeat the attribute, so only the first line is read.
read_attr() { sed -n "s/.*[^a-z]$1=\"\([0-9]\+\)\".*/\1/p" "$xml" | head -1; }
total="$(read_attr tests)"
failures="$(read_attr failures)"
errors="$(read_attr errors)"
total="${total:-0}"; failures="${failures:-0}"; errors="${errors:-0}"
bad=$(( failures + errors ))

# The floor is 400 against a current 404. These cases are registered from
# `tests/w3c/conformance/fixtures.json` through generated runners, so a
# registration that stops matching leaves the suite reporting green on a
# fraction of itself.
if (( total < 400 )); then
    sce_gate_fail "only $total W3C case(s) ran (expected at least 400) — the suite reported on a smaller set than its name claims"
fi

if (( bad > 0 )); then
    grep -E '<failure|<error' "$xml" | head -n 20 >&2 || true
    sce_gate_fail "$bad of $total W3C case(s) failed"
fi

# The regression this lane already shipped: failures in the report, success in
# the exit status. Checked in both directions so neither channel can carry the
# verdict alone.
if (( status != 0 )); then
    tail -n 30 "$REPORT/w3c_test_output.log" >&2
    sce_gate_fail "the suite exited $status while its report showed no failure — the two verdicts disagree"
fi

sce_gate_step "$total W3C case(s) passed"
