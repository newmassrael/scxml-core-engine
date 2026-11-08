#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: rust-workspace-tests.yml
#
# Rust workspace suite (mirror of rust-workspace-tests.yml).
#
# Default test profile, matching CI. This ran `--release` until 2026-08-04,
# which was weaker and slower at once. Weaker because `[profile.test]` keeps
# debug-assertions on and release turns them off: an integer overflow panics
# for CI and wraps here, so the one gate meant to catch it before the runner
# could not. Slower because development builds are unoptimised, so a release
# sweep shared no artifacts with the tree and rebuilt the workspace every
# push. `hook_ci_parity` (in `tree-hygiene`) fails if either side drifts back.
#
# `--features cli` is load-bearing, not decoration. sce-build declares test
# targets with `required-features = ["cli"]`; cargo excludes an
# unmet-features target SILENTLY — never built, never reported as skipped —
# so without the flag this gate ran a strictly smaller suite than its name
# claims. How many such targets there are is derived by
# `cli_feature_gating` rather than restated here, and that gate is also
# what fails if a command reaching one of them drops the flag.
#
# The W3C HTTP fixture server is started here rather than by the runner.
# Rust integration tests for W3C SCXML C.2 BasicHTTPEventProcessor
# (test_201, test_509, test_513, test_518-520, test_532, test_534,
# test_567) issue real HTTP POSTs to the port SCE_W3C_HTTP_PORT names, which
# this gate exports once the server has bound one the kernel assigned, and
# refuse to run without it. Owning it here is what makes this gate runnable
# on its own — under the previous arrangement the server was set up by the
# hook, so running the suite by hand meant remembering to start it, and
# remembering to stop it afterwards.

# What the suites run besides the workspace, prepared here so that a pass does
# not depend on what the machine happened to build before. Until 2026-11-08 the
# lane built these in steps of its own and the gate did not, so a machine that
# had never built them skipped their cases with a warning and passed: the
# Kotlin runtime jars (`forge_conformance` puts the forge one on `kotlinc -cp`,
# the Kotlin smoke tests the other), the step server the world simulation
# starts per Kotlin actor, and the two JVM programs that replay the group key
# agreement and the peer core over their C ABIs. And the Python the suites
# start is the repository's environment where there is one, as the lane's is
# a Python with the runtime's Lua binding.
#
# A case that would still skip, for a tool the machine lacks, skips outside a
# landing as it always has; in a landing `sce_gate_run_suite` runs the suites
# under SCE_REQUIRE_TOOLS and reads such a miss as could not run (lib.sh, "A
# landing counts no skip as a pass").

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

sce_gate_require_jdk "$SCE_REPO_ROOT/.github/workflows/rust-workspace-tests.yml"
sce_gate_step "building the Kotlin runtime jars and the JVM programs the suites run"
# One task list per line, so `hook_ci_parity` reads each task as the gate's.
./gradlew --console=plain :sce-forge-runtime-kotlin:jvmJar :sce-kotlin-runtime:jvmJar \
    || sce_gate_fail "the Kotlin runtime jars did not build"
./gradlew --console=plain :sce-kotlin-world:installDist \
    || sce_gate_fail "the Kotlin step server did not build"
./gradlew --console=plain :sce-kotlin-mls-replay:installDist :sce-kotlin-peer-replay:installDist \
    || sce_gate_fail "the Kotlin replays of the group key agreement and the peer core did not build"

sce_gate_python
sce_gate_python_requires_lupa "the workspace suite"
if [ "$SCE_GATE_PYTHON" != python3 ]; then
    PATH="$(dirname "$SCE_GATE_PYTHON"):$PATH"
    export PATH
fi

sce_gate_http_fixture_server

# The generator, built as a target of its own before the suite, as the CI
# lane's "Build sce-codegen" step builds it. Cargo writes
# `target/debug/sce-codegen.d`, the list of every file the binary compiled in,
# only for a target it was asked for, and `generator_source_witness` holds the
# generator's source witness to that list; built only as a dependency of the
# tests, the list would be that of whichever tree last built it here. A
# reused checkout must not select an old release binary over the debug binary
# built here, so a stale one is dropped first.
source "$(dirname "${BASH_SOURCE[0]}")/../lib/sce_codegen.sh"
sce_codegen_drop_stale_release
cargo build --bin sce-codegen --features cli -p sce-build \
    || sce_gate_fail "cargo build --bin sce-codegen --features cli -p sce-build"

