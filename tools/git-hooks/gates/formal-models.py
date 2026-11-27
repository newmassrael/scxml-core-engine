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
# a push has five for everything. The judge is `sce-codegen formal models`
# (docs/adr/0146); TLC is installed by `sce-codegen formal install` and admitted
# by the digest the judge pins; the gate answers "could not run" where it has not.
{
    "workflows": ["formal-models.yml"],
    "runner_workflow": True,
    # The judge is `target/debug/sce-codegen formal` (docs/adr/0146).
    "deps": ["codegen-build"],
    # Runs in the host image (tools/ci-images/host, docs/adr/0145): the JDK TLC
    # runs on, libxml2 the generator builds with, and curl and unzip, which
    # `sce-codegen formal install` fetches and unpacks the pinned tool with.
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # Its runs are spread over the lane's three jobs when SCE_FORMAL_MODELS_SHARD
    # names `k/N` (`shard_runs` in sce-build/src/formal/models.rs): whole, they took
    # 1042 s on a hosted runner (job 138489726305). A plan makes one unit per
    # part (`shard_count`).
    "shard_env": "SCE_FORMAL_MODELS_SHARD",
    # The judge and its pins are sce-build's (docs/adr/0146), and the consumer
    # fixture is judged with this repository's models.
    "extra": ["docs/formal/**", "sce-build/src/formal/**",
              "sce-build/tests/fixtures/formal_consumer/**",
              # The machine whose properties the gate judges, each form held and
              # each broken on purpose (docs/adr/0115, D15).
              "sce-build/tests/fixtures/formal_properties/**"],
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
    # TLC runs as many workers as the unit is granted (`--workers` from
    # `sce_build_jobs` in scripts/gates/formal-models.sh), one run at a time.
    "parallelism": PARALLELISM_COMPILE,
}
