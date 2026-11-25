#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: swift-backend.yml
#
# The Swift backend's own lane: six of the SwiftPM packages under `backends/swift/`
# built and tested with the toolchain the workflow pins.
#
#   runtime          the statechart runtime (`SceRuntime`)
#   host-features    the reference host of the standard host features (`SceHostFeatures`)
#   mesh             the host Mesh core (`SceMesh`): the standard Mesh documents generated from
#                    `stdlib/mesh` and the router core that applies them, held to the Kotlin and
#                    Rust cores' tests
#   mesh-wss         the WebSocket binding's SwiftNIO client for that core (`SceMeshWss`), run
#                    against the Rust core's server machine over one real socket
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
#
# In shards (ADR 0120, D3). When SCE_SWIFT_BACKEND_SHARD names `k/N`, as the lane's jobs and a
# landing's units do, the gate runs the k-th line of SWIFT_BACKEND_SHARDS below and holds only
# that part's verdicts; N must be the number of lines. Unset, it runs every line, which is the
# script's default list. Whole, the gate ran 1209 s of a 1842 s job (job 138967193099,
# 57602a3bb8): the `tests` package about eight minutes with its regeneration, the two
# equivalence tests about seven (their `sce-build` build), and the other five packages about
# six together, the WebSocket client's SwiftNIO build three of those. The parts follow that
# split, so each job is its part and the generator build and the toolchain every job pays.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# The toolchain is the one `bx.toml` pins, which is also what the runner installs
# (`scripts/install_swift_toolchain.sh`): one source, so the packages are verified on the
# compiler the build machines have.
sce_gate_require_swift

# The parts, one job of the lane each. Together they are the default list of
# `scripts/test_swift_backend.sh`, each package once; `hook_ci_parity` holds the two lists to
# each other and the lane's shard count to the number of lines.
SWIFT_BACKEND_SHARDS=(
    "tests"
    "transition-outputs"
    "runtime host-features feature-conformance mesh mesh-wss"
)

# The `Test run with N tests` lines each package reports, one per test target: the tests
# package's two targets, one for every other package, none for the equivalence tests, which
# cargo runs and which are read by name below.
declare -A MIN_RUNS=([runtime]=1 [host-features]=1 [tests]=2 [feature-conformance]=1
                     [mesh]=1 [mesh-wss]=1 [transition-outputs]=0)

PACKAGES=()
if [[ -n "${SCE_SWIFT_BACKEND_SHARD:-}" ]]; then
    [[ "$SCE_SWIFT_BACKEND_SHARD" =~ ^([0-9]+)/([0-9]+)$ ]] \
        || sce_gate_fail "SCE_SWIFT_BACKEND_SHARD is k/N, got '$SCE_SWIFT_BACKEND_SHARD'"
    shard="${BASH_REMATCH[1]}"
    count="${BASH_REMATCH[2]}"
    (( count == ${#SWIFT_BACKEND_SHARDS[@]} )) \
        || sce_gate_fail "SCE_SWIFT_BACKEND_SHARD names $count shard(s), and the gate has ${#SWIFT_BACKEND_SHARDS[@]} parts"
    (( shard >= 1 && shard <= count )) \
        || sce_gate_fail "SCE_SWIFT_BACKEND_SHARD names shard $shard of $count"
    read -r -a PACKAGES <<<"${SWIFT_BACKEND_SHARDS[shard - 1]}"
    sce_gate_step "shard $shard of $count: ${PACKAGES[*]}"
else
    for part in "${SWIFT_BACKEND_SHARDS[@]}"; do
        read -r -a listed <<<"$part"
        PACKAGES+=("${listed[@]}")
    done
fi

runs_wanted=0
declare -A RUNNING=()
for package in "${PACKAGES[@]}"; do
    [[ -n "${MIN_RUNS[$package]+set}" ]] \
        || sce_gate_fail "the gate has no count of test runs for the package '$package'"
    runs_wanted=$(( runs_wanted + ${MIN_RUNS[$package]} ))
    RUNNING[$package]=1
done

LOG="$(mktemp)"
sce_gate_on_exit "rm -f '$LOG'"

# The status of the script, not of the `tee` it is piped to.
status=0
sce_gate_tested --log "$LOG" -- scripts/test_swift_backend.sh "${PACKAGES[@]}" || status=$?
[ "$status" -eq 0 ] \
    || sce_gate_fail "scripts/test_swift_backend.sh exited $status (a Swift package failed to build or a test failed; the output above names it)"

# One `Test run with N tests` line per test target that ran (`MIN_RUNS`): the runtime, the
# reference host, the tests package's two targets, the feature conformance package's one, the
# host Mesh core's and its WebSocket client's, as many as this part runs.
runs="$(grep -cE 'Test run with [0-9]+ tests? ' "$LOG" || true)"
[ "$runs" -ge "$runs_wanted" ] \
    || sce_gate_fail "$runs Swift test runs reported, expected one per test target ($runs_wanted or more): a package built and ran nothing"
if grep -qE 'Test run with 0 tests ' "$LOG"; then
    sce_gate_fail "a Swift test run reported 0 tests: its test target was not discovered"
fi
sce_gate_step "$runs Swift test runs, each of at least one test"

# The two programs `sce-build`'s equivalence tests build in Swift (the `transition-outputs` arm),
# each read by name: a filter that matched no test runs nothing and exits 0, and a skip is refused
# by `SCE_REQUIRE_TOOLS` before it can read as a pass.
if [[ -n "${RUNNING[transition-outputs]:-}" ]]; then
    for held in the_swift_table_compiles_and_holds_the_export \
        the_swift_function_compiles_alone_and_answers_as_the_producer; do
        grep -qF "test $held ... ok" "$LOG" \
            || sce_gate_fail "$held did not run and pass: the Swift transition table or function was not held to the export or the producer"
    done
    sce_gate_step "the Swift transition table and transition function held by their equivalence tests"
fi

# The feature conformance package (`backends/swift/feature-conformance`, docs/adr/0057 §10) is one
# of the test runs counted above; the refusal of a build that breaks the model's edge is a
# compiler run of its own, read by its verdict line.
if [[ -n "${RUNNING[feature-conformance]:-}" ]]; then
    grep -qF "feature-conformance: the build that breaks an edge is refused by the model's #error" "$LOG" \
        || sce_gate_fail "the Swift machine for every build was not shown to refuse a build that breaks the feature model's edge"
    sce_gate_step "the Swift machine for every build refuses a build that breaks an edge"
fi
