# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `rustdoc-links`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# Mirrors doc-check.yml. The numbered table mapped this to
# sce-rust-runtime-no-std.yml while the gate's own comment named
# doc-check.yml; the two share the `backends/rust/runtime/**` trigger,
# so the mismatch never showed as a missed run except for edits to
# doc-check.yml itself.
{
    "workflows": ["doc-check.yml"],
    "runner_workflow": True,
    # 4s where the landing queue judges it: the median of its last ten
    # landing runs in the integrator's timings.tsv (2026-11-28), the seat the
    # budget bounds ("The push budget" in gate_registry.py). It read
    # 4s warm on a developer's tree.
    "cost_s": 4,
    "summary": "cargo doc broken intra-doc links, both profiles",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
