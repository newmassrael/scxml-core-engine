#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: deploy-visualizer.yml
#
# The WASM builds behind the published visualizer. A deploy workflow reads as
# "nothing to verify locally" and is not: three of its steps are builds, and a
# build that fails is a verdict on the commit. Its trigger includes
# `sce-build/**`, so a Rust change that every gate here passes can still turn
# this lane red — which is only visible after the push.
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

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# Both outputs are gitignored build products (`/web/visualizer/wasm/`,
# `build_wasm/`), so writing them here does not touch a tracked file, and
# writing them where the lane writes them is what lets the deploy steps keep
# working when this gate is the thing that produced them.
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

# `emcc` reaches PATH through `source emsdk_env.sh`, which the lane does in the
# same step that calls this gate.
if sce_gate_requires_tool emcmake emsdk; then
    sce_gate_step "building the visualizer (emscripten)"
    GENERATOR=()
    command -v ninja >/dev/null 2>&1 && GENERATOR=(-G Ninja)
    emcmake cmake -S . -B build_wasm -DCMAKE_BUILD_TYPE=Release \
        ${GENERATOR+"${GENERATOR[@]}"} -Wno-dev >/dev/null \
        || sce_gate_fail "visualizer emcmake configure"
    cmake --build build_wasm --target visualizer -j "$(nproc)" >/dev/null \
        || sce_gate_fail "visualizer WASM build"

    sce_gate_step "building the DOOM example (emscripten)"
    ( cd examples/doom_wasm && ./build.sh ) >/dev/null \
        || sce_gate_fail "DOOM WASM example build"
fi
