#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# The emscripten tree (`build_wasm/`), built whole: the engine library, the
# visualizer, the W3C runner and the engine's suites.
#
# It is the tree the gates build in (`visualizer-wasm`, `w3c-wasm`,
# `cpp-suite-wasm`), configured by the one helper they configure it with. This
# script used to spell its own configure: Makefiles where the gates chose
# Ninja, the examples off where they were on, and the tree a gate had built
# deleted first. Two spellings of one configure are two trees in one directory
# (lib.sh, `sce_gate_configure_wasm_tree`), and a gate run after this script
# met a tree configured for another generator.
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

sce_gate_step "configuring $SCE_WASM_BUILD_DIR (emscripten, Release)"
sce_gate_configure_wasm_tree \
    || sce_gate_fail "WASM emcmake configure"

sce_gate_step "building every target of $SCE_WASM_BUILD_DIR"
cmake --build "$SCE_WASM_BUILD_DIR" --parallel "$(sce_build_jobs)"

sce_gate_step "built $SCE_REPO_ROOT/$SCE_WASM_BUILD_DIR; the visualizer is tests/visualizer.{js,wasm} there"
