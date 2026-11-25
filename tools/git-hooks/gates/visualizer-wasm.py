# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `visualizer-wasm`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["deploy-visualizer.yml"],
    # The page's engine is the main tree built with emscripten, and the
    # codegen WASM is `sce-build-wasm` built by wasm-pack.
    "builds": ["main-tree", "codegen-wasm"],
    "runner_workflow": True,
    "ci_only": "77s measured 2026-08-22 (declared 42) building three "
               "WASM artifacts. deploy-visualizer.yml runs it, and this "
               "gate declares no trigger of its own, so the workflow's "
               "filter IS the trigger — the two cannot disagree.",
    "cost_s": 77,
    "summary": "codegen + visualizer + DOOM WASM builds",
    # The SDK is bx.toml's pinned emsdk tool, wasm-pack a need of the
    # platform.
    "cache": True,
    "kept_only_from_a_landing": "the emscripten half runs only where emsdk is "
                            "found, so a pass without it judged one "
                            "artifact of three",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
