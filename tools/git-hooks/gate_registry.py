#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The gate registry: what gates exist, what triggers each, what each needs
# built first, and what each costs.
#
# Gates are identified by a slug, never by position. The previous table
# keyed them by run order ("1", "1b", "2c", "8b"), which made the identifier
# and the schedule the same thing: reordering renamed gates, so the order
# could not be improved without breaking every reference to it, and each new
# gate had to be wedged in with a letter suffix instead of taking a place of
# its own. The evidence that this had stopped working was in the hook
# itself — three gates carried block comments naming a number two lower than
# their own label, left behind by an earlier renumbering, four gates
# mirroring one workflow were split across two number families, five gates
# were missing from the header list, and every label read "N/8" while
# seventeen gates existed.
#
# With slugs, order is derived rather than declared: `deps` says what must
# run first, `cost_s` says what is cheap, and the runner sorts by both. A
# gate added tomorrow lands in the right place without touching any other
# entry.
#
# Which gate a change needs is answered by the `paths:` filter on the CI
# workflow it mirrors, so those filters are read out of
# `.github/workflows/*.yml` rather than restated here — the hook cannot
# drift from CI the way a hand-copied table would, and a workflow that gains
# a path starts triggering its gate on the same commit that widens CI.
#
# Three rules keep the filtering from becoming a coverage hole, which is the
# failure mode a selective hook invites:
#
#   1. A changed path matching NO known glob — not a gate trigger and not
#      the inert list below — forces the FULL run. A new top-level directory
#      therefore gets more verification, never less.
#   2. Gates with no CI counterpart carry an explicit local trigger with the
#      reason written next to it, rather than being silently always-on or
#      silently never-on.
#   3. A gate whose trigger is the catch-all `**` is selected always and
#      counts as classifying nothing. Its workflow runs on every push, so a
#      match carries no information about whether a path is understood —
#      letting it count would mark every path in the repository known and
#      rule 1 would never fire again.

from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys
from pathlib import Path

# ── Gate table ────────────────────────────────────────────────────
#
# `workflows` names the CI workflow(s) a gate mirrors; its `paths:` filter
# is the trigger. `extra` adds locally-known triggers for what the
# workflow's filter cannot express, and `local` is for gates CI has no
# counterpart for. `deps` names gates whose output this one executes.
#
# `cost_s` is wall-clock seconds, MEASURED by `scripts/gate --measure --all`
# on a warm tree, not estimated. Warm is the right basis: a push happens on
# a tree the developer has just built, and a cold first build is paid once
# in whatever order the gates run. Two limits are worth stating rather than
# discovering later. The numbers come from a full run, so a gate that
# benefits from an earlier gate having warmed the cargo cache reads cheaper
# here than it would in isolation — which is the number that matters for
# ordering a full run, and the wrong one for judging a single gate. And the
# clock is whole seconds, so everything under a second reads 0.
#
# Re-measure after any gate's work changes materially; a stale cost is a
# wrong order, not a wrong comment. The value that motivated all of this
# was `ledger-citations`: the hook's prose called it "sub-second per
# workspace" and it sat last of seventeen, while a measurement puts the
# whole gate at 157s — still nowhere near last, and the prose was
# describing one step of it.
# ── The push budget ───────────────────────────────────────────────
#
# The ceiling on what a push may cost the developer, in the same warm
# wall-clock seconds `cost_s` is measured in. Set by the owner: a push
# takes under five minutes.
#
# It bounds the WORST case, not the average. Rule 1 hands back every gate
# a push can run for any unclassified path, so that set is what a developer
# can actually be made to wait for, and bounding the median would leave the
# longest pushes — the ones that decide whether anybody keeps using the
# hook — unbounded.
#
# The costs it sums are the WORSE of two `--measure` runs of the same set,
# not one run. Measured 2026-08-22: the same gates varied by as much as 27s
# between consecutive warm runs (`clippy` alone read 15s and then 42s), so a
# ceiling built on a single run is a ceiling met on lucky runs. Taking the
# max is what makes "under five minutes" a property of every push rather
# than of the run that happened to be measured.
#
# What this replaced is worth recording, because the numbers were wrong in a
# direction nobody would have noticed. Before the ceiling, the local set was
# never timed end to end: the declared costs summed to 659s and the first
# real measurement of them was 529s — with `tree-hygiene` alone reading 193s
# against a declared 13. Selections over the previous forty commits, priced
# with those stale numbers, came out at median 382s and worst 969s, so even
# the diagnosis that motivated this was built on figures that were fifteen
# times off in one place.
#
# `budget_is_not_exceeded` in the self-test enforces it. That is the point
# of writing it down as a number: a gate that grows past the ceiling, or a
# new one that does not fit, fails here rather than being discovered as a
# push somebody started bypassing. The fix is to move the most expensive
# gate to `ci_only` — the same greedy rule that produced today's set —
# never to raise this constant without the owner saying so.
PUSH_BUDGET_S = 300

# ── When each cost_s was measured ─────────────────────────────────
#
# The header above says `cost_s` is warm wall-clock, measured rather than
# estimated, and that "a stale cost is a wrong order, not a wrong comment".
# It gives no way to ASK whether one is stale. Every date lives in prose
# beside the number — "77s measured 2026-08-22 (declared 42)", "41s,
# pace-normalised from the 2026-08-23 push" — and prose is not a thing a
# gate can read. Measured 2026-09-02: of 35 slugs, prose carries a date
# near 15 of them, and a scan cannot even tell those apart from the dates
# that describe an incident rather than a measurement, because a slug's
# comment block runs into its neighbour's.
#
# So the date moves here, where it is data. A slug ABSENT from this map is
# unmeasured, and that is not a free pass: `every_cost_declares_when_it_was
# _measured` in the self-test holds the absent count to the ceiling below,
# and the ceiling may only fall. Both directions fail — more unmeasured
# slugs than the ceiling is a regression, and FEWER means somebody measured
# one and left the ceiling behind, which is how a ratchet quietly stops
# ratcheting.
#
# Filling one in means running it: `scripts/gate --measure <slug>` on a warm
# tree, then putting the number in `cost_s` and today's date here. CI cannot
# do it — CI is cold by construction, and this repository's own measurement
# says the two differ by 24x in the worst case (`cpp-suite`: declared 110s,
# 2647s on a runner) and by 0.25x in the other direction (`ledger-citations`:
# declared 121s, 30s on a runner). That spread is why the date matters more
# than another number would: it is the only thing that says whether the
# figure still describes the gate.
COST_MEASURED: dict[str, str] = {
    # `scripts/gate --measure rust-modrs-drift` on 2026-09-02 reported 0 —
    # the same figure the table already carried. That is the point rather
    # than an anticlimax: the NUMBER was right and unaskable, and what the
    # run produced was the date. A cost that has not moved in three weeks
    # and a cost nobody has timed since look identical until one of them
    # carries this.
    "rust-modrs-drift": "2026-09-02",
    # The four cheapest unmeasured gates, timed together on 2026-09-02.
    # Every `cost_s` here survived the re-measurement unchanged, and the two
    # that looked like drift are why this field is a DATE and not a second
    # number:
    #
    #   * `http-endpoint-ssot` ran 0s against a declared 4s and the drift
    #     report named it. Its own comment already explains that 4 — "4s,
    #     pace-normalised on 2026-08-24. It read 0 because the scan itself
    #     is 124ms; what the 0 left out is the process the runner has to
    #     start around it". Following the report would have undone a
    #     deliberate decision, so the number stays and the date moves.
    #   * `rustdoc-links` ran 3s against 4s. The report stayed quiet — that
    #     is inside its noise floor — and a one-second wobble is not a
    #     reason to rewrite a figure somebody chose.
    #
    # So a measurement can confirm a cost without changing it, and until
    # this map existed there was no way to record that it had happened.
    # The residue, stated rather than hidden: the registry still cannot say
    # WHICH numbers are pace-normalised — that lives in prose, and a future
    # drift report will keep pointing at `http-endpoint-ssot` every time.
    "embed-manifest-failfast": "2026-09-02",
    "http-endpoint-ssot": "2026-09-02",
    "license-ssot": "2026-09-02",
    "rustdoc-links": "2026-09-02",
}

# Exactly how many slugs are absent from COST_MEASURED. Not an upper bound
# with slack — an equality, so measuring one gate forces this down in the
# same commit and the count cannot drift away from the map.
UNMEASURED_COST_CEILING = 30

# ── Costs that are deliberately NOT the raw stopwatch reading ─────
#
# Some `cost_s` values were normalised by hand: a figure taken on a loaded
# machine scaled to a quiet one, or a sub-second scan raised to cover the
# process the runner has to start around it. They are chosen numbers, and
# a re-measurement is SUPPOSED to disagree with them.
#
# `order_drift` did not know that, and the consequence was measured on
# 2026-09-02: timing the four cheapest gates produced
# `http-endpoint-ssot declared 4s ran 0s` and the advice "update cost_s".
# Following it would have undone a decision whose reasoning was sitting
# three lines above the number, in a comment — *"It read 0 because the
# scan itself is 124ms; what the 0 left out is the process the runner has
# to start around it"*. Six slugs carry that kind of note and the report
# would mislead about every one of them.
#
# So the fact moves here, where the report can read it. The report still
# NAMES the slug — suppressing it would hide a genuine regression behind
# a flag, and rule 6 of this repository's round rules is exactly that —
# it just stops telling the reader to lower the number.
#
# ⚠ The value is the reason, not a restatement of the number. It is
# printed to whoever is looking at the drift, so it has to say why the
# two figures differ.
PACE_NORMALISED: dict[str, str] = {
    "codegen-build": "41s, paced from the 2026-08-23 push rather than timed alone",
    "forge-go": "36s on 2026-08-24, paced down from a declared 52s",
    "http-endpoint-ssot": "the scan is 124ms; 4s covers the process around it",
    "nostd-mcu": "16s, paced from the 2026-08-23 push",
    "w3c-go": "52s, paced from the 2026-08-23 push",
    "w3c-kotlin": "152s, what it cost on 2026-08-24 under a full run",
}

