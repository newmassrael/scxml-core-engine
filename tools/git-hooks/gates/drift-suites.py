# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `drift-suites`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["drift-verify.yml"],
    "runner_workflow": True,
    "ci_only": "146s. It is declared serial on purpose and that is what "
               "it costs; drift-verify.yml runs it.",
    "cost_s": 146,
    "summary": "committed-tree drift + sourcemap, serial",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
