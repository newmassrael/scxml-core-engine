#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: swift-backend.yml
#
# The Swift backend's own lane: the three SwiftPM packages under `backends/swift/`
# built and tested with the toolchain the workflow pins.
#
#   runtime          the statechart runtime (`SceRuntime`)
#   forge-runtime    the checked arithmetic and bounded values the generated code calls
#   tests            the machines the generator writes, replayed against the shared
#                    scenarios of `sce-build/tests/fixtures/static_datamodel/scenarios/`
#
# Until this gate existed nothing a hosted runner ran compiled a line of Swift: the other
# backends' suites are reached through their W3C and Forge lanes, and Swift has neither
# lane yet (its `ACCEPTED` rows in `a_backend_holds_what_the_other_backends_hold` say so).
# `regen-reproduces` judges that the committed Swift machines are what the generator writes;
# this judges that what the generator writes COMPILES and BEHAVES.
#
# `scripts/test_swift_backend.sh` regenerates the machines before it compiles them, so what
# runs is what the templates write today. A tree that drifted from the generator is
# `regen-reproduces`' finding and not this gate's, and the two are kept apart on purpose: a
# gate that regenerated and compared would be a second `regen-reproduces` with a Swift
# toolchain's cost.
#
# The verdict is read from the output as well as from the exit status. A package whose test
# target was not discovered builds, runs nothing and exits 0, which is the shape of a lane
# that was never testing. Each package must report a run of at least one test.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# The toolchain is the one `bx.toml` pins, which is also what the runner installs
# (`scripts/install_swift_toolchain.sh`): one source, so the packages are verified on the
# compiler the build machines have.
sce_gate_require_swift

LOG="$(mktemp)"
sce_gate_on_exit "rm -f '$LOG'"

# The status of the script, not of the `tee` it is piped to.
set +e
scripts/test_swift_backend.sh 2>&1 | tee "$LOG"
status="${PIPESTATUS[0]}"
set -e
[ "$status" -eq 0 ] \
    || sce_gate_fail "scripts/test_swift_backend.sh exited $status (a Swift package failed to build or a test failed; the output above names it)"

# One `Test run with N tests` line per test target that ran: the runtime, the forge runtime,
# and the tests package's two targets.
runs="$(grep -cE 'Test run with [0-9]+ tests? ' "$LOG" || true)"
[ "$runs" -ge 3 ] \
    || sce_gate_fail "$runs Swift test runs reported, expected one per package (3 or more): a package built and ran nothing"
if grep -qE 'Test run with 0 tests ' "$LOG"; then
    sce_gate_fail "a Swift test run reported 0 tests: its test target was not discovered"
fi
sce_gate_step "$runs Swift test runs, each of at least one test"