GATES: dict[str, dict] = {
    # Prerequisite, not a gate: several gates execute target/debug/sce-codegen.
    # Triggered by its own sources, and forced on by its dependents.
    "codegen-build": {
        "local": ["sce-build/**", "Cargo.toml", "Cargo.lock"],
        "no_ci_reason": "not a check — it produces the binary other gates "
                        "run. CI builds it per workflow that needs it "
                        "(`debug_only_codegen_builds_drop_the_stale_release_binary` "
                        "counts those steps), so there is no verdict here "
                        "for a workflow to mirror.",
        # Stays local however expensive it gets: `drop_ci_only` removes a
        # gate without rescuing its dependents, so a `ci_only` dependency
        # would leave the gates that execute its binary with nothing to run.
        # 41s, pace-normalised from the 2026-08-23 push.
        "cost_s": 41,
        "summary": "build target/debug/sce-codegen",
    },
    # Structural check over the Rust module tree — only a .rs add/remove can
    # break it, so the Rust trigger set is exactly right.
    "rust-modrs-drift": {
        "local": ["**/*.rs"],
        "no_ci_reason": "CI catches the same defect as a compile error in "
                        "rust-workspace-tests.yml — an aggregator naming a "
                        "missing subdir is E0432, and an unnamed subdir is "
                        "dead code its importers fail on. This gate exists "
                        "to say so in under a second instead of after 30s "
                        "of compilation, so a bypass loses speed, not "
                        "coverage.",
        "cost_s": 0,
        "summary": "mod.rs <-> subdirectory drift",
    },
    # Guards the clang degraded-AST regression in the manifest emitter.
    # Its own lane in the embed workflow rather than a step in the smoke
    # job: it needs clang and a second, and the smoke build takes seven
    # minutes. Until that lane existed the emitter was outside every CI
    # trigger — `embed-vendor-smoke.yml` filtered on package/verify but
    # not on `emit_embed_manifest.sh` — so this ran at push time and
    # nowhere else, which `--no-verify` skips.
    "embed-manifest-failfast": {
        "workflows": ["embed-vendor-smoke.yml"],
        "runner_workflow": True,
        "cost_s": 0,
        "summary": "emit_embed_manifest fail-fast case",
    },
    # `roadmap_marker_gate` reads every tracked file and
    # `workflow_trigger_coverage` reads every workflow, so tree-hygiene.yml
    # declares no `paths:` filter — no glob list is both correct and
    # narrower than the tree. That missing filter is what makes this gate
    # unconditional, derived the same way every other workflow-backed
    # trigger is rather than hand-set to "always".
    "tree-hygiene": {
        "workflows": ["tree-hygiene.yml"],
        "runner_workflow": True,
        # 193s measured 2026-08-22, against a declared 13 — the single
        # largest gate a push could run, and it runs on EVERY push because
        # its workflow is unfiltered. The old number was not a lie about the
        # work, it was fifteen times out of date, which is the failure mode a
        # hand-written cost has and the reason `--measure` exists.
        "ci_only": "193s measured, on every push, because a tree-wide gate "
                   "has no path filter to narrow it. Nothing else in the "
                   "budget comes close. tree-hygiene.yml is unfiltered too, "
                   "so CI runs it on exactly the pushes this would have — "
                   "the delegation is total rather than partial.",
        "cost_s": 193,
        "summary": "tree-wide marker + trigger + parity gates",
    },
    # Rides the same unfiltered workflow, for the same reason in a different
    # alphabet: what this gate reads is the union of every `mutation_targets`
    # declaration inside `sce-build/tests/mutations/*.cases`, which today
    # spans `sce/`, `sce-build/`, `backends/`, generator templates, a
    # CMakeLists, two workflows and a gate script. A `paths:` list restating
    # that union would be a second copy of a declaration the casefiles
    # already carry, stale the first time a case names a new file.
    "mutation-cases": {
        "workflows": ["tree-hygiene.yml"],
        "runner_workflow": True,
        "ci_only": "168s checking 500 cases across 72 casefiles, and it is "
                   "ALWAYS-ON — its workflow is unfiltered, so every push paid "
                   "it. Declared 33s until it was measured in a run of its "
                   "own: that reading came from a full sweep where earlier "
                   "gates had warmed everything, and a push runs a subset "
                   "where each gate pays its own warm-up. tree-hygiene.yml is "
                   "unfiltered too, so CI runs it on every push regardless.",
        "cost_s": 168,
        "summary": "every mutation casefile still applies",
    },
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
    "mutation-rounds": {
        "workflows": ["mutation-rounds.yml"],
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
        "cost_s": 877,
        "summary": "changed mutation targets still turn their suites red",
    },
    "clippy": {
        "workflows": ["clippy-check.yml"],
        "runner_workflow": True,
        # The one that hurts. Lint is the fastest feedback a Rust change can
        # get and it leaves anyway, because the budget is a ceiling on what a
        # developer waits for and is not a ranking of what is useful. It was
        # measured at 15s, 42s and 82s in one afternoon on this machine, which
        # is also the clearest single illustration of why the numbers here
        # cannot be trusted to a factor of two.
        "ci_only": "82s under load. clippy-check.yml runs it. Kept local "
                   "through three earlier cuts and only moved when the table "
                   "was priced at what a loaded machine charges.",
        "cost_s": 82,
        "summary": "cargo clippy --workspace --all-targets",
    },
    "nostd-mcu": {
        "workflows": ["sce-rust-runtime-no-std.yml"],
        "runner_workflow": True,
        "deps": ["codegen-build"],
        # 16s, pace-normalised from the 2026-08-23 push — cheaper than the 29s
        # it declared, so this one was making the table pessimistic rather than
        # optimistic. Both directions are drift; both are worth correcting.
        "cost_s": 16,
        "summary": "no_std MCU build + clippy + probes",
    },
    # Mirrors doc-check.yml. The numbered table mapped this to
    # sce-rust-runtime-no-std.yml while the gate's own comment named
    # doc-check.yml; the two share the `backends/rust/runtime/**` trigger,
    # so the mismatch never showed as a missed run except for edits to
    # doc-check.yml itself.
    "rustdoc-links": {
        "workflows": ["doc-check.yml"],
        "runner_workflow": True,
        "cost_s": 4,
        "summary": "cargo doc broken intra-doc links, both profiles",
    },
    "embed-vendor": {
        "workflows": ["embed-vendor-smoke.yml"],
        # The reason this lane kept its own copy — "it also calls a sibling
        # gate, so its steps are a superset" — did not hold: the sibling
        # (`embed-manifest-failfast`) runs in a SEPARATE job of the same
        # workflow, and this job restated the three scripts verbatim.
        "runner_workflow": True,
        "extra": ["sce/include/**", "embed/MANIFEST.json"],
        # The most expensive gate in the set: three scratch-directory builds,
        # one of which packages embed/ from scratch and builds a consumer
        # against it.
        "ci_only": "311s, and on its own it is more than the whole push "
                   "budget of 300s. Three scratch-directory builds, one of "
                   "which packages embed/ from scratch and builds a consumer "
                   "against it — none of that gets cheaper on a warm tree, "
                   "because the scratch directories are the point.",
        "cost_s": 311,
        "summary": "embed manifest drift + payload lag + consumer smoke",
    },
    # Mirrors the Rust workspace suite only. `w3c-tests.yml` is deliberately
    # NOT listed: it has no `paths:` filter (it runs on every CI push) and
    # it drives the C++ ctest suite, which this hook does not mirror at all.
    # Mapping it here would make its catch-all the trigger for the hook's
    # longest gate and, worse, would classify every path — defeating the
    # unclassified-forces-FULL rule that keeps this file honest.
    # `extra` covers inputs the workflow filter misses because they are
    # `include_str!`-ed into the suite rather than compiled: the acceptance
    # doc, the wire schemas, and the forge-AST export schema.
    "workspace-tests": {
        "workflows": ["rust-workspace-tests.yml"],
        # The lane's one difference — `--no-fail-fast`, so a run nobody can
        # iterate on reports every failure — was a reporting choice by its own
        # description, which made it a switch rather than a reason to keep two
        # spellings. The gate reads `SCE_GATE_NO_FAIL_FAST` and the lane sets
        # it.
        "runner_workflow": True,
        # ⚠ `.github/workflows/**` is deliberately NOT here, though the suite
        # reads every file in that directory at run time
        # (`mutation_round_survives_the_next_push`, `ci_supersession_policy`).
        # `extra` is the HOOK's trigger list as well as the CI-coverage one,
        # and adding it classified every workflow path — which broke
        # `unfiltered-workflow-self`: editing a workflow that declares no
        # `paths:` of its own is meant to be unclassified and buy the full run,
        # and a `ci_only` gate cannot take that job because the hook is never
        # offered it. The path would have read as known and selected nothing.
        #
        # The coverage the suite needs is one-directional and belongs on the
        # other side: `rust-workspace-tests.yml` lists `.github/workflows/**`
        # in its `paths:`, so the lane starts for a workflow edit. A workflow
        # filter WIDER than its gate's triggers is what `ci-only-coverage`
        # allows; the reverse is what it refuses.
        "extra": ["docs/SCE_ACCEPTED_SUBSET.md", "schemas/**", "apis/**"],
        # The crate whose test suite this gate runs. `include-str-coverage`
        # in the self-test reads it: every file the sources under this root
        # `include_str!` is an input the suite asserts on, so some lane that
        # runs the suite has to start when it changes. Declaring the root is
        # the hand-written half; which paths it implies comes from the tree,
        # so an `include_str!` added tomorrow is covered the day it is
        # written rather than the day somebody remembers a list.
        "include_str_roots": ["sce-build"],
        # 151s — half the budget for one gate, and the hardest of the four to
        # give up, so the reason is stated rather than left to the number.
        # This is the densest correctness net in the tree: every contract test
        # under `sce-build/tests/` runs here. It leaves anyway because the
        # budget is a ceiling on what a developer waits for, not a ranking of
        # what is valuable, and a gate that eats half the ceiling decides the
        # budget for every other gate. `rust-workspace-tests.yml` runs the
        # same command with `--no-fail-fast`.
        "ci_only": "151s, half the 300s push budget for one gate. Fully "
                   "mirrored by rust-workspace-tests.yml, which runs the same "
                   "command. The trade is the sharpest of the four: this is "
                   "where every sce-build contract test lives, so a broken "
                   "contract now reaches main and is answered a round later.",
        "cost_s": 151,
        "summary": "cargo test --workspace --features cli",
    },
    # The one reader of the axis both drift hashes miss. `source-hash` and
    # `template-hash` cover the INPUTS — documents and templates — so an edit
    # to the emit code under `sce-build/src` that changes what comes out moves
    # neither, and the committed trees drift with every gate green. This
    # regenerates them and compares, which is also what makes the procedure's
    # own documented claim ("regenerate and expect no diff") checked rather
    # than asserted.
    "regen-reproduces": {
        "workflows": ["regen-reproduces.yml"],
        "runner_workflow": True,
        "deps": ["codegen-build"],
        "ci_only": "117s regenerating every committed tree, which is work "
                   "proportional to the corpus and not to the change. "
                   "regen-reproduces.yml runs it. What this costs is worth "
                   "naming: a template edit that does not carry its "
                   "regenerated trees now lands, and that exact miss shipped "
                   "once already (PR-90, where enter_at panicked in the real "
                   "generated code while the branch's own suite was green).",
        "cost_s": 117,
        "summary": "regeneration reproduces every committed tree",
    },
    "drift-suites": {
        "workflows": ["drift-verify.yml"],
        "runner_workflow": True,
        "ci_only": "146s. It is declared serial on purpose and that is what "
                   "it costs; drift-verify.yml runs it.",
        "cost_s": 146,
        "summary": "committed-tree drift + sourcemap, serial",
    },
    # forge-conformance.yml verifies the language arms in parallel jobs, and
    # its path filter is workflow-wide: a change under
    # backends/*/forge-runtime/** starts all of them in CI. These gates
    # therefore fire together too, which is the parity we want. The split is
    # for attribution — a failure names its arm — and each `extra` widens
    # the trigger past the workflow filter rather than narrowing it, so a
    # change outside forge-runtime still reaches the arm it could break.
    "forge-go": {
        "workflows": ["forge-conformance.yml"],
        "runner_workflow": True,
        "extra": ["backends/go/**"],
        "deps": ["codegen-build"],
        # 36s, pace-normalised on 2026-08-24, down from a declared 52s. Left
        # local: it is now the cheapest of the forge arms and the only one that
        # still runs before a push.
        "cost_s": 36,
        "summary": "Go forge conformance regenerate + test",
    },
    "forge-rust": {
        "workflows": ["forge-conformance.yml"],
        "runner_workflow": True,
        "extra": ["backends/rust/**"],
        # Warm reads as 0; the release profile it needs is a separate build
        # tree from every other gate, so a cold run pays that once.
        "ci_only": "96s, a release build of the Rust forge arm. Its siblings "
                   "are already in CI for the same reason and they share one "
                   "workflow, forge-conformance.yml.",
        "cost_s": 96,
        "summary": "Rust forge conformance (numerical, release)",
    },
    "forge-python": {
        "workflows": ["forge-conformance.yml"],
        "runner_workflow": True,
        "extra": ["backends/python/**"],
        "ci_only": "107s. forge-conformance.yml verifies every language arm "
                   "in parallel jobs, which is where this belongs — the arms "
                   "fire together anyway, so paying for them serially at push "
                   "time bought attribution and nothing else.",
        "cost_s": 107,
        "summary": "Python forge conformance (numerical)",
    },
    "forge-cpp": {
        "workflows": ["forge-conformance.yml"],
        # Two reasons were recorded here for keeping a second spelling and
        # neither survived. The log artifact does not need the lane's own
        # commands — `scripts/gate forge-cpp | tee` keeps it. The build
        # configuration was real: the lane built RelWithDebInfo under Ninja
        # while the gate left CMAKE_BUILD_TYPE unset, so a local pass said
        # nothing about the binary CI judged. The gate now carries the build
        # type (and Ninja where installed), which cost 6s -> 22s and is what
        # the mirror is for.
        "runner_workflow": True,
        "extra": ["backends/cpp/**", "sce/**"],
        "deps": ["codegen-build"],
        "ci_only": "59s, the last move needed to fit the ceiling. "
                   "forge-conformance.yml runs it beside the other arms.",
        "cost_s": 59,
        "summary": "C++ forge conformance build + test",
    },
    # Catches codegen breakage in the example documents (the namespace
    # migration that broke them shipped green otherwise) and lints every
    # document this repository authors. The trigger comes from the
    # workflow's `paths:` like every other mirrored gate; it used to be a
    # `local` list, and that list named `examples/**` while the gate's own
    # sweep also read `integration_resources/*/*.scxml` — so editing an
    # authored document there ran the lint never. The self-test now
    # compares a gate's sweep against its trigger.
    "example-codegen": {
        "workflows": ["example-codegen.yml"],
        # The workflow runs `scripts/gate example-codegen` instead of
        # restating this gate's commands, so the check has one spelling
        # for both callers. Every other mirrored gate still carries a
        # second copy of its commands in its workflow; this flag is what
        # makes the difference measurable per gate rather than a claim
        # about the set, and the self-test below holds the delegation.
        "runner_workflow": True,
        "deps": ["codegen-build"],
        "ci_only": "99s against a declared 4s — the widest gap in the table, "
                   "and the clearest case of a warmed-sweep reading being "
                   "mistaken for what a push pays. It generates from every "
                   "authored document under examples/ and integration_"
                   "resources/, which is work proportional to the corpus. "
                   "example-codegen.yml runs it.",
        "cost_s": 99,
        "summary": "example SCXML codegen smoke + authored-document lint",
    },
    # The lane that starts on a compiled-in INPUT rather than on Rust.
    #
    # `workspace-tests` runs the same library assertions, but its filter is
    # narrow on purpose — it is 151s, half the push budget, and widening it
    # to reach `sce/include/**`, `backends/**` and `tools/codegen/**` would
    # start it on nearly every push. So the wide filter lives here, on a
    # lane that only builds the library suite. Both gates declare the same
    # `include_str_roots`, and `include-str-coverage` checks their UNION:
    # what has to hold is that SOME lane running the suite starts for the
    # path, which is exactly what splitting by cost is allowed to satisfy.
    "doc-content": {
        "workflows": ["doc-content-gate.yml"],
        "runner_workflow": True,
        "include_str_roots": ["sce-build"],
        "ci_only": "the library suite plus its build, on a filter wide "
                   "enough to reach every compiled-in input. It is here "
                   "rather than in the push because the push already "
                   "delegates the same assertions through workspace-tests; "
                   "what this lane adds is the paths that lane does not "
                   "start for. doc-content-gate.yml runs it.",
        "cost_s": 60,
        "summary": "sce-build lib suite, compiled-in inputs",
    },
    "ledger-citations": {
        "workflows": ["spec-citations.yml"],
        "runner_workflow": True,
        # Where the time goes, re-measured 2026-08-11 evening after the
        # mnemosyne pin moved to c9b276bf. `validate-code-refs` used to be
        # 108.9s of the 110s this gate took alone, because the symbol resolver
        # re-read, re-parsed and re-compiled its tree-sitter query once per
        # CITATION. Upstream now resolves once per FILE, and this tree is where
        # that ratio is visible: 4487 citations over 290 files across the five
        # workspaces, so the call count fell 15.5x and the wall clock fell
        # further still (synth 82.1 -> 0.90, scxml 11.5 -> 0.54, mesh 12.5 ->
        # 0.29, wire 0.40, bytesguard 0.45 — 2.6s in total). The whole-tree
        # existence sweep, 4.4s over 6357 files, is now the larger half.
        # `cost_s` stays at the `--measure` figure the runner compares against.
        "ci_only": "121s across five workspaces. The registry's own header "
                   "already told this story once — the hook's prose called it "
                   "sub-second per workspace while a measurement put the whole "
                   "gate at 157s — and the 15s it was declared at was the same "
                   "mistake in the other direction, read off a warmed full "
                   "sweep. spec-citations.yml runs it.",
        "cost_s": 121,
        "summary": "spec-citation ledgers, 5 workspaces",
    },
    # The gate whose absence let a stale verifies-catalog reach CI red:
    # `ledger-citations` runs mnemosyne-cli, this workflow runs a separate
    # python generator, and the hook mirrored only the first.
    "spec-snapshot": {
        "workflows": ["spec-snapshot-drift.yml"],
        "runner_workflow": True,
        "cost_s": 13,
        "summary": "spec snapshot integrity + verifies-catalog drift",
    },
    # A compliance verdict — an LGPL section 1 / MIT section 1 violation in a
    # release tarball — that was reachable only after a push, on a check that
    # needs nothing but bash and takes three hundredths of a second.
    "license-ssot": {
        "workflows": ["license-verify.yml"],
        "runner_workflow": True,
        "cost_s": 0,
        "summary": "license SSOT vs the tree it describes",
    },
    # Where the W3C BasicHTTP fixture listener answers had come to be spelled in
    # five places at once — twelve C11 runners, two gate helpers, a CI job, the
    # Rust and Go harnesses, two Python suites — and the drift only became
    # visible when a second checkout could not run the BasicHTTP suites while
    # the first held the port. `tests/w3c/basic_http_test_endpoint.h` owns it
    # now; this keeps that ownership a property of the tree.
    #
    # Trigger is deliberately the catch-all, and that is affordable rather than
    # lazy: a re-spelling can appear in any file, and narrowing to the paths we
    # happen to remember would miss exactly the one nobody thought of. The gate
    # asks `git grep` for the handful of files that name the number at all and
    # runs its comment-stripper only on those — 124ms measured, down from 14.7s
    # when it stripped every tracked source file.
    #
    # It gets its own lane rather than riding another's because nothing else
    # can catch the defect: a re-spelled port that equals the owner's still
    # WORKS, so every suite stays green and the drift surfaces only the day
    # someone moves the endpoint. There is no compile error to lean on.
    "http-endpoint-ssot": {
        "workflows": ["http-endpoint-ssot.yml"],
        "runner_workflow": True,
        # 4s, pace-normalised on 2026-08-24. It read 0 because the scan itself
        # is 124ms; what the 0 left out is the process the runner has to start
        # around it, which every gate pays and only the cheap ones notice.
        "cost_s": 4,
        "summary": "BasicHTTP fixture endpoint has one owner",
    },
    # The generated codecs are committed goldens, not workspace members:
    # `cargo metadata` lists eight packages and none of them is a codec, so
    # `clippy --workspace` lints the generator and never its output. Both arms
    # were CI-only until the registry started asking which workflows lack a
    # gate.
    "codec-clippy": {
        "workflows": ["sce-forge-codec-clippy.yml"],
        "runner_workflow": True,
        "ci_only": "106s linting 95 generated codec goldens. "
                   "sce-forge-codec-clippy.yml runs it.",
        "cost_s": 106,
        "summary": "clippy generated codecs, alloc on",
    },
    "codec-no-alloc": {
        "workflows": ["sce-forge-codec-no-alloc.yml"],
        "runner_workflow": True,
        "ci_only": "90s. sce-forge-codec-no-alloc.yml runs it, next to its "
                   "alloc-on sibling.",
        "cost_s": 90,
        "summary": "generated codecs compile without alloc",
    },
    # The two conformance surfaces nothing local ran. Both narrow their
    # workflow's catch-all (see `narrows` in `gate_triggers`) and state their
    # real inputs here: the engine, the generator, the templates that emit the
    # AOT runners, and the fixtures the runners are registered from.
    #
    # The Rust arm of the same workflow is deliberately absent: `sce-rust-tests`
    # is a workspace member, so `workspace-tests` already runs its 202 cases and
    # a second gate would be a second spelling of the same run.
    "w3c-cpp": {
        "workflows": ["w3c-tests.yml"],
        "runner_workflow": True,
        "narrows": "the lane declares no `paths:` filter, which is a statement "
                   "about a runner paid by the minute, not about what the "
                   "suite reads. Inheriting it would run the longest gate in "
                   "the set for a README edit.",
        "extra": ["sce/**", "tests/w3c/**", "tests/CMakeLists.txt",
                  "resources/**", "tools/codegen/templates/**",
                  "sce-build/src/**"],
        "deps": ["codegen-build"],
        "ci_only": "55s, and it is the gate that decides whether the budget "
                   "is met on a good run or on every run. Without it the set "
                   "measured 286s against a 300s ceiling, and the same gates "
                   "measured twice varied by as much as 27s (clippy alone "
                   "swung 15s to 42s), so fourteen seconds of headroom is "
                   "none. w3c-tests.yml declares no `paths:` filter, so CI "
                   "runs this arm on every push regardless.",
        "cost_s": 55,
        "summary": "W3C conformance, C++ Interpreter + AOT",
    },
    # A deploy workflow whose three build steps are verdicts. Its trigger
    # already includes `sce-build/**`, so a Rust change every other gate passes
    # could still turn it red — after the push.
    "visualizer-wasm": {
        "workflows": ["deploy-visualizer.yml"],
        "runner_workflow": True,
        "ci_only": "77s measured 2026-08-22 (declared 42) building three "
                   "WASM artifacts. deploy-visualizer.yml runs it, and this "
                   "gate declares no trigger of its own, so the workflow's "
                   "filter IS the trigger — the two cannot disagree.",
        "cost_s": 77,
        "summary": "codegen + visualizer + DOOM WASM builds",
    },
    # The other half of the main tree's ctest partition, and the half nothing
    # ran. Measured 2026-08-12 with a logging shim in place of `ctest` during a
    # full `scripts/gate --all`: 28 gates passed and the only runs against this
    # build were `w3c-c11`'s two. 159 of 382 registered cases — every `mesh_*`
    # case and every C++ unit suite — were executed by no gate and by no
    # workflow, since none configures the main tree.
    "cpp-suite": {
        "workflows": ["cpp-suite.yml"],
        "runner_workflow": True,
        "narrows": "the lane declares no `paths:` ALLOW filter, for the same "
                   "reason `w3c-cpp`'s does not: that is a statement about a "
                   "runner paid by the minute, not about what the suite "
                   "reads. This one builds the whole C++ tree, so its real "
                   "inputs are the engine, its tests, the mesh sources and "
                   "the CMake definitions that register them. Since "
                   "2026-08-29 it DOES declare a `paths-ignore:` deny list "
                   "(docs/**, **/*.md, .claude/**), which narrows nothing "
                   "this table claims: a deny list still starts the lane for "
                   "every path it does not name, so the catch-all reading "
                   "above holds. What the deny list changes is per-change "
                   "delegation, and `ci_owed` is where that is read.",
        "extra": ["sce/**", "tests/**", "cmake/**", "CMakeLists.txt",
                  "examples/**", "resources/**", "tools/codegen/templates/**",
                  "sce-build/src/**"],
        "deps": ["codegen-build"],
        "ci_only": "110s building and running the whole C++ ctest tree. "
                   "cpp-suite.yml runs it. Its `extra` list is wide on "
                   "purpose — sce/**, tests/**, examples/** — so it was "
                   "selected by most pushes, which is what made it a budget "
                   "problem rather than an occasional one.",
        "cost_s": 110,
        "summary": "C++ ctest suite (engine + mesh), the non-c11 half",
    },
    # The only gate that configures the OTHER value of SCE_SCRIPT_ENGINE.
    #
    # Every gate above builds the quickjs selection, which is what
    # `tests/CMakeLists.txt` said three times in as many words — "no gate
    # configures -DSCE_SCRIPT_ENGINE=lua" — and each of those comments named a
    # file compiled by every build and run by none. This one compiles a C++
    # artifact generated with `--script-engine lua` and runs it, which is the
    # measurement `docs/SCE_LUA_TRANSLATION_SEAM.md` ended without.
    #
    # Its workflow declares a `paths:` filter, so the filter IS the trigger and
    # there is no `narrows`/`extra` pair to keep in step with it. `sce/**` is
    # deliberately NOT in that filter as a whole: the selection reaches only the
    # scripting seam, the shared helpers the generated code calls, and the
    # option's own CMakeLists.
    "ecma262-lowered-cpp": {
        "workflows": ["ecma262-lowered-cpp.yml"],
        "runner_workflow": True,
        "deps": ["codegen-build"],
        # The scratch tree is thrown away on every run, so there is no warm
        # case for this gate the way there is for one that reuses `build/`:
        # `SCE_SCRIPT_ENGINE` is PUBLIC on `sce_scripting`, the selection is
        # therefore a property of the whole tree, and a developer's `build/` is
        # the quickjs one and must stay that way. What that costs is the sce
        # libraries from cold on every invocation, which is minutes rather than
        # the ~60s an already-built tree takes.
        #
        # 123s measured by `scripts/gate --measure` on the build machine
        # 2026-08-29, with ccache warm. 86 of those seconds are the
        # `sce_build_is_current` ctest FIXTURE, not the suite — the suite itself
        # runs in 0.16s — so the number is dominated by a staleness check the
        # tree makes every ctest case pay.
        #
        # So it is CI's, not the push's. A developer who wants it runs
        # `scripts/gate ecma262-lowered-cpp` by name.
        "ci_only": "it configures and builds a second tree from cold — the "
                   "`lua` selection cannot share the quickjs `build/` because "
                   "the option is PUBLIC on sce_scripting. "
                   "ecma262-lowered-cpp.yml runs it, on a filter naming the "
                   "seam's two halves and the two tables the fixture's "
                   "population comes from.",
        "cost_s": 123,
        "summary": "ECMA-262 through a Lua-lowered C++ artifact",
    },
    # The Kotlin twin of the row above, and it exists because the argument that
    # row makes applies unchanged to a second backend that had crossed the same
    # seam. Kotlin had `ScriptSource.lua`, a lowered arm on `LuaScriptEngine`
    # and a frontend that answers the shared table — and no lane that ran an
    # ARTIFACT through any of it.
    #
    # ⚠ It found a defect on its first run: the generated Kotlin evaluates a
    # transition's `cond` twice, so a guard with a side effect records the
    # wrong arm's answer. Declared in
    # `tests/ecmascript/kotlin_lowered_artifact_defects.json`, which this gate
    # holds in both directions.
    "ecma262-lowered-kotlin": {
        "workflows": ["ecma262-lowered-kotlin.yml"],
        "runner_workflow": True,
        "deps": ["codegen-build"],
        # Not `ci_only`, and the number is why. Unlike the C++ twin this gate
        # configures no second tree: it regenerates two machines into the
        # module's own Gradle build and runs 98 cases against them.
        #
        # ⚠ THE BASIS IS NOT THE WARM ONE THIS TABLE'S HEADER DESCRIBES, and
        # the departure is a faithful reading of that header rather than an
        # exception to it. "Warm" is defended up there as "a push happens on a
        # tree the developer has just built" — and NOTHING but this gate builds
        # this module. Its generation, its two machines and its test sources
        # are reached by no other target, so a warm reading here means "this
        # gate ran before", not "the developer just built the tree". The state
        # a push actually finds it in is the one its own `paths:` filter
        # selects for: a change under `tools/codegen/templates/**` or
        # `sce-build/src/**` invalidates the generation and the compile.
        #
        # So the number is measured from THAT state, worse of two runs, the
        # way the header takes the worse of two:
        #
        #   ./gradlew :sce-kotlin-lowered-ecma262:clean
        #   GRADLE_OPTS="-Dorg.gradle.caching=false" \
        #       scripts/gate --measure ecma262-lowered-kotlin
        #
        # 2026-08-30 on the build machine: 11s and 4s from clean; 1s and 1s
        # warm. 11 is declared. Over-declaring is the safe direction — the
        # ceiling check only refuses a gate that grew PAST its declaration —
        # and it keeps this row above `COST_NOISE_S`, so the gate contributes
        # a ratio to the pace estimate instead of being invisible to it.
        #
        # ⚠⚠ The line this replaced claimed "2s … INCLUDING a clean". Two
        # clean runs measured 4s and 11s, so that number was a warm reading
        # wearing a cold justification. Re-derive with the two commands above
        # rather than trusting this paragraph.
        #
        # ⚠ The suite itself always RUNS: the gate passes `--rerun`, because
        # its verdict is read from the run's output and Gradle answers an
        # unchanged test task UP-TO-DATE without producing any. So this is not
        # a cached green being timed.
        "cost_s": 11,
        "summary": "ECMA-262 through a Lua-lowered Kotlin artifact",
    },
    "w3c-c11": {
        "workflows": ["w3c-tests.yml"],
        "runner_workflow": True,
        "narrows": "same catch-all as `w3c-cpp`, same reason. This arm's real "
                   "inputs are the C backend, the C templates and the "
                   "generator.",
        "extra": ["backends/c/**", "tools/codegen/templates/**",
                  "sce-build/src/**"],
        "deps": ["codegen-build"],
        "ci_only": "391s inside a push, measured 2026-08-23 — and that number "
                   "is the point. The same 245 tests read 85s and 95s run on "
                   "their own, 258s and 391s inside the push that also builds "
                   "the generator and runs three other suites. Four readings "
                   "spanning 4.6x is not a cost that can be declared, and no "
                   "value fits a 300s ceiling that this gate alone can exceed. "
                   "It kept its place through two re-prices before the numbers "
                   "settled the argument; the owner's rule decides it — the "
                   "slow ones go to CI so a push stays under five minutes. "
                   "`w3c-tests.yml` has no `paths:` filter, so it runs on every "
                   "push and nothing narrows what CI sees.",
        "cost_s": 258,
        "summary": "W3C conformance, C11 MCU backend",
    },
    "w3c-go": {
        "workflows": ["w3c-tests.yml"],
        "runner_workflow": True,
        "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
                   "the Go backend, the Go templates and the generator.",
        "extra": ["backends/go/**", "tools/codegen/templates/**",
                  "sce-build/src/**"],
        "deps": ["codegen-build"],
        # 52s, pace-normalised from the 2026-08-23 push.
        "cost_s": 52,
        "summary": "W3C conformance, Go AOT",
    },
    "w3c-kotlin": {
        "workflows": ["w3c-tests.yml"],
        "runner_workflow": True,
        "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
                   "the Kotlin backend, the Kotlin templates and the "
                   "generator.",
        "extra": ["backends/kotlin/**", "tools/codegen/templates/**",
                  "sce-build/src/**"],
        "deps": ["codegen-build"],
        # 69s was measured on a push whose Kotlin tree was already current.
        # 152s, pace-normalised, is what it cost on 2026-08-24 when a
        # `tools/codegen/templates/**` edit invalidated every generated Kotlin
        # file — and that is not the unusual case for this row, because that
        # path is one of its own triggers. A gate is worth what it costs when
        # its trigger actually fires, so the higher reading is the one written
        # down. The earlier note said 10s / 34s, both taken with fewer suites
        # in the same run.
        "ci_only": "152s when a template edit fires its own trigger — half "
                   "the 300s push budget for one gate, and it breached the "
                   "ceiling on the push that measured it. w3c-tests.yml is "
                   "unfiltered, so CI runs this arm on every push regardless: "
                   "the same trade `w3c-python` took at less than half the "
                   "cost. What is given up is that a Kotlin AOT regression "
                   "now reaches main and is answered a round later.",
        "cost_s": 152,
        "summary": "W3C conformance, Kotlin/JVM AOT (Rhino + QuickJS + Lua, "
                   "each over the machines emitted for the language it reads)",
    },
    "w3c-python": {
        "workflows": ["w3c-tests.yml"],
        "runner_workflow": True,
        "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
                   "the Python backend, the Python templates and the "
                   "generator.",
        "extra": ["backends/python/**", "tools/codegen/templates/**",
                  "sce-build/src/**"],
        "deps": ["codegen-build"],
        "ci_only": "72s. w3c-tests.yml is unfiltered, so CI runs this arm on "
                   "every push.",
        "cost_s": 72,
        "summary": "W3C conformance, Python AOT",
    },
    # The wrapper layer, not the engine under it: the trigger is the binding
    # sources, because `w3c-cpp` already judges the same interpreter 404 cases
    # at a time. This carried no `cost_s` for as long as no machine with the
    # Python development headers had run it, which put it last by default.
    # Measured 2026-08-12 on a machine that has them: it builds the pybind11
    # extension and runs all 202 fixtures through it in 47s.
    "w3c-python-bindings": {
        "workflows": ["w3c-tests.yml"],
        "runner_workflow": True,
        "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
                   "the pybind11 wrapper sources.",
        "extra": ["backends/python/bindings/**"],
        "ci_only": "184s, the most expensive gate a push could still run once "
                   "the table was priced at what a loaded machine charges. "
                   "w3c-tests.yml declares no `paths:` filter, so CI runs this "
                   "arm on every push.",
        "cost_s": 184,
        "summary": "W3C conformance, pybind11 -> C++ Interpreter",
    },
}

