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
# WHAT IT DOES NOT HOLD: the queue and broadcast-log histories the other arms judge with
# `check-queue-history` and `check-broadcast-history`. Swift has no queue, and its broadcast log is
# a row of its own; when either exists its arm of this gate reads the histories the way the others do.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# The toolchain is the one `bx.toml` pins, which is also what the runner installs
# (`scripts/install_swift_toolchain.sh`).
sce_gate_require_swift

LOG="$(mktemp)"
sce_gate_on_exit "rm -f '$LOG'"

# The status of the script, not of the `tee` it is piped to.
set +e
scripts/test_swift_backend.sh forge 2>&1 | tee "$LOG"
status="${PIPESTATUS[0]}"
set -e
[ "$status" -eq 0 ] \
    || sce_gate_fail "scripts/test_swift_backend.sh forge exited $status (the forge runtime or a generated unit failed to build or to pass; the output above names it)"

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
