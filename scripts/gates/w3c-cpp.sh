#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
# The suite is the ctest label `w3c_cpp`: one program per W3C test number,
# each holding the AOT machines of that number's fixtures alone, and the
# interactive suite (tests/w3c/W3CPrograms.cmake). Two verdicts are read, not
# one: the status of each program and the JUnit reports they write, joined and
# held to every fixture of the registry on both engines. The rules are in
# `sce_gate_w3c_cpp_suite` (lib.sh), since `w3c-wasm` judges the emscripten
# builds of the same programs by them.
#
# SHARDED, when `SCE_W3C_CPP_SHARD=k/N` asks for one part: w3c-tests.yml runs
# the suite as N jobs, since building the one program that held every fixture
# was 41 of the job's 47 minutes and running it 51 seconds (job 138891691638).
# scripts/lib/ctest_shards.py reads the label's tests from ctest and their
# build closures from ninja, spreads them, and each job builds and runs only
# its part; the lane's last job holds the parts together (`ctest_shards.py
# verify`, and the joined reports against the registry). Unset, the gate
# builds every program of the label and runs every test, which is what the
# build machines and a developer run.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

sce_main_build_dir
BUILD_DIR="$SCE_MAIN_BUILD_DIR"

# A killed run leaves its log temporary behind forever; the gate that creates
# them is the one that clears them.
sce_prune_ctest_temporaries "$BUILD_DIR"

# What is registered is a fact of the configure, so it is counted before
# anything is built: a shard plans from it. 200 test numbers and the
# interactive suite today.
registered="$(ctest --test-dir "$BUILD_DIR" -N -L w3c_cpp | grep -cE '^ *Test +#' || true)"
sce_gate_step "registered w3c_cpp tests: $registered"
if (( registered < 200 )); then
    sce_gate_fail "only $registered w3c_cpp test(s) registered (expected at least 200) — the gate would report on a smaller set than its name claims"
fi

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

# ONE SHARD OF THE SUITE, when `SCE_W3C_CPP_SHARD=k/N` asks for one, planned
# from the configured tree before anything is built: the tests the same
# selector registers (`--expect` holds the helper to the count above), the
# outputs each one's command names, and the compile and link steps under each
# in ninja's graph. Every job computes the whole plan and writes it beside its
# own part, so the lane can hold the parts to one plan and to the whole suite.
SHARD_PLAN=""
if [[ -n "${SCE_W3C_CPP_SHARD:-}" ]]; then
    SHARD_PLAN="$BUILD_DIR/ctest-shard-w3c-cpp"
    rm -rf "$SHARD_PLAN"
    plan_status=0
    python3 scripts/lib/ctest_shards.py plan --build "$BUILD_DIR" --label w3c_cpp \
        --expect "$registered" --out "$SHARD_PLAN" "$SCE_W3C_CPP_SHARD" || plan_status=$?
    case "$plan_status" in
        0) ;;
        3) sce_gate_cannot_run "scripts/lib/ctest_shards.py could not plan shard $SCE_W3C_CPP_SHARD on this machine (above)" ;;
        *) sce_gate_fail "scripts/lib/ctest_shards.py refused to plan shard $SCE_W3C_CPP_SHARD (above)" ;;
    esac
    if (( $(grep -c . "$SHARD_PLAN/tests" || true) == 0 )); then
        python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" --executed 0 --disabled 0
        sce_gate_step "shard $SCE_W3C_CPP_SHARD holds no test; the other shards run the suite"
        exit 0
    fi
fi

sce_gate_w3c_cpp_suite "$BUILD_DIR" "$SHARD_PLAN" "$REPORT" \
    "the main tree${SHARD_PLAN:+ (shard $SCE_W3C_CPP_SHARD)}"

sce_gate_step "$SCE_GATE_W3C_CASES W3C case(s) passed, $SCE_GATE_W3C_SKIPPED skipped${SHARD_PLAN:+ in shard $SCE_W3C_CPP_SHARD}"

# What this shard ran, for the lane's last job to hold beside the other
# shards' (`ctest_shards.py verify`).
if [[ -n "$SHARD_PLAN" ]]; then
    python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" \
        --executed "$SCE_GATE_W3C_EXECUTED" --disabled 0
fi