# ── Workflows with no gate ────────────────────────────────────────
#
# The other direction of the mirror. Every case in the self-test below asks
# whether a GATE has a workflow — it maps one that exists, it delegates or
# says why, it declares `no_ci_reason` when CI has no counterpart. Nothing
# asked whether a WORKFLOW has a gate, and a registry that only checks its
# own entries can only find the gaps it already knows about.
#
# What that blindness cost is measured. On 2026-08-11 seven of nineteen
# workflows had no local mirror and nothing recorded that they did not — the
# largest being `w3c-tests.yml`, whose seven jobs run the C++, Rust, Kotlin,
# Python, Go and C11 conformance suites. Its C++ suite had been failing 233
# of 234 cases for three weeks while every gate in this file passed, because
# no gate ran it and the lane itself could not turn red.
#
# An entry here is the same kind of claim `no_ci_reason` is, pointed the
# other way: a statement that this workflow's verdict is reproduced
# somewhere a developer reaches before pushing, or that it has no verdict to
# reproduce. A `commit-hook:<command>` reason is checked rather than
# believed — the named command must appear in the commit hook.
UNMIRRORED_WORKFLOWS: dict[str, str] = {
    # Both run at commit time instead of push time, which is earlier than a
    # gate would catch them: a formatting fix-up never becomes its own
    # commit. The hook's stage comments named `rust-ci.yml` for years — a
    # workflow this repository does not have — which is exactly the drift
    # `rustdoc-links` had and the reason these claims are now checked.
    "fmt-check.yml": "commit-hook: cargo fmt --all -- --check",
    "clang-format-check.yml": "commit-hook: scripts/check_clang_format.sh",
}


