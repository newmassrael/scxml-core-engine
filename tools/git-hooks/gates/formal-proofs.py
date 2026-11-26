# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `formal-proofs`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The proofs of the host core under Verus, as docs/formal/proofs.json declares them
# (docs/adr/0115, D13).
#
# `ci_only`: the push set runs on a developer's machine, which need not carry Verus
# and the Rust release it runs on (bx.toml declares both for the build machines).
{
    "workflows": ["formal-proofs.yml"],
    "runner_workflow": True,
    # The judge is `target/debug/sce-codegen formal` (docs/adr/0146).
    "deps": ["codegen-build"],
    "extra": ["docs/formal/proofs.json", "sce-build/src/formal/**",
              "backends/rust/mesh/src/https/**"],
    "ci_only": "22s measured 2026-11-23 on a host that has Verus, and the push set runs "
               "on a developer's machine, which need not carry it or the Rust release it "
               "runs on. formal-proofs.yml runs it on every change to a proved file or its "
               "judge, and bx.toml declares both tools for the build machines.",
    "cost_s": 22,
    "summary": "Verus proofs of the host core, with each break of it refused",
    # Verus answers one way for one input: the file, the manifest and the pinned tool.
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 2,
    "parallelism": PARALLELISM_SCRIPT,
}
