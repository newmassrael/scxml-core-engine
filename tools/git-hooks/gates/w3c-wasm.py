# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `w3c-wasm`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The W3C runner built with emscripten and run under Node, and the
# interactive suite beside it. Nothing ran that route from 2025-11-16 until
# this gate, and it rotted: seven defects, each invisible to every native
# build, repaired in b48c4f8bc1..48f14e6e6c.
#
# ⚠ `cost_s` is a SUM of three readings taken through `bx --local` on
# 2026-11-04 with three build slots, before the script existed: an
# incremental relink of both programs (60s), the W3C run (49s) and the
# interactive run (4s). The script itself has not been timed, so the slug
# is absent from COST_MEASURED. A cold configure and build of the tree was
# about 600s wall (some 3000 cpu-s, the generator's build included) on the
# same day. Its `peak_gb` was read off that cold build.
#
# Since 2026-11-05 it runs both programs in BOTH emscripten trees, the
# threaded engine's and the host-driven engine's (the browser's), so a run
# does that work twice and `cost_s` is twice the sum above: still a sum,
# still not a timing of the script.
{
    "workflows": ["w3c-tests.yml"],
    "builds": ["main-tree"],
    "runner_workflow": True,
    "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
               "what the native runner reads, plus the CMake that "
               "configures the two emscripten trees and the Node host the "
               "programs are linked with (tests/w3c/wasm_polyfill.js).",
    # What the two programs are built from and run against: the engine,
    # the runner and its Node host (tests/w3c/), the mock the runner
    # links, the CMake that configures the tree and registers the
    # fixtures (the root list carries the WASM link settings), the
    # generator and its templates, and the documents. And the emsdk pin
    # the build machines install it with (`scripts/install_emsdk.sh`).
    "extra": ["scripts/install_emsdk.sh",
              "sce/**", "tests/w3c/**", "tests/mocks/**",
              "tests/CMakeLists.txt", "CMakeLists.txt", "cmake/**",
              "tools/codegen/templates/**", "sce-build/src/**",
              "resources/**"],
    "deps": ["codegen-build"],
    "ci_only": "226s warm for the two trees, summed from three readings "
               "of one tree rather than timed as a script, and about "
               "1200s cold. No other gate a push runs builds both "
               "emscripten trees, so the cold figure is the state a "
               "trigger usually finds them in, and it needs emsdk, which "
               "a push host is not required to carry. w3c-tests.yml "
               "declares no `paths:` filter, so CI runs this arm on every "
               "push, in a job of its own (`test-cpp-wasm`).",
    "cost_s": 226,
    "summary": "W3C conformance and the interactive suite, C++ "
               "Interpreter + AOT built with emscripten, under Node, on "
               "the threaded and the host-driven engine",
    # The SDK and the Node it ships are bx.toml's pinned emsdk tool.
    "cache": True,
    "kept_only_from_a_landing": "it skips and exits 0 on a machine without "
                            "emsdk (`[platform.linux.tools.emsdk]`)",
    "inputs": None,
    "commits": [],
    "peak_gb": 5.3,
    "parallelism": PARALLELISM_COMPILE,
}
