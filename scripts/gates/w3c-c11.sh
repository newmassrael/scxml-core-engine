#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The C11 (MCU backend) conformance surface: 204 W3C cases plus integration
# and unit fixtures, registered under the `c11` ctest label.
#
# This backend is what a downstream MCU consumer ships, and until its CI lane
# existed nothing verified it but whoever happened to run ctest. The lane
# closed that; this gate closes the other half, because a lane still reports
# after the push rather than before it.
#
# The floor and the executed-vs-registered comparison are the lane's, kept
# because they answer the two ways a labelled ctest run reports green on
# nothing: a configure that stops registering the fixtures, and a run that
# covers a subset of what it registered.
#
# SHARDED, when `SCE_W3C_C11_SHARD=k/N` asks for one part: w3c-tests.yml runs
# the suite as N jobs, since building `sce_c11_tests` was 13 of the job's 24
# minutes and running it 43 seconds (job 138891691071, 4c71fb062c).
# scripts/lib/ctest_shards.py reads the c11 tests from ctest and their build
# closures from ninja, spreads them, and each job builds and runs only its
# part; the lane's last job holds the parts together (`ctest_shards.py
# verify`). Unset, the gate builds `sce_c11_tests` and runs every case, which
# is what the build machines and a developer run.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

sce_main_build_dir
BUILD_DIR="$SCE_MAIN_BUILD_DIR"

# A killed run leaves its log temporary behind forever; the gate that creates
# them is the one that clears them.
sce_prune_ctest_temporaries "$BUILD_DIR"

# What is registered is a fact of the configure, so it is counted before
# anything is built: a shard plans from it.
registered="$(ctest --test-dir "$BUILD_DIR" -N -L c11 | grep -cE '^ *Test +#')"
sce_gate_step "registered c11 tests: $registered"
if (( registered < 200 )); then
    sce_gate_fail "only $registered c11 test(s) registered (expected at least 200) — the gate would report on a smaller set than its name claims"
fi

# ONE SHARD OF THE SUITE, when `SCE_W3C_C11_SHARD=k/N` asks for one, planned
# from the configured tree before anything is built: the tests the same
# selector registers (`--expect` holds the helper to the count above), the
# outputs each one's command, environment and required files name, and the
# compile and link steps under each in ninja's graph. Every job computes the
# whole plan and writes it beside its own part, so the lane can hold the parts
# to one plan and to the whole suite.
SHARD_PLAN=""
SHARD_TARGETS=()
RUN_SELECTION=()
expected="$registered"
if [[ -n "${SCE_W3C_C11_SHARD:-}" ]]; then
    SHARD_PLAN="$BUILD_DIR/ctest-shard-c11"
    rm -rf "$SHARD_PLAN"
    plan_status=0
    python3 scripts/lib/ctest_shards.py plan --build "$BUILD_DIR" --label c11 \
        --expect "$registered" --out "$SHARD_PLAN" "$SCE_W3C_C11_SHARD" || plan_status=$?
    case "$plan_status" in
        0) ;;
        3) sce_gate_cannot_run "scripts/lib/ctest_shards.py could not plan shard $SCE_W3C_C11_SHARD on this machine (above)" ;;
        *) sce_gate_fail "scripts/lib/ctest_shards.py refused to plan shard $SCE_W3C_C11_SHARD (above)" ;;
    esac
    expected="$(grep -c . "$SHARD_PLAN/tests" || true)"
    while IFS= read -r line; do
        [[ -n "$line" ]] && SHARD_TARGETS+=("$line")
    done <"$SHARD_PLAN/targets"
    if (( expected == 0 )); then
        python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" --executed 0 --disabled 0
        sce_gate_step "shard $SCE_W3C_C11_SHARD holds no test; the other shards run the suite"
        exit 0
    fi
    RUN_SELECTION=(-R "$(<"$SHARD_PLAN/run-regex")")
    fixture_regex="$(<"$SHARD_PLAN/fixture-regex")"
    if [[ -n "$fixture_regex" ]]; then
        RUN_SELECTION+=(--fixture-exclude-setup "$fixture_regex")
    fi
fi

# Whole, `sce_c11_tests`: it aggregates every target the C directory defines,
# so a fixture added later is built without anyone updating a list, and
# building it rather than the default target skips the C++ engine, mesh, forge
# and the examples, none of which this backend links. A shard builds the
# outputs its tests name, which ctest's own registration names, so a case
# added anywhere brings its target with it.
if [[ -n "$SHARD_PLAN" ]]; then
    sce_gate_step "building the ${#SHARD_TARGETS[@]} output(s) shard $SCE_W3C_C11_SHARD runs and what they are built from"
    if (( ${#SHARD_TARGETS[@]} > 0 )); then
        sce_gate_build "$BUILD_DIR" --target "${SHARD_TARGETS[@]}" \
            || sce_gate_fail "shard $SCE_W3C_C11_SHARD build"
    fi
else
    sce_gate_step "building sce_c11_tests"
    sce_gate_build "$BUILD_DIR" --target sce_c11_tests \
        || sce_gate_fail "sce_c11_tests build"
fi

LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

# `--no-tests=error` covers the zero case even if the floor above is ever
# loosened; the count comparison below covers a partial run. A shard adds its
# selection (`-R` naming its tests), within the same label.
ctest --test-dir "$BUILD_DIR" -L c11 ${RUN_SELECTION[@]+"${RUN_SELECTION[@]}"} \
      --output-on-failure --no-tests=error -j "$(sce_build_jobs)" 2>&1 | tee "$LOG/ctest.log"
ctest_status="${PIPESTATUS[0]}"
(( ctest_status == 0 )) || sce_gate_fail "c11 ctest suite failed"

executed="$(grep -oE 'out of [0-9]+' "$LOG/ctest.log" | tail -1 | grep -oE '[0-9]+')"
if [[ "$executed" != "$expected" ]]; then
    sce_gate_fail "ran ${executed:-0} of $expected registered c11 test(s)${SHARD_PLAN:+ of shard $SCE_W3C_C11_SHARD}"
fi

sce_gate_step "$executed c11 test(s) passed${SHARD_PLAN:+ in shard $SCE_W3C_C11_SHARD}"

# What this shard ran, for the lane's last job to hold beside the other
# shards' (`ctest_shards.py verify`).
if [[ -n "$SHARD_PLAN" ]]; then
    python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" \
        --executed "$executed" --disabled 0
fi