# Lines that report rather than run. A hook's `log_step "Stage 0/3 — cargo fmt
# --all -- --check"` names the command it is about to run, and searching the
# file as text cannot tell that line from the invocation on the next one — so
# deleting the invocation left the claim satisfied by its own announcement.
# Measured, not supposed: that mutation passed before this existed. It is the
# defect T1 found in its own parity gate, which is that a comment stood in for
# the thing it describes.
_REPORTING_PREFIXES = ("log_step", "echo", "printf", "cat", ":")


def hook_runs(hook_src: str, command: str) -> bool:
    """Whether `command` appears on a line of `hook_src` that executes it."""
    for line in hook_src.splitlines():
        stripped = line.strip()
        if stripped.startswith("#"):
            continue
        if stripped.split(" ", 1)[0] in _REPORTING_PREFIXES:
            continue
        if command in stripped:
            return True
    return False

# Paths that drive no path-scoped gate. Anything here is classified — and
# therefore may skip the full run — so an entry is a claim that no such gate
# reads it. Rule 3 gates are outside that claim by construction: they run
# for every change, so an inert path is still judged by them. Everything NOT
# matched here and NOT matched by a gate trigger forces FULL, which is what
# keeps this list from being load-bearing in the unsafe direction.
INERT = [
    ".claude/**",
    ".gitignore",
    ".gitattributes",
    "*.md",
    "docs/adr/**",
    # `*.md` above is fnmatch, whose `*` does not cross `/`, so a document
    # one directory down falls through to FULL. This one is read by a gate —
    # `integration_stem_registration` requires every stem to appear in it —
    # and is inert anyway for the reason at the top of this list: that test
    # lives in `tree-hygiene`, whose workflow declares no `paths:` filter and
    # therefore runs for every change. Listed by name rather than as
    # `docs/**`, because a document under `docs/` that IS a path-scoped
    # gate's input and has no trigger would then be skipped in silence.
    #
    # ⚠ What that choice COSTS, measured 2026-08-29 and previously unpriced:
    # every other document under `docs/` is unclassified, so editing one
    # takes the Rule 1 branch and reaches ALL 34 gates. For the SSOT this
    # axis edits every round that is 10 local gates plus 24 handed to CI, of
    # which CI actually starts 8 — the rest now print under `--ci-unowed`.
    # Re-derive before acting on it, since both counts move with the table:
    #   printf 'docs/SCE_LUA_TRANSLATION_SEAM.md\n' > /tmp/c
    #   python3 tools/git-hooks/gate_registry.py --changed-from /tmp/c --explain
    # The fix is NOT to add `docs/**` here — that is the silence the comment
    # above refuses. It is to give the documents that drive no gate their own
    # named entries, one at a time, each a claim somebody checked.
    "docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md",
    "LICENSE*",
    # Local tooling. The one gate that reads these, `roadmap_marker_gate`,
    # reaches every change through `tree-hygiene`, and this file's own
    # cases run unconditionally before any gate is chosen.
    #
    # Narrower than it reads: a gate whose workflow delegates to the
    # runner (`runner_workflow`) names its own script, `scripts/gate` and
    # `scripts/gates/lib.sh` in that workflow's `paths:`, and a gate
    # trigger is consulted before this list — so editing one of those
    # selects its gate rather than falling through to here. What stays
    # inert is the scripts of gates CI still mirrors by restating their
    # commands, where the script is not an input to any workflow.
    "tools/git-hooks/**",
    "scripts/gates/**",
    "scripts/gate",
    # The mutation harness, for the same reason one line up: its only gate
    # is `mutation-cases`, which rides an unfiltered workflow and therefore
    # runs for every change regardless of what this list says. Without the
    # entry an edit here is an unclassified path, and rule 1 buys the entire
    # suite to verify a script that one always-on gate already exercises.
    "scripts/mutate",
]

# Entries here list only TRACKED paths, because a changed path comes from
# `git diff --name-only` and a gitignored one can never appear there. An
# ignored directory in this list would be dead config that reads as a
# decision.


def gate_script(repo_root: Path, slug: str) -> Path:
    return repo_root / "scripts" / "gates" / f"{slug}.sh"


def glob_to_regex(glob: str) -> re.Pattern:
    """Translate a GitHub Actions path glob into an anchored regex.

    Handles the three shapes the workflows actually use — `dir/**`,
    `**/*.ext`, and literal paths — plus the single-segment `*` that
    `backends/*/forge-runtime/**` needs. `**` crosses separators, `*` does
    not, matching Actions' own semantics.
    """
    out = ["^"]
    i = 0
    while i < len(glob):
        c = glob[i]
        if glob.startswith("**/", i):
            out.append("(.*/)?")
            i += 3
        elif glob.startswith("**", i):
            out.append(".*")
            i += 2
        elif c == "*":
            out.append("[^/]*")
            i += 1
        else:
            out.append(re.escape(c))
            i += 1
    out.append("$")
    return re.compile("".join(out))


def workflow_paths(repo_root: Path, name: str) -> list[str]:
    """The deduped ALLOW-side (`paths:`) globs a workflow declares under `on:`.

    A workflow with no `paths:` filter runs on every push, so it returns the
    catch-all — the honest translation of "CI always runs this".

    ⚠ Allow-side ONLY, and the asymmetry is deliberate rather than an
    oversight. A `paths-ignore:` workflow really does start on every push in
    this sense: there is no path list CI checks a change INTO. What it has is
    a list it checks changes OUT of, which is a different question with a
    different reader — `workflow_ignored_paths` below — because the two are
    consumed in different places. `narrows` asks this one ("is CI's trigger
    the catch-all, so the local table may state the real inputs instead?")
    and the answer stays yes. `ci_owed` asks the other ("will CI actually
    start for THIS change?"), and that is where a deny-list bites.

    Reading a deny-list as "unfiltered" here and stopping would have made the
    narrowing self-test pass by blindness the moment the first `paths-ignore`
    landed: `.github/workflows/cpp-suite.yml` acquired one on 2026-08-29 and
    every check in this file went on reporting the lane as unfiltered.
    `sce-build/tests/workflow_trigger_coverage.rs` already counted it
    ("`paths-ignore` counts: it is the same filter with the sense flipped"),
    so two readers of one file disagreed about whether it was filtered, and
    nothing held them in step. `both_filter_readers_agree` in the self-test
    below is now what does.
    """
    wf = repo_root / ".github" / "workflows" / name
    if not wf.is_file():
        # Missing workflow: refuse to guess. The caller turns an empty
        # trigger set into "always run", which is the safe direction.
        return ["**"]
    text = wf.read_text(encoding="utf-8")
    on_block = re.search(r"^on:\n(.*?)(?=^\S)", text, re.S | re.M)
    if not on_block:
        return ["**"]
    globs: list[str] = []
    for block in re.finditer(r"^(\s+)paths:\s*\n((?:\1\s+-.*\n|\s*#.*\n)*)",
                             on_block.group(1), re.M):
        for line in block.group(2).splitlines():
            line = line.strip()
            if line.startswith("- "):
                globs.append(line[2:].strip().strip("'\""))
    if not globs:
        return ["**"]
    return list(dict.fromkeys(globs))


def workflow_ignored_paths(repo_root: Path, name: str) -> list[str]:
    """The deduped DENY-side (`paths-ignore:`) globs a workflow declares.

    Empty means "nothing is excluded", which is what every workflow without
    the key says. A missing workflow also returns empty: the safe reading is
    that nothing is excluded, so no caller may conclude CI will skip.

    GitHub Actions refuses `paths` and `paths-ignore` on the same event, so
    this and `workflow_paths` are never both narrowing at once.
    """
    wf = repo_root / ".github" / "workflows" / name
    if not wf.is_file():
        return []
    text = wf.read_text(encoding="utf-8")
    on_block = re.search(r"^on:\n(.*?)(?=^\S)", text, re.S | re.M)
    if not on_block:
        return []
    globs: list[str] = []
    for block in re.finditer(r"^(\s+)paths-ignore:\s*\n((?:\1\s+-.*\n|\s*#.*\n)*)",
                             on_block.group(1), re.M):
        for line in block.group(2).splitlines():
            line = line.strip()
            if line.startswith("- "):
                globs.append(line[2:].strip().strip("'\""))
    return list(dict.fromkeys(globs))


def workflow_starts_for(repo_root: Path, name: str, changed: list[str]) -> bool:
    """Whether CI would start `name` for this change set.

    `workflow_paths` answers what a workflow is triggered BY in general;
    this answers whether a particular push starts it, which is the only
    form of the question `ci_owed` can honestly delegate on. A gate whose
    workflow does not start is not "owned by CI" — for that push it is not
    run anywhere, which `drop_ci_only` calls a gate that was deleted.
    """
    if not changed:
        return False
    ignore = [glob_to_regex(g) for g in workflow_ignored_paths(repo_root, name)]
    allow = workflow_paths(repo_root, name)
    allow_pats = None if allow == ["**"] else [glob_to_regex(g) for g in allow]
    for path in changed:
        if any(p.match(path) for p in ignore):
            continue
        if allow_pats is None or any(p.match(path) for p in allow_pats):
            return True
    return False


# Assembled rather than spelled so this file does not match itself when a
# reader greps for scripts that enumerate the tree.
_SWEEP_CALL = re.compile("ls-" + r"files((?:\s+'[^']*')+)")


def swept_globs(src: str) -> list[str]:
    """Globs a gate script hands to git's tracked-file enumeration.

    A sweep is what makes a gate's input set wider than its trigger. The
    trigger below decides WHEN a gate runs, from a path list written
    here; a sweep inside the script decides WHAT it judges, from the tree
    at run time. The two are separate statements about the same gate and
    nothing kept them in step.
    """
    out: list[str] = []
    for match in _SWEEP_CALL.finditer(src):
        out.extend(re.findall(r"'([^']*)'", match.group(1)))
    return out


def tracked_matching(repo_root: Path, glob: str) -> list[str]:
    """Tracked paths a sweep glob actually yields, asked of git itself.

    Comparing the sweep glob to a trigger glob as text would need a
    containment rule between two glob dialects. The tree answers the
    question directly and without a rule to get wrong.
    """
    proc = subprocess.run(
        ["git", "-C", str(repo_root), "ls-" + "files", glob],
        capture_output=True, text=True, check=False,
    )
    return [line for line in proc.stdout.splitlines() if line]


def gate_triggers(repo_root: Path) -> dict[str, list[str]]:
    """Each gate's trigger globs, derived from its workflow and its own lists.

    `narrows` is the one case where a gate does not inherit its workflow's
    trigger. A workflow with no `paths:` filter is telling CI "run this on
    every push", which is a statement about a runner that is paid for by the
    minute and not about what the check reads. Inheriting it locally makes
    the catch-all the trigger for the gate, so the whole conformance suite
    would run for a README edit. A narrowing gate states its real inputs in
    `local`/`extra` instead, and the self-test holds the two properties that
    keep the narrowing from becoming a coverage hole: the workflow it
    narrows must actually be unfiltered (or there is nothing to narrow), and
    the gate must be left with triggers of its own (or it never runs).
    """
    triggers: dict[str, list[str]] = {}
    for slug, spec in GATES.items():
        globs: list[str] = list(spec.get("local", []))
        for wf in spec.get("workflows", []):
            wf_globs = workflow_paths(repo_root, wf)
            if spec.get("narrows") and wf_globs == ["**"]:
                # The catch-all is dropped; the workflow FILE is not. Editing
                # a lane is a reason to run what that lane runs, and every
                # filtered workflow classifies its own edits by naming itself
                # in `paths:`. Without this line an unfiltered lane's file is
                # unclassified, so touching it buys the full suite — measured
                # at 27 gates including the 408s vendor smoke.
                globs.append(f".github/workflows/{wf}")
                continue
            globs.extend(wf_globs)
        globs.extend(spec.get("extra", []))
        triggers[slug] = list(dict.fromkeys(globs))
    return triggers


def run_order(slugs, table=None) -> list[str]:
    """Order the given gates: dependencies first, then cheapest first.

    Two properties, in that priority. A gate never runs before something it
    executes the output of, and among gates that are free to move, the one
    that can fail soonest goes first. Nothing about the order is written
    down as an order — it falls out of `deps` and `cost_s`, so adding a gate
    cannot silently push a cheap check behind an expensive one the way the
    hand-placed sequence did.

    A gate with no measured cost sorts last among the gates available at
    that moment rather than being guessed at: an unmeasured gate should not
    claim a cheap slot it has not earned.

    One gate is taken at a time, not one dependency level at a time. Level
    order looks equivalent and is not: it puts every dependent behind every
    independent gate, so a 1s gate that only needs the 1s generator build
    would run after the 399s vendor smoke. Taking the cheapest currently
    available gate lets a dependency unlock cheap work that then goes
    straight to the front.
    """
    table = GATES if table is None else table
    remaining = set(slugs)
    ordered: list[str] = []
    while remaining:
        ready = [s for s in remaining
                 if all(d not in remaining for d in table[s].get("deps", []))]
        if not ready:
            # A dependency cycle is a registry bug, not a runtime condition.
            raise ValueError(f"dependency cycle among {sorted(remaining)}")
        pick = min(ready, key=lambda s: (table[s].get("cost_s") is None,
                                         table[s].get("cost_s") or 0.0,
                                         s))
        ordered.append(pick)
        remaining.discard(pick)
    return ordered


# Measurement noise floor for the drift report below. `scripts/gate` times
# with bash's `SECONDS`, whose resolution is one second, and a gate's reading
# moves by a second or two between runs for reasons that say nothing about the
# declaration — a warm cache, a busy machine. A declared cost is treated as
# still true unless the run disagrees by more than this.
COST_NOISE_S = 2.0
COST_NOISE_FRACTION = 0.25


def cost_is_stale(declared, measured) -> bool:
    """Whether a run's timing genuinely disagrees with the declared cost."""
    if declared is None:
        return True  # an unmeasured gate has just been measured
    return abs(measured - declared) > max(COST_NOISE_S, declared * COST_NOISE_FRACTION)


def order_drift(measured: dict, table=None):
    """Report whether this run's timings would have ordered the gates differently.

    `cost_s` is hand-updated from `scripts/gate --measure --all`, and nothing
    noticed when a value went stale: every gate still passed, and the run order
    — the only thing the number decides — was quietly wrong. The registry's own
    order case cannot see it either, and says so: it derives the expected order
    from `cost_s` and then checks the order against `cost_s`, so editing a cost
    moves the gate and the property still holds.

    The question that is not a tautology is whether the TRUTH would have
    ordered them differently. This substitutes the run's own measurements for
    the declarations that have genuinely moved and re-derives the order; a
    difference means the declared costs are steering the run wrongly, and the
    caller says so. It is a report, not a verdict: a slow machine is not a
    reason to refuse a push.

    Returns None when the order stands, otherwise the two orders and the
    entries whose cost moved.
    """
    table = GATES if table is None else table
    slugs = [s for s in measured if s in table]
    if len(slugs) < 2:
        return None  # one gate cannot be out of order with itself
    declared_order = run_order(slugs, table)

    moved, fresh = [], {}
    for s in slugs:
        declared = table[s].get("cost_s")
        if cost_is_stale(declared, measured[s]):
            moved.append((s, declared, measured[s]))
            fresh[s] = dict(table[s], cost_s=measured[s])
        else:
            fresh[s] = table[s]
    if not moved:
        return None
    measured_order = run_order(slugs, fresh)
    if measured_order == declared_order:
        return None
    return {"declared": declared_order, "measured": measured_order, "moved": moved}


