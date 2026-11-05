#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# The two emscripten trees, each built whole: `build_wasm/` with threads (the
# engine's suites and the W3C runner, under Node) and `build_wasm_web/`
# without them (the browser visualizer).
#
# They are the trees the gates build in (`cpp-suite-wasm`, `w3c-wasm`,
# `visualizer-wasm`), configured by the helpers they configure them with. This
# script used to spell its own configure: Makefiles where the gates chose
# Ninja, the examples off where they were on, and the tree a gate had built
# deleted first. Two spellings of one configure are two trees in one directory
# (lib.sh, the two emscripten trees), and a gate run after this script met a
# tree configured for another generator.
#
# Run the suites it built with `scripts/gate cpp-suite-wasm`, which builds the
# same targets first.

source "$(dirname "${BASH_SOURCE[0]}")/gates/lib.sh"

sce_gate_activate_emsdk
command -v emcmake >/dev/null 2>&1 \
    || sce_gate_fail "emscripten not found: install emsdk at ~/emsdk, or set EMSDK to the directory that holds emsdk_env.sh"

# The configure asks the generator for its plans and refuses a stale one.
sce_gate_codegen >/dev/null \
    || sce_gate_fail "could not provide sce-codegen, which configuring $SCE_WASM_BUILD_DIR requires"

sce_gate_step "configuring $SCE_WASM_BUILD_DIR (emscripten, threads, Release)"
sce_gate_configure_wasm_tree \
    || sce_gate_fail "WASM emcmake configure ($SCE_WASM_BUILD_DIR)"

sce_gate_step "building every target of $SCE_WASM_BUILD_DIR"
cmake --build "$SCE_WASM_BUILD_DIR" --parallel "$(sce_build_jobs)" \
    || sce_gate_fail "WASM build ($SCE_WASM_BUILD_DIR)"

sce_gate_step "configuring $SCE_WASM_WEB_BUILD_DIR (emscripten, no threads, Release)"
sce_gate_configure_wasm_web_tree \
    || sce_gate_fail "WASM emcmake configure ($SCE_WASM_WEB_BUILD_DIR)"

sce_gate_step "building every target of $SCE_WASM_WEB_BUILD_DIR"
cmake --build "$SCE_WASM_WEB_BUILD_DIR" --parallel "$(sce_build_jobs)" \
    || sce_gate_fail "WASM build ($SCE_WASM_WEB_BUILD_DIR)"

sce_gate_step "built $SCE_REPO_ROOT/$SCE_WASM_BUILD_DIR and $SCE_REPO_ROOT/$SCE_WASM_WEB_BUILD_DIR; the visualizer is tests/visualizer.{js,wasm} in the second"
