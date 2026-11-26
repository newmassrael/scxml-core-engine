# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `clang-format`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# Every tracked C and C++ file is formatted as .clang-format says, by
# clang-format 19. The lane ran `scripts/check_clang_format.sh` itself, on a
# clang-format-19 it installed from the live archive at job time; it runs this
# gate now, because a gate is how a lane enters the image its packages come
# from (docs/adr/0145). Until then the workflow was recorded in
# UNMIRRORED_WORKFLOWS as the commit hook's, which runs the same script on the
# staged files and still does.
{
    "workflows": ["clang-format-check.yml"],
    "runner_workflow": True,
    # The formatter is the host image's (tools/ci-images/host), at the patch
    # release its snapshot of the archive holds, wherever the gate runs.
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    "ci_only": "the commit hook runs scripts/check_clang_format.sh on the "
               "staged C and C++ files, which is earlier than a push and "
               "covers what a push brings; clang-format-check.yml runs it "
               "over the whole tree, in the image, on every change to a "
               "C or C++ file.",
    # Unmeasured as a gate: the lane's median run was 0.8 minutes before it
    # pulled an image (ci_supersession_policy.rs), most of it the install
    # this replaces. The formatting of every tracked file, spread over the
    # machine's cores, is what is left.
    "cost_s": 48,
    "summary": "every tracked C and C++ file is formatted as .clang-format says",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_COMPILE,
}
