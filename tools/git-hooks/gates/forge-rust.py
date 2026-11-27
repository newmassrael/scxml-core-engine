# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `forge-rust`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["forge-conformance.yml"],
    "runner_workflow": True,
    "extra": ["backends/rust/**", "tools/ci-images/host/**"],
    # The host image (ADR 0145, family `host`): the toolchains of the arm and
    # the packages the generator links against, the same wherever the gate
    # runs. The runner pulls the image its Containerfile's digest names and
    # runs `scripts/gate forge-rust` inside it, the generator build among its
    # deps ("Gates that run in an image" in tools/git-hooks/gate_registry.py).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    "deps": ["codegen-build"],
    # Warm reads as 0; the release profile it needs is a separate build
    # tree from every other gate, so a cold run pays that once.
    "ci_only": "96s, a release build of the Rust forge arm, and about "
               "three minutes more for the queue runtime, the Rust "
               "runtime's bounded inbox queue and the session pool under loom, "
               "Miri and ThreadSanitizer. Its siblings are already in CI "
               "for the same reason and they share one workflow, "
               "forge-conformance.yml.",
    "cost_s": 280,
    "summary": "Rust forge conformance (numerical + codec round-trips, release)",
    "cache": False,
    "no_cache_reason": "samples thread interleavings: its stress runs record "
                       "the histories the scheduler happened to produce and "
                       "ThreadSanitizer reports only the races it saw, so a "
                       "pass is one draw and keeping it would stop the drawing",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
