# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `spec-snapshot`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The gate whose absence let a stale verifies-catalog reach CI red:
# `ledger-citations` runs mnemosyne-cli, this workflow runs a separate
# python generator, and the hook mirrored only the first.
{
    "workflows": ["spec-snapshot-drift.yml"],
    "runner_workflow": True,
    "cost_s": 13,
    # Its tooling suite judges the citations of every tree the ledgers
    # enrol, so those trees are its trigger, derived from the ledgers'
    # configs (`enrolled_by`, see `enrolled_globs`) and never listed here.
    # Until 2026-11-09 the trigger was the workflow's filter alone, and
    # bx-fleet check learn found it reading 1072 files that filter did not
    # name. The workflow's own `paths:` cannot derive, so CI starts this
    # lane for fewer paths than a push selects it for.
    "enrolled_by": "docs/*/*/mnemosyne.toml",
    "summary": "spec snapshot integrity + verifies-catalog drift",
    # `check_spec_drift.py --mode integrity` is the offline mode: it hashes
    # the snapshot against its recorded provenance. The `upstream` mode,
    # which fetches the published specification, is not what this runs.
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
