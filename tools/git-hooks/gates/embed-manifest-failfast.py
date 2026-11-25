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
    # Kept only from a landing unit, where the skip below cannot pass, and
    # a landing's unit goes only to a machine with clang-19 installed.
    "cache": True,
    "kept_only_from_a_landing": "`scripts/test_emit_manifest_fail_fast.sh` "
                            "skips and exits 0 on a machine without "
                            "clang++-19",
    "packages": ["clang-19"],
    # Read by `check learn` (2026-11-24, bx 340e9f9e): the fail-fast case,
    # the emitter it drives and the two Python modules the emitter runs.
    # The run lists `scripts/`. The case writes its header tree under a
    # temporary directory, so nothing of `sce/include` is read.
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
