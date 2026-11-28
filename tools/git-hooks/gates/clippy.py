# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `clippy`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["clippy-check.yml"],
    "runner_workflow": True,
    # Lint is the fastest feedback a Rust change can get. It was measured
    # at 15s, 42s and 82s in one afternoon on this machine, which is also
    # the clearest single illustration of why the numbers here cannot be
    # trusted to a factor of two.
    #
    # Local: the landing queue judges it. Out of the merge queue, on
    # 2026-11-20 item 290 landed two `explicit_auto_deref` errors and main
    # stayed red until item 298. Where the queue judges it, it takes 260s
    # (1621s once on a machine short of memory), which fits the ceiling as
    # the judgment's length on the build machines (docs/adr/0157) though not
    # as a sum.
    # 260s where the landing queue judges it: the median of its last ten
    # landing runs in the integrator's timings.tsv (2026-11-28), the seat the
    # budget bounds ("The push budget" in gate_registry.py). It read
    # 82s warm on a developer's tree.
    "cost_s": 260,
    "summary": "cargo clippy --workspace --all-targets",
    # The lint set moves with each stable release, which is why the plan
    # names the release in every unit's argv: a new one is a new key.
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
