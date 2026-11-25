# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `authoring-core`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The authoring core's claim is that it knows no subject matter. A claim
# like that decays silently -- one helpful special case at a time -- so it
# is a test. It was declared Python only at 0.262s; the suite since grew
# to 413 cases, 86 of which drive a document through `sce-codegen`, and
# the figure was never retaken.
#
# ⚠ What moved it: a push on 2026-09-24 was refused at 331s of the 300s
# ceiling with every gate green, this one reading 81s paced. Timed alone
# it was 155s with the `cargo build` (opt 0) binary `codegen-build`
# leaves for it, and 72s once `sce-build` builds at opt 1 in the dev
# profile (`Cargo.toml`). The run-driven 86 are also why CI now builds
# the generator for this lane: without it they skipped, and the lane
# passed without judging them.
{
    "workflows": ["authoring-core.yml"],
    "runner_workflow": True,
    # Its test modules are spread over the three jobs of the lane's suite with
    # the generator built when SCE_AUTHORING_CORE_SHARD names `k/N`
    # (`--unittest-modules` in scripts/lib/workspace_shards.py): whole, that job
    # took 16 minutes on a hosted runner (job 139000500233). A plan makes one
    # unit per part (`shard_count`).
    "shard_env": "SCE_AUTHORING_CORE_SHARD",
    "ci_only": "72s measured with the generator built at opt 1, 155s "
               "at opt 0, against a declared 0.262. authoring-core.yml "
               "runs it twice: Python only, and with sce-codegen built "
               "so the 86 run-driven cases are judged, not skipped.",
    "cost_s": 72,
    "summary": "authoring core is domain-free, and its refusals still fire",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
