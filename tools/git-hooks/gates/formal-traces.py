# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `formal-traces`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The recorded runs of the code under TLC, as docs/formal/traces.json declares
# them (docs/adr/0115, D4 and D8): each run accepted by the model it names, each
# altered run refused where it was altered. Nothing ran them before this gate.
#
# `ci_only`: it needs Java and the pinned TLC, which the push set's machine need
# not carry. The judge is `sce-codegen formal traces` (docs/adr/0146); TLC is
# installed by `sce-codegen formal install` and admitted by the digest the judge
# pins; the gate answers "could not run" where it has not.
{
    "workflows": ["formal-traces.yml"],
    "runner_workflow": True,
    # The judge is `target/debug/sce-codegen formal` (docs/adr/0146).
    "deps": ["codegen-build"],
    # Runs in the host image (tools/ci-images/host, docs/adr/0145): the JDK TLC
    # runs on, libxml2 the generator builds with, and curl and unzip, which
    # `sce-codegen formal install` fetches and unpacks the pinned tool with.
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # The judge, its pins and the reader of the run trace format are sce-build's
    # (docs/adr/0146); the runs and the models they are judged against are
    # docs/formal's.
    "extra": ["docs/formal/**", "sce-build/src/formal/**", "sce-build/src/run_trace.rs"],
    "ci_only": "Every recorded run and each altered run is a run of TLC, eighteen of "
               "them today, on a machine with Java and the pinned TLC, which the push "
               "set's machine need not carry. formal-traces.yml runs it on every change "
               "to a run, a model or its judge.",
    # Not timed yet: eighteen runs of TLC over three small models, each a JVM
    # start and seconds of search, estimated at the time formal-models takes for
    # as many of its smallest runs. Its first run reads it
    # (UNMEASURED_COST_CEILING in gate_registry.py).
    "cost_s": 90,
    "summary": "recorded runs of the code under TLC, with each altered run refused",
    # TLC explores the whole trace module, and the verdict is the state graph's,
    # so one input gives one answer.
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 4,
    # TLC runs as many workers as the unit is granted (`--workers` from
    # `sce_build_jobs` in scripts/gates/formal-traces.sh), one run at a time.
    "parallelism": PARALLELISM_COMPILE,
}
