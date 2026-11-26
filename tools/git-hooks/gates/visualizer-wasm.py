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
    # Its script builds one of its three parts when SCE_VISUALIZER_WASM_SHARD
    # names `k/N` (the `VISUALIZER_WASM_PARTS` lines of
    # scripts/gates/visualizer-wasm.sh), as the lane's three jobs do: one after
    # another they took 15.2 min of a 25.1 min hosted job (job 139033453523),
    # 8.9 of it the codegen WASM. No `shard_env`, so a plan runs it whole:
    # `visualizer-layout` depends on it, and the unit that runs the layout
    # builds what it measures in its own checkout first, where a part built
    # by another unit on another machine is not there to measure.
    "ci_only": "77s measured 2026-08-22 (declared 42) building three "
               "WASM artifacts. deploy-visualizer.yml runs it, and this "
               "gate declares no trigger of its own beyond its image, so "
               "the workflow's filter IS the trigger — the two cannot "
               "disagree.",
    "cost_s": 77,
    "summary": "codegen + visualizer + DOOM WASM builds",
    # It runs in the host image wherever it runs (ADR 0145): the runner runs
    # `scripts/gate` inside the image its Containerfile addresses, so the
    # hosted lane, a build machine and a workstation judge it in one
    # environment, and no lane installs a package for it. A machine takes its
    # unit only where docker runs (`needs`).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # What its lane sets for it, handed into the image (`container_env`):
    # a missing emsdk as a failure rather than a skip, and the part a
    # lane job builds.
    "container_env": ["SCE_REQUIRE_TOOLS", "SCE_VISUALIZER_WASM_SHARD"],
    # The SDK and wasm-pack are the image's, at the releases it pins, and the
    # image is addressed by its tracked Containerfile.
    "cache": True,
    "kept_only_from_a_landing": "the emscripten half runs only where emsdk is "
                            "found, so a pass without it judged one "
                            "artifact of three",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
