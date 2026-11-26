# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `codec-no-alloc`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["sce-forge-codec-no-alloc.yml"],
    "runner_workflow": True,
    # As `codec-clippy`: the goldens compile against the runtime's build.
    "builds": ["forge-runtime-rust"],
    "ci_only": "90s. sce-forge-codec-no-alloc.yml runs it, next to its "
               "alloc-on sibling.",
    # The text algorithms it compiles are read from a materialization of
    # `rust-static-datamodel` (docs/adr/0097), which sce-codegen writes.
    "deps": ["codegen-build"],
    # Runs in the host image (tools/ci-images/host, docs/adr/0145), with the
    # generator it builds there and the libxml2 that build links.
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    "cost_s": 90,
    "summary": "generated codecs compile without alloc",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
