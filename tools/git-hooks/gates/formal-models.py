# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `formal-models`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The TLA+ models of docs/formal under TLC, as docs/formal/models.json declares
# them (docs/adr/0115, D5). Nothing ran them before this gate.
#
# `ci_only`: the largest configuration is explored whole, which is minutes, and
# a push has five for everything. TLC is fetched by the runner that needs it and
# checked against the manifest's digest; Java is what every runner here has, and
# the gate answers "could not run" (exit 3) where it has not.
{
    "workflows": ["formal-models.yml"],
    "runner_workflow": True,
    # Its runs are spread over the lane's three jobs when SCE_FORMAL_MODELS_SHARD
    # names `k/N` (`shard_runs` in scripts/formal/models.py): whole, they took
    # 1042 s on a hosted runner (job 138489726305). A plan makes one unit per
    # part (`shard_count`).
    "shard_env": "SCE_FORMAL_MODELS_SHARD",
    # The spreading is `scripts/lib/ctest_shards.py`'s, which models.py imports.
    "extra": ["docs/formal/**", "scripts/formal/**", "tools/git-hooks/spec_isolation.py",
              "scripts/lib/ctest_shards.py"],
    "ci_only": "Every model is explored whole (the largest, BroadcastLog, 15.4 million "
               "states), which takes minutes of a push that has five for everything. "
               "formal-models.yml runs it on every change to a model or its judge.",
    "cost_s": 367,
    "summary": "TLA+ models under TLC, with every protocol part taken out in turn",
    # TLC explores every reachable state of a configuration, and the manifest
    # records the count, so one input gives one answer.
    "cache": True,
    # Keyed on its triggers: `bx --checks prepush --learn formal-models`
    # (pc2, 2026-11-24) found 124 reads and 4 listings, every one inside
    # the triggers and the runner's inputs.
    "inputs": INPUTS_ARE_TRIGGERS,
    "commits": [],
    "peak_gb": 8,
    # TLC runs as many workers as the machine has cores (`-workers auto`).
    "parallelism": PARALLELISM_COMPILE,
}
