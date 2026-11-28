# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `nostd-mcu`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["sce-rust-runtime-no-std.yml"],
    "runner_workflow": True,
    "deps": ["codegen-build"],
    # 9s, timed alone on a warm tree on 2026-09-02 and no longer a
    # normalised figure. Nine consecutive runs read 22 22 13 7 7 6 9 9 7:
    # the first three were still filling caches, and the settled band is
    # 6-9s. 9 is the worse of the last pair, which is the rule the budget
    # header sets ("the WORSE of two `--measure` runs") so that the
    # ceiling holds on unlucky pushes rather than lucky ones.
    # 32s where the landing queue judges it: the median of its last ten
    # landing runs in the integrator's timings.tsv (2026-11-28), the seat the
    # budget bounds ("The push budget" in gate_registry.py). It read
    # 9s warm on a developer's tree.
    "cost_s": 32,
    "summary": "no_std MCU build + clippy + probes; std runtime without http-send",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
