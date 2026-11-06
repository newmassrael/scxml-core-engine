#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: swift-backend.yml
#
# The Swift Forge conformance arm: the forge runtime's own tests, and every unit the Swift backend
# writes for a forge document held to the numerical oracle (`tests/forge/conformance/
# numerical_reference.json`). It is the Swift twin of `forge-kotlin`, and `swift-backend` keeps
# judging the packages it judged before.
#
# THE POPULATION is the fixtures the generator lowers for Swift (`conformance::lang_supports_fixture`):
# a document the backend refuses by name has no unit to hold to the oracle and is left out of the
# generated suite and of the sweep that writes the units alike. What this gate holds is that every
# test the generator wrote ran, and that the generator still writes a population worth the name.
#
# THE QUEUE HISTORIES: the queue's stress runs (SCE Protocol-Synthesis RFC section synth-5-P) write
# their histories into the directory SCE_QUEUE_HISTORY_DIR names, under `target/` and emptied first (a
# file left from an earlier run would be judged in place of one this run did not write), and the
# command every backend's histories are judged by reads them. Linearizability is that command's to
# judge, not a checker of the arm's own.
#
# THE BROADCAST-LOG HISTORIES: the broadcast log's stress runs (RFC section synth-5-S, ADR 0062) are
# written and emptied the same way, to the directory SCE_BROADCAST_HISTORY_DIR names, by the same
# Swift run, and `check-broadcast-history` judges them as it judges the other arms'.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# The toolchain is the one `bx.toml` pins, which is also what the runner installs
# (`scripts/install_swift_toolchain.sh`).
sce_gate_require_swift

source "$SCE_REPO_ROOT/scripts/lib/sce_codegen.sh"
QUEUE_HISTORIES="$SCE_REPO_ROOT/target/queue-histories/swift"
rm -rf "$QUEUE_HISTORIES"
mkdir -p "$QUEUE_HISTORIES"
BROADCAST_HISTORIES="$SCE_REPO_ROOT/target/broadcast-histories/swift"
rm -rf "$BROADCAST_HISTORIES"
mkdir -p "$BROADCAST_HISTORIES"

LOG="$(mktemp)"
sce_gate_on_exit "rm -f '$LOG'"

# The status of the script, not of the `tee` it is piped to.
set +e
SCE_QUEUE_HISTORY_DIR="$QUEUE_HISTORIES" SCE_BROADCAST_HISTORY_DIR="$BROADCAST_HISTORIES" \
    scripts/test_swift_backend.sh forge 2>&1 | tee "$LOG"
status="${PIPESTATUS[0]}"
set -e
[ "$status" -eq 0 ] \
    || sce_gate_fail "scripts/test_swift_backend.sh forge exited $status (the forge runtime or a generated unit failed to build or to pass; the output above names it)"

# 4 Lamport capacities recorded 3 times, 6 SCQ shapes recorded 25 times and 5 segmented shapes
# recorded 25 times. A run that recorded fewer would pass the judgement below, so the count is held.
shopt -s nullglob
queue_histories=("$QUEUE_HISTORIES"/*.json)
shopt -u nullglob
(( ${#queue_histories[@]} >= 287 )) \
    || sce_gate_fail "Swift queue histories: ${#queue_histories[@]} written, expected at least 287"
"$(sce_codegen_require "$SCE_REPO_ROOT")" check-queue-history "${queue_histories[@]}" \
    || sce_gate_fail "Swift queue histories are not linearizable"
sce_gate_step "${#queue_histories[@]} Swift queue histories judged, all linearizable"

# 4 shapes recorded 25 times. A run that recorded fewer would pass the judgement below, so the count is held.
shopt -s nullglob
broadcast_histories=("$BROADCAST_HISTORIES"/*.json)
shopt -u nullglob
(( ${#broadcast_histories[@]} >= 100 )) \
    || sce_gate_fail "Swift broadcast-log histories: ${#broadcast_histories[@]} written, expected at least 100"
"$(sce_codegen_require "$SCE_REPO_ROOT")" check-broadcast-history "${broadcast_histories[@]}" \
    || sce_gate_fail "Swift broadcast-log histories are not linearizable"
sce_gate_step "${#broadcast_histories[@]} Swift broadcast-log histories judged, all linearizable"

# The test runs the lane reported, in the order it ran them: the forge runtime, then the
# conformance tests. A package whose tests were not discovered builds, runs nothing and exits 0,
# which is the shape of a lane that was never testing.
mapfile -t runs < <(grep -oE 'Test run with [0-9]+ tests?' "$LOG" | grep -oE '[0-9]+')
[ "${#runs[@]}" -eq 2 ] \
    || sce_gate_fail "${#runs[@]} Swift test run(s) reported, expected 2 (the forge runtime and the conformance tests): a package built and ran nothing, or ran twice"
runtime_ran="${runs[0]}"
ran="${runs[1]}"
[ "$runtime_ran" -ge 1 ] \
    || sce_gate_fail "the forge runtime ran $runtime_ran test(s): its test target was not discovered"

# The number the generator wrote, read off the file it wrote, and the number the run ran. A
# fixture whose test was written and not run is a unit held to nothing, and a file the generator
# truncated is a suite that passes by being small, so the count is compared and floored.
suite="backends/swift/tests/Tests/StaticDatamodelTests/NumericalConformanceTests.swift"
[ -f "$suite" ] \
    || sce_gate_fail "$suite was not written: the run above held units to an oracle this check cannot find"
written="$(grep -cE '^@Test func [A-Za-z0-9_]+MatchesReference\(\)' "$suite" || true)"
written="${written:-0}"
if (( written < 100 )); then
    sce_gate_fail "the generator wrote $written conformance test(s): the oracle holds well over a hundred fixtures a Swift unit can be held to, so this suite was not generated from it"
fi
[ "$ran" -eq "$written" ] \
    || sce_gate_fail "the generator wrote $written conformance test(s) and $ran ran: a test was written and not run, or run twice"
sce_gate_step "the forge runtime ran $runtime_ran test(s), and $ran generated units were held to the oracle"
