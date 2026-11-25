# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `cpp-suite-wasm`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The emscripten tree, all of it: every target built, every registered
# ctest case run under Node. Measured 2026-11-05: the tree declared 388
# programs and the two gates above built three of them, and
# `integration_tests` could not build there at all.
#
# ⚠ `cost_s` is HALF a reading. The first run built the tree cold in 3328s
# on a hosted runner and failed seven links before ctest started, so the
# Node half is not in the figure; the arrival estimate it replaces, 2400s
# scaled from the `w3c-wasm` lane, was low by the build alone. The first
# green run reads the whole.
{
    "workflows": ["cpp-suite.yml"],
    "builds": ["main-tree"],
    "runner_workflow": True,
    "narrows": "the same catch-all reading as `cpp-suite`, whose "
               "WebAssembly half this is: it builds the whole emscripten "
               "tree, so its inputs are the engine, its tests, the CMake "
               "that registers them (the root list carries the WASM "
               "settings and says what the tree does not declare), the "
               "generator and its templates, and the documents.",
    # The emsdk pin the build machines install for it comes first.
    "extra": ["scripts/install_emsdk.sh",
              "sce/**", "tests/**", "cmake/**", "CMakeLists.txt",
              "resources/**", "integration_resources/**",
              "tools/codegen/templates/**", "sce-build/src/**"],
    "deps": ["codegen-build"],
    "ci_only": "the whole emscripten tree, each suite linked with "
               "ASYNCIFY, and every case run under Node. Its first run "
               "(cpp-suite.yml run 41956988917, 2026-11-05) configured "
               "and built the tree cold in 3328s and stopped at seven "
               "links, so the figure is that build alone and the ctest "
               "half has no reading yet. It needs emsdk, which a push "
               "host is not required to carry. cpp-suite.yml runs it in "
               "a job of its own beside the native suite.",
    "cost_s": 3328,
    "summary": "C++ ctest suite built with emscripten, every case under Node",
    # The SDK and the Node it ships are bx.toml's pinned emsdk tool.
    "cache": True,
    "kept_only_from_a_landing": "it skips and exits 0 on a machine without "
                            "emsdk (`[platform.linux.tools.emsdk]`)",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
