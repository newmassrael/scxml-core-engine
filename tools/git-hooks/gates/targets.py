# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `targets`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The logical interface every target shares, and the library built from it. The generator and
# a compiler are needed (the parity cases build and run C++), so unlike authoring-core's
# Python-only job this one has no run without them: the gate refuses to start.
{
    "workflows": ["targets.yml"],
    "runner_workflow": True,
    "ci_only": "81s with the generator built at opt 1: the interface cases are seconds each, "
               "the three parity cases build and run a C++ library. Not taken by "
               "`scripts/gate --measure`, which is refused locally for this repository.",
    "cost_s": 81,
    "summary": "the logical interface is platform-free, and the library judges as verify does",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
