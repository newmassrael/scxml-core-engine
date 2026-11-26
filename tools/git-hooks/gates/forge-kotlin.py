# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `forge-kotlin`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# ⚠ The SIXTH arm, and it was the one the set was missing. Measured
# 2026-09-18: `forge-conformance.yml` has six jobs — rust, python, go,
# kotlin, cpp, c — and `scripts/gates/` mirrored five. Nothing local ran the
# Kotlin forge harness; `w3c-kotlin` runs a different Gradle project. So a
# break confined to the Kotlin forge lowering passed every local lane and
# went red only after the push, which is the half of `forge-c11`'s story with
# the sides swapped: there the code existed and nothing ran it, here CI ran
# it and the mirror did not. Five siblings made the set look complete.
#
# ⚠⚠ `ci_only` on purpose. 52s is over what the push budget will spend on a
# sixth forge arm, and its five siblings already settled that argument —
# forge-conformance.yml runs all six in parallel jobs, so paying serially at
# push time buys attribution and nothing else.
{
    "workflows": ["forge-conformance.yml"],
    "runner_workflow": True,
    "extra": ["backends/kotlin/**", "tools/ci-images/host/**"],
    # The host image (ADR 0145, family `host`): the toolchains of the arm and
    # the packages the generator links against, the same wherever the gate
    # runs. The runner pulls the image its Containerfile's digest names and
    # runs `scripts/gate forge-kotlin` inside it, the generator build among its
    # deps ("Gates that run in an image" in tools/git-hooks/gate_registry.py).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    "deps": ["codegen-build"],
    "ci_only": "52s, the Gradle jvmTest arm. Its five siblings are in CI "
               "for the same reason and they share one workflow, "
               "forge-conformance.yml.",
    "cost_s": 52,
    "summary": "Kotlin forge conformance (Gradle jvmTest)",
    "cache": False,
    "no_cache_reason": "samples thread interleavings: its queue and "
                       "broadcast-log stress runs record what the JVM's "
                       "threads happened to do, so a pass is one draw and "
                       "keeping it would stop the drawing",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
