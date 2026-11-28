# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `formal-isolation`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# Each commit of a push changes what a formal check says by itself and for a
# reason (docs/adr/0115, D11). Until this gate the push hook ran the Python judge
# inline, over the refs it published, and no lane ran it; the judge is now
# `sce-codegen formal isolation` (docs/adr/0146, order step 5), which the hook
# cannot run before the generator is built, so it is a gate unit handed the
# push's ranges, and the lane runs the same unit over its own.
{
    "workflows": ["formal-isolation.yml"],
    "runner_workflow": True,
    # The judge is `target/debug/sce-codegen formal` (docs/adr/0146).
    "deps": ["codegen-build"],
    # Runs in the host image (tools/ci-images/host, docs/adr/0145), where the
    # generator it runs is built with the libxml2 the image carries.
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # A commit that changes what a check says changes a file of docs/formal or a
    # file docs/formal/proofs.json declares proved, so those select it; a commit
    # that changes neither changes no check. The self-test holds every proved file
    # the manifest declares to these globs (`formal-isolation-sees-every-proved-file`).
    # The judge's own source selects it too; a commit may change a judge beside a
    # check no more than any other file.
    "extra": ["docs/formal/**", "sce-build/src/formal/**",
              "backends/rust/mesh/src/https/**", "backends/rust/mesh/src/outbound.rs",
              "backends/rust/mesh/src/outbound/**", "backends/rust/runtime/src/inbox.rs",
              "backends/rust/runtime/src/inbox/**", "backends/rust/session-store/src/rules.rs",
              "backends/rust/session-store/src/rules/**", "backends/rust/mesh/src/pool.rs",
              "backends/rust/mesh/src/pool/**"],
    # Not timed yet: git reads each commit of the push and its parent's
    # manifests, a second or two for the pushes this repository makes; the
    # generator it runs is `codegen-build`'s cost. Its first run reads it
    # (UNMEASURED_COST_CEILING in gate_registry.py).
    "cost_s": 3,
    "summary": "a commit that changes a formal check changes nothing else, for a reason",
    # The verdict is a function of the commits the ranges name, and their ids are
    # in the unit's argv (`--change-set`), so a kept pass is the same commits'.
    # Its unit asks bx for the tree's commit and its history (`commit` in the
    # plan), since a copy of the files alone has no range to read.
    "cache": True,
    "inputs": None,
    "commits": ["CHANGES"],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
