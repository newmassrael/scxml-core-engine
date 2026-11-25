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
    # ⚠ Local, not `ci_only`. It was moved out of the push when the push
    # was something a developer waited for. Since the landing queue
    # judges every item with this hook on the build fleet (bx RFC 0024),
    # a gate left out of the push is a gate left out of the merge queue:
    # on 2026-11-20 item 290 landed two `explicit_auto_deref` errors and
    # main stayed red until item 298. With `cache: True` a kept pass
    # costs nothing, and 82s fits the ceiling beside the local set.
    "cost_s": 82,
    "summary": "cargo clippy --workspace --all-targets",
    # The lint set moves with each stable release, which is why the plan
    # names the release in every unit's argv: a new one is a new key.
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
