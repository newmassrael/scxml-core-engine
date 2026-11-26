# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `embed-manifest-failfast`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# Guards the clang degraded-AST regression in the manifest emitter.
# Its own lane in the embed workflow rather than a step in the smoke
# job: it needs clang and a second, and the smoke build takes seven
# minutes. Until that lane existed the emitter was outside every CI
# trigger — `embed-vendor-smoke.yml` filtered on package/verify but
# not on `emit_embed_manifest.sh` — so this ran at push time and
# nowhere else, which `--no-verify` skips.
{
    "workflows": ["embed-vendor-smoke.yml"],
    "runner_workflow": True,
    "cost_s": 0,
    "summary": "emit_embed_manifest fail-fast case",
    # It runs in the host image wherever it runs (ADR 0145): the runner runs
    # `scripts/gate` inside the image its Containerfile addresses, so the
    # hosted lane, a build machine and a workstation judge it in one
    # environment, and no lane installs a package for it. A machine takes its
    # unit only where docker runs (`needs`).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # What its lane sets for it, handed into the image (`container_env`):
    # a missing clang-19 as a failure rather than a skip.
    "container_env": ["SCE_REQUIRE_TOOLS"],
    # Kept only from a landing unit, where the skip below cannot pass; the
    # image the gate runs in holds clang-19.
    "cache": True,
    "kept_only_from_a_landing": "`scripts/test_emit_manifest_fail_fast.sh` "
                            "skips and exits 0 on a machine without "
                            "clang++-19",
    # Read by `check learn` (2026-11-24, bx 340e9f9e): the fail-fast case,
    # the emitter it drives and the two Python modules the emitter runs.
    # The run lists `scripts/`. The case writes its header tree under a
    # temporary directory, so nothing of `sce/include` is read. The image's
    # build context is in the unit's key through `unit_inputs`.
    "inputs": [
        "scripts/",
        "scripts/_embed_manifest_filter.py",
        "scripts/embed_manifest.py",
        "scripts/emit_embed_manifest.sh",
        "scripts/test_emit_manifest_fail_fast.sh",
    ],
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
