# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `mutation-swift`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The Swift share of `mutation-rounds`: the casefiles whose runner is
# `mutation_swift`, each round a Swift package built and its tests run once
# per case. A gate runs in one image (ADR 0145, D4), and these rounds need the
# Swift toolchain, which the swift image holds and the host image that
# `mutation-rounds` runs in does not. Its script is `mutation-rounds`'s,
# sourced with the family `swift` (scripts/gates/mutation-swift.sh), so
# the two gates choose from the corpus by one set of rules; the lane runs this
# one for a matrix entry whose runner is `swift`.
#
# Every attribute that is `mutation-rounds`'s for the reason that entry gives
# is this one's for the same reason: `ci_only` and `audited_after_landing` for
# a cost the change sets, `narrows` for an unfiltered workflow, no cache for a
# gate that reads the clock and git refs, and the change set handed in.
{
    "workflows": ["mutation-rounds.yml"],
    "runner_workflow": True,
    # The swift image (ADR 0145, family `swift`): the pinned Swift toolchain
    # and the packages it links against. The runner pulls the image its
    # Containerfile's digest names and runs `scripts/gate mutation-swift`
    # inside it.
    "container": "tools/ci-images/swift/Containerfile",
    "needs": ["docker"],
    # What the lane's round job hands the run inside: the casefile and the
    # slice it runs, and when the job started.
    "container_env": ["SCE_MUTATION_ROUNDS", "SCE_MUTATION_SHARD",
                      "SCE_MUTATION_JOB_STARTED"],
    "narrows": "the workflow is unfiltered so CI starts it on every push; "
               "what actually decides a round is the casefiles' own "
               "`mutation_targets`, which the gate reads at run time. "
               "There is no glob list that is both correct and narrower "
               "than the tree.",
    "ci_only": "one build and one test run of a Swift package per case its "
               "change set reaches, in the swift image a push would pull "
               "first; mutation-rounds.yml runs it, at the price of a red "
               "arriving one round later.",
    "audited_after_landing": "one build and one test run per mutation case "
                             "its change set reaches, so its cost has no "
                             "bound in the tree; mutation-rounds.yml "
                             "shards it and the weekly sweep runs the "
                             "corpus whole",
    # The harness, the cost model that sizes and judges every job, the
    # lockfile (a change to it moves every tree a `<tree>:<path>` target
    # names), the script this gate's own sources, the image the rounds run in
    # and the pin the harness finds the toolchain by.
    "extra": ["scripts/mutate", "scripts/lib/mutation_rounds_cost.sh", "Cargo.lock",
              "scripts/gates/mutation-rounds.sh",
              "tools/ci-images/swift/**", "scripts/lib/swift_pin.sh"],
    # Not measured: the Swift rounds ran inside `mutation-rounds` until this
    # gate took them, and that gate's figure is the one its first push that
    # reached casefiles took. Declared the same until a run of this gate is
    # timed.
    "cost_s": 877,
    "summary": "changed Swift mutation targets still turn their suites red",
    "cache": False,
    "no_cache_reason": "reads the clock and git refs: it judges each round's "
                       "duration against the cost model (`date +%s`), and with "
                       "no change set it takes its base from the upstream ref",
    "inputs": None,
    "commits": ["CHANGES"],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
