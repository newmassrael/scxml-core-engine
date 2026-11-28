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
    # `ci_only` again, by the owner's decision of 2026-11-25 (a landing
    # judges the set that fits the budget; a red found after landing is
    # repaid by a commit of its own). It was local while the landing queue
    # judged everything: on 2026-11-20 item 290 landed two
    # `explicit_auto_deref` errors and main stayed red until item 298 —
    # the trade the owner's decision accepts. Where the queue judges it, it
    # takes 260s (and 1621s on a machine short of memory), which the
    # budget cannot hold beside the rest.
    "ci_only": "260s where the landing queue judges it, which the push budget "
               "cannot hold beside the rest; clippy-check.yml runs it after landing.",
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
