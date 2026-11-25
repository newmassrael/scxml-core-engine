# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `tree-hygiene`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# `roadmap_marker_gate` reads every tracked file and
# `workflow_trigger_coverage` reads every workflow, so tree-hygiene.yml
# declares no `paths:` filter — no glob list is both correct and
# narrower than the tree. That missing filter is what makes this gate
# unconditional, derived the same way every other workflow-backed
# trigger is rather than hand-set to "always".
{
    "workflows": ["tree-hygiene.yml"],
    "runner_workflow": True,
    # Its targets are spread over the lane's six jobs when SCE_TREE_HYGIENE_SHARD
    # names `k/N` (`--gate-targets` in scripts/lib/workspace_shards.py, which
    # reads them from the gate's own `cargo test` command): whole, the job took
    # 37 minutes on a hosted runner (job 138916754622). A plan makes one unit
    # per part (`shard_count`).
    "shard_env": "SCE_TREE_HYGIENE_SHARD",
    # 193s measured 2026-08-22, against a declared 13 — the single
    # largest gate a push could run, and it runs on EVERY push because
    # its workflow is unfiltered. The old number was not a lie about the
    # work, it was fifteen times out of date, which is the failure mode a
    # hand-written cost has and the reason `--measure` exists.
    "ci_only": "193s measured, on every push, because a tree-wide gate "
               "has no path filter to narrow it. Nothing else in the "
               "budget comes close. tree-hygiene.yml is unfiltered too, "
               "so CI is OFFERED exactly the pushes this would have "
               "run — and whether it JUDGES them is a property of that "
               "workflow, not of this delegation. It failed to on "
               "2026-09-02: ten consecutive pushes cancelled, the "
               "repair of the lane's own red among them. The repair is "
               "cancel-in-progress: false there, held by "
               "`ci_supersession_policy`, which measures this lane "
               "against a "
               "17.6 min median gap between pushes to main — not by "
               "spacing the pushes. That test now holds every prose "
               "copy of the gap to its constant, this one included.",
    "cost_s": 193,
    "summary": "tree-wide marker + trigger + parity gates",
    # Reads HEAD's history: `lowering_decision_ledger` asks git whether
    # each pinned commit is an object, an ancestor of HEAD and holds the
    # paths it names, and `hook_ci_parity` adds a worktree of HEAD. One
    # tree can sit on histories that answer differently, so the unit
    # carries HEAD's id (`commits`). Its other targets read the tracked
    # tree or fixture repositories of their own, and the tools they skip
    # without (the pinned mnemosyne-cli, cmake) are ones bx.toml requires.
    "cache": True,
    "inputs": None,
    "commits": ["HEAD"],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
