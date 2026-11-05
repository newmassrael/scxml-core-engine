#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The W3C runner built with emscripten and run under Node: the same 404 cases
# `w3c-cpp` runs natively (Interpreter and AOT), then the interactive suite
# that steps every fixture forward, back and to its reset.
#
# Both, in both emscripten trees (lib.sh, the two emscripten trees), because
# the engine is a different one in each (sce/include/common/ThreadSupport.h):
#
#   build_wasm      threads: the threaded engine, the one the native build runs
#   build_wasm_web  no threads: the host-driven engine, the one the browser
#                   visualizer runs, where a delayed event fires when the
#                   machine's thread polls and a BasicHTTP post runs on the
#                   thread that reads its answer
#
# Until the engine chose its model by whether the build has threads, the one
# tree there was ran the host-driven engine under programs linked with threads;
# a run in the threaded tree alone would leave the browser's engine with no
# run at all.
#
# Nothing ran this route from 2025-11-16 (fea003c12c) until this gate, and it
# rotted in that time: seven defects, from exception catching compiled into one
# link only to a BasicHTTP answer that lost its data, each invisible to every
# native build. A route nothing checks rots again; this is the check.
#
# Both programs are judged by what they print and write, not by the status
# alone. The W3C runner is judged by the rules `w3c-cpp` uses for the native
# one (`sce_gate_judge_w3c_report`). The interactive suite's summary lines are
# read as well as its status.
#
# Skip-capable: emscripten (and the Node it ships) is a toolchain a developer
# may not have. The lane sets SCE_REQUIRE_TOOLS, so the skip that keeps a local
# run going is a failure where the check is claimed to have run.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

sce_gate_activate_emsdk
sce_gate_requires_tool emcmake emsdk || exit 0
sce_gate_requires_tool node emsdk || exit 0

# The fixture listener is started by the program's own Node host
# (tests/w3c/wasm_polyfill.js) on the endpoint's port, for both programs, so a
# listener already there makes the BasicHTTP fixtures fail against somebody
# else's server. A port named in SCE_W3C_HTTP_PORT is the one both sides use:
# the host binds it, and the program reads it from the host's environment,
# which NODERAWFS makes the program's own.
sce_gate_requires_free_http_port

# The configure asks the generator for its plans and refuses a stale one.
sce_gate_codegen >/dev/null \
    || sce_gate_fail "could not provide sce-codegen, which configuring the emscripten trees requires"

# Configure one tree, build the two programs in it, run both and judge them.
#   $1  the build directory
#   $2  the function that configures it (lib.sh)
#   $3  the engine it builds, for the messages
w3c_wasm_run_tree() {
    local dir="$1" configure="$2" engine="$3"

    sce_gate_step "configuring $dir (emscripten, $engine engine, Release)"
    "$configure" \
        || sce_gate_fail "WASM emcmake configure ($dir)"

    sce_gate_step "building w3c_test_cli and comprehensive_interactive_test ($dir)"
    sce_gate_build "$dir" --target w3c_test_cli comprehensive_interactive_test \
        || sce_gate_fail "WASM build of w3c_test_cli and comprehensive_interactive_test ($dir)"

    # Each program writes into the directory it is started from, which is the
    # build tree's (gitignored): the runner its `w3c_test_results.xml`, the
    # `.html` beside it and `w3c_failed_test_logs/`, and this gate the two run
    # logs. The runner's three are removed first, so a run that dies before
    # writing cannot be judged by the report of an earlier one.
    local run_dir="$dir/tests"
    rm -rf "$run_dir/w3c_test_results.xml" "$run_dir/w3c_test_results.html" \
        "$run_dir/w3c_failed_test_logs"

    sce_gate_step "running the W3C conformance suite under Node, $engine engine (Interpreter + AOT)"
    local status=0
    ( cd "$run_dir" && node w3c_test_cli.js ) >"$run_dir/w3c_wasm_output.log" 2>&1 || status=$?

    # The floor is the native suite's: the WASM configure registers the same
    # set from the same registry, 404 cases today.
    sce_gate_judge_w3c_report "$run_dir/w3c_test_results.xml" \
        "$run_dir/w3c_wasm_output.log" "$status" 400
    sce_gate_step "$SCE_GATE_W3C_CASES W3C case(s) passed, $SCE_GATE_W3C_SKIPPED skipped, under Node, $engine engine"

    sce_gate_step "running the interactive suite under Node, $engine engine"
    local log="$run_dir/comprehensive_interactive_wasm_output.log"
    status=0
    ( cd "$run_dir" && node comprehensive_interactive_test.js ) >"$log" 2>&1 || status=$?

    # GoogleTest's summary: `[  PASSED  ] <n> tests.`, and a `[  FAILED  ]`
    # line per failing test. Read as well as the status, so the verdict is the
    # suite's own account of its cases and not only how Node ended.
    #
    # 928 pass today and 75 are skipped by design. The floor stands under the
    # 928 so a run that loaded fewer fixtures (each one it cannot find is a
    # skip, not a failure) is refused rather than reported as a smaller green
    # suite.
    if grep -q '^\[  FAILED  ]' "$log"; then
        grep '^\[  FAILED  ]' "$log" | head -n 20 >&2 || true
        sce_gate_fail "the interactive suite reported failing tests under Node ($engine engine)"
    fi
    if (( status != 0 )); then
        tail -n 30 "$log" >&2
        sce_gate_fail "the interactive suite exited $status under Node ($engine engine) while it reported no failing test"
    fi
    local passed
    passed="$(sed -n '/^\[  PASSED  ] [0-9]\{1,\} tests\{0,1\}\./{s/^\[  PASSED  ] \([0-9]\{1,\}\).*/\1/p;q;}' "$log")"
    passed="${passed:-0}"
    if (( passed < 900 )); then
        tail -n 30 "$log" >&2
        sce_gate_fail "only $passed interactive test(s) passed under Node ($engine engine; expected at least 900) — the suite covered less than its name claims"
    fi
    sce_gate_step "$passed interactive test(s) passed under Node, $engine engine"
}

w3c_wasm_run_tree "$SCE_WASM_BUILD_DIR" sce_gate_configure_wasm_tree threaded
w3c_wasm_run_tree "$SCE_WASM_WEB_BUILD_DIR" sce_gate_configure_wasm_web_tree host-driven
