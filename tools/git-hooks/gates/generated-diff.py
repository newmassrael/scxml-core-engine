# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `generated-diff`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# What a change does to the generated code, rendered (ADR 0097, section 6),
# and the one verdict the rendering reaches: the head materializes every tree
# it declares. The lane ran `scripts/generated_diff.sh` itself, with packages
# it installed at job time, and the workflow was recorded in
# UNMIRRORED_WORKFLOWS as a publication; it runs this gate now, because a gate
# is how a lane enters the image its packages come from (docs/adr/0145).
{
    "workflows": ["generated-diff.yml"],
    "runner_workflow": True,
    # The generator, clang-format-19 and Go of the host image
    # (tools/ci-images/host), wherever the diff is rendered.
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # The commits the lane resolved, which the script renders between.
    "container_env": ["SCE_GENERATED_DIFF_BASE", "SCE_GENERATED_DIFF_HEAD"],
    "ci_only": "it materializes every declared tree twice, at a base and at the "
               "head, which is work proportional to the corpus and not to the "
               "change; generated-diff.yml renders it for every push and pull "
               "request and publishes the patches.",
    # Unmeasured as a gate. The lane's job builds the generator of each side
    # and materializes every tree on each; regen-reproduces, which
    # regenerates every committed tree once, is 117s.
    "cost_s": 240,
    "summary": "the generated diff of a change, and the head materializes every tree",
    # What it renders is for the base the lane names, which no key carries,
    # and the lane reads the patches the run writes: a kept verdict would
    # publish nothing.
    "cache": False,
    "no_cache_reason": "what it renders is the diff from the base the lane "
                       "names, which no unit key carries, and the lane "
                       "publishes the patches the run writes",
    "inputs": None,
    "commits": ["HEAD"],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
