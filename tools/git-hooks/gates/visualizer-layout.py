# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `visualizer-layout`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# A deploy workflow whose three build steps are verdicts. Its trigger
# already includes `sce-build/**`, so a Rust change every other gate passes
# could still turn it red — after the push.
# The diagram's geometry, judged without a browser. Every defect it was
# written for was invisible to every other lane: ELK's routes deleted 23
# lines after being stored, a container shrunk after its edges were routed
# to it, a cross-hierarchy route read in the wrong coordinate frame, a
# collapse leaving eight edges touching nothing. None of them broke a
# build, and a rendered diagram is not bytes another test can diff.
#
# ⚠ `ci_only`, and not because it is slow — it is seconds. It needs the
# visualizer WASM, which `visualizer-wasm` builds in the same workflow and
# which a push does not build.
{
    "workflows": ["deploy-visualizer.yml"],
    "runner_workflow": True,
    "ci_only": "needs the codegen and engine WASMs that visualizer-wasm "
               "builds in the same workflow; a push builds no WASM, so the "
               "measurement would have nothing to turn a document into a "
               "structure with, nor to compare that structure against.",
    # ⚠ `web/visualizer/**` and no wider, which took two attempts to get
    # right. The gate first swept every tracked `.scxml`; declaring that
    # honestly meant `**/*.scxml`, and `ci-only-coverage` then refused the
    # entry with the sentence worth keeping: "The gate was not moved to
    # CI, it was removed" — deploy-visualizer.yml does not run for an
    # arbitrary document, so the lane would have been advertised and never
    # run. The fixtures are pinned in `web/visualizer/measure/fixtures.txt`
    # instead, which is inside this trigger; what that costs is written
    # there.
    "extra": ["web/visualizer/**"],
    # The WASMs it measures are `visualizer-wasm`'s output, in gitignored
    # directories, so that gate runs first: a landing unit runs on a
    # build machine's checkout, where they are absent (the gate ends 3,
    # and the item would never land) or left by an older tree (a verdict
    # about WASMs this change did not build). deploy-visualizer.yml runs
    # the two as steps of one job, so there the second run of
    # `visualizer-wasm` finds its output built.
    "deps": ["visualizer-wasm"],
    # 22s measured 2026-09-18, up from 6s: the lane gained a stress half
    # that replays 25 seeded gestures per fixture over all ten. The seed
    # is pinned in the gate script, so the cost is stable and a red is
    # reproducible.
    # The scenario half (2026-10-13) replays 31 scenarios of the shared
    # corpus on the browser's engine; the gate's cost is unmeasured since,
    # and a hosted run is what measures it.
    "cost_s": 22,
    "summary": "diagram layout invariants, drag/collapse/stress survival, "
               "and the shared scenarios on the browser's engine",
    # The WASMs it reads are built in the same unit (`deps`), and the
    # stress half's seed is pinned in the script, so the verdict is the
    # tree's. Kept only from a landing unit, through `visualizer-wasm`.
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
