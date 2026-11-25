#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The C++ W3C programs built with emscripten and run under Node: the same 404
# cases `w3c-cpp` runs natively (Interpreter and AOT), one program per test
# number, and the interactive suite that steps every fixture forward, back and
# to its reset — the ctest label `w3c_cpp` (tests/w3c/W3CPrograms.cmake).
#
# In both emscripten trees (lib.sh, the two emscripten trees), because the
# engine is a different one in each (sce/include/common/ThreadSupport.h):
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
# Every run is judged by what the programs write as well as by how they exit,
# by the rules `w3c-cpp` uses natively (`sce_gate_w3c_cpp_suite`, lib.sh).
#
# ONE TREE, when `SCE_W3C_WASM_TREE` names it (`threaded` or `host-driven`),
# and ONE SHARD of that tree's suite, when `SCE_W3C_WASM_SHARD=k/N` asks for
# one: w3c-tests.yml runs each tree as N jobs, since each tree spent about 22
# minutes building the one program that held every fixture and a minute
# running it (job 138891691350), and the two trees ran one after the other.
# scripts/lib/ctest_shards.py plans the part as `w3c-cpp` plans its own, and
# the lane's last job holds each tree's parts together. Unset, the gate builds
# and runs every test of the label in both trees, which is what the build
# machines and a developer run.
#
# Skip-capable: emscripten (and the Node it ships) is a toolchain a developer
# may not have. The lane sets SCE_REQUIRE_TOOLS, so the skip that keeps a local
# run going is a failure where the check is claimed to have run.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

sce_gate_activate_emsdk
sce_gate_requires_tool emcmake emsdk || exit 0
sce_gate_requires_tool node emsdk || exit 0

# The fixture listener is started by the program's own Node host
# (tests/w3c/wasm_polyfill.js), for every program, on the port SCE_W3C_HTTP_PORT
# names or else one the kernel assigns; the host then sets SCE_W3C_HTTP_PORT to
# the port it bound, and the program reads it from the host's environment, which
# NODERAWFS makes the program's own. No port need be free, and programs run side
# by side each start their own.

# The configure asks the generator for its plans and refuses a stale one.
sce_gate_codegen >/dev/null \
    || sce_gate_fail "could not provide sce-codegen, which configuring the emscripten trees requires"

case "${SCE_W3C_WASM_TREE:-}" in
    "") TREES=(threaded host-driven) ;;
    threaded | host-driven) TREES=("$SCE_W3C_WASM_TREE") ;;
    *) sce_gate_fail "SCE_W3C_WASM_TREE is '$SCE_W3C_WASM_TREE', which names neither tree (threaded, host-driven)" ;;
esac
# A shard is a part of one tree's suite: the two trees plan apart.
if [[ -n "${SCE_W3C_WASM_SHARD:-}" && -z "${SCE_W3C_WASM_TREE:-}" ]]; then
    sce_gate_fail "SCE_W3C_WASM_SHARD names a shard of one tree's suite, and SCE_W3C_WASM_TREE names no tree"
fi

# Configure one tree, then build, run and judge its suite, whole or the shard
# SCE_W3C_WASM_SHARD names.
#   $1  the tree: threaded or host-driven
w3c_wasm_run_tree() {
    local tree="$1" dir configure
    case "$tree" in
        threaded) dir="$SCE_WASM_BUILD_DIR" configure=sce_gate_configure_wasm_tree ;;
        host-driven) dir="$SCE_WASM_WEB_BUILD_DIR" configure=sce_gate_configure_wasm_web_tree ;;
    esac

    sce_gate_step "configuring $dir (emscripten, $tree engine, Release)"
    "$configure" \
        || sce_gate_fail "WASM emcmake configure ($dir)"
    sce_prune_ctest_temporaries "$dir"

    # The floor is the native suite's: the WASM configure registers the same
    # set from the same registry, 200 test numbers and the interactive suite.
    local registered
    registered="$(ctest --test-dir "$dir" -N -L w3c_cpp | grep -cE '^ *Test +#' || true)"
    sce_gate_step "registered w3c_cpp tests in $dir: $registered"
    if (( registered < 200 )); then
        sce_gate_fail "only $registered w3c_cpp test(s) registered in $dir (expected at least 200) — the gate would report on a smaller set than its name claims"
    fi

    # One shard of this tree's suite, planned as `w3c-cpp` plans its own
    # (`--expect` holds the helper to the count above).
    local plan="" plan_status=0
    if [[ -n "${SCE_W3C_WASM_SHARD:-}" ]]; then
        plan="$dir/ctest-shard-w3c-wasm"
        rm -rf "$plan"
        python3 scripts/lib/ctest_shards.py plan --build "$dir" --label w3c_cpp \
            --expect "$registered" --out "$plan" "$SCE_W3C_WASM_SHARD" || plan_status=$?
        case "$plan_status" in
            0) ;;
            3) sce_gate_cannot_run "scripts/lib/ctest_shards.py could not plan shard $SCE_W3C_WASM_SHARD of $dir on this machine (above)" ;;
            *) sce_gate_fail "scripts/lib/ctest_shards.py refused to plan shard $SCE_W3C_WASM_SHARD of $dir (above)" ;;
        esac
        if (( $(grep -c . "$plan/tests" || true) == 0 )); then
            python3 scripts/lib/ctest_shards.py report --out "$plan" --executed 0 --disabled 0
            sce_gate_step "shard $SCE_W3C_WASM_SHARD of $dir holds no test; the other shards run the suite"
            return 0
        fi
    fi

    # The joined report and the ctest log go into the tree's tests directory
    # (gitignored), where the lane uploads them from.
    sce_gate_w3c_cpp_suite "$dir" "$plan" "$dir/tests" \
        "$dir, the $tree engine, under Node${plan:+ (shard $SCE_W3C_WASM_SHARD)}"
    sce_gate_step "$SCE_GATE_W3C_CASES W3C case(s) passed, $SCE_GATE_W3C_SKIPPED skipped, under Node, $tree engine${plan:+, shard $SCE_W3C_WASM_SHARD}"

    # What this shard ran, for the lane's last job to hold beside the other
    # shards' of the tree (`ctest_shards.py verify`).
    if [[ -n "$plan" ]]; then
        python3 scripts/lib/ctest_shards.py report --out "$plan" \
            --executed "$SCE_GATE_W3C_EXECUTED" --disabled 0
    fi
}

for tree in "${TREES[@]}"; do
    w3c_wasm_run_tree "$tree"
done
