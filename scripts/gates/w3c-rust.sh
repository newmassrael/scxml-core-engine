#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The Rust AOT conformance arm: the W3C cases, the modules of the one test
# target `w3c` (`backends/rust/tests/tests/w3c.rs`), plus the hand-written tests
# beside them in `sce-rust-tests`.
#
# It was the one W3C arm with no gate. The `test-rust` job spelled its
# generation, its fixture server and its `cargo test` inline, while the Go,
# Python, Kotlin, C++ and C11 jobs each run `scripts/gate w3c-<backend>`. A
# backend's lanes are read from the gate registry by the rule
# `<family>-<canonical name>`, so a lane with no gate was a row nobody could
# derive. The job runs this script now, and the commands live here once.
#
# The Rust W3C tree is a build output (docs/adr/0097): the crate's build
# script writes it into OUT_DIR from the generator in the working tree, so the
# build below judges the generator as it stands, with no generation step here.
#
# `--release` and `--no-fail-fast`, as the lane ran it. The same cases also run
# in the test profile inside `workspace-tests`; this is the arm that runs them
# optimised, the way the crate's own documentation runs it.
#
# The BasicHTTP fixtures need the echo server, and the harness addresses
# whatever port `SCE_W3C_HTTP_PORT` names, which the fixture helper exports
# once the server reports itself ready. The lane used to start it with a
# backgrounded `node` and a one-second sleep, the timer `lib.sh` records as
# answering before the port was bound.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

