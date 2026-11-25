# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `mutation-cases`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# Rides the same unfiltered workflow, for the same reason in a different
# alphabet: what this gate reads is the union of every `mutation_targets`
# declaration inside `sce-build/tests/mutations/*.cases`, which today
# spans `sce/`, `sce-build/`, `backends/`, generator templates, a
# CMakeLists, two workflows and a gate script. A `paths:` list restating
# that union would be a second copy of a declaration the casefiles
# already carry, stale the first time a case names a new file.
{
    "workflows": ["tree-hygiene.yml"],
    "runner_workflow": True,
    "ci_only": "168s checking 500 cases across 72 casefiles, and it is "
               "ALWAYS-ON — its workflow is unfiltered, so every push paid "
               "it. Declared 33s until it was measured in a run of its "
               "own: that reading came from a full sweep where earlier "
               "gates had warmed everything, and a push runs a subset "
               "where each gate pays its own warm-up. tree-hygiene.yml is "
               "unfiltered too, so CI runs it on every push regardless.",
    "cost_s": 168,
    "summary": "every mutation casefile still applies",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