# `SCE_GATE_NO_FAIL_FAST` is the one thing the CI lane wanted that this gate
# did not offer, and it kept the lane restating the command instead of calling
# it. It is a reporting choice, not a different verification: a remote run
# nobody can iterate on wants every failure in one go, a developer wants the
# first one now. Making it a switch is what let the lane delegate.
FAIL_FAST=()
case "${SCE_GATE_NO_FAIL_FAST:-}" in
    "" | 0 | false) ;;
    *) FAIL_FAST=(--no-fail-fast); sce_gate_step "reporting every failure (--no-fail-fast)" ;;
esac

# The machine generated for every build of a product line is built under each
# feature set of that product line and held to the machine the set's variant
# generates alone (docs/adr/0057 §6, §9). The sweep built the crate under no
# feature, the first set; a test cannot choose its own features, so the other
# two are built here. `session-unicast-accept` is named alone: the manifest
# implies the feature it requires, which is the edge of the model.
product_line_sets() {
    local features
    for features in session-unicast session-unicast-accept; do
        sce_gate_run_suite "the feature conformance suite under $features" \
            cargo test -p sce-feature-conformance --features "$features" ${FAIL_FAST+"${FAIL_FAST[@]}"}
    done
}

# ONE SHARD OF THE SUITE, when `SCE_WORKSPACE_SHARD=k/N` asks for one: the CI
# lane runs the suite as N jobs, one shard each (rust-workspace-tests.yml), since
# it outgrew one job. scripts/lib/workspace_shards.py reads the units from cargo
# and spreads them; a unit is an integration test target, run with `--test`, or
# `core`, which is the rest of what the unsharded sweep below runs: the library
# and binary unit tests, the doc tests (`--doc` cannot be asked with another
# target), the examples it builds, and the product-line sets. `hook_ci_parity`
# holds the shards' union to the unsharded sweep. Unset, the gate runs the whole
# suite, which is what a push and a developer run.
if [[ -n "${SCE_WORKSPACE_SHARD:-}" ]]; then
    shard_units="$(python3 scripts/lib/workspace_shards.py \
        --features cli,sce-rust-mesh/wss "$SCE_WORKSPACE_SHARD")" \
        || sce_gate_cannot_run "scripts/lib/workspace_shards.py could not name shard $SCE_WORKSPACE_SHARD"
    selection=()
    core=0
    while IFS= read -r unit; do
        case "$unit" in
            core) core=1 ;;
            "test "*) selection+=(--test "${unit#test }") ;;
            "") ;;
            *) sce_gate_cannot_run "scripts/lib/workspace_shards.py named a unit it does not define: $unit" ;;
        esac
    done <<<"$shard_units"
    told="shard $SCE_WORKSPACE_SHARD: $(( ${#selection[@]} / 2 )) integration test target(s)"
    if (( core )); then
        selection=(--lib --bins ${selection[@]+"${selection[@]}"})
        told="$told, and the core"
    fi
    sce_gate_step "$told"
    if (( ${#selection[@]} )); then
        sce_gate_run_suite "the workspace suite, shard $SCE_WORKSPACE_SHARD" \
            cargo test --workspace --features cli,sce-rust-mesh/wss ${FAIL_FAST+"${FAIL_FAST[@]}"} "${selection[@]}"
    fi
    if (( core )); then
        sce_gate_run_suite "the workspace doc tests" \
            cargo test --workspace --features cli,sce-rust-mesh/wss ${FAIL_FAST+"${FAIL_FAST[@]}"} --doc
        cargo build --workspace --features cli,sce-rust-mesh/wss --examples \
            || sce_gate_fail "cargo build --workspace --features cli,sce-rust-mesh/wss --examples"
        product_line_sets
    fi
    exit 0
fi

# `sce-rust-mesh/wss`: the WebSocket binding's tests open real sockets on
# loopback, and they sit behind a non-default feature the sweep must ask for.
sce_gate_run_suite "the workspace suite" \
    cargo test --workspace --features cli,sce-rust-mesh/wss ${FAIL_FAST+"${FAIL_FAST[@]}"}

product_line_sets
