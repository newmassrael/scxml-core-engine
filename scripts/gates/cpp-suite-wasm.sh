#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: cpp-suite.yml
#
# The emscripten tree, all of it: every target it declares built, and every
# ctest case it registers run under Node. It is the C++ engine's own suites
# compiled the way the published visualizer is, and the WebAssembly half of
# what `cpp-suite` runs natively.
#
# Measured 2026-11-05, and the measurement is why this exists. The tree
# declared 388 programs, and the two gates that built in it built three of
# them (`visualizer-wasm` one, `w3c-wasm` two). The other 385 were built by
# nothing, and `integration_tests` could not be: its AOT sources include
# machines the tree generated for native builds only. The suites carried their
# WebAssembly settings (`apply_sce_test_policies`, the ctest registrations)
# from when they ran under Node, and nothing had run them since.
#
# The default target, not a list, for the reason `cpp-suite` gives: a list is
# the defect this gate removes. A target the tree declares is a target this
# gate builds, and the tree declares under Emscripten only what has a
# WebAssembly purpose (the root CMakeLists.txt says why the C11 harness and the
# examples are native only).
#
# Skip-capable: emscripten (and the Node it ships) is a toolchain a developer
# may not have. The lane sets SCE_REQUIRE_TOOLS, so the skip that keeps a local
# run going is a failure where the check is claimed to have run.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

sce_gate_activate_emsdk
sce_gate_requires_tool emcmake emsdk || exit 0
sce_gate_requires_tool node emsdk || exit 0

# The registered set includes the W3C runner, whose Node host starts the
# BasicHTTP fixture listener on a port the kernel assigns and hands it to the
# program (tests/w3c/wasm_polyfill.js), so no port need be free.

# The configure asks the generator for its plans and refuses a stale one.
sce_gate_codegen >/dev/null \
    || sce_gate_fail "could not provide sce-codegen, which configuring $SCE_WASM_BUILD_DIR requires"

sce_gate_step "configuring $SCE_WASM_BUILD_DIR (emscripten, Release)"
sce_gate_configure_wasm_tree \
    || sce_gate_fail "WASM emcmake configure"

# What is registered is a fact of the configure, so it is counted before
# anything is built: a shard plans from it.
count_registered() {
    ctest --test-dir "$SCE_WASM_BUILD_DIR" -N | grep -cE '^ *Test +#'
}
registered="$(count_registered)"
sce_gate_step "registered tests: $registered"

# ONE SHARD OF THE SUITE, when `SCE_CPP_SUITE_WASM_SHARD=k/N` asks for one:
# cpp-suite.yml runs the suite as N jobs, since the tree alone built cold in
# 3328s on a hosted runner before any case ran (run 41956988917) and the job
# ran 4745s before it failed (job 137573089028). scripts/lib/ctest_shards.py
# plans from the configured tree before anything is built, as it does for
# `cpp-suite`: every test the tree registers (`--expect` holds it to the count
# above), the outputs each one's command, environment and required files name,
# and the compile and link steps under each in ninja's graph. Every job writes
# the whole plan beside its own part, so the lane's last job can hold the parts
# to one plan and to the whole suite (`ctest_shards.py verify`). The setup of
# the whole-tree build fixture is left out of the run: this gate builds the
# closure of what it runs, and that setup would build everything else.
#
# Unset, the gate builds the default target and runs every case, which is what
# the build machines and a developer run.
SHARD_PLAN=""
SHARD_TARGETS=()
RUN_SELECTION=()
expected="$registered"
if [[ -n "${SCE_CPP_SUITE_WASM_SHARD:-}" ]]; then
    SHARD_PLAN="$SCE_WASM_BUILD_DIR/ctest-shard"
    rm -rf "$SHARD_PLAN"
    plan_status=0
    python3 scripts/lib/ctest_shards.py plan --build "$SCE_WASM_BUILD_DIR" \
        --expect "$registered" --out "$SHARD_PLAN" "$SCE_CPP_SUITE_WASM_SHARD" || plan_status=$?
    case "$plan_status" in
        0) ;;
        3) sce_gate_cannot_run "scripts/lib/ctest_shards.py could not plan shard $SCE_CPP_SUITE_WASM_SHARD on this machine (above)" ;;
        *) sce_gate_fail "scripts/lib/ctest_shards.py refused to plan shard $SCE_CPP_SUITE_WASM_SHARD (above)" ;;
    esac
    expected="$(grep -c . "$SHARD_PLAN/tests" || true)"
    while IFS= read -r line; do
        [[ -n "$line" ]] && SHARD_TARGETS+=("$line")
    done <"$SHARD_PLAN/targets"
    if (( expected == 0 )); then
        python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" --executed 0 --disabled 0
        sce_gate_step "shard $SCE_CPP_SUITE_WASM_SHARD holds no test; the other shards run the suite"
        exit 0
    fi
    RUN_SELECTION=(-R "$(<"$SHARD_PLAN/run-regex")")
    fixture_regex="$(<"$SHARD_PLAN/fixture-regex")"
    if [[ -n "$fixture_regex" ]]; then
        RUN_SELECTION+=(--fixture-exclude-setup "$fixture_regex")
    fi
