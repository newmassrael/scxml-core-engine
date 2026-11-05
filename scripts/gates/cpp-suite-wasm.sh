#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
# BasicHTTP fixture listener on the endpoint's port (w3c-wasm says why a
# listener already there is refused).
sce_gate_requires_free_http_port

# The configure asks the generator for its plans and refuses a stale one.
sce_gate_codegen >/dev/null \
    || sce_gate_fail "could not provide sce-codegen, which configuring $SCE_WASM_BUILD_DIR requires"

sce_gate_step "configuring $SCE_WASM_BUILD_DIR (emscripten, Release)"
sce_gate_configure_wasm_tree \
    || sce_gate_fail "WASM emcmake configure"

# Every failing edge, not the first: ninja is told to go on past a failure, and
# each failed target is named, so one run says how much of the tree does not
# build rather than one defect per run. The log is kept in the build tree
# (gitignored), where the lane uploads it.
sce_gate_step "building every target of $SCE_WASM_BUILD_DIR (emscripten)"
BUILD_LOG="$SCE_WASM_BUILD_DIR/cpp_suite_wasm_build.log"
KEEP_GOING=(-k)
grep -q '^CMAKE_GENERATOR:INTERNAL=Ninja$' "$SCE_WASM_BUILD_DIR/CMakeCache.txt" \
    && KEEP_GOING=(-k 0)
if ! cmake --build "$SCE_WASM_BUILD_DIR" --parallel "$(sce_build_jobs)" \
        -- "${KEEP_GOING[@]}" >"$BUILD_LOG" 2>&1; then
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
    sce_gate_fail "the emscripten tree does not build (full log: $BUILD_LOG)"
fi

count_registered() {
    ctest --test-dir "$SCE_WASM_BUILD_DIR" -N | grep -cE '^ *Test +#'
}
registered="$(count_registered)"
sce_gate_step "registered tests: $registered"

# The cases run through the emulator the emscripten toolchain file names
# (CMAKE_CROSSCOMPILING_EMULATOR, Node), so a case registered with its target
# as the command runs the target's `.js` under Node.
LOG="$SCE_WASM_BUILD_DIR/cpp_suite_wasm_ctest.log"
ctest_status=0
ctest --test-dir "$SCE_WASM_BUILD_DIR" \
      --output-on-failure --no-tests=error -j "$(sce_build_jobs)" >"$LOG" 2>&1 \
    || ctest_status=$?
if (( ctest_status != 0 )); then
    sed -n '/tests passed,/,$p' "$LOG" >&2
    sce_gate_fail "the ctest suite failed under Node (full log: $LOG)"
fi

# Nothing goes missing silently: a case either ran or declared itself
# disabled, as in `cpp-suite`.
executed="$(grep -oE 'out of [0-9]+' "$LOG" | tail -1 | grep -oE '[0-9]+')"
disabled="$(sed -n '/The following tests did not run:/,$p' "$LOG" \
    | grep -cE '\(Disabled\)' || true)"
if (( ${executed:-0} + disabled != registered )); then
    sce_gate_fail "ran ${executed:-0} and skipped $disabled disabled of $registered registered test(s) under Node — $((registered - ${executed:-0} - disabled)) neither ran nor declared themselves disabled"
fi
sce_gate_step "$executed test(s) passed under Node ($disabled disabled)"
