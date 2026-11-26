# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `mutation-rounds`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The other half of the same corpus: whether a case still turns its suite
# red, which only a build can answer.
#
# `ci_only`, and the number is the reason. Measured on the first push
# whose change set reached its casefiles: 877s, eight rounds, against 4s
# on a push that reached none. No value of `cost_s` describes both, and a
# push length that swings by a quarter of an hour on what the author
# happened to edit is one somebody eventually bypasses. The trade is
# stated rather than hidden: a red now arrives one round later.
#
# Its workflow declares no `paths:` filter, and the gate narrows inside
# itself — the same arrangement `mutation-cases` has, for the same reason.
# What decides a round is the union of every `mutation_targets`
# declaration in the casefiles, and a filter restating that union would be
# a second copy of it. The first draft got this wrong in a way worth
# recording: it triggered on `sce-build/tests/mutations/**`, so a change
# to a declared TARGET — the case the gate exists for — selected nothing,
# while a change to a casefile selected a gate with no round to run. The
# two conditions were nearly exclusive and the session that landed it
# happened to satisfy both.
#
# No `deps`, though the first draft named `workspace-tests` and
# `cpp-suite`: a round's cost is dominated by its baseline build and those
# two have already paid it, which is a reason to want them first and not a
# dependency. `deps` here means a gate whose OUTPUT this one executes, and
# a round builds whatever it needs itself.
{
    "workflows": ["mutation-rounds.yml"],
    # Its ctest casefiles run in the main tree, which it configures
    # (scripts/prepare_ctest_tree.sh) in its image, and where none is.
    "builds": ["main-tree"],
    # The host image (ADR 0145, family `host`): the toolchains of every round
    # but the Swift ones, which `mutation-swift` runs in the swift
    # image over the same selection ("Two gates over one corpus" in
    # scripts/gates/mutation-rounds.sh). The runner pulls the image its
    # Containerfile's digest names and runs `scripts/gate mutation-rounds`
    # inside it.
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # What the lane hands the run inside: the casefile and the slice a round
    # job runs and when the job started, and for the selection, the change set
    # and the switch that only names what it chose.
    "container_env": ["SCE_MUTATION_ROUNDS", "SCE_MUTATION_SHARD",
                      "SCE_MUTATION_JOB_STARTED", "SCE_MUTATION_ROUNDS_DRY_RUN",
                      "SCE_GATE_CHANGED_FILE"],
    "runner_workflow": True,
    "narrows": "the workflow is unfiltered so CI starts it on every push; "
               "what actually decides a round is the casefiles' own "
               "`mutation_targets`, which the gate reads at run time. "
               "There is no glob list that is both correct and narrower "
               "than the tree.",
    # No `local: ["**"]`, and the first draft's is why the note above the
    # `narrows` branch in `gate_triggers` exists. A catch-all makes a gate
    # always-on, and rule 3 then holds it out of the classifiers so it
    # cannot mark any path as understood. The one path that needed marking
    # was this lane's own workflow file: with the catch-all it matched
    # nothing, so editing it was unclassified and bought the full suite —
    # measured on the commit that introduced it, 30 gates for a 3-file
    # change. Without the catch-all, `narrows` leaves the gate exactly one
    # trigger, its workflow file, which is what classifies the edit. CI is
    # unaffected either way: the workflow declares no `paths:` and CI does
    # not read this table.
    "ci_only": "measured 877s when the change set reached its casefiles "
               "and 4s when it did not; a push that swings by a quarter "
               "of an hour on what the author edited is one that gets "
               "bypassed. Runs in CI, where the cost is not the "
               "developer's wait, at the price of a red arriving one "
               "round later.",
    "audited_after_landing": "one build and one test run per mutation case "
                             "its change set reaches, so its cost has no "
                             "bound in the tree; mutation-rounds.yml "
                             "shards it and the weekly sweep runs the "
                             "corpus whole",
    # The harness itself. The gate invokes `scripts/mutate` twice — once
    # to ask a casefile what it declares, once to run the round — so an
    # edit to it changes every verdict this lane produces. It was not
    # declared, and `--ci-owed` therefore did not name this lane for a
    # commit that changed nothing else (`script-read-coverage`).
    #
    # The cost model, because the gate sizes every job from it and judges
    # every job's duration against it: an edit to a runner's row changes
    # how many jobs the lane schedules and which of them turn red.
    #
    # And the lockfile: a change to it moves every tree a `<tree>:<path>`
    # target names (docs/adr/0097), so it selects the rounds that mutate one.
    #
    # And the script a landing configures `build/` with for the ctest
    # casefiles (the gate runs it itself there, rather than refusing).
    #
    # And the two readings of a ctest tree the harness shares with the C++
    # suite's shards: the commands a configured tree registers, and the
    # fixtures whose setup builds the whole tree.
    #
    # And the image the rounds run in, and the installer of the rev-pinned
    # validator the gate runs there.
    "extra": ["scripts/mutate", "scripts/lib/mutation_rounds_cost.sh", "Cargo.lock",
              "scripts/prepare_ctest_tree.sh",
              "tools/ci-images/host/**", "scripts/install_mnemosyne_cli.sh",
              "scripts/lib/ctest_registered_commands.cmake",
              "scripts/lib/ctest_shards.py"],
    "cost_s": 877,
    "summary": "changed mutation targets still turn their suites red",
    "cache": False,
    "no_cache_reason": "reads the clock and git refs: it judges each round's "
                       "duration against the cost model (`date +%s`), and with "
                       "no change set it takes its base from the upstream ref",
    "inputs": None,
    # It picks casefiles by the change set, so a run away from the push's
    # checkout is handed the range (`scripts/gate --change-set`): that
    # checkout's upstream says nothing about the push (see `commits` above).
    # A landing runs none (`audited_after_landing`).
    "commits": ["CHANGES"],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