fi

# Every failing edge, not the first: ninja is told to go on past a failure, and
# each failed target is named, so one run says how much of the tree (or of the
# shard) does not build rather than one defect per run. The log is kept in the
# build tree (gitignored), where the lane uploads it. A shard builds the
# outputs its tests name, which ctest's own registration names, so a case added
# anywhere brings its target with it.
BUILD_LOG="$SCE_WASM_BUILD_DIR/cpp_suite_wasm_build.log"
KEEP_GOING=(-k)
grep -q '^CMAKE_GENERATOR:INTERNAL=Ninja$' "$SCE_WASM_BUILD_DIR/CMakeCache.txt" \
    && KEEP_GOING=(-k 0)
# A shard whose tests name no output (scripts alone) builds nothing.
BUILD_WANTED=1
BUILD_ARGS=()
BUILT="tree"
if [[ -n "$SHARD_PLAN" ]]; then
    BUILT="outputs of shard $SCE_CPP_SUITE_WASM_SHARD"
    sce_gate_step "building the ${#SHARD_TARGETS[@]} output(s) shard $SCE_CPP_SUITE_WASM_SHARD runs and what they are built from (emscripten)"
    if (( ${#SHARD_TARGETS[@]} > 0 )); then
        BUILD_ARGS=(--target "${SHARD_TARGETS[@]}")
    else
        BUILD_WANTED=0
    fi
else
    sce_gate_step "building every target of $SCE_WASM_BUILD_DIR (emscripten)"
fi
if (( BUILD_WANTED )) && ! cmake --build "$SCE_WASM_BUILD_DIR" ${BUILD_ARGS[@]+"${BUILD_ARGS[@]}"} \
        --parallel "$(sce_build_jobs)" -- "${KEEP_GOING[@]}" >"$BUILD_LOG" 2>&1; then
    failed_edges="$(grep -c '^FAILED:' "$BUILD_LOG" || true)"
    if (( failed_edges > 0 )); then
        printf '  [%s] %s edge(s) failed:\n' "$SCE_GATE_SLUG" "$failed_edges" >&2
        grep '^FAILED:' "$BUILD_LOG" >&2
        # Line ranges in `sed` rather than `| head`: under `pipefail` a reader
        # that closes early is a writer killed by SIGPIPE, which `cpp-suite`
        # measured as a red verdict with no failing test.
        first_failed="$(sed -n '/^FAILED:/{=;q;}' "$BUILD_LOG")"
        printf '  [%s] from the first FAILED edge:\n' "$SCE_GATE_SLUG" >&2
        sed -n "${first_failed},$((first_failed + 79))p" "$BUILD_LOG" >&2
    else
        tail -n 60 "$BUILD_LOG" >&2
    fi
    sce_gate_fail "the emscripten $BUILT did not build (full log: $BUILD_LOG)"
fi

# The cases run through the emulator the emscripten toolchain file names
# (CMAKE_CROSSCOMPILING_EMULATOR, Node), so a case registered with its target
# as the command runs the target's `.js` under Node. A shard adds its
# selection (`-R` naming its tests, and the whole-tree build fixture's setup
# left out).
LOG="$SCE_WASM_BUILD_DIR/cpp_suite_wasm_ctest.log"
ctest_status=0
ctest --test-dir "$SCE_WASM_BUILD_DIR" ${RUN_SELECTION[@]+"${RUN_SELECTION[@]}"} \
      --output-on-failure --no-tests=error -j "$(sce_build_jobs)" >"$LOG" 2>&1 \
    || ctest_status=$?
if (( ctest_status != 0 )); then
    sed -n '/tests passed,/,$p' "$LOG" >&2
    sce_gate_fail "the ctest suite failed under Node${SHARD_PLAN:+ in shard $SCE_CPP_SUITE_WASM_SHARD} (full log: $LOG)"
fi

# Nothing goes missing silently: a case either ran or declared itself
# disabled, as in `cpp-suite`.
executed="$(grep -oE 'out of [0-9]+' "$LOG" | tail -1 | grep -oE '[0-9]+')"
disabled="$(sed -n '/The following tests did not run:/,$p' "$LOG" \
    | grep -cE '\(Disabled\)' || true)"
if (( ${executed:-0} + disabled != expected )); then
    sce_gate_fail "ran ${executed:-0} and skipped $disabled disabled of $expected registered test(s) under Node${SHARD_PLAN:+ of shard $SCE_CPP_SUITE_WASM_SHARD} — $((expected - ${executed:-0} - disabled)) neither ran nor declared themselves disabled"
fi
sce_gate_step "$executed test(s) passed under Node ($disabled disabled)${SHARD_PLAN:+ in shard $SCE_CPP_SUITE_WASM_SHARD}"

# What this shard ran, for the lane's last job to hold beside the other
# shards' (`ctest_shards.py verify`).
if [[ -n "$SHARD_PLAN" ]]; then
    python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" \
        --executed "$executed" --disabled "$disabled"
fi
