# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `regen-reproduces`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The one reader of the content axis. The header's `source-hash` covers
# the INPUT documents, so an edit to the templates or to the emit code
# under `sce-build/src` that changes what comes out does not move it, and
# the committed trees drift with every other gate green. This regenerates
# them and compares, which is also what makes the procedure's own
# documented claim ("regenerate and expect no diff") checked rather than
# asserted.
#
# Its script regenerates one shard of the steps when SCE_REGEN_SHARD names
# `k/N`, as regen-reproduces.yml's three jobs do
# (scripts/lib/workspace_shards.py plans the part). No `shard_env`: the
# build machines run it whole.
{
    "workflows": ["regen-reproduces.yml"],
    "runner_workflow": True,
    "deps": ["codegen-build"],
    # Runs in the host image (tools/ci-images/host, docs/adr/0145): libxml2,
    # clang-format-19 and clang-19 for the procedure's C++ and embed-manifest
    # stages, and the Go the go-round-trip and forge-goldens steps build with.
    # The lane's part (SCE_REGEN_SHARD) goes in by name: a plan runs the gate
    # whole, so it is no `shard_env`.
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    "container_env": ["SCE_REGEN_SHARD"],
    "ci_only": "117s regenerating every committed tree, which is work "
               "proportional to the corpus and not to the change. "
               "regen-reproduces.yml runs it. What this costs is worth "
               "naming: a template edit that does not carry its "
               "regenerated trees now lands, and that exact miss shipped "
               "once already (PR-90, where enter_at panicked in the real "
               "generated code while the branch's own suite was green).",
    "cost_s": 117,
    "summary": "regeneration reproduces every committed tree",
    # Regenerates in a worktree of HEAD, with each submodule at the commit
    # HEAD records, so the unit carries HEAD's id (`commits`). The plan
    # also keeps no verdict for a tree that differs from HEAD.
    "cache": True,
    "inputs": None,
    "commits": ["HEAD"],
    "peak_gb": 6.5,
    "parallelism": PARALLELISM_COMPILE,
}
