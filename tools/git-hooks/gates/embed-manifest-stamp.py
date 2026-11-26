# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `embed-manifest-stamp`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# Whether embed/MANIFEST.json was emitted from the inputs it is committed
# with, against its stamp embed/MANIFEST.inputs (scripts/embed_manifest.py).
# `embed-vendor` checks the content and is CI-only at 311s, so until this
# gate a header edit reached the push and the CI run before anything said
# the manifest had not moved with it; five of eight manifest commits in
# October and November 2026 were repairs made after the fact. This one is
# a list of git object ids against another, under a second, so it runs at
# the push. The pre-commit hook runs the same comparison over the index;
# this is its backstop for a commit made with the hook skipped.
{
    "workflows": ["embed-vendor-smoke.yml"],
    "runner_workflow": True,
    "cost_s": 0,
    "summary": "embed manifest emitted from the inputs it is committed with",
    # It runs in the host image wherever it runs (ADR 0145): the runner runs
    # `scripts/gate` inside the image its Containerfile addresses, so the
    # hosted lane, a build machine and a workstation judge it in one
    # environment, and no lane installs a package for it. A machine takes its
    # unit only where docker runs (`needs`).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # Reads HEAD's objects (`check --rev HEAD`, `git show`) and the
    # commit each submodule is at, so the unit carries HEAD's id
    # (`commits`). The plan also keeps no verdict for a tree that differs
    # from HEAD (`plan_units`).
    "cache": True,
    "inputs": None,
    "commits": ["HEAD"],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