def drift_report_lines(drift, paced=None) -> list[str]:
    """What a reader is told when the declared costs mis-order a run.

    A function rather than prints inlined in `main` so the self-test can
    read it. The advice this returns is the whole point of PACE_NORMALISED
    — an inlined version was written first and nothing could check that the
    advice ever changed, which is the same "no witness for the effect"
    shape the registry keeps being bitten by.

    `paced` is injectable for the same reason `order_drift` takes a table:
    the self-test drives this from a FIXTURE, not from the live map. A case
    that reached into PACE_NORMALISED for a slug to demonstrate on would go
    vacuous the day the map empties — and an empty map is this work's
    SUCCESS condition, because each entry leaves when its gate is honestly
    re-measured on a quiet machine. A witness that dies when the work
    finishes is not a witness.
    """
    paced = PACE_NORMALISED if paced is None else paced
    lines = ["\ngate: the declared costs no longer order this run correctly."]
    for slug, declared, actual in drift["moved"]:
        was = "unmeasured" if declared is None else f"{declared}s"
        lines.append(f"    {slug:<26} declared {was:>12}  ran {actual:g}s")
        if slug in paced:
            lines.append(f"    {'':<26} ^ pace-normalised on purpose: "
                         f"{paced[slug]}")
    lines.append(f"  ran in:   {' '.join(drift['declared'])}")
    lines.append(f"  would be: {' '.join(drift['measured'])}")
    if any(s in paced for s, _, _ in drift["moved"]):
        # Named, not hidden: the slug still appears above, because a
        # normalised cost can go stale like any other. What changes is the
        # instruction — "update cost_s" is wrong for these, and a reader
        # who follows it undoes somebody's measurement.
        lines.append("  A pace-normalised cost is EXPECTED to read lower "
                     "than it declares.")
        lines.append("  Do not lower it to the raw figure; re-measure the "
                     "others.\n")
    else:
        lines.append("  Re-measure with `scripts/gate --measure --all` and "
                     "update cost_s.\n")
    return lines


def budget_breach(measured: dict, table=None):
    """Whether this run PROVES the push budget is breached.

    `order_drift` above deliberately stops at a report, and the reason it
    gives is right: a slow machine is not a reason to refuse a push. But the
    number it is reporting on does not only decide the run order — it is what
    `budget_is_not_exceeded` sums, so a stale `cost_s` also means the ceiling
    is being checked against a fiction. Measured 2026-08-22: `tree-hygiene`
    was declared 13s and ran 193s, and the runner threw that measurement away
    (`scripts/gate` called this file with `|| true`), so a gate fifteen times
    its declared cost sat inside the budget for as long as nobody timed it by
    hand.

    The slow-machine objection is answered rather than ignored. A machine that
    is uniformly slow inflates every gate by about the same FACTOR, so the
    run's own median ratio estimates its pace and dividing it out leaves each
    gate expressed in the units `cost_s` is written in. What survives that is
    a gate whose cost moved relative to its siblings — which is a fact about
    the table, not about the machine, and is the only thing this refuses a
    push for.

    Returns None when the ceiling still stands, otherwise the recomputed worst
    case and the gates whose measurement put it over.
    """
    table = GATES if table is None else table
    local = {s for s in table if not table[s].get("ci_only")}
    seen = [s for s in measured if s in local]

    # Pace needs several gates with a non-trivial declared cost: one gate
    # cannot tell a slow machine from a grown gate, and a gate declared 0s
    # has no ratio to contribute.
    ratios = sorted(
        measured[s] / table[s]["cost_s"]
        for s in seen
        if (table[s].get("cost_s") or 0) >= COST_NOISE_S
    )
    if len(ratios) < 3:
        return None
    mid = len(ratios) // 2
    pace = ratios[mid] if len(ratios) % 2 else (ratios[mid - 1] + ratios[mid]) / 2
    if pace <= 0:
        return None
    # A pace below 1 is not divided out — it is clamped away.
    #
    # This threshold fires in ONE direction: it refuses when the total is too
    # high and is silent otherwise. So the two halves of the pace estimate are
    # not symmetric. A pace ABOVE 1 (a slow machine) divides measurements DOWN,
    # which is the forgiving side and the reason the estimate exists. A pace
    # BELOW 1 divides them UP — it manufactures cost, on the only side that can
    # refuse a push, out of a run that was FASTER than the table.
    #
    # And the median is pulled below 1 structurally, not by accident.
    # `PACE_NORMALISED` holds five of the eight local gates whose `cost_s`
    # clears the noise floor, and every one of those figures is a share of a
    # full push rather than a stopwatch read of the gate alone — this file
    # says so where it prints them ("A pace-normalised cost is EXPECTED to
    # read lower than it declares"). Their ratios therefore sit near 0.3
    # whatever the machine is doing, and they are the majority of the sample.
    #
    # Measured 2026-09-02: `55620099a7` was refused at "317s over the 300s
    # ceiling" with all nine gates passing and ~101s actually spent. The
    # median came to 0.33, and dividing by it read `nostd-mcu` at 32s as 96s
    # and `rustdoc-links` — which had matched its declaration to the second —
    # as 12s. The same commit was pushed again minutes later and passed; the
    # only thing that had changed was a warm build cache. A gate that answers
    # red and then green for one tree is not measuring the tree.
    #
    # Clamping keeps the half that was doing work and drops the half that was
    # inventing it: a slow machine is still forgiven, and a genuinely grown
    # gate is still caught, because at pace 1 the comparison is simply the
    # measurement against the declaration.
    pace = max(pace, 1.0)

    total = 0.0
    grown = []
    for slug in local:
        declared = table[slug].get("cost_s") or 0
        if slug not in measured:
            total += declared
            continue
        # What this gate would have cost on the machine the table was measured
        # on, so the comparison is like for like.
        at_table_pace = measured[slug] / pace
        if cost_is_stale(declared, at_table_pace):
            total += at_table_pace
            grown.append((slug, declared, round(at_table_pace)))
        else:
            total += declared
    if total <= PUSH_BUDGET_S:
        return None
    grown.sort(key=lambda row: row[1] - row[2])
    return {"total": round(total), "grown": grown, "pace": round(pace, 2)}


def transitive_deps(slug: str, table=None) -> set[str]:
    """Every gate `slug` needs, directly or through another gate."""
    table = GATES if table is None else table
    seen: set[str] = set()
    stack = list(table[slug].get("deps", []))
    while stack:
        d = stack.pop()
        if d in seen:
            continue
        seen.add(d)
        stack.extend(table[d].get("deps", []))
    return seen


def matched(repo_root: Path, changed: list[str]) -> tuple[set[str], str]:
    """Every gate the change set reaches, BEFORE `ci_only` is applied.

    Split out of `select` so the two readers of this answer share one
    implementation. `select` drops the `ci_only` gates and runs the rest;
    `ci_owed` keeps exactly the ones it dropped, which is what the hook
    prints so a developer knows which verdicts the push handed to CI. A
    second copy of the matching would be a second answer to "what does this
    change touch", and the two would disagree the first time a trigger moved.
    """
    if not changed:
        return (set(), "no changed paths — nothing to verify")

    triggers = gate_triggers(repo_root)
    # Rule 3. Held out of `compiled` so they cannot classify a path, and
    # seeded into `selected` so they still run. `workflow_paths` also
    # returns the catch-all for a workflow it cannot find, which lands here
    # as "run it, learn nothing" — the safe reading of a missing file.
    always_on = {k for k, globs in triggers.items() if "**" in globs}
    compiled = {k: [glob_to_regex(g) for g in v]
                for k, v in triggers.items() if k not in always_on}
    inert = [glob_to_regex(g) for g in INERT]

    selected: set[str] = set(always_on)
    for path in changed:
        hit = False
        for slug, pats in compiled.items():
            if any(p.match(path) for p in pats):
                selected.add(slug)
                hit = True
        if hit:
            continue
        if any(p.match(path) for p in inert):
            continue
        # Rule 1: an unclassified path buys the full run rather than silence.
        return (set(GATES),
                f"unclassified path '{path}' — running every gate")

    # Dependency closure, applied until it stops growing: a dep may itself
    # have deps.
    changed_set = True
    while changed_set:
        changed_set = False
        for slug in list(selected):
            for dep in GATES[slug].get("deps", []):
                if dep not in selected:
                    selected.add(dep)
                    changed_set = True

    return (selected, "path-scoped selection")


def select(repo_root: Path, changed: list[str]) -> tuple[list[str], str]:
    """Return (gates to run in order, reason). Empty list means nothing to do."""
    reached, reason = matched(repo_root, changed)
    return (run_order(drop_ci_only(reached)), reason)


def ci_owed(repo_root: Path, changed: list[str]) -> list[str]:
    """The gates this change reaches that only CI will run.

    A push that silently stops running four gates is how a lane stays red for
    eight pushes without anybody noticing — measured on this repository. The
    hook prints this list so the delegation is stated at the moment it
    happens, in the same terminal, naming the workflow that now owns each
    verdict.

    ⚠ A gate is only listed when a workflow carrying it would actually START
    for this change. Reaching a gate and delegating it are two different
    claims, and until 2026-08-29 nothing separated them, because no workflow
    had ever declared a `paths-ignore:`. `cpp-suite.yml` then acquired one
    and this list went on promising a `cpp-suite` verdict for a docs-only
    push that CI had just been told to skip — a promise nobody could see
    break, since the whole purpose of the line is to be the last word before
    the push. `drop_ci_only` already names what that would be: "a gate
    excluded from the push and absent from CI is a gate that was deleted."
    The exclusion is per-change, so the check has to be too.

    Reported rather than silently dropped is deliberate: `ci_unowed` below
    hands the hook the gates a change reaches that NEITHER side will run, so
    a deny-list that quietly removes coverage is visible at the same moment
    in the same terminal rather than inferred later from a lane that stopped
    reporting.
    """
    reached, _ = matched(repo_root, changed)
    return run_order({
        s for s in reached
        if GATES[s].get("ci_only")
        and any(workflow_starts_for(repo_root, wf, changed)
                for wf in GATES[s].get("workflows", []))
    })


def ci_unowed(repo_root: Path, changed: list[str]) -> list[str]:
    """`ci_only` gates this change reaches that NO workflow will start for.

    The residue of `ci_owed`, and the reason a deny-list cannot remove
    coverage in silence. A gate here is run by neither the push nor CI, so
    the change is going out unjudged by it. That is sometimes exactly right
    — a README cannot break a C++ ctest suite — and the point is that it is
    said out loud rather than discovered when a lane has been quiet for
    eight pushes.
    """
    reached, _ = matched(repo_root, changed)
    return run_order({
        s for s in reached
        if GATES[s].get("ci_only")
        and not any(workflow_starts_for(repo_root, wf, changed)
                    for wf in GATES[s].get("workflows", []))
    })


def drop_ci_only(selected) -> set[str]:
    """Remove the gates a push does not run, keeping their deps.

    The hook mirrors CI for every other gate, and that symmetry is worth
    stating before breaking it: a check that runs in both places can be
    verified before pushing, which is the property the runner's toolchain pin
    exists to protect.

    `ci_only` buys back the two things a push cannot afford.

    The first is a cost unbounded by anything the developer chose.
    `mutation-rounds` measured 877s on the first push whose change set
    reached its casefiles, against 4s on a push that reached none, and there
    is no value of `cost_s` that is honest about both.

    The second is simply not fitting in `PUSH_BUDGET_S`. The set is derived,
    not curated: take the most expensive gate still local until the rest fit
    the ceiling. That produced `embed-vendor`, `workspace-tests`,
    `regen-reproduces` and `cpp-suite`, in that order, and left 280s. Greedy
    by cost is the honest reading of a budget — it moves the fewest gates,
    and it does not let "this one feels important" quietly raise the ceiling
    for everything else. Each moved gate states its own trade in its entry;
    `workspace-tests` has the sharpest one and says so.

    The trade recorded here is the owner's: a red arrives one round later,
    and the push stays a length somebody will keep paying. What makes that
    survivable is that every gate here has a CI workflow that runs it —
    `every_ci_only_gate_has_a_workflow` refuses one that does not, because a
    gate excluded from the push and absent from CI is a gate that was
    deleted.

    Applied at the end so it covers the full-run path too. Rule 1 hands back
    every gate for an unclassified path, and a gate excluded from the hook has
    to be excluded there as well or the exclusion holds only while the paths
    are understood — which is the one case it would matter least.
    """
    return {slug for slug in selected if not GATES[slug].get("ci_only")}


def mapping_table() -> dict[str, dict]:
    """The table `--mapping` publishes: GATES plus what only prose knew.

    `cost_s` alone does not say whether it is a stopwatch reading. Six of
    them are not — they are paced figures — and until this key existed the
    only place that was written was a comment, so a consumer comparing its
    own timings against the table had to scrape this file's source or get
    six gates wrong. Asking the registry is the whole point of `--mapping`;
    this makes the question askable.

    The key is emitted for EVERY gate, `None` where the cost is a raw
    reading. An absent key would mean both "this cost is raw" and "the
    registry is too old to say", and a consumer cannot tell those apart —
    the same absence-as-answer shape that made the drift report advise
    lowering numbers somebody had deliberately chosen. Its value is the
    REASON rather than a flag, because `true` alone sends the reader back
    to the prose this key exists to replace.
    """
    return {
        slug: dict(spec, pace_normalised=PACE_NORMALISED.get(slug))
        for slug, spec in GATES.items()
    }


def registry_source() -> list[str]:
    """This file's own lines, for the self-test cases that read its prose.

    Two of them do: the `# Not delegated:` note a delegating gate has to
    carry, and the pace-normalised comments PACE_NORMALISED mirrors. Both
    compare prose against a literal defined in this same file, so both
    have to read THIS file. `--repo-root` defaults to `.`, so resolving
    the path that way would let a run started from another directory
    check one checkout's comments against another checkout's tables and
    call the disagreement a defect.
    """
    return pathlib.Path(__file__).read_text(encoding="utf-8").splitlines()


