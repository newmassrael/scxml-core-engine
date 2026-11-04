#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The W3C runner built with emscripten and run under Node, the engine compiled
# the way the published visualizer's is: the same 404 cases `w3c-cpp` runs
# natively (Interpreter and AOT), then the interactive suite that steps every
# fixture forward, back and to its reset.
#
# Nothing ran this route from 2025-11-16 (fea003c12c) until this gate, and it
# rotted in that time: seven defects, from exception catching compiled into one
# link only to a BasicHTTP answer that lost its data, each invisible to every
# native build. A route nothing checks rots again; this is the check.
#
# Both programs are judged by what they print and write, not by the status
# alone. The W3C runner is judged by the rules `w3c-cpp` uses for the native
# one (`sce_gate_judge_w3c_report`). The interactive suite is GoogleTest linked
# with EXIT_RUNTIME=0, so its summary lines are read as well as its status.
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
# the host binds it and hands it to the program's environment.
sce_gate_requires_free_http_port

# The configure asks the generator for its plans and refuses a stale one.
sce_gate_codegen >/dev/null \
    || sce_gate_fail "could not provide sce-codegen, which configuring $SCE_WASM_BUILD_DIR requires"

sce_gate_step "configuring $SCE_WASM_BUILD_DIR (emscripten, Release)"
sce_gate_configure_wasm_tree \
    || sce_gate_fail "WASM emcmake configure"

sce_gate_step "building w3c_test_cli and comprehensive_interactive_test (emscripten)"
sce_gate_build "$SCE_WASM_BUILD_DIR" --target w3c_test_cli comprehensive_interactive_test \
    || sce_gate_fail "WASM build of w3c_test_cli and comprehensive_interactive_test"

# Each program writes into the directory it is started from, which is the
# build tree's (gitignored): the runner its `w3c_test_results.xml`, the `.html`
# beside it and `w3c_failed_test_logs/`, and this gate the two run logs. The
# runner's three are removed first, so a run that dies before writing cannot be
# judged by the report of an earlier one.
RUN_DIR="$SCE_WASM_BUILD_DIR/tests"
rm -rf "$RUN_DIR/w3c_test_results.xml" "$RUN_DIR/w3c_test_results.html" \
    "$RUN_DIR/w3c_failed_test_logs"

sce_gate_step "running the W3C conformance suite under Node (Interpreter + AOT)"
status=0
( cd "$RUN_DIR" && node w3c_test_cli.js ) >"$RUN_DIR/w3c_wasm_output.log" 2>&1 || status=$?

# The floor is the native suite's: the WASM configure registers the same set
# from the same registry, 404 cases today.
sce_gate_judge_w3c_report "$RUN_DIR/w3c_test_results.xml" \
    "$RUN_DIR/w3c_wasm_output.log" "$status" 400
sce_gate_step "$SCE_GATE_W3C_CASES W3C case(s) passed under Node"

sce_gate_step "running the interactive suite under Node"
log="$RUN_DIR/comprehensive_interactive_wasm_output.log"
status=0
( cd "$RUN_DIR" && node comprehensive_interactive_test.js ) >"$log" 2>&1 || status=$?

# GoogleTest's summary: `[  PASSED  ] <n> tests.`, and a `[  FAILED  ]` line
# per failing test. Read as well as the status because the program keeps its
# runtime alive after main() returns (EXIT_RUNTIME=0) and, unlike the W3C
# runner, does not force its exit, so the status Node exits with need not be
# the one main() returned.
#
# 928 pass today and 75 are skipped by design. The floor stands under the 928
# so a run that loaded fewer fixtures (each one it cannot find is a skip, not a
# failure) is refused rather than reported as a smaller green suite.
if grep -q '^\[  FAILED  ]' "$log"; then
    grep '^\[  FAILED  ]' "$log" | head -n 20 >&2 || true
    sce_gate_fail "the interactive suite reported failing tests under Node"
fi
if (( status != 0 )); then
    tail -n 30 "$log" >&2
    sce_gate_fail "the interactive suite exited $status under Node while it reported no failing test"
fi
passed="$(sed -n '/^\[  PASSED  ] [0-9]\{1,\} tests\{0,1\}\./{s/^\[  PASSED  ] \([0-9]\{1,\}\).*/\1/p;q;}' "$log")"
passed="${passed:-0}"
if (( passed < 900 )); then
    tail -n 30 "$log" >&2
    sce_gate_fail "only $passed interactive test(s) passed under Node (expected at least 900) — the suite covered less than its name claims"
fi
sce_gate_step "$passed interactive test(s) passed under Node"
