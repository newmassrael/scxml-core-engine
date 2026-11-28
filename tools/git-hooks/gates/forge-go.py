# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `forge-go`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# forge-conformance.yml verifies the language arms in parallel jobs, and
# its path filter is workflow-wide: a change under
# backends/*/forge-runtime/** starts all of them in CI. These gates
# therefore fire together too, which is the parity we want. The split is
# for attribution — a failure names its arm — and each `extra` widens
# the trigger past the workflow filter rather than narrowing it, so a
# change outside forge-runtime still reaches the arm it could break.
{
    "workflows": ["forge-conformance.yml"],
    "runner_workflow": True,
    "extra": ["backends/go/**", "tools/ci-images/host/**"],
    # The host image (ADR 0145, family `host`): the toolchains of the arm and
    # the packages the generator links against, the same wherever the gate
    # runs. The runner pulls the image its Containerfile's digest names and
    # runs `scripts/gate forge-go` inside it, the generator build among its
    # deps ("Gates that run in an image" in tools/git-hooks/gate_registry.py).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    "deps": ["codegen-build"],
    # 15s, timed on a warm tree on 2026-09-02 and no longer a normalised
    # figure. Seven observations that day — five `--measure` runs reading
    # 27 17 16 15 10 and two real pushes reading 10 and 12 — put the
    # settled band at 10-17s once the first run's cache-filling is
    # dropped. 15 is the worse of the last measured pair, the rule the
    # budget header sets. Still the cheapest forge arm and the only one
    # that runs before a push.
    "ci_only": "336s where the landing queue judges it, which the push budget "
               "cannot hold beside the rest; forge-conformance.yml runs it after landing.",
    # 336s where the landing queue judges it: the median of its last ten
    # landing runs in the integrator's timings.tsv (2026-11-28), the seat the
    # budget bounds ("The push budget" in gate_registry.py). It read
    # 15s warm on a developer's tree.
    "cost_s": 336,
    "summary": "Go forge conformance regenerate + test",
    "cache": False,
    "no_cache_reason": "samples thread interleavings: its queue and "
                       "broadcast-log stress runs under `go test -race` record "
                       "what the machine's scheduler happened to run, so a pass "
                       "is one draw and keeping it would stop the drawing",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