def self_test(repo_root: Path) -> int:
    """Cases that pin the rules the registry must not break."""
    failures = []
    cases = 0

    def check(label, changed, want_full=None, want_has=(), want_lacks=()):
        nonlocal cases
        cases += 1
        keys, reason = select(repo_root, changed)
        # "Full" is every gate a PUSH can run. `select` is what the hook
        # calls, so a `ci_only` gate is not missing from its answer — it was
        # never on offer, and counting it would make rule 1 unprovable here.
        full = len(keys) == len(drop_ci_only(set(GATES)))
        if want_full is not None and full != want_full:
            failures.append(f"{label}: full={full}, wanted {want_full} ({reason})")
        for k in want_has:
            if k not in keys:
                failures.append(f"{label}: gate {k} missing from {keys}")
        for k in want_lacks:
            if k in keys:
                failures.append(f"{label}: gate {k} unexpectedly selected")

    # Rule 1 — the safety property. A path nobody classified runs everything.
    check("unclassified", ["some_new_top_level_dir/thing.txt"], want_full=True)
    # A `ci_only` gate is not on offer to the hook, including on the path
    # that hands back everything. Rule 1 is the case where an exclusion is
    # easiest to forget and least visible: it returns the whole table, so a
    # filter applied only to the narrow path would put the expensive gate
    # back into exactly the pushes that are already the longest.
    check("ci-only-stays-out-of-the-full-run",
          ["some_new_top_level_dir/thing.txt"], want_lacks=["mutation-rounds"])
    # Both halves of the mutation corpus are `ci_only` now — the applies-check
    # measured 168s once it was timed in a run of its own — so what this case
    # can still say is that editing a casefile is CLASSIFIED and buys neither
    # of them. That is the property rule 1 would otherwise hide by handing
    # back everything.
    check("ci-only-stays-out-of-a-scoped-run",
          ["sce-build/tests/mutations/ai_loop_history_rust.cases"],
          want_full=False, want_lacks=["mutation-rounds", "mutation-cases"])
    # Rule 3 — an always-on gate must not classify. Were the catch-all
    # allowed to count, this path would read as known and rule 1 would never
    # fire again for anything.
    # The witness is `mutation-cases` rather than `tree-hygiene`: both ride
    # the same unfiltered workflow and are therefore both always-on, and
    # `tree-hygiene` became `ci_only` when it was measured at 193s, so it is
    # no longer in any answer the hook gives. The property under test is
    # unchanged — an always-on gate still runs, and still does not classify.
    check("catch-all-does-not-classify", ["brand_new_dir/file.xyz"],
          want_full=True, want_has=["http-endpoint-ssot"])
    # Every other workflow classifies its own edits by naming itself in its
    # `paths:`. The unfiltered one has no such list to name itself in, so
    # editing it is unclassified and buys the full run.
    check("unfiltered-workflow-self", [".github/workflows/tree-hygiene.yml"],
          want_full=True)
    # Docs-only runs the tree-wide gates and nothing else: prose is still
    # tracked source as far as the marker gate is concerned.
    check("inert", ["README.md", ".claude/settings.json"],
          want_full=False, want_has=["http-endpoint-ssot"], want_lacks=["workspace-tests"])
    # A SCE-VERIFIES marker must reach the catalog gate.
    check("verifies-marker", ["tests/mesh/CustomTcpSocketOptionsTest.cpp"],
          want_has=["spec-snapshot"])
    # The hook's own sources are read by the tree-wide gates. They reach the
    # change through the unfiltered workflow, so the whole workspace sweep
    # no longer has to run to judge a hook edit.
    check("hook-self", ["tools/git-hooks/gate_registry.py"],
          want_full=False, want_has=["http-endpoint-ssot"], want_lacks=["workspace-tests"])
    # A gate script edit is judged the same way, for the same reason.
    check("gate-script-self", ["scripts/gates/clippy.sh"],
          want_full=False, want_has=["http-endpoint-ssot"], want_lacks=["workspace-tests"])
    # A ledger-only edit needs the citation gates, not the C++ build.
    check("ledger-only", ["docs/sce-ledger/mesh/.atomic/workspace.atomic.json"],
          want_full=False, want_lacks=["nostd-mcu", "forge-cpp", "ledger-citations"])
    # Rust source pulls in the Rust gates a push still runs. `workspace-tests`
    # used to be asserted here and is now `ci_only`, so it is checked from the
    # other side: the path must still be CLASSIFIED. A ci_only gate keeps its
    # triggers for exactly this reason — they are what stop rule 1 from
    # handing back the whole table for a path the registry does understand.
    check("rust", ["sce-build/src/mesh/deploy.rs"], want_full=False,
          want_has=["codegen-build", "rust-modrs-drift"],
          want_lacks=["workspace-tests", "clippy"])
    # Dependency closure: an example-only change still builds sce-codegen.
    # `example-codegen` is `ci_only` now, but the closure it pulls in is not:
    # the dependency must survive its dependent leaving, or a gate that
    # remains local loses the binary it executes.
    check("example-dep", ["examples/smart_light/smart_light.scxml"],
          want_has=["codegen-build"], want_lacks=["example-codegen"])
    # A template edit must reach the committed-tree drift gate.
    check("template", ["tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2"],
          want_full=False, want_has=["codegen-build"],
          want_lacks=["drift-suites"])
    # doc-check.yml edits must reach the gate that mirrors it. Under the
    # numbered table this gate was mapped to the no_std workflow, so an edit
    # to doc-check.yml alone did not select it.
    cases += 1
    keys, _ = select(repo_root, [".github/workflows/doc-check.yml"])
    if "rustdoc-links" not in keys:
        failures.append("doc-check-self: rustdoc-links missing from " + str(keys))

    # A gate that enumerates the tree must be triggered by everything it
    # enumerates. The two statements about a gate — when it runs and what
    # it judges — were written in different places and nothing compared
    # them, so they had drifted: `example-codegen` lints every authored
    # document under `integration_resources/` and its trigger named only
    # `examples/**`. Editing one of those documents ran the lint never.
    #
    # Worse than never-selected. Another gate's trigger matched the path,
    # so it counted as classified and rule 1 did not fire either — the
    # safety net that catches an unknown path cannot catch a known one
    # routed to the wrong gate.
    cases += 1
    triggers = gate_triggers(repo_root)
    sweeps_seen = 0
    for slug in sorted(GATES):
        script = gate_script(repo_root, slug)
        if not script.is_file():
            continue
        pats = [glob_to_regex(g) for g in triggers[slug]]
        for swept in swept_globs(script.read_text(encoding="utf-8")):
            sweeps_seen += 1
            for path in tracked_matching(repo_root, swept):
                if not any(p.match(path) for p in pats):
                    failures.append(
                        f"trigger: {slug} sweeps '{swept}' and judges "
                        f"'{path}', which no trigger of its own selects — "
                        f"the gate reads a path that never starts it")
                    break
    # Lower bound: a sweep extractor that stops matching would leave this
    # case reading nothing and still passing.
    if sweeps_seen < 1:
        failures.append(
            "trigger: no gate script was seen sweeping tracked files — "
            "the extractor is broken, not the gates")

    # The push budget, held as a number rather than as an intention.
    #
    # What is bounded is the WORST case: rule 1 hands back every gate a push
    # can run for any unclassified path, so that set is what a developer can
    # actually be made to wait for. Bounding an average would leave the
    # longest pushes unbounded, and those are the ones that decide whether
    # the hook gets bypassed.
    cases += 1
    local = drop_ci_only(set(GATES))
    worst = sum(GATES[slug].get("cost_s") or 0 for slug in local)
    if worst > PUSH_BUDGET_S:
        over = sorted(((GATES[s].get("cost_s") or 0, s) for s in local),
                      reverse=True)[:3]
        failures.append(
            f"budget: a push can be made to run {worst}s of gates, over the "
            f"{PUSH_BUDGET_S}s ceiling. Most expensive still local: "
            + ", ".join(f"{s} ({c}s)" for c, s in over)
            + ". Move the top one to `ci_only` — that is the rule the current "
              "set was derived by — rather than raising PUSH_BUDGET_S, which "
              "is the owner's number.")

    # The budget's other half: a run that PROVES the ceiling is breached must
    # say so, and a slow machine must not be mistaken for one.
    #
    # The second property is the subtle one and the one a later change is
    # likely to drop, because dropping it makes the check "stricter" and a
    # stricter check looks like a better one. It is not: refusing a push
    # because the developer's laptop is busy teaches people to reach for
    # SKIP_PREPUSH, and a bypassed gate is the state this repository has
    # already paid for twice.
    cases += 1
    local_costs = {s: (GATES[s].get("cost_s") or 0)
                   for s in GATES if not GATES[s].get("ci_only")}
    if budget_breach(dict(local_costs)) is not None:
        failures.append(
            "budget-breach: a run that matches the declared costs exactly was "
            "read as a breach")
    for factor in (2, 3):
        slow = {s: c * factor for s, c in local_costs.items()}
        if budget_breach(slow) is not None:
            failures.append(
                f"budget-breach: a machine running uniformly {factor}x slower "
                f"was read as a breach — the pace division is not working, and "
                f"a push refused for being on a busy laptop is a push somebody "
                f"bypasses")

    # The other half of that property, and the one it was missing: a run read
    # FASTER than the table must not be read as a breach either.
    #
    # The slow cases above are uniform, and a uniform run is the easy half —
    # every ratio agrees, so any median describes it. What actually reaches
    # this check is a SPREAD: `PACE_NORMALISED` figures read low because they
    # are shares of a full push, while a gate carrying a cold build reads
    # high, and the median lands among the low ones. Dividing by a median
    # below 1 then multiplies the high readings instead of excusing them.
    #
    # These are the timings `scripts/gate` actually handed this function
    # while pushing `55620099a7` on 2026-09-02. Nine gates passed, ~101s was
    # spent, and the answer was "317s over the 300s ceiling". Pushing the
    # identical commit again minutes later passed, the only difference being
    # a warm cache — so this row is a tree that was called red and green.
    #
    # It is written as measurements rather than as a factor because that is
    # what made it wrong: no single factor reproduces it.
    cases += 1
    a_fast_spread = {
        "embed-manifest-failfast": 0, "rust-modrs-drift": 0,
        "http-endpoint-ssot": 0, "rustdoc-links": 4, "codegen-build": 13,
        "ecma262-lowered-kotlin": 25, "nostd-mcu": 32, "forge-go": 12,
        "w3c-go": 15,
    }
    known = {s: v for s, v in a_fast_spread.items() if s in local_costs}
    if len(known) < 5:
        failures.append(
            "budget-breach: the recorded fast run names "
            f"{len(known)} gate(s) this table still has — re-record it from a "
            "push rather than deleting it, or this case stops measuring")
    elif budget_breach(dict(known)) is not None:
        breach = budget_breach(dict(known))
        failures.append(
            "budget-breach: a run FASTER than the table was read as a breach "
            f"({breach['total']}s, pace {breach['pace']}x) — a pace below 1 "
            "inflates every reading on the one side that can refuse a push, "
            "and this exact run was refused and then passed on a retry")
    grown = dict(local_costs)
    # Whichever gate is dearest today, rather than a name: the set moves every
    # time the table is re-priced, and a hard-coded slug that has since become
    # `ci_only` leaves this case silently reading nothing.
    victim = max(local_costs, key=lambda s: (local_costs[s], s))
    grown[victim] = PUSH_BUDGET_S * 2
    breach = budget_breach(grown)
    if breach is None:
        failures.append(
            "budget-breach: a gate grown to twice the whole budget did not "
            "register — the check reads nothing")
    elif not any(row[0] == victim for row in breach["grown"]):
        failures.append(
            f"budget-breach: the breach did not name the gate that caused it: "
            f"{breach['grown']}")

    # A gate held out of the push must still run somewhere, or it was not
    # moved to CI — it was deleted. Naming a workflow is the first half.
    cases += 1
    ci_only_seen = 0
    for slug in sorted(GATES):
        if not GATES[slug].get("ci_only"):
            continue
        ci_only_seen += 1
        if not GATES[slug].get("workflows"):
            failures.append(
                f"ci-only-coverage: {slug} is ci_only and names no workflow, "
                f"so nothing runs it at all")
    # Lower bound, for the same reason every sweep here carries one: an empty
    # ci_only set satisfies the loop above and proves nothing.
    if ci_only_seen < 1:
        failures.append(
            "ci-only-coverage: no gate is ci_only, so this case read nothing")

    # The second half, and the one that actually bites. A gate that runs
    # locally may be triggered by paths its own workflow's `paths:` filter
    # does not list — `extra` exists precisely for inputs the filter cannot
    # express. While the gate is local that asymmetry is harmless: the hook
    # covers the difference. The moment it becomes `ci_only` the difference
    # becomes a hole, and a hole in the shape nobody looks at, because both
    # halves report green.
    #
    # Found this way rather than reasoned: `workspace-tests` declares
    # `schemas/**`, `apis/**` and the acceptance doc, and
    # rust-workspace-tests.yml listed none of the three. Moving the gate as
    # it stood would have left an edit to any of them verified nowhere.
    #
    # Compared over tracked files rather than glob text, because two globs
    # that describe the same set rarely spell it the same way — the same
    # method the `trigger:` case above uses.
    cases += 1
    pairs_seen = 0
    for slug in sorted(GATES):
        spec = GATES[slug]
        if not spec.get("ci_only"):
            continue
        wf_globs: list[str] = []
        for wf in spec.get("workflows") or []:
            wf_globs.extend(workflow_paths(repo_root, wf))
        if "**" in wf_globs:
            # An unfiltered workflow runs on every push, so it cannot miss a
            # path. `cpp-suite` and `mutation-rounds` are both this shape.
            pairs_seen += 1
            continue
        wf_pats = [glob_to_regex(g) for g in wf_globs]
        for own in (spec.get("local") or []) + (spec.get("extra") or []):
            pairs_seen += 1
            for path in tracked_matching(repo_root, own):
                if not any(p.match(path) for p in wf_pats):
                    failures.append(
                        f"ci-only-coverage: {slug} is ci_only and is "
                        f"triggered by '{own}', but "
                        f"{','.join(spec['workflows'])} does not run for "
                        f"'{path}'. The gate was not moved to CI, it was "
                        f"removed — widen the workflow's `paths:` to cover "
                        f"the gate's own triggers, or keep the gate local.")
                    break
    if pairs_seen < 1:
        failures.append(
            "ci-only-coverage: no ci_only gate declared a trigger to compare "
            "— the case read nothing")

    # The question both lists above are blind to.
    #
    # `ci-only-coverage` compares a gate's list against its workflow's, and
    # two lists can agree with each other while agreeing with nothing else.
    # What decides whether a suite goes red is what its sources
    # `include_str!`: those files are compiled into the test binary, so
    # editing one changes what the suite asserts without touching a `.rs`.
    # Measured 2026-08-27: nineteen such inputs reached NEITHER list —
    # `SCE_ERROR_CONTRACT.md` and `SCE_MESH.md` among them — so a commit
    # editing only those documents ran the test that judges them in no lane
    # at all, and `contract_docs_cite_only_real_codes` exists to catch a
    # phantom slug in exactly those two files.
    #
    # The check is a UNION over the gates that declare a root, not a
    # per-gate one: what has to be true is that SOME lane running the suite
    # starts for the path. Splitting the cheap assertions into a lane with a
    # wide filter is a valid way to satisfy it, and a per-gate check would
    # call that arrangement broken.
    cases += 1
    include_re = re.compile(r'include_str!\(\s*"([^"]+)"\s*\)')
    triggers_by_slug = gate_triggers(repo_root)
    roots: dict[str, list[str]] = {}
    for slug in sorted(GATES):
        for root_glob in GATES[slug].get("include_str_roots") or []:
            roots.setdefault(root_glob, []).append(slug)
    targets_seen = 0
    for root_glob, owners in sorted(roots.items()):
        # Each owner's reach: its triggers AND its workflow's filter, since
        # a gate held out of the push only runs when the workflow starts.
        reaches = []
        for slug in owners:
            wf_globs: list[str] = []
            for wf in GATES[slug].get("workflows") or []:
                wf_globs.extend(workflow_paths(repo_root, wf))
            trig = [glob_to_regex(g) for g in triggers_by_slug.get(slug, [])]
            if "**" in wf_globs:
                reaches.append((slug, trig, None))
            else:
                reaches.append((slug, trig, [glob_to_regex(g) for g in wf_globs]))
        for src in tracked_matching(repo_root, f"{root_glob}/**"):
            if not src.endswith(".rs"):
                continue
            try:
                text = (repo_root / src).read_text(encoding="utf-8")
            except OSError:
                continue
            for rel in include_re.findall(text):
                resolved = (repo_root / src).parent.joinpath(rel).resolve()
                try:
                    path = str(resolved.relative_to(repo_root))
                except ValueError:
                    continue
                targets_seen += 1
                if any(
                    any(p.match(path) for p in trig)
                    and (wf is None or any(p.match(path) for p in wf))
                    for _, trig, wf in reaches
                ):
                    continue
                failures.append(
                    f"include-str-coverage: {src} compiles in '{path}', but "
                    f"no gate running the '{root_glob}' suite "
                    f"({', '.join(owners)}) starts for that path. Editing it "
                    f"changes what the suite asserts and no lane runs — widen "
                    f"a covering workflow's `paths:`, or give the assertion a "
                    f"lane whose filter reaches it.")
    if not roots:
        failures.append(
            "include-str-coverage: no gate declared `include_str_roots` — "
            "the case read nothing")
    if targets_seen < 1:
        failures.append(
            "include-str-coverage: no `include_str!` target was found under "
            "the declared roots — the case read nothing")

    # Rule 2, held rather than described. A gate with no CI counterpart
    # states why in a field, not in a comment a reader has to find and a
    # test cannot read. Checked both ways: a gate that maps a workflow
    # must not also claim to have none, or the claim is stale the moment
    # the counterpart lands.
    cases += 1
    for slug, spec in sorted(GATES.items()):
        has_workflow = bool(spec.get("workflows"))
        reason = spec.get("no_ci_reason")
        if has_workflow and reason:
            failures.append(
                f"no-ci: {slug} maps {spec['workflows']} and also carries "
                f"no_ci_reason — drop the reason, it now has a counterpart")
        if not has_workflow and not reason:
            failures.append(
                f"no-ci: {slug} has no CI counterpart and no no_ci_reason. "
                f"A gate that runs only at push time is skipped by "
                f"`--no-verify`; say why that is acceptable here, or give "
                f"it a workflow")

    # The same rule pointed the other way: a workflow with no gate.
    #
    # Read from the directory rather than from a list, so a workflow added
    # tomorrow is judged by this case on the commit that adds it. The
    # asymmetry this closes is not hypothetical — see UNMIRRORED_WORKFLOWS.
    cases += 1
    wf_dir = repo_root / ".github" / "workflows"
    mirrored = {w for spec in GATES.values() for w in spec.get("workflows", [])}
    workflows_seen = 0
    for wf in sorted(wf_dir.glob("*.yml")) + sorted(wf_dir.glob("*.yaml")):
        workflows_seen += 1
        name = wf.name
        if name in mirrored:
            if name in UNMIRRORED_WORKFLOWS:
                failures.append(
                    f"unmirrored: {name} is now mirrored by a gate and still "
                    f"carries an UNMIRRORED_WORKFLOWS reason — drop the reason")
            continue
        if name not in UNMIRRORED_WORKFLOWS:
            failures.append(
                f"unmirrored: {name} has no gate and no reason. CI runs a "
                f"check nothing local reproduces, so it can only be seen "
                f"after a push; add a gate that mirrors it, or record in "
                f"UNMIRRORED_WORKFLOWS where its verdict is reached instead")
    # Lower bound: a glob that stopped matching would leave this case
    # reading nothing and still passing — the failure mode it exists for.
    if workflows_seen < 10:
        failures.append(
            f"unmirrored: only {workflows_seen} workflow file(s) enumerated — "
            f"the sweep is broken, not the coverage")

    # A narrowing gate must be narrowing something, and must be left with a
    # trigger. Both halves have a failure mode worth naming: a `narrows` on a
    # gate whose workflow is filtered is dead config that reads as a decision,
    # and a `narrows` that removes the only trigger a gate had turns it off
    # while every other case here still passes.
    cases += 1
    narrowing = 0
    for slug, spec in sorted(GATES.items()):
        if not spec.get("narrows"):
            continue
        narrowing += 1
        unfiltered = [w for w in spec.get("workflows", [])
                      if workflow_paths(repo_root, w) == ["**"]]
        if not unfiltered:
            failures.append(
                f"narrows: {slug} declares a narrowing and none of "
                f"{spec.get('workflows')} is unfiltered — there is nothing "
                f"to narrow, so the field is dead config")
        # A `ci_only` gate is not selected by this table at all, so "left
        # with a trigger" cannot be what keeps it running — the unfiltered
        # workflow above is, and that is checked rather than assumed. The
        # requirement is replaced, not waived: for a hook-run gate the
        # trigger is what makes it run, and for this one the lane is, so each
        # is held to the thing that actually runs it.
        if not (spec.get("local") or spec.get("extra") or spec.get("ci_only")):
            failures.append(
                f"narrows: {slug} narrows its workflow's catch-all away and "
                f"declares no local/extra trigger — the gate can never be "
                f"selected")
        if spec.get("ci_only") and not unfiltered:
            failures.append(
                f"narrows: {slug} is ci_only and narrows away the catch-all "
                f"of a filtered workflow — nothing selects it locally and "
                f"nothing starts it in CI either")
        if not str(spec["narrows"]).strip():
            failures.append(
                f"narrows: {slug} declares an empty narrowing reason — the "
                f"field is where the reader learns what CI runs that the "
                f"hook does not")

    # `both_filter_readers_agree`. One workflow file, two readers: this one
    # and `declares_path_filter` in
    # `sce-build/tests/workflow_trigger_coverage.rs`, which counts
    # `paths-ignore` because it is "the same filter with the sense flipped".
    # Until 2026-08-29 no workflow declared one, so the disagreement was
    # latent; `cpp-suite.yml` then acquired a deny-list and this file went on
    # calling the lane unfiltered. Two sources that agree only because
    # neither has been asked the hard case cannot be a check — the reader
    # here has to see the same three shapes the Rust one does.
    cases += 1
    for wf in sorted(p.name for p in (repo_root / ".github" / "workflows").glob("*.yml")):
        allow = workflow_paths(repo_root, wf)
        ignore = workflow_ignored_paths(repo_root, wf)
        text = (repo_root / ".github" / "workflows" / wf).read_text(encoding="utf-8")
        on = re.search(r"^on:\n(.*?)(?=^\S)", text, re.S | re.M)
        spells_ignore = bool(on) and any(
            line.strip() == "paths-ignore:" for line in on.group(1).splitlines())
        if spells_ignore and not ignore:
            failures.append(
                f"filters: {wf} spells `paths-ignore:` under `on:` and this "
                f"file reads no globs out of it — the deny-list is invisible "
                f"to every caller, which is how a skipped lane reads as a "
                f"passing one")
        if ignore and allow != ["**"]:
            failures.append(
                f"filters: {wf} declares both `paths:` and `paths-ignore:` — "
                f"GitHub Actions refuses that pairing, so one of the two is "
                f"not doing what its author believes")

    # `a_deny_list_cannot_remove_coverage_in_silence`. A `ci_only` gate is
    # excluded from the push on the promise that CI runs it; a `paths-ignore`
    # on its workflow is the one way that promise can lapse without anything
    # being deleted. The rule is not "no deny-list" — a README genuinely
    # cannot break a C++ ctest suite — it is that the lapse has a NAME, so
    # `ci_unowed` must report exactly the gates `ci_owed` stops claiming.
    # Held as a partition rather than as two lists, because two lists drift
    # and a partition cannot: every reached ci_only gate is in exactly one.
    cases += 1
    for probe in (["docs/SCE_LUA_TRANSLATION_SEAM.md"],
                  ["README.md"],
                  ["sce/src/scripting/LuaEngine.cpp"],
                  ["examples/cmake_function/README.md"]):
        reached, _ = matched(repo_root, probe)
        ci_gates = {s for s in reached if GATES[s].get("ci_only")}
        owed = set(ci_owed(repo_root, probe))
        unowed = set(ci_unowed(repo_root, probe))
        if owed | unowed != ci_gates:
            failures.append(
                f"ci-owed: for {probe} the owed/unowed split lost "
                f"{sorted(ci_gates - (owed | unowed))} — a gate the change "
                f"reaches that neither list accounts for is one nobody is "
                f"told about")
        if owed & unowed:
            failures.append(
                f"ci-owed: for {probe} {sorted(owed & unowed)} is reported "
                f"as both owed and unowed")
        for slug in owed:
            if not any(workflow_starts_for(repo_root, wf, probe)
                       for wf in GATES[slug].get("workflows", [])):
                failures.append(
                    f"ci-owed: {slug} is promised to CI for {probe} but no "
                    f"workflow carrying it starts for that change — the hook "
                    f"would name a verdict that never arrives")

    # A recorded reason is checked, not believed. `commit-hook:<command>`
    # claims the commit hook reaches the same verdict, so the command has to
    # be in it; a reason naming a workflow that no longer exists is stale.
    cases += 1
    hook_src = ""
    hook_path = repo_root / "tools" / "git-hooks" / "pre-commit"
    if hook_path.is_file():
        hook_src = hook_path.read_text(encoding="utf-8")
    for name, reason in sorted(UNMIRRORED_WORKFLOWS.items()):
        if not (wf_dir / name).is_file():
            failures.append(
                f"unmirrored: {name} is recorded here but no longer exists — "
                f"drop the entry")
            continue
        kind, _, arg = reason.partition(":")
        if kind != "commit-hook":
            continue
        if not arg.strip():
            failures.append(f"unmirrored: {name} declares commit-hook with no command")
        elif not hook_runs(hook_src, arg.strip()):
            failures.append(
                f"unmirrored: {name} claims the commit hook runs "
                f"`{arg.strip()}` and no executed line of the hook does — the "
                f"claim is the only thing standing in for a gate here")

    # A gate whose workflow delegates to the runner must actually call it.
    #
    # Mapping a workflow proves only that a file exists with the right
    # `paths:`. A workflow that stopped running its gate would keep the
    # trigger derivation working, keep every existing parity check green,
    # and verify nothing — the shape a push-time-only gate takes when it
    # is given a CI counterpart in name.
    cases += 1
    delegating = 0
    for slug, spec in sorted(GATES.items()):
        if not spec.get("runner_workflow"):
            continue
        delegating += 1
        for name in spec.get("workflows", []):
            wf = repo_root / ".github" / "workflows" / name
            if not wf.is_file():
                continue  # `every_workflow_the_registry_maps_exists` owns this
            if f"scripts/gate {slug}" not in wf.read_text(encoding="utf-8"):
                failures.append(
                    f"delegation: {name} is declared as running {slug} "
                    f"through the runner but never calls "
                    f"`scripts/gate {slug}` — either restore the call or "
                    f"drop `runner_workflow` and mirror the commands")
    if delegating < 1:
        failures.append(
            "delegation: no gate declares runner_workflow — the flag that "
            "tracks CI-calls-the-runner progress reads nothing")

    # ...and must not keep a second copy of what it delegates.
    #
    # Calling the runner while the old block sits beside it is the worst
    # of both: the commands still live in two files, and the flag now
    # says they do not. The check is on command lines rather than on
    # prose, so a workflow may still explain what the gate does.
    cases += 1
    verbs = ("cargo ", "ctest", "go test", "./gradlew", "python3 ", "bash scripts/")
    for slug, spec in sorted(GATES.items()):
        if not spec.get("runner_workflow"):
            continue
        script = gate_script(repo_root, slug)
        if not script.is_file():
            continue
        # Continuation backslashes and run-step indentation are
        # spelling, not command: a gate that wraps its arguments and a
        # workflow that does not were the same command, and comparing
        # the raw lines matched neither. Normalising is what lets the
        # duplicate be seen.
        def normalise(line: str) -> str:
            line = line.strip()
            if line.startswith("run:"):
                line = line[4:].strip()
            return " ".join(line.rstrip("\\").split())

        gate_cmds = {
            normalise(line)
            for line in script.read_text(encoding="utf-8").splitlines()
            if normalise(line).startswith(verbs)
        }
        for name in spec.get("workflows", []):
            wf = repo_root / ".github" / "workflows" / name
            if not wf.is_file():
                continue
            body = wf.read_text(encoding="utf-8")
            wf_cmds = {
                normalise(line)
                for line in body.splitlines()
                if normalise(line).startswith(verbs)
            }
            for dup in sorted(gate_cmds & wf_cmds):
                failures.append(
                    f"delegation: {name} calls `scripts/gate {slug}` and "
                    f"still restates its command `{dup}` — one of the two "
                    f"spellings is the one that will be edited")

    # A gate whose workflow does not delegate says why, next to itself.
    #
    # The count of delegating gates is a progress measure, and a
    # progress measure with no reason attached to the remainder reads
    # as "not done yet" forever. Each non-delegating entry carries a
    # `# Not delegated:` note stating the measured difference — the
    # lane adds a reporting flag, uploads an artifact, installs a tool.
    # Reading the registry's own source for the note is what keeps the
    # note from being optional; the same shape the `local`-only gates'
    # reasons take.
    cases += 1
    registry_src = registry_source()
    for slug, spec in sorted(GATES.items()):
        if not spec.get("workflows") or spec.get("runner_workflow"):
            continue
        entry = next(
            (i for i, line in enumerate(registry_src) if line.strip().startswith(f'"{slug}": {{')),
            None,
        )
        if entry is None:
            failures.append(f"delegation: {slug} is not declared in this file's source")
            continue
        window = "\n".join(registry_src[entry : entry + 12])
        if "Not delegated:" not in window:
            failures.append(
                f"delegation: {slug} still restates its gate's commands in CI and "
                f"says nothing about why — add a `# Not delegated:` note with the "
                f"measured difference, or convert the workflow to `scripts/gate {slug}`")

    # Every gate in the table has a script, and every script is in the table.
    cases += 1
    for slug in GATES:
        if not gate_script(repo_root, slug).is_file():
            failures.append(f"registry: {slug} has no scripts/gates/{slug}.sh")
    script_dir = repo_root / "scripts" / "gates"
    if script_dir.is_dir():
        for path in sorted(script_dir.glob("*.sh")):
            if path.name == "lib.sh":
                continue
            if path.stem not in GATES:
                failures.append(f"registry: scripts/gates/{path.name} is not registered")

    # Order is derived, not declared: dependencies first, then cheapest.
    cases += 1
    order = run_order(GATES.keys())
    pos = {s: i for i, s in enumerate(order)}
    for slug, spec in GATES.items():
        for dep in spec.get("deps", []):
            if pos[dep] > pos[slug]:
                failures.append(f"order: {slug} runs before its dependency {dep}")

    # Nothing expensive runs ahead of something cheap unless the cheap one
    # was waiting on it. Note what this does and does not catch: because the
    # order is computed from `cost_s`, editing a cost moves the gate and the
    # property still holds — so this cannot detect a stale measurement. What
    # it does detect is an ordering that stops consulting cost at all.
    cases += 1
    for i, earlier in enumerate(order):
        for later in order[i + 1:]:
            ce = GATES[earlier].get("cost_s")
            cl = GATES[later].get("cost_s")
            if ce is None or cl is None:
                continue
            if ce <= cl:
                continue
            # "Unless the cheap one was waiting on it" — waiting on ANY gate,
            # not only on this one. A gate whose dependency had not run yet
            # could not have been picked no matter what `earlier` cost, so
            # reporting it says nothing about whether cost is being consulted.
            #
            # The narrower reading (excuse only a direct dependency) held
            # while every dependency was cheap enough to be picked first, and
            # stopped holding the moment `codegen-build` was measured
            # honestly: at 19s it sorts behind eight cheaper gates, and the
            # nine gates that wait on it were all reported as having been
            # overtaken. Measured 2026-08-22 — nine false failures, no
            # ordering defect.
            blocked_by = {d for d in transitive_deps(later) if pos[d] >= i}
            if blocked_by:
                continue
            failures.append(
                f"order: {earlier} ({ce}s) runs before the cheaper "
                f"{later} ({cl}s), which was not waiting on anything")

    # The distinction the check above is blind to, on a fixture that pins it.
    # Taking one dependency LEVEL at a time satisfies every property stated
    # so far and still puts a 1s gate behind a 999s one, because the cheap
    # gate happens to have a dependency. Only taking the cheapest currently
    # available gate gets this right, and only a synthetic table can tell the
    # two apart — the real one would have to be reshaped to expose it.
    cases += 1
    fixture = {
        "prereq": {"cost_s": 1},
        "cheap-dependent": {"deps": ["prereq"], "cost_s": 1},
        "expensive-independent": {"cost_s": 999},
    }
    got = run_order(fixture.keys(), table=fixture)
    want = ["prereq", "cheap-dependent", "expensive-independent"]
    if got != want:
        failures.append(f"order: level-at-a-time regression — got {got}, want {want}")

    # Drift report — the answer to what the order case above is blind to. Each
    # case runs against a synthetic table, because the point is the rule and
    # not this week's numbers.
    drift_table = {
        "prereq": {"cost_s": 1},
        "cheap": {"cost_s": 2},
        "dear": {"cost_s": 100},
        "dependent": {"deps": ["prereq"], "cost_s": 3},
    }
    cases += 1
    if order_drift({s: drift_table[s]["cost_s"] for s in drift_table},
                   table=drift_table) is not None:
        failures.append("drift: a run that matches the declarations reported drift")

    cases += 1
    swap = order_drift({"prereq": 1, "cheap": 200, "dear": 100, "dependent": 3},
                       table=drift_table)
    if swap is None:
        failures.append("drift: a gate measured 100x its declared cost went unreported")
    elif swap["measured"].index("dear") > swap["measured"].index("cheap"):
        failures.append(f"drift: measured order did not reseat the pair: {swap}")

    cases += 1
    # Within the noise floor: `SECONDS` granularity must not nag.
    if order_drift({"prereq": 2, "cheap": 3, "dear": 101, "dependent": 4},
                   table=drift_table) is not None:
        failures.append("drift: reported a difference inside the measurement noise floor")

    cases += 1
    # A dependency still binds: measuring the dependent as free cannot lift it
    # above the gate whose output it consumes.
    dep = order_drift({"prereq": 50, "cheap": 2, "dear": 100, "dependent": 0},
                      table=drift_table)
    if dep is not None and dep["measured"].index("prereq") > dep["measured"].index("dependent"):
        failures.append(f"drift: measured order broke a dependency: {dep}")

    cases += 1
    # A gate with no declared cost is reported the first time it is timed —
    # `run_order` sorts it last, which is a guess the measurement can correct.
    if order_drift({"prereq": 1, "cheap": 2, "unmeasured": 0},
                   table=dict(drift_table, unmeasured={})) is None:
        failures.append("drift: an unmeasured gate's first timing went unreported")

    cases += 1
    # A date for a slug that no longer exists is worse than no date: it makes
    # the unmeasured count look smaller than it is, which is the one number
    # the ceiling below is trying to hold.
    stale_keys = sorted(set(COST_MEASURED) - set(GATES))
    if stale_keys:
        failures.append(
            f"COST_MEASURED names slugs the registry does not have: {stale_keys}")

    cases += 1
    # The value has to be a date, not a promise. "soon", "see comment" or an
    # empty string would satisfy "the key is present" while telling the next
    # reader nothing, and a field that accepts anything is not a field.
    for slug, when in sorted(COST_MEASURED.items()):
        if not (len(when) == 10 and when[4] == "-" and when[7] == "-"
                and when[:4].isdigit() and when[5:7].isdigit()
                and when[8:].isdigit()):
            failures.append(
                f"COST_MEASURED[{slug!r}] = {when!r} is not a YYYY-MM-DD date")

    cases += 1
    # The ratchet, and it fails in BOTH directions on purpose. More unmeasured
    # slugs than declared means a gate arrived without its measurement date.
    # Fewer means somebody measured one and left the ceiling where it was —
    # the failure mode that turns a ratchet into a decoration, and the reason
    # this is an equality rather than `<=`.
    unmeasured = sorted(set(GATES) - set(COST_MEASURED))
    if len(unmeasured) != UNMEASURED_COST_CEILING:
        failures.append(
            f"UNMEASURED_COST_CEILING says {UNMEASURED_COST_CEILING} but "
            f"{len(unmeasured)} slug(s) carry no measurement date "
            f"({'more arrived' if len(unmeasured) > UNMEASURED_COST_CEILING else 'one was measured — lower the ceiling'}): "
            f"{unmeasured}")

    cases += 1
    unknown_paced = sorted(set(PACE_NORMALISED) - set(GATES))
    if unknown_paced:
        failures.append(
            f"PACE_NORMALISED names slugs the registry does not have: {unknown_paced}")

    cases += 1
    # The value is printed to whoever is reading a drift report, so an
    # empty one turns the note into a shrug.
    for slug, why in sorted(PACE_NORMALISED.items()):
        if len(why.strip()) < 10:
            failures.append(
                f"PACE_NORMALISED[{slug!r}] = {why!r} does not say why the "
                f"declared cost differs from a raw timing")

    cases += 1
    # The half that keeps this from rotting: the PROSE and the DATA have to
    # name the same slugs. Six comments in this file say a cost was
    # pace-normalised, and until 2026-09-02 that was the only place it was
    # written — so the drift report told readers to "update cost_s" on
    # every one of them. A new normalisation that lands as a comment and
    # not as an entry here puts us straight back there, and nothing would
    # notice. Rule 10 of this repository's round rules, applied to the
    # registry's own file.
    prose_says, current = set(), None
    for line in registry_source():
        # Leaving the GATES literal ends the current slug's block. Without
        # this the scanner attributes every later mention to whichever slug
        # happened to be last — and it caught itself doing exactly that on
        # 2026-09-02: the four mentions in this function and in the drift
        # printer landed on `w3c-python-bindings`, and the case failed with
        # a slug that has no such comment. Fourth time this repository has
        # been bitten by a scanner reading its own text.
        if line and not line[0].isspace():
            current = None
        match = re.match(r'    "([a-z0-9-]+)": \{', line)
        if match:
            current = match.group(1)
        if current and "pace-normalis" in line:
            prose_says.add(current)
    if prose_says != set(PACE_NORMALISED):
        only_prose = sorted(prose_says - set(PACE_NORMALISED))
        only_data = sorted(set(PACE_NORMALISED) - prose_says)
        failures.append(
            f"PACE_NORMALISED disagrees with the comments in this file: "
            f"prose-only={only_prose} data-only={only_data} — a normalised "
            f"cost the report cannot read is the defect this map exists for")

    cases += 1
    # The EFFECT, not just the data. A map nothing reads is a comment with
    # brackets, so this drives the report itself and checks all three of
    # the things that make it useful: the slug is still named (hiding it
    # would bury a real regression), the reason is printed, and the "update
    # cost_s" advice — correct everywhere else, wrong here — is gone.
    # Driven from a FIXTURE, not from the live map. Reaching into
    # PACE_NORMALISED for a slug to demonstrate on would tie this witness
    # to the defect's own survival: the map empties as its six gates are
    # honestly re-measured, and on the day the last one leaves, a case
    # built that way starts passing by having nothing to check. It would
    # also have died on a bogus entry with a KeyError, which prints no
    # failure list at all and so would swallow every other diagnostic in
    # the run — including the one naming the bogus entry.
    fixture = {"paced-gate": "a reason long enough to be a reason"}
    lines = drift_report_lines(
        {
            "declared": ["paced-gate", "other-gate"],
            "measured": ["other-gate", "paced-gate"],
            "moved": [("paced-gate", 4, 0)],
        },
        paced=fixture,
    )
    report = "\n".join(lines)
    # The slug's OWN row, not merely the string somewhere in the report:
    # the run-order lines at the foot spell every slug out, so a "is it
    # mentioned" check stays true even with the row suppressed, and the row
    # is the part a reader needs in order to see a normalised cost go stale.
    if not any("paced-gate" in line for line in lines if " declared " in line):
        failures.append(
            "drift report no longer gives a pace-normalised slug a row of "
            "its own — such a cost goes stale like any other, and dropping "
            "the row hides that behind the note")
    if "pace-normalised on purpose" not in report:
        failures.append(
            "drift report does not say why the declared cost differs")
    if fixture["paced-gate"] not in report:
        failures.append(
            "drift report prints the note without the reason behind it")
    if "update cost_s" in report:
        failures.append(
            "drift report still tells the reader to update a pace-normalised "
            "cost — following that undoes the normalisation")

    cases += 1
    # The other reader of the map is the published table, and it is the one
    # that matters to anybody outside this file: `--mapping` exists so a
    # consumer asks the registry instead of scraping it, and a consumer
    # asking "is this cost_s a stopwatch reading?" got no answer at all
    # until 2026-09-02. Three things have to hold, and each has been a real
    # defect elsewhere in this file: the key reaches every gate (so its
    # absence means an old registry, not "raw"), it carries the reason
    # rather than a flag, and it names the same six the map does.
    published = mapping_table()
    if set(published) != set(GATES):
        failures.append(
            f"--mapping no longer publishes every gate: "
            f"missing={sorted(set(GATES) - set(published))}")
    silent = sorted(s for s in published if "pace_normalised" not in published[s])
    if silent:
        failures.append(
            f"--mapping omits pace_normalised for {silent} — a consumer "
            f"cannot tell 'this cost is raw' from 'the registry is too old "
            f"to say', which is the absence-as-answer this key replaced")
    said = {s: v for s, v in published.items() if v.get("pace_normalised")}
    if set(said) != set(PACE_NORMALISED):
        failures.append(
            f"--mapping and PACE_NORMALISED name different gates: "
            f"published={sorted(said)} map={sorted(PACE_NORMALISED)}")
    for slug, spec in sorted(said.items()):
        why = spec["pace_normalised"]
        if not isinstance(why, str) or len(why.strip()) < 10:
            failures.append(
                f"--mapping publishes pace_normalised={why!r} for {slug} — a "
                f"flag sends the reader back to the prose this key exists to "
                f"replace")

    cases += 1
    # And the ordinary path still gives the ordinary advice, or the case
    # above would pass by having deleted the instruction for everyone.
    ordinary = "\n".join(drift_report_lines(
        {
            "declared": ["plain-gate", "other-gate"],
            "measured": ["other-gate", "plain-gate"],
            "moved": [("plain-gate", 4, 999)],
        },
        paced=fixture,
    ))
    if "update cost_s" not in ordinary:
        failures.append(
            "a slug that is NOT pace-normalised lost the re-measure advice")

    cases += 1
    # The binding the fixtures cannot see. Every case above drives an
    # injected map, so the live one could be disconnected from the report
    # entirely and they would all still pass. Both arms below are
    # assertions rather than a skip: the second is what a correct registry
    # looks like once the last paced cost has been honestly re-measured,
    # which is the state this work is trying to reach.
    live = next(iter(PACE_NORMALISED), None)
    if live is not None:
        default_report = "\n".join(drift_report_lines({
            "declared": [live, "other-gate"],
            "measured": ["other-gate", live],
            "moved": [(live, GATES.get(live, {}).get("cost_s"), 0)],
        }))
        if PACE_NORMALISED[live] not in default_report:
            failures.append(
                f"drift_report_lines no longer reads PACE_NORMALISED by "
                f"default — {live!r} loses its reason in the real report")
    else:
        default_report = "\n".join(drift_report_lines({
            "declared": ["plain-gate", "other-gate"],
            "measured": ["other-gate", "plain-gate"],
            "moved": [("plain-gate", 4, 999)],
        }))
        if "update cost_s" not in default_report:
            failures.append(
                "PACE_NORMALISED is empty, so every gate should take the "
                "ordinary re-measure branch, and one does not")

    if failures:
        for f in failures:
            print(f"SELF-TEST FAIL: {f}", file=sys.stderr)
        return 1
    print(f"gate_registry self-test: {cases} cases OK", file=sys.stderr)
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo-root", default=".", type=Path)
    ap.add_argument("--changed-from", type=Path,
                    help="file with one changed path per line ('-' for stdin)")
    ap.add_argument("--changed", nargs="*", default=None)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--list", action="store_true",
                    help="print every gate in run order")
    ap.add_argument("--explain", action="store_true",
                    help="also print the selection reason to stderr")
    ap.add_argument("--ci-owed", action="store_true",
                    help="print the gates this change reaches that only CI "
                         "runs, one 'slug workflow' pair per line, instead "
                         "of the gates the push runs")
    ap.add_argument("--ci-unowed", action="store_true",
                    help="print the gates this change reaches that NEITHER "
                         "the push nor CI will run, one 'slug workflow' pair "
                         "per line — the residue of --ci-owed, and the only "
                         "place a path filter that removes coverage is said "
                         "out loud")
    ap.add_argument("--mapping", action="store_true",
                    help="print the table as JSON, so a consumer reads the "
                         "registry instead of parsing this file's source")
    ap.add_argument("--order-drift", nargs="*", metavar="SLUG=SECONDS",
                    help="compare a run's timings against the declared costs "
                         "and report when they would have ordered the gates "
                         "differently")
    args = ap.parse_args()

    repo_root = args.repo_root.resolve()

    if args.self_test:
        return self_test(repo_root)

    if args.mapping:
        # A reader that scrapes this file's source picks up the self-test's
        # synthetic tables along with the real one — measured, three fixture
        # gates and four more. Emitting the table is the answer.
        json.dump(mapping_table(), sys.stdout, indent=1, sort_keys=True,
                  default=str)
        sys.stdout.write("\n")
        return 0

    if args.order_drift is not None:
        measured = {}
        for pair in args.order_drift:
            slug, _, secs = pair.partition("=")
            if slug in GATES:
                measured[slug] = float(secs)
        # The verdict half, printed before the report half because it is the
        # one that stops the run. A breach is a claim about the TABLE, so it
        # is worth failing for; the ordering report below is a claim about
        # this machine, so it is not.
        breach = budget_breach(measured)
        if breach is not None:
            print(f"\ngate: this run puts the push budget at {breach['total']}s, "
                  f"over the {PUSH_BUDGET_S}s ceiling.", file=sys.stderr)
            print(f"  This machine ran at {breach['pace']}x the pace cost_s was "
                  f"measured at, and that has been divided out — what is left "
                  f"is the table being wrong, not the machine being slow.",
                  file=sys.stderr)
            for slug, declared, actual in breach["grown"]:
                print(f"    {slug:<26} declared {declared:>5}s  is really "
                      f"{actual:>5}s", file=sys.stderr)
            print("  Re-measure with `scripts/gate --measure --all`, write the "
                  "numbers into cost_s, and move the most expensive gate to "
                  "`ci_only` until the rest fit.\n", file=sys.stderr)

        drift = order_drift(measured)
        if drift is None:
            return 1 if breach is not None else 0
        for line in drift_report_lines(drift):
            print(line, file=sys.stderr)
        return 1 if breach is not None else 0

    if args.list:
        for slug in run_order(GATES.keys()):
            cost = GATES[slug].get("cost_s")
            cost_s = f"{cost:>7.1f}s" if cost is not None else "      ?"
            print(f"{cost_s}  {slug:<26} {GATES[slug].get('summary', '')}")
        return 0

    if args.changed is not None:
        changed = list(args.changed)
    elif args.changed_from:
        text = sys.stdin.read() if str(args.changed_from) == "-" \
            else args.changed_from.read_text(encoding="utf-8")
        changed = [ln.strip() for ln in text.splitlines() if ln.strip()]
    else:
        ap.error("one of --changed / --changed-from / --self-test / --list is required")
        return 2

    if args.ci_owed:
        for slug in ci_owed(repo_root, changed):
            workflows = ",".join(GATES[slug].get("workflows") or ["?"])
            print(f"{slug} {workflows}")
        return 0

    if args.ci_unowed:
        for slug in ci_unowed(repo_root, changed):
            workflows = ",".join(GATES[slug].get("workflows") or ["?"])
            print(f"{slug} {workflows}")
        return 0

    keys, reason = select(repo_root, changed)
    if args.explain:
        print(f"  selection: {reason}", file=sys.stderr)
    for k in keys:
        print(k)
    return 0


if __name__ == "__main__":
    sys.exit(main())
