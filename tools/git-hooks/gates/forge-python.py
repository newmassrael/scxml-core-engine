# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `forge-python`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["forge-conformance.yml"],
    "runner_workflow": True,
    "extra": ["backends/python/**", "tools/ci-images/host/**"],
    # The host image (ADR 0145, family `host`): the toolchains of the arm and
    # the packages the generator links against, the same wherever the gate
    # runs. The runner pulls the image its Containerfile's digest names and
    # runs `scripts/gate forge-python` inside it, the generator build among its
    # deps ("Gates that run in an image" in tools/git-hooks/gate_registry.py).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    "deps": ["codegen-build"],
    "ci_only": "107s. forge-conformance.yml verifies every language arm "
               "in parallel jobs, which is where this belongs — the arms "
               "fire together anyway, so paying for them serially at push "
               "time bought attribution and nothing else.",
    "cost_s": 107,
    "summary": "Python forge conformance (numerical)",
    "cache": False,
    "no_cache_reason": "samples thread interleavings: its queue and "
                       "broadcast-log stress runs record what the machine's "
                       "scheduler happened to run, so a pass is one draw and "
                       "keeping it would stop the drawing",
    "inputs": None,
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
