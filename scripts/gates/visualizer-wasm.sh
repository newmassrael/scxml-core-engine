#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: deploy-visualizer.yml
#
# The WASM builds behind the published visualizer. A deploy workflow reads as
# "nothing to verify locally" and is not: three of its steps are builds, and a
# build that fails is a verdict on the commit. Its trigger includes what
# cargo compiles of `sce-build`, so a Rust change that every gate here
# passes can still turn this lane red — which is only visible after the push.
#
# The codegen WASM is the part that has already failed this way. It used to be
# a committed binary under `web/visualizer/wasm/` that nothing in the
# repository could rebuild, because `sce-build` had no cdylib crate-type and
# wasm-pack refused it. The shipped copy drifted from the templates it embeds
# and rendered imports its own template list did not contain, so every browser
# generation failed while every native build passed.
#
# Emscripten is optional here and required in the lane: a developer without
# emsdk still gets the wasm-pack verdict, and the lane sets SCE_REQUIRE_TOOLS
# so the same skip is a failure where the check is claimed to have run.
#
# In parts (ADR 0120, D3). When SCE_VISUALIZER_WASM_SHARD names `k/N`, as the
# lane's jobs do, the gate builds the k-th line of VISUALIZER_WASM_PARTS below;
# N must be the number of lines. Unset, it builds every line, in this order. The three are independent builds of one artifact
# each, and one after another they took 15.2 min of a 25.1 min hosted job
# (job 139033453523, 2026-11-25): the codegen WASM 8.9, the engine 3.3 and the
# DOOM example 3.1. A plan runs it whole: `visualizer-layout` depends on it
# (tools/git-hooks/gates/visualizer-wasm.py says why).

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# Both outputs are gitignored build products (`/web/visualizer/wasm/`,
# `build_wasm_web/`), so writing them here does not touch a tracked file, and
# writing them where the lane writes them is what lets the deploy steps keep
# working when this gate is the thing that produced them.

part_codegen() {
    # wasm-pack is a cargo install, not a toolchain: a developer who changed the
    # codegen crate can get it in one command, and skipping here would report
    # green on the artifact the browser actually runs.
    command -v wasm-pack >/dev/null 2>&1 \
        || sce_gate_fail "wasm-pack is not on PATH (cargo install wasm-pack). This gate was selected because the codegen WASM's inputs changed, so a skip would report green on the artifact the browser runs."

    sce_gate_step "building the codegen WASM (wasm-pack)"
    wasm-pack build sce-build-wasm \
        --target web \
        --out-dir ../web/visualizer/wasm \
        --out-name sce_build \
        || sce_gate_fail "codegen WASM build — the browser generator would ship broken or stale"
}

# Whether this machine has an emscripten to build the two emscripten parts
# with. An emsdk that is installed and not active is found (lib.sh says why
# this matters for a local run).
emscripten_found() {
    sce_gate_activate_emsdk
    sce_gate_requires_tool emcmake emsdk
}

# The visualizer is built in the tree without threads: it runs in a page
# without SharedArrayBuffer, on the host-driven engine (lib.sh, the two
# emscripten trees).
part_engine() {
    emscripten_found || return 0
    sce_gate_step "building the visualizer (emscripten, no threads)"
    sce_gate_configure_wasm_web_tree \
        || sce_gate_fail "visualizer emcmake configure"
    sce_gate_build "$SCE_WASM_WEB_BUILD_DIR" --target visualizer \
        || sce_gate_fail "visualizer WASM build"
}

part_doom() {
    emscripten_found || return 0
    sce_gate_step "building the DOOM example (emscripten)"
    ( cd examples/doom_wasm && ./build.sh ) >/dev/null \
        || sce_gate_fail "DOOM WASM example build"
}

# The parts, one job of the lane each, each a `part_<name>` above;
# `hook_ci_parity` holds the lane's shard count to the number of lines.
VISUALIZER_WASM_PARTS=(
    "codegen"
    "engine"
    "doom"
)

PARTS=("${VISUALIZER_WASM_PARTS[@]}")
if [[ -n "${SCE_VISUALIZER_WASM_SHARD:-}" ]]; then
    [[ "$SCE_VISUALIZER_WASM_SHARD" =~ ^([0-9]+)/([0-9]+)$ ]] \
        || sce_gate_fail "SCE_VISUALIZER_WASM_SHARD is k/N, got '$SCE_VISUALIZER_WASM_SHARD'"
    shard="${BASH_REMATCH[1]}"
    count="${BASH_REMATCH[2]}"
    (( count == ${#VISUALIZER_WASM_PARTS[@]} )) \
        || sce_gate_fail "SCE_VISUALIZER_WASM_SHARD names $count shard(s), and the gate has ${#VISUALIZER_WASM_PARTS[@]} parts"
    (( shard >= 1 && shard <= count )) \
        || sce_gate_fail "SCE_VISUALIZER_WASM_SHARD names shard $shard of $count"
    read -r -a PARTS <<<"${VISUALIZER_WASM_PARTS[shard - 1]}"
    sce_gate_step "shard $shard of $count: ${PARTS[*]}"
fi

for part in "${PARTS[@]}"; do
    "part_$part"
done
