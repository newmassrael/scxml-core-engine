# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `doc-content`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The lane that starts on a compiled-in INPUT rather than on Rust.
#
# `workspace-tests` runs the same library assertions, but its filter is
# narrow on purpose — it is 151s, half the push budget, and widening it
# to reach `sce/include/**`, `backends/**` and `tools/codegen/**` would
# start it on nearly every push. So the wide filter lives here, on a
# lane that only builds the library suite. Both gates declare the same
# `include_str_roots`, and `include-str-coverage` checks their UNION:
# what has to hold is that SOME lane running the suite starts for the
# path, which is exactly what splitting by cost is allowed to satisfy.
{
    "workflows": ["doc-content-gate.yml"],
    "runner_workflow": True,
    "include_str_roots": ["sce-build"],
    "ci_only": "the library suite plus its build, on a filter wide "
               "enough to reach every compiled-in input. It is here "
               "rather than in the push because the push already "
               "delegates the same assertions through workspace-tests; "
               "what this lane adds is the paths that lane does not "
               "start for. doc-content-gate.yml runs it.",
    "cost_s": 60,
    "summary": "sce-build lib suite, compiled-in inputs",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