# ONE SHARD OF THE SUITE, when `SCE_W3C_RUST_SHARD=k/N` asks for one: the CI
# lane runs the crate's suite as N jobs (w3c-tests.yml). Run whole it took 1605
# s (job 138891691369): 7 minutes building the target `w3c` with what it links,
# 15 building every other target, and about ten seconds of tests in all, so a
# shard is a part of the crate's targets, each built and run where it is
# planned, and the target `w3c` is one unit, not split by case: its cases run
# in under six seconds, and a second job running some of them would build the
# whole target again for them. scripts/lib/workspace_shards.py reads the units
# from cargo (`--package`): a unit is an integration test target, run with
# `--test`, or `core`, the library and binary unit tests and the doc tests,
# which is the rest of what the unsharded run below runs. Every job writes the
# whole plan beside its part, and the lane's last job holds the plans of all
# the shards to one digest and every shard to a report that each of its units
# ran (`ctest_shards.py verify`); `hook_ci_parity` holds the units to every
# target the unsharded run runs, each in one shard. Unset, the gate runs the
# whole suite, which is what a developer and the build machines run.
SHARD_PLAN=""
SHARD_UNITS=()
SELECTION=()
CORE=0
W3C=1
if [[ -n "${SCE_W3C_RUST_SHARD:-}" ]]; then
    SHARD_PLAN="$SCE_REPO_ROOT/target/w3c-rust-shard"
    rm -rf "$SHARD_PLAN"
    shard_units="$(python3 scripts/lib/workspace_shards.py --package sce-rust-tests \
        --out "$SHARD_PLAN" "$SCE_W3C_RUST_SHARD")" \
        || sce_gate_cannot_run "scripts/lib/workspace_shards.py could not name shard $SCE_W3C_RUST_SHARD"
    W3C=0
    while IFS= read -r unit; do
        case "$unit" in
            core) CORE=1; SHARD_UNITS+=("$unit") ;;
            "test "*)
                SHARD_UNITS+=("$unit")
                SELECTION+=(--test "${unit#test }")
                if [[ "$unit" == "test w3c" ]]; then
                    W3C=1
                fi
                ;;
            "") ;;
            *) sce_gate_cannot_run "scripts/lib/workspace_shards.py named a unit it does not define: $unit" ;;
        esac
    done <<<"$shard_units"
    if (( CORE )); then
        SELECTION=(--lib --bins ${SELECTION[@]+"${SELECTION[@]}"})
    fi
    if (( ${#SHARD_UNITS[@]} == 0 )); then
        python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" --executed 0 --disabled 0
        sce_gate_step "shard $SCE_W3C_RUST_SHARD holds no target; the other shards run the suite"
        exit 0
    fi
    told="shard $SCE_W3C_RUST_SHARD: $(( ${#SHARD_UNITS[@]} - CORE )) test target(s)"
    if (( W3C )); then
        told="$told, the target w3c among them"
    fi
    if (( CORE )); then
        told="$told, and the core"
    fi
    sce_gate_step "$told"
fi

# The population the run is held to, read from what the build placed rather
# than written here. Each W3C case is one module of the target `w3c` holding
# one `#[test]` named after it (`test_403a::test_403a`), so the cases the
# target lists are the denominator and the passing case lines the numerator.
#
# The floor under the denominator is the one the Go and Python arms hold, 200,
# against 202 cases on the day this was written: a generation that emitted
# nothing would otherwise leave a denominator of whatever the build compiled,
# and a gate that compares a run to an empty set compares nothing.
# A shard that does not hold the target `w3c` has no case to count.
expected=0
if (( W3C )); then
    sce_gate_step "listing the Rust W3C suite the build placed"
    cargo test --release -p sce-rust-tests --test w3c -- --list >"$LOG/list.log" 2>&1 \
        || {
            cat "$LOG/list.log" >&2
            sce_gate_repro_from "$LOG/list.log" -- cargo test --release -p sce-rust-tests --test w3c -- --list
            sce_gate_fail "the Rust W3C suite did not build"
        }
    expected="$(grep -cE '^test_[0-9]+[a-z]?::test_[0-9]+[a-z]?: test$' "$LOG/list.log" || true)"
    if (( expected < 200 )); then
        sce_gate_fail "the build placed only $expected W3C case(s) in the target w3c (expected at least 200) — the suite would cover less than its name claims"
    fi
fi

sce_gate_http_fixture_server

sce_gate_step "running the Rust conformance suite"

# The log is printed whole, as it comes, because the lane's summary step reads
# its `test result:` lines out of what this gate writes.
status=0
sce_gate_tested --log "$LOG/rust.log" -- cargo test --release --no-fail-fast -p sce-rust-tests \
    ${SELECTION[@]+"${SELECTION[@]}"} || status=$?
# `--doc` cannot be asked with another target, so the core's doc tests are a
# run of their own, as the unsharded run's last part; its log joins the run's,
# which the lines below read.
if [[ -n "$SHARD_PLAN" ]] && (( CORE )) && (( status == 0 )); then
    sce_gate_tested --log "$LOG/rust-doc.log" -- cargo test --release --no-fail-fast \
        -p sce-rust-tests --doc || status=$?
    cat "$LOG/rust-doc.log" >>"$LOG/rust.log"
fi

if (( status != 0 )); then
    grep -m 40 -E '^(test .* \.\.\. FAILED|error(\[E[0-9]+\])?:)' "$LOG/rust.log" >&2 || true
    sce_gate_fail "Rust conformance suite failed"
fi

# A test binary that compiled no cases, or a target cargo was never asked to
# build, exits 0 with no case lines at all, which reads as a pass in the
# status alone. Every case the build placed has to answer.
cases="$(grep -cE '^test test_[0-9]+[a-z]?::test_[0-9]+[a-z]? \.\.\. ok$' "$LOG/rust.log" || true)"
if (( cases < expected )); then
    tail -n 20 "$LOG/rust.log" >&2
    sce_gate_fail "only $cases of the $expected W3C case(s) the generator wrote passed — the suite covered less than its name claims"
fi

# What this shard ran, for the lane's last job to hold beside the other
# shards' (`ctest_shards.py verify`): a unit counts when cargo started its
# binary, `core` by its library's unit tests, and a unit cargo never started
# fails the shard here rather than reading as a pass.
if [[ -n "$SHARD_PLAN" ]]; then
    ran=0
    missing=()
    for unit in "${SHARD_UNITS[@]}"; do
        case "$unit" in
            core) pattern=' unittests src/lib\.rs \(' ;;
            *) pattern=" tests/${unit#test }(\.rs|/main\.rs) \(" ;;
        esac
        if grep -qE "Running.*$pattern" "$LOG/rust.log"; then
            ran=$((ran + 1))
        else
            missing+=("$unit")
        fi
    done
    if (( ${#missing[@]} > 0 )); then
        sce_gate_fail "cargo started no binary for ${#missing[@]} unit(s) of shard $SCE_W3C_RUST_SHARD: ${missing[*]}"
    fi
    python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" --executed "$ran" --disabled 0
    if (( W3C )); then
        sce_gate_step "$cases Rust W3C case(s) passed, of $expected generated"
    fi
    sce_gate_step "shard $SCE_W3C_RUST_SHARD: $ran unit(s) ran and passed"
    exit 0
fi

sce_gate_step "$cases Rust W3C case(s) passed, of $expected generated"
