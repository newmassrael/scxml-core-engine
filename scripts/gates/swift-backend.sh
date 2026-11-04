#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: swift-backend.yml
#
# The Swift backend's own lane: four of the SwiftPM packages under `backends/swift/`
# built and tested with the toolchain the workflow pins.
#
#   runtime          the statechart runtime (`SceRuntime`)
#   host-features    the reference host of the standard host features (`SceHostFeatures`)
#   tests            the machines the generator writes, replayed against the shared
#                    scenarios of `sce-build/tests/fixtures/static_datamodel/scenarios/`
#   feature-conformance
#                    the machine for every build of `tests/product_line/session.scxml`,
#                    compiled under each feature set and held to that set's own machine,
#                    and refusing a build that breaks the feature model (docs/adr/0057 §10)
#
# and, on the same toolchain, the Swift transition table and transition function held to
# `sce-codegen transition-table` and to their producer by `sce-build`'s two equivalence
# tests (`transition-outputs`), which skip on a host with no Swift and must not skip here.
#
# Until this gate existed nothing a hosted runner ran compiled a line of Swift: the other
# backends' suites are reached through their W3C and Forge lanes. Swift has both now:
# `w3c-swift` writes and runs the W3C suite, and `forge-swift` runs the forge runtime
# (`forge-runtime`, the checked arithmetic and bounded values the generated code calls) and
# holds the units the generator writes for forge documents to the numerical oracle. Neither
# is run here.
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

# One `Test run with N tests` line per test target that ran: the runtime, the reference host,
# the tests package's two targets and the feature conformance package's one.
runs="$(grep -cE 'Test run with [0-9]+ tests? ' "$LOG" || true)"
[ "$runs" -ge 5 ] \
    || sce_gate_fail "$runs Swift test runs reported, expected one per test target (5 or more): a package built and ran nothing"
if grep -qE 'Test run with 0 tests ' "$LOG"; then
    sce_gate_fail "a Swift test run reported 0 tests: its test target was not discovered"
fi
sce_gate_step "$runs Swift test runs, each of at least one test"

# The two programs `sce-build`'s equivalence tests build in Swift (the `transition-outputs` arm),
# each read by name: a filter that matched no test runs nothing and exits 0, and a skip is refused
# by `SCE_REQUIRE_TOOLS` before it can read as a pass.
for held in the_swift_table_compiles_and_holds_the_export \
    the_swift_function_compiles_alone_and_answers_as_the_producer; do
    grep -qF "test $held ... ok" "$LOG" \
        || sce_gate_fail "$held did not run and pass: the Swift transition table or function was not held to the export or the producer"
done
sce_gate_step "the Swift transition table and transition function held by their equivalence tests"

# The feature conformance package (`backends/swift/feature-conformance`, docs/adr/0057 §10) is one
# of the test runs counted above; the refusal of a build that breaks the model's edge is a
# compiler run of its own, read by its verdict line.
grep -qF "feature-conformance: the build that breaks an edge is refused by the model's #error" "$LOG" \
    || sce_gate_fail "the Swift machine for every build was not shown to refuse a build that breaks the feature model's edge"
sce_gate_step "the Swift machine for every build refuses a build that breaks an edge"
