#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
import ast
import importlib.util
import json
import pathlib
import posixpath
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
# `builds` names what the gate builds that other gates build alike (the main
# CMake tree, the generator); what each of those reads is derived once, in
# `BUILT_INPUTS`, and joins the trigger of every gate that names it (see
# "What a build reads" below GATES).
#
# `cost_s` is wall-clock seconds, MEASURED by `scripts/gate --measure --all`
# on a warm tree, not estimated. Warm is the right basis: a push happens on
# a tree the developer has just built, and a cold first build is paid once
# in whatever order the gates run. Two limits are worth stating rather than
# discovering later. The numbers come from a full run, so a gate that
# benefits from an earlier gate having warmed the cargo cache reads cheaper
# here than it would in isolation — which is the number that matters for
# ordering a full run, and the wrong one for judging a single gate.
#
# The clock used to be whole seconds, which put a floor under this table: a
# gate finishing in a fifth of a second read 0, so its declaration could
# never be checked against anything and stayed a chosen number. `scripts/gate`
# keeps a millisecond clock since 2026-09-02 and `cost_s` carries fractions
# where a gate earns them; `a sub-second cost keeps the clock that read it`
# in the self-test refuses to let that instrument go away again.
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
# takes under five minutes. It bounds a push of any ref but main; an update
# to main is the landing queue's judgment on the build machines and runs the
# `ci_only` gates too (see "What `ci_only` means" above GATES).
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

# ── How much of that ceiling is declared in figures no run can check ──
#
# `PACE_NORMALISED` below records the costs that are a share of a full push
# rather than a stopwatch read of the gate alone. That distinction has a
# consequence the map itself does not state: such a figure cannot be
# checked. `budget_breach` compares a declaration against a measurement,
# and those two are the same kind of number only for a gate that was timed
# alone — so every paced entry is a slice of the ceiling that no run,
# however carefully it measures, can say is still true.
#
# `drift_report_lines` calls an empty map this work's success condition,
# "because each entry leaves when its gate is honestly re-measured on a
# quiet machine". Nothing measured that sentence and the map grew to six
# entries carrying 149s of the 177s a push can select, so the sentence
# became this number.
#
# The road works, and 2026-09-02 walked it to the end. Five entries left in
# one day — `nostd-mcu` 16->9, `forge-go` 36->15, `w3c-go` 52->10,
# `http-endpoint-ssot` 4->0.185, `codegen-build` 41->0.111 — and the share
# went 0.842, 0.782, 0.651, 0.421, 0.397, 0.0. Every declared cost a push
# can select is now a figure somebody read off a clock.
#
# ⚠ Zero does not retire this constant, it arms it. The check is an
# EQUALITY, so the first pace-normalised cost to reach the local set fails
# here — which is the state this was written to reach, not a reason to
# delete it.
#
# ⚠⚠ What zero does NOT cover, stated rather than hidden: `w3c-kotlin` is
# still in the map, and it is `ci_only`, so no push selects it and this
# fraction cannot see it. That is the right scope for a PUSH budget, and it
# does mean the map itself is not empty — the last entry is watched by
# nothing here.
#
# ⚠ Two things that attempt got wrong, worth keeping because both are easy
# to repeat. It first read a COLD figure (a push measured 32s that hour) as
# evidence against the warm one and kept the stale 16 — but the header says
# warm is the basis, and a cold reading says nothing about a warm figure.
# And "on a quiet machine" is not a condition this box can meet: five AI
# loops live here and the load moved 9 -> 95 while those runs were taken.
# What worked was repeating the measurement until it settled and reading
# the band, then taking the worse of the last pair. A single reading here
# says nothing; the first three of nine were still filling caches.
# 0.287 -> 0.237 on 2026-09-17. Nothing about the paced figures changed; the
# DENOMINATOR grew, because `forge-c11` added 17 measured seconds to what a push
# can select. The share is therefore smaller for a good reason, and the rule
# here is to follow it down rather than bank the slack: a ceiling left above
# the real share stops being a ratchet and silently permits the next
# unmeasured cost to be added back.
# 0.237 -> 0.133 on 2026-11-20, for the same kind of reason: `clippy` came back
# into the push with 82 measured seconds, so the denominator grew to 188.485s
# while the 25 paced seconds of `codegen-build` stayed where they were.
PACED_BUDGET_SHARE_CEILING = 0.133

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
    # Measured on arrival at 0.262 on 2026-09-19, and not again while the
    # suite grew to 413 cases. Retaken 2026-09-24 as the suite run directly
    # (`python3 -m unittest discover`), because `scripts/gate --measure` is
    # now refused locally for this repository: 155s with an opt-0 generator,
    # 72s with the opt-1 one `Cargo.toml` now builds, under load 23. The
    # figure is a stopwatch read of the gate's own work, not paced.
    "authoring-core": "2026-09-24",
    # The suite run directly (`python3 -m unittest discover`) with the pinned generator, warm, 34
    # cases, 81s. The first runner time will replace it.
    "targets": "2026-10-14",
    # The gate script run once end to end through `bx --local` on 2026-10-08,
    # with Node and Chrome both replaying: 107s with the dependencies built,
    # under load 5. The run before Chrome was added read 128s on a first
    # dev-profile build, under load 20. Not taken by `scripts/gate --measure`,
    # which is refused locally for this repository.
    "wasm32-scenarios": "2026-10-08",
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
    # `codegen-build` left `PACE_NORMALISED` on 2026-09-02 and took the last
    # local entry with it: no gate a push can select now declares a
    # pace-normalised cost, so `PACED_BUDGET_SHARE_CEILING` is 0.0.
    #
    # It was the entry three rounds called un-retirable, and the reason was
    # a misreading each time — first that a stopwatch could not replace a
    # paced figure, then that the warm 0 was the gate's nature rather than
    # the clock's floor. Both were answered by reading this file's own
    # header: warm is the basis, cold is paid once and out of scope.
    "codegen-build": "2026-09-02",
    "embed-manifest-failfast": "2026-09-02",
    # 0.04s on 2026-11-04 (`check --rev HEAD` over 196 inputs): two git
    # listings and a comparison, no build and no clang.
    "embed-manifest-stamp": "2026-11-04",
    # `forge-go` left `PACE_NORMALISED` on 2026-09-02, the second entry to go.
    # Seven readings: five `--measure` runs at 27 17 16 15 10 and two real
    # pushes at 10 and 12. Drop the first — caches were still filling — and
    # the band is 10-17s against a declared 36, which `cost_is_stale` calls
    # stale by a wide margin.
    #
    # ⚠ The three runs meant to confirm the band never happened, and the
    # reason is worth more than they were: `~/.cargo/.package-cache` is a
    # GLOBAL lock, and a sibling session's `cargo build --release
    # --workspace` in another repository held it for the whole attempt. This
    # gate looks like a Go gate but `conformance/generate.sh` rebuilds
    # `sce-codegen` when cargo is on PATH, so it queues behind that lock like
    # the other 14 of the 36 gate scripts that invoke cargo. A run that spent
    # its time blocked measures the lock, not the gate — such a reading is
    # discarded, not averaged in. The discriminator is `ps` for the holder,
    # never load average.
    "forge-go": "2026-09-02",
    "http-endpoint-ssot": "2026-09-02",
    "license-ssot": "2026-09-02",
    # 0.01s, three direct runs reading 0.01 0.01 0.01. It stats two
    # directories and greps one document; there is nothing in it that grows
    # with the tree except the number of vendored libraries, of which there
    # are two.
    "web-vendor-licenses": "2026-09-17",
    # 5.5s, three runs reading 5507 5593 5581 ms. Almost all of it is loading
    # the engine WASM once and laying out 13 documents; it grows with the
    # fixture count, which the gate derives rather than fixes.
    "visualizer-layout": "2026-09-17",
    # Re-timed 2026-09-15, when row S3 began asking the AOT registrar by
    # compiling a probe against it: `scripts/gate --measure nl-ir-closure`
    # read 1.198s, and five direct runs of the script read 1188 1223 1244
    # 1186 1309 ms, against 158-176 ms on the day it was written. The
    # difference is those two syntax-only compiles; the rest is still grep
    # over source (binaries and build trees pruned) plus one short Python
    # pass over the AOT headers. Load average was 1.3 with other sessions'
    # loops running, and this tree's costs are not trustworthy to within a
    # factor of two under that load.
    #
    # Re-timed again the same day, when row G4 began asking `sce-codegen
    # requirement-closure` the way S3 asks the registrar: 1.332 1.311 1.330s
    # with the binary current, so the three process starts cost about 0.13s
    # and the declared figure moved 1.2 -> 1.3. The run that found the binary
    # stale read 12.946s, 11.53s of it the rebuild — recorded here so the next
    # reader of a 13s reading knows which of the two they took.
    "nl-ir-closure": "2026-09-15",
    # `nostd-mcu` was re-timed on 2026-09-02 and LEFT `PACE_NORMALISED`: the
    # first entry to do so. Nine consecutive runs read
    #
    #     22  22  13  7  7  6  9  9  7
    #
    # — the first three still filling caches, then a settled 6-9s band. The
    # declared 16 was genuinely stale (`cost_is_stale(16, 9)` is true), so
    # `cost_s` is now 9, the worse of the last pair.
    #
    # ⚠ The first attempt at this round kept the 16 and argued that `cost_s`
    # has to predict a COLD push, because the same gate read 32s cold in a
    # push that hour. That reasoning contradicts the header twelve lines
    # above — "Warm is the right basis: a push happens on a tree the
    # developer has just built, and a cold first build is paid once" — and
    # the paragraph right here, which says filling an entry in means
    # "putting the number in `cost_s`". A cold reading is not evidence about
    # a warm figure. Recorded because the argument was persuasive and wrong.
    "nostd-mcu": "2026-09-02",
    "rustdoc-links": "2026-09-02",
    # `w3c-go` left `PACE_NORMALISED` on 2026-09-02, the third and cleanest.
    # Six runs read 9 10 9 10 10 10 with no cache-filling ramp and no lock
    # contention, against a declared 52 — stale by five times. The lock check
    # (`ps` for a cargo holder before starting, and a grep for "Blocking
    # waiting for file lock" in each run's log afterwards) is what turned a
    # measurement that had to be abandoned twice into one that settled on the
    # first try.
    "w3c-go": "2026-09-02",
    # 17s on a quiet machine (load 1.33, nothing else building in this tree).
    # Measured on the lane's first existence rather than inherited from a
    # sibling: the C11 arm generates 12 headers and compiles one C file, so
    # borrowing forge-cpp's 59s would have overstated it more than threefold
    # and eaten push budget that is not being spent.
    "forge-c11": "2026-09-17",
    # 52s, timed on the lane's first existence rather than inherited from a
    # sibling. Two readings were taken, 52 and 1, and the 1 is DISCARDED for
    # the reason `forge-go` above discards a lock-bound run: Gradle found every
    # task up to date and did no work, so that reading measures the cache and
    # not the gate. 52 is what it costs when it has something to do, which is
    # the only time the change set selects it.
    "forge-kotlin": "2026-09-18",
    # 38s, the worse of two runs of the gate script through bx (38 and 19).
    "swift-backend": "2026-10-28",
    # 76s, the lane script under the gate through bx (see the gate's own entry).
    "w3c-swift": "2026-11-01",
    # 11s, the lane script under the gate through bx (see the gate's own entry).
    "ecma262-lowered-swift": "2026-11-01",
    # 102s, the lane script under the gate through bx (see the gate's own entry).
    "forge-swift": "2026-11-01",
    # 72s, `scripts/gate genmc-queue` on pc2 with two harnesses and ten mutants
    # each, the first run that read GenMC's reports rather than its exit codes.
    "genmc-queue": "2026-10-10",
    # 367s, the gate through bx with four cores, the first run with the models of
    # ADR 0116 and ADR 0117 (208s on hkl002 with three cores before them).
    "formal-models": "2026-11-23",
    # 22s, `scripts/formal/proofs.py` on the workstation, eleven mutants, Verus fetched.
    "formal-proofs": "2026-11-23",
    # 451s, the gate's script through bx on hkl002, cold (its entry says why
    # that and not a warm reading).
    "rust-version": "2026-10-20",
    # 1363s against a declared 151 — the widest gap this table has recorded,
    # and the first entry for a `ci_only` gate, which is why it went unseen:
    # the drift report prices what a push selects, and nothing selects this.
    # `scripts/gate --measure workspace-tests` on a warm tree, load 4, with
    # `SCE_GATE_NO_FAIL_FAST` unset — the switch changes what is REPORTED,
    # not what is run, so it does not change the figure. Two nearby readings
    # corroborate the band rather than the number: 1231s for the same command
    # timed from outside, and 1184s for a fail-fast run that stopped at 180 of
    # 504 binaries. Load cannot account for nine times.
    "workspace-tests": "2026-09-20",
}

# Exactly how many slugs are absent from COST_MEASURED. Not an upper bound
# with slack — an equality, so measuring one gate forces this down in the
# same commit and the count cannot drift away from the map.
#
# 25 -> 26 on 2026-10-20, the one direction this is not meant to move, and
# named here so it is not mistaken for slack: `w3c-rust` arrived as the gate
# form of a lane that had been running inline, with no timing of the script.
# Timing it on a warm tree and dating it in COST_MEASURED brings this back.
#
# 26 -> 27 on 2026-10-27, the same direction for the same kind of reason:
# `cpp-interface-standard` arrived with an estimated cost and no timing of its
# script. Its entry says how to take one.
#
# 27 -> 28 on 2026-10-27, once more in that direction: `install-consumer`
# arrived as the lane for a test no lane ran, with an estimated cost. Its entry
# says how to time it.
#
# 28 -> 29 on 2026-11-04, the same direction again: `w3c-wasm` arrived with a
# cost summed from readings of its parts, not a timing of its script. Its entry
# says which parts.
#
# 29 -> 30 on 2026-11-05, again: `cpp-suite-wasm` arrived as the lane for a
# tree nothing built whole, so its cost is an estimate its entry derives. Its
# first run read the build half only; the first green run reads the whole.
#
# 30 -> 31 on 2026-11-18, again: `windows-msvc` arrived as the first lane on a
# Windows host, which no push host is, so its cost is an estimate its entry
# derives. Its first run reads it.
#
# 31 -> 32, the same direction: `generated-deterministic` arrived declared and
# skipped, since the fleet cache it compares against does not exist yet. Its
# skip costs a shell start; its run is timed on the first machine that names a
# fleet.
#
# 32 -> 33 on 2026-11-20, the same direction: `c11-mcu` arrived written where
# no ARM compiler could run, so its cost is an estimate its entry derives. Its
# first run reads it.
UNMEASURED_COST_CEILING = 33

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
    "codegen-build": "25s is the Cargo.lock rebuild its own script records "
                     "at 24.6s; the 7.2-8.0s one-crate case and the 0.111s "
                     "no-op are both timed, that worst case is not",
    "w3c-kotlin": "152s, what it cost on 2026-08-24 under a full run",
}

# ── Whether a gate's verdict may be kept ──────────────────────────
#
# A push hands its gates to bx as a set of check units (`[checks.prepush]` in
# bx.toml): one unit per gate the change selects, planned from this table by
# `scripts/gate --plan` (`plan_units` below). bx keeps a unit's PASS in the
# site's action cache under a key made of the unit's argv, the content of its
# inputs and the platform bx.toml declares (bx RFC 0020, section 4), and does
# not run a unit again whose key already has a kept pass.
#
# That is correct only for a gate whose verdict is a function of that key, so
# every entry says which it is, and nothing defaults (`every-gate-says-whether-
# its-verdict-is-kept` in the self-test):
#
#   `"cache": True`   the verdict is a function of the tracked tree, of the
#                     platform bx.toml declares (its tools at their pinned
#                     versions) and of the Rust release the plan names. A fetch
#                     of content the tree pins (a digest, a revision, a version
#                     written in a tracked file) keeps that property.
#   `"cache": False`  it is not, and `no_cache_reason` says what else it reads:
#                     the clock, git history or refs, an artifact another gate
#                     leaves in an ignored directory, a tool whose absence makes
#                     it skip and pass, a host compiler or browser no
#                     declaration names, or thread interleavings it samples.
#
# A gate whose verdict outside a landing can rest on the machine says how in
# `kept_only_from_a_landing`: it skips and exits 0 where a tool or a
# prerequisite is missing, or it takes a tool the machine carries where the
# pinned one is absent. A landing closes every such path: its units run with
# `--landing`, the runner exports SCE_GATE_LANDING, and `sce_gate_requires_tool`
# (and the other skip sites of scripts/gates/lib.sh, and the gate's own check
# for its pinned tool) then end with 3, could not run, which bx reports as not
# measured and the integrator never lands. A skip is not a pass. Such a gate
# may claim `cache: True`, and its pass is kept only from a landing unit: a
# unit without `--landing` could still have skipped (`plan_units`).
#
# `needs` and `packages` name what one gate's unit needs of a machine beyond
# bx.toml's platform, as bx reads them on a plan line (bx RFC 0004, section 6):
# commands, and system packages. A machine without them is not given the unit
# and takes the others; the platform's own lists would refuse it for every job.
# What a gate may name in each is said beside `peak_gb` below (`packages`) and
# in "Gates that run in an image" (`needs`, of which only docker is one today).
#
# The residue, stated rather than hidden: a tool bx.toml requires by name only
# (`needs`, `packages`) is whatever version the machine carries, and a verdict
# that moved with that version would be kept across it. bx runs one kept pass in
# sixteen again and blocks a key whose verdict changed (RFC 0020, section 9),
# which is the instrument that finds such a gate.
#
# `inputs` is the other half of the key. `None` is the whole tracked tree; a list
# of patterns (bx RFC 0003, section 6) narrows the key to those files, and bx
# then runs the unit in a copy of them alone, under strace, and calls a read
# outside them "could not measure" (RFC 0020, section 5.2). The copy is a
# repository of one commit whose tree is the inputs (bx RFC 0026, implemented
# in bx 80cdd0de), so the runner and the gate library find their root in it as
# they do in a checkout. ADR 0120 (D2) decides that every gate's key is to be
# narrowed this way, a gate at a time.
#
# A gate's `inputs` names what its OWN script reads. `unit_inputs` adds what
# every unit reads around it: `RUNNER_INPUTS`, the gate's script and the
# workflows it mirrors (the runner reads them for the submodule question).
# Beyond the files the gate opens, the entry has to name:
#   * a directory the gate LISTS (`find`, a glob, cargo's discovery of
#     `tests/`, `examples/` and `src/bin`). bx keys a listing only when the
#     inputs give it (bx RFC 0020 section 4.2, key format 3, bx 340e9f9e): a
#     pattern that ends in `/` gives that directory's names (`third_party/`,
#     `./` for the root), and `dir/**` gives every listing under `dir`. A
#     listing the inputs do not give is "could not measure", and `check learn`
#     proposes the pattern for it;
#   * the paths a gate finds through git (`git ls-files`, `git grep`). The
#     copy's index holds the inputs alone, and the trace does not see git read
#     its objects, so the run in the copy searches a smaller tree and passes
#     where the checkout would fail. Such a gate is narrowed only to patterns
#     that cover every path its pathspec can name; one that searches the whole
#     tree (`http-endpoint-ssot`) keeps `None`;
#   * a commit. The copy's one commit is not HEAD, so a gate that reads HEAD
#     or the change set keeps `inputs: None` until it is split (ADR 0120 D2).
#     `a-narrowed-gate-reads-no-commit` in the self-test holds that.
#
# What whole-tree keys buy, measured against what was asked of them: one tree
# is judged once (a push retried over the same commits, a full run after a
# scoped one, two sessions verifying the same tree). A rebase over commits that
# touch nothing a gate reads still moves the tree, so it is a new key for every
# unit. A narrowed key survives every rebase that leaves its files alone.
#
# `commits` is what a verdict reads of git beyond the tree: the commits whose
# ids, ancestry or objects it asks about. The key holds the tree's content and
# no commit, so a verdict that reads one is a function of the key only when the
# commit's id is in the unit's argv. The plan carries two names. `"HEAD"`: it
# resolves HEAD once and puts `--at <id>` in the argv of every unit that runs
# such a gate, and the runner refuses (exit 3, "could not run") a checkout that
# is at any other commit. HEAD's id fixes its ancestry and the submodule commits
# its tree records, so one id covers what these gates read. `"CHANGES"`: the
# commits the push publishes, for a gate that does its work per changed path
# (it reads `SCE_GATE_CHANGED_FILE`). The plan puts each `--changed-in
# <base>..<tip>` it was handed in the argv of every unit that runs such a gate,
# and the runner hands the gate the set those ids name: two ids mean the same on
# every machine the history reached, where the hook's path list is a file on
# this one. A unit cannot find the set itself, since the checkout it runs in is
# a build machine's and its refs say nothing about the push. Measured
# 2026-11-20: the first landing whose plan held `mutation-rounds` ran it as a
# bare unit, the gate took its base from that checkout's upstream (a51553493c)
# and selected 578 of 579 casefiles where the push's own range selects 14; it
# ran 2.4 hours on one core and held the queue behind it. A gate that reads a
# ref the plan cannot carry (an upstream branch, which moves) is `cache: False`
# and says so. `[]` is a verdict that reads no commit: `git ls-files` and
# `git grep` read the tracked tree, which is the key. The generator names
# itself by the digest of the sources it is built from
# (`GENERATOR_SOURCE_DIGEST`), a function of the tree, and carries no commit,
# so building it is not reading a commit. `the_binary_carries_no_commit` reads
# HEAD only to show the binary does not hold its id, which is the same verdict
# at every commit of one tree: no file of a tree can hold the id of a commit
# that contains it.
# `a-verdict-that-reads-head-carries-it` derives the HEAD readers a script
# shows and refuses one that does not declare it, and
# `a-verdict-that-reads-the-change-set-carries-it` does the same for a script
# that reads `SCE_GATE_CHANGED_FILE`.
#
# `peak_gb` is the memory one run of the gate's OWN script takes at its peak,
# in GiB; a unit asks bx for the largest figure among the gates it runs
# (`mem_gb`, bx RFC 0004, section 6). bx starts units on a machine only while
# their figures fit what it has free, and gives a unit no hard limit by it. The
# figures in `PEAK_MEASURED` were read off a run; every other one is the figure
# of its class, and is a scheduling estimate until a run is read: 5 for a gate
# that compiles (cargo, C and C++, Gradle, Go, Swift), 1 for a script that runs
# built tools over the tree.
#
# `packages` is the system packages a gate's own script runs a program of, past
# what the platform of bx.toml asks of every machine; a unit names those of
# every gate it runs (`packages`, bx RFC 0004, section 6), and bx places it only
# on a machine that has them. It is for a program whose release the verdict
# does not move with: one whose release it does move with is a tool of bx.toml
# with pinned bytes (bx RFC 0009, section 4.8), as the ARM compiler is. Absent
# is none, unlike `cache`, which nothing defaults, because the
# two errors differ: a gate that needs a package it does not name is placed on
# a machine without it and ends "could not run" (exit 3), which names the
# machine and claims nothing about the tree, where an unsaid `cache` would keep
# a pass it never earned.
# `every-unit-names-the-packages-its-gates-need` in the self-test holds the
# shape, and holds each name to the gate's script or one it runs, so a
# declaration cannot outlive the program it was for.
PEAK_MEASURED: dict[str, str] = {
    "codegen-build": "4.98 GB at the peak of an sce-codegen build (bx, "
                     "2026-11-04)",
    "regen-reproduces": "5 to 6.3 GB over a full regeneration of the "
                        "committed trees (bx, 2026-11-04)",
    "w3c-wasm": "5.3 GB at the peak of a cold emscripten configure and build "
                "of both programs with three build slots (bx --local, "
                "2026-11-04), before the gate script existed",
    "workspace-tests": "10.5 to 12.8 GiB committed at the peak of four passing "
                       "runs (bx peaks.tsv, 2026-11-10), and killed at its own "
                       "16 GiB limit on hkl001 (2026-11-13); declared 5, it was "
                       "killed at 5.2 and 7.8 GB in a landing (2026-11-20)",
}

# `parallelism` is how many processes one run of the gate's OWN script can keep
# busy; a unit asks bx for the largest figure among the gates it runs
# (`parallelism` on a plan line, bx RFC 0004 section 6, named `cores` before bx
# RFC 0081 4.8 renamed it). It is the upper end of the job slots a unit is
# given in its batch: its own jobserver and `BX_GRANT_CORES` (bx RFC 0062,
# 4.5), which `scripts/lib/sce_build_jobs.sh` hands to the builds. Admission
# counts no cores (bx RFC 0081): a busy machine shares its cores by weight, and
# the figure caps what the unit's builds start, not what it may take from
# another run. bx does not learn a unit's figure (bx RFC 0062, 7), so the figure
# here is the whole of what the unit is given. A unit that named none
# was given one slot. MEASURED 2026-11-21: in a landing whose units named
# none, a unit's run was given one core and used 1.3 on average, one unit was
# still running after 7547s, and the landing's judge reached its four-hour
# deadline three times without a verdict.
#
# Every figure is the one of its class, as `peak_gb`'s are until a run is
# read: `PARALLELISM_COMPILE` for a gate that compiles, `PARALLELISM_SCRIPT` for a script
# that runs built tools over the tree. Eight is the core count of the smallest
# build machines the fleet has: a compiling unit is given all of one of those
# and a share of a larger one. A tree build parallelises past eight, but a
# floor above a machine's cores is capped there, and on a busy machine the
# floor is a weight the kernel shares, not cores taken from another run.
PARALLELISM_COMPILE = 8
PARALLELISM_SCRIPT = 1

# A gate whose entry says `"inputs": INPUTS_ARE_TRIGGERS` is keyed on its
# trigger globs (`gate_triggers`): the same derivation that decides whether a
# change selects the gate decides which changes its kept pass outlives (ADR
# 0120, D2). A gate a change does not select is one the change cannot affect,
# so a key over the same globs is that claim restated, and bx tests it: a run
# in the copy that reads outside the globs is "could not measure" (bx RFC 0020
# 5.2), which is also a file the selection was missing. A gate takes the value
# once `bx --checks prepush --learn <slug>` has shown its reads inside its
# triggers; until then it keeps `None` and costs nothing it did not cost.
# A gate that names a shared build (`builds`) is keyed as well on what cargo
# reads for that build without compiling it (`gate_reads`, "What a build
# reads that does not select it"), which changes no verdict and so selects
# nothing.
INPUTS_ARE_TRIGGERS = "triggers"

# Every unit names this, so a machine runs one gate of a set at a time. The
# gates were written for a runner that ran them one after another in one
# checkout, and several share what that checkout holds: the CMake tree under
# `build/`, the endpoint port the C++ suites bind themselves, Gradle's project
# directories, and the headers `genmc-queue` rewrites in place while it runs. A
# batch's units run in one checkout, so two at once could read each other's
# half-written state, and a kept pass could be one of those readings. Spreading
# a set over machines is what runs gates side by side.
UNIT_EXCLUSIVE = "checkout"

# The shape of a Rust release a plan may name (`rustc --version`'s second word).
RUST_RELEASE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")

# The keys of an entry that a plan line carries to bx as they are: what its
# unit needs of a machine. Absent is none.
UNIT_REQUIRES_KEYS = ("needs", "packages")

# The shape of a full commit id a plan carries (SHA-1, or SHA-256).
COMMIT_ID = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$")

# The commit names a gate's `commits` may hold: the ones the plan carries.
CARRIED_COMMITS = ("HEAD", "CHANGES")

# The shape of a Debian package name (Debian Policy, section 5.6.1), which a
# gate's `packages` may hold.
PACKAGE_NAME = re.compile(r"^[a-z0-9][a-z0-9+.-]+$")

# The name of the one unit a plan carries when the change selects no gate: bx
# refuses a plan that lists nothing ("a plan that lists nothing has measured
# nothing"), and a selection that is empty is an answer, so it is said as a
# unit whose result is already known.
NOTHING_TO_VERIFY = "no-gate-applies"

# ── What `ci_only` means, and the one push that runs those gates anyway ──
#
# `ci_only` says that a push of any ref OTHER than main does not run the gate.
# It used to say that no push ran it: the gate did not fit `PUSH_BUDGET_S`, a
# ceiling on what a push from this workstation could make its author wait for,
# so it was handed to hosted CI and judged a round later. The measured figures
# beside each gate are why that trade was made, and they still price a push of
# any other ref.
#
# main moves only through the bx landing queue (bx RFC 0024), whose integrator
# judges each candidate by running the pre-push hook for the landing push's own
# line (`refs/heads/main <candidate> refs/heads/main <base>`), and the hook
# hands the units to the build machines. The budget's reason does not hold
# there: the gates run side by side on the fleet, not one after another here.
# Measured 2026-11-08: a landed item (cdba8e0eb0) was judged on 7 gates while
# 34 `ci_only` gates it reached ran only after it landed, and main went red. The
# owner decided that the queue's judgment runs every gate the change reaches,
# `ci_only` included, still path-scoped. That selection is `select(...,
# landing=True)`; the hook asks for it (`--landing`) for a push that updates
# `refs/heads/main`, which only the integrator can make. Hosted CI stays, as an
# audit after landing; `--ci-owed` and the workflow mapping still read the field.
#
# ── Where a gate can run: `platform` ─────────────────────────────────
#
# A gate runs on the platform bx.toml declares (`[platform.<name>]`) unless it
# names another in `platform`, with `platform_reason` saying what that runner
# has that the fleet does not. A landing unit bx cannot place on any machine
# ends "not measured" (exit 2), and the integrator sends such an item back
# unlanded; a gate only a hosted runner can run would therefore keep every item
# that reaches it out of main for ever. So the landing selection leaves out a
# gate whose `platform` bx.toml does not declare, names it, and hosted CI keeps
# auditing it after landing (`drop_hosted_only`). One attribute, read against
# bx.toml: when the fleet gains that platform, the gate joins the judgment
# without an edit here. A toolchain the fleet merely lacks is not a platform:
# it is a tool of bx.toml's platform, with an install, so bx provisions it and
# places the unit where it is (Swift, GenMC, the Python environment, emsdk, the
# mesh SDKs, Chrome for Testing, wasm-bindgen), and a compiler a gate's verdict
# belongs to is an image the gate runs in (`container`). One gate names a
# platform the fleet lacks: `windows-msvc`, whose verdict is MSVC's on a hosted
# Windows runner, and which every build machine, being the Linux platform
# bx.toml declares, cannot give.
# `a-gate-off-the-fleet-is-ci-only-and-says-why` in the self-test holds that
# such a gate is `ci_only` (no push of another ref could run it either), names
# a workflow, and gives its reason.
BX_PLATFORM_HEADER = re.compile(r"^\[platform\.([A-Za-z0-9_-]+)\]\s*$")

# ── A gate whose cost the change sets: `audited_after_landing` ─────────
#
# The queue judges one candidate at a time for everybody, so what a landing may
# run is bounded: the cost of a gate is the tree's, and the fleet's machines
# and bx's kept passes share it out. The cost of a gate that does one build per
# mutation case its change reaches is the change's, and nothing bounds it.
# Measured 2026-11-21 on item 346: its change set reaches 15 casefiles and 152
# cases, and in its second attempt the one unit that ran them held one machine
# 10367s of the judgment's four hours (named by elimination: the account times
# the units it does not name). The unit keeps no verdict, so every attempt
# begins it again. Over the sixteen commits of main before
# 2026-11-22 the change set reaches 188 casefiles and 1207 cases. No shard
# count makes that a landing's: at a minute a case it is a machine-day, and a
# ctest case is tens of minutes. The textbook place for it is the one it had,
# an audit of what landed, where hosted CI already shards it and defers what
# is past its own ceiling (`SCE_PUSH_ROUND_JOBS`).
#
# So a gate that declares `audited_after_landing` is left out of the landing,
# with the reason as its value, and named in the judgment beside the gates off
# the fleet (`drop_hosted_only`). It is the same kind of answer, "hosted CI
# audits it after landing", for another reason. It is for a gate whose cost has
# no bound in the tree, never for one that is merely slow: a slow gate is made
# a unit that fits, since what the landing does not judge lands unjudged.
# `a-gate-audited-after-landing-is-ci-only-and-says-why` in the self-test holds
# that such a gate is `ci_only`, names the workflow that audits it, and gives
# its reason.

# ── Gates that run in an image: `container` ───────────────────────────
#
# A gate whose verdict is one compiler's names, in `container`, the tracked
# Containerfile of the image it runs in. The runner then never runs it on the
# machine it is on: it builds that image (tagged by the file's digest) and runs
# `scripts/gate <slug>` inside it, with the checkout mounted at its own path,
# so the gate and everything it depends on (`deps`, the generator build among
# them) run there and nowhere else. Hosted CI, a build machine and a
# workstation therefore judge it with the same compiler, and the file is the
# one place that compiler is named. `container_placement` says which gates of a
# run go into an image and which run where the runner is; the dependencies only
# a gate in an image needs go in with it.
#
# A build machine runs containers only under its run account's rootless Docker
# (the registry's `containers = "rootless"`), so such a gate names
# `needs = ["docker"]` and bx gives its unit only to a
# machine that has one. `a-gate-in-an-image-says-so-everywhere` in the
# self-test holds the three together: the file exists, an edit of it selects
# the gate, and the unit asks for docker.
#
# The image carries no Rust. No gate installs a toolchain (the runner's "The
# Rust release the gates run on"), and an image that installed one would build
# the gate's generator with a compiler the unit's key does not name. The runner
# mounts the toolchain it selected, read-only, where the runner inside looks
# for a declared release, and names that release with `--rust`.
# `a-gate-image-builds-on-the-selected-rust` holds both halves: no Containerfile
# line installs Rust (`IMAGE_INSTALLS_RUST`), and `gate_in_container` mounts
# the selected toolchain and passes its release in.
CONTAINER_NEED = "docker"
IMAGE_INSTALLS_RUST = re.compile(
    r"\brustup\b|sh\.rustup\.rs|static\.rust-lang\.org|\bcargo\s+install\b"
    r"|\brustc\b|\bcargo\b")
CONTAINER_RUNNER_MARKS = ('--rust "$release"', '.local/rust/$release:ro')

GATES: dict[str, dict] = {
    # Prerequisite, not a gate: several gates execute target/debug/sce-codegen.
    # Triggered by its own sources, and forced on by its dependents.
    "codegen-build": {
        # What a cargo build of the generator reads, derived (see "What a
        # build reads"): the workspace manifests, `sce-build` and its path
        # dependencies, the generator witness and its compiled-in files.
        "builds": ["sce-codegen"],
        "no_ci_reason": "not a check — it produces the binary other gates "
                        "run. CI builds it per workflow that needs it "
                        "(`debug_only_codegen_builds_drop_the_stale_release_binary` "
                        "counts those steps), so there is no verdict here "
                        "for a workflow to mirror.",
        # Stays local however expensive it gets: `drop_ci_only` removes a
        # gate without rescuing its dependents, so a `ci_only` dependency
        # would leave the gates that execute its binary with nothing to run.
        #
        # A CONDITIONAL cost, and the reason `cost_s_no_op` exists.
        #
        # Reached through `deps` on a tree whose `sce-build/**` has not
        # moved, cargo finds nothing to do: six runs on 2026-09-02 read
        # 0.099 0.100 0.087 [0.153] 0.107 0.111, the bracketed one queued
        # behind the cargo lock and discarded. That is `cost_s_no_op`.
        #
        # Selected by its OWN triggers, it rebuilds, and those triggers are
        # exactly `sce-build/**` and the manifests below. Three runs after
        # `touch sce-build/src/lib.rs` read 7.254 7.960 7.136 — one crate
        # recompiling. A `Cargo.lock` move rebuilds the dependency tree
        # instead, and this gate's own script records that at 24.6s "in a
        # tree those gates had already warmed". The budget bounds the WORST
        # case, so `cost_s` is that one.
        #
        # ⚠ 25 is pace-normalised — quoted from that script, not timed here.
        # `cargo clean` would be needed to reproduce it and this machine's
        # target/ is shared with sibling sessions. That is why this slug is
        # back in PACE_NORMALISED after a round that wrongly emptied it:
        # declaring the 0.111 alone left PUSH_BUDGET_S blind to tens of
        # seconds on precisely the pushes that select this gate.
        "cost_s": 25,
        "cost_s_no_op": 0.111,
        "summary": "build target/debug/sce-codegen",
        "cache": True,
        # Keyed on its triggers, which are what the build compiles, and on
        # what cargo reads beside them to load the workspace and fingerprint
        # the generator's build script (`gate_reads`, "What a build reads
        # that does not select it"). Read off `bx --checks prepush --learn`
        # of `forge-c11` and `forge-cpp` (2026-11-24), whose units run it.
        "inputs": INPUTS_ARE_TRIGGERS,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The NL→IR closure ledger, measured against the tree it describes.
    #
    # `docs/SCE_NL_IR_CLOSURE.md` is a denominator — the closed list of what
    # remains on SCE's side of the specification→IR path, written before and
    # independently of whatever closes its rows. This gate is what keeps it
    # from becoming prose: it measures every row and fails when the recorded
    # status disagrees with the tree IN EITHER DIRECTION. A row recorded
    # closed that the tree contradicts is a false green; a row recorded open
    # that the tree has closed is a stale denominator.
    #
    # ⚠ It does NOT fail for unfinished work. The `closure:` line on stdout
    # carries that answer, and the exit status carries whether the record is
    # honest. Collapsing the two would make the gate unrunnable in the push
    # path for as long as any row is open, which is the whole of its useful
    # life.
    #
    # Catch-all trigger, for the endpoint-SSOT lane's reason: a row can close
    # in a template, a header, a CMake list, a Rust module or the visualizer,
    # and a `paths:` list narrowed to what anyone remembers misses the one
    # nobody thought of. Affordable — grep, one short Python pass, the two
    # syntax-only compiles row S3 asks the AOT registrar with, and three runs
    # of `sce-codegen requirement-closure` over manifests row G4 writes. The
    # runs behind that number are in the measurement-date map.
    #
    # ⚠ WARM, and for row G4 warm now means more than a filled page cache: the
    # binary must be current. The shared locator rebuilds one built from other
    # sources — deliberately, because asking a stale binary answers about a
    # tree that no longer exists — so a push whose own change touched
    # `sce-build/src/` pays that build here if nothing else has paid it yet.
    # Measured 2026-09-15: 1.33 1.31 1.33s with the binary current, and 12.9s
    # on the run that rebuilt it. The warm figure is the one declared, on the
    # basis this table's header already gives: a push happens on a tree the
    # developer has just built, and a cold build is paid once.
    "nl-ir-closure": {
        "workflows": ["nl-ir-closure.yml"],
        "runner_workflow": True,
        # Rows G2-G4 run `sce-codegen` through `sce_gate_codegen`. Without
        # this edge the gate sorted ahead of the build on its 1.3s and paid
        # for it: 288s on 2026-09-24, the push after `sce-build`'s dev
        # profile changed. The self-test now derives the edge from the script.
        "deps": ["codegen-build"],
        "cost_s": 1.3,
        "summary": "the NL→IR closure ledger vs the tree it describes",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
        "parallelism": PARALLELISM_SCRIPT,
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
        # Kept only from a landing unit, where the skip below cannot pass, and
        # a landing's unit goes only to a machine with clang-19 installed.
        "cache": True,
        "kept_only_from_a_landing": "`scripts/test_emit_manifest_fail_fast.sh` "
                                "skips and exits 0 on a machine without "
                                "clang++-19",
        "packages": ["clang-19"],
        # Read by `check learn` (2026-11-24, bx 340e9f9e): the fail-fast case,
        # the emitter it drives and the two Python modules the emitter runs.
        # The run lists `scripts/`. The case writes its header tree under a
        # temporary directory, so nothing of `sce/include` is read.
        "inputs": [
            "scripts/",
            "scripts/_embed_manifest_filter.py",
            "scripts/embed_manifest.py",
            "scripts/emit_embed_manifest.sh",
            "scripts/test_emit_manifest_fail_fast.sh",
        ],
        "commits": [],
        "peak_gb": 1,
        "parallelism": PARALLELISM_SCRIPT,
    },
    # Whether embed/MANIFEST.json was emitted from the inputs it is committed
    # with, against its stamp embed/MANIFEST.inputs (scripts/embed_manifest.py).
    # `embed-vendor` checks the content and is CI-only at 311s, so until this
    # gate a header edit reached the push and the CI run before anything said
    # the manifest had not moved with it; five of eight manifest commits in
    # October and November 2026 were repairs made after the fact. This one is
    # a list of git object ids against another, under a second, so it runs at
    # the push. The pre-commit hook runs the same comparison over the index;
    # this is its backstop for a commit made with the hook skipped.
    "embed-manifest-stamp": {
        "workflows": ["embed-vendor-smoke.yml"],
        "runner_workflow": True,
        "cost_s": 0,
        "summary": "embed manifest emitted from the inputs it is committed with",
        # Reads HEAD's objects (`check --rev HEAD`, `git show`) and the
        # commit each submodule is at, so the unit carries HEAD's id
        # (`commits`). The plan also keeps no verdict for a tree that differs
        # from HEAD (`plan_units`).
        "cache": True,
        "inputs": None,
        "commits": ["HEAD"],
        "peak_gb": 1,
        "parallelism": PARALLELISM_SCRIPT,
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
                   "so CI is OFFERED exactly the pushes this would have "
                   "run — and whether it JUDGES them is a property of that "
                   "workflow, not of this delegation. It failed to on "
                   "2026-09-02: ten consecutive pushes cancelled, the "
                   "repair of the lane's own red among them. The repair is "
                   "cancel-in-progress: false there, held by "
                   "`ci_supersession_policy`, which measures this lane "
                   "against a "
                   "17.6 min median gap between pushes to main — not by "
                   "spacing the pushes. That test now holds every prose "
                   "copy of the gap to its constant, this one included.",
        "cost_s": 193,
        "summary": "tree-wide marker + trigger + parity gates",
        # Reads HEAD's history: `lowering_decision_ledger` asks git whether
        # each pinned commit is an object, an ancestor of HEAD and holds the
        # paths it names, and `hook_ci_parity` adds a worktree of HEAD. One
        # tree can sit on histories that answer differently, so the unit
        # carries HEAD's id (`commits`). Its other targets read the tracked
        # tree or fixture repositories of their own, and the tools they skip
        # without (the pinned mnemosyne-cli, cmake) are ones bx.toml requires.
        "cache": True,
        "inputs": None,
        "commits": ["HEAD"],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
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
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
        "parallelism": PARALLELISM_SCRIPT,
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
        # Its ctest casefiles run in the main tree, which it configures
        # (scripts/prepare_ctest_tree.sh) where none is.
        "builds": ["main-tree"],
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
        "extra": ["scripts/mutate", "scripts/lib/mutation_rounds_cost.sh", "Cargo.lock",
                  "scripts/prepare_ctest_tree.sh",
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
    },
    "clippy": {
        "workflows": ["clippy-check.yml"],
        "runner_workflow": True,
        # Lint is the fastest feedback a Rust change can get. It was measured
        # at 15s, 42s and 82s in one afternoon on this machine, which is also
        # the clearest single illustration of why the numbers here cannot be
        # trusted to a factor of two.
        #
        # ⚠ Local, not `ci_only`. It was moved out of the push when the push
        # was something a developer waited for. Since the landing queue
        # judges every item with this hook on the build fleet (bx RFC 0024),
        # a gate left out of the push is a gate left out of the merge queue:
        # on 2026-11-20 item 290 landed two `explicit_auto_deref` errors and
        # main stayed red until item 298. With `cache: True` a kept pass
        # costs nothing, and 82s fits the ceiling beside the local set.
        "cost_s": 82,
        "summary": "cargo clippy --workspace --all-targets",
        # The lint set moves with each stable release, which is why the plan
        # names the release in every unit's argv: a new one is a new key.
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # Shares clippy's workflow because it shares clippy's inputs: the
    # declarations clippy measures its lints against are the ones this keeps
    # true. Not the warm figure this table asks for: 451s is the first run on
    # a build machine that had none of the six toolchains, so it installed
    # every one and built every package cold, the state a CI runner is always
    # in. The six builds alone read 21, 16, 4.5, 60, 8 and 14 seconds.
    "rust-version": {
        "workflows": ["clippy-check.yml"],
        "runner_workflow": True,
        "ci_only": "451s cold, six toolchains installed. clippy-check.yml "
                   "runs it beside clippy.",
        "cost_s": 451,
        "summary": "every package builds with the Rust it declares",
        # Each toolchain it installs is the version a tracked manifest declares.
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    "nostd-mcu": {
        "workflows": ["sce-rust-runtime-no-std.yml"],
        "runner_workflow": True,
        "deps": ["codegen-build"],
        # 9s, timed alone on a warm tree on 2026-09-02 and no longer a
        # normalised figure. Nine consecutive runs read 22 22 13 7 7 6 9 9 7:
        # the first three were still filling caches, and the settled band is
        # 6-9s. 9 is the worse of the last pair, which is the rule the budget
        # header sets ("the WORSE of two `--measure` runs") so that the
        # ceiling holds on unlucky pushes rather than lucky ones.
        "cost_s": 9,
        "summary": "no_std MCU build + clippy + probes; std runtime without http-send",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
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
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    "embed-vendor": {
        "workflows": ["embed-vendor-smoke.yml"],
        # The reason this lane kept its own copy — "it also calls a sibling
        # gate, so its steps are a superset" — did not hold: the sibling
        # (`embed-manifest-failfast`) runs in a SEPARATE job of the same
        # workflow, and this job restated the three scripts verbatim.
        "runner_workflow": True,
        "extra": ["sce/include/**", "embed/MANIFEST.json",
                  "backends/cpp/forge-runtime/include/**",
                  "scripts/smoke_bazel_consumer.sh", "scripts/sce_repo.bzl",
                  "scripts/lib/bazel_pins.sh", "tests/bazel_consumer/**"],
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
        # The Bazel consumer's versions are pinned in the tree
        # (`scripts/lib/bazel_pins.sh`), and a machine without bazel ends the
        # smoke "could not measure" (exit 3) rather than passing.
        # `package_embed.sh` writes `git describe` into the package's VERSION
        # file, which no step compares, so the gate reads no commit.
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
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
        # Run whole it is the longest unit of a landing after cpp-suite-wasm
        # (1363 s), and its script already runs one shard of itself when
        # SCE_WORKSPACE_SHARD names `k/N`, as the lane's four jobs do. A plan
        # makes one unit per shard of the lane's count (`shard_count`).
        "shard_env": "SCE_WORKSPACE_SHARD",
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
        # `tests/json_text/**` and `tests/durations/**` are tables the runtime's
        # and sce-build's unit tests read at run time — every engine is held to
        # them — so an edit to a table alone has to start the suite that reads it.
        "extra": [
            # What spreads the suite over the lane's shards
            # (`SCE_WORKSPACE_SHARD`), which the gate script runs.
            "scripts/lib/workspace_shards.py",
            "docs/SCE_ACCEPTED_SUBSET.md",
            "schemas/**",
            "apis/**",
            "tests/json_text/**",
            "tests/durations/**",
            "tests/document_stem/**",
            "tests/external_event_budget/**",
            "tests/scripting/**",
            # The shared Mesh-target predicate table the build reads.
            "tests/mesh/**",
            # The server suite (docs/adr/0107, D10), a workspace member outside
            # `backends/rust/**`.
            "tests/server/**",
            # The policy documents `forge_conformance` compiles into its
            # per-language run tests.
            "tests/forge/resources/**",
            # The forge conformance catalog and the document sets beside it,
            # which `generation_can_be_planned` and
            # `configure_plans_are_kept_and_asked_together` plan and generate a
            # harness and a set from.
            "tests/forge/conformance/**",
            # The integration stems `the_emitted_transition_function_equals_the_producer`
            # generates and states the table of.
            "integration_resources/flat_lifecycle/**",
            "integration_resources/nested_lifecycle/**",
            # The conformance documents, which suites here read by path
            # (`w3c_name_system_variable`, `codegen_depfile_content`,
            # `cli_lint_parity`, `a_regeneration_removes_what_it_no_longer_writes`)
            # and parse as a corpus.
            "resources/**",
            # The two CMake routes that generate at build time and the one module
            # they ask the generator's plan through, which
            # `the_conformance_cmake_route_declares_what_the_generator_plans`
            # compiles in and holds to declaring what the generator plans.
            "cmake/SCECodegenPlan.cmake",
            "cmake/SCEStaticW3CTest.cmake",
            "cmake/SCEStaticIntegrationFixture.cmake",
            # The module the plan module includes, which
            # `a_configure_asks_each_plan_once_per_input_state` configures a
            # project through with a real `cmake`.
            "cmake/SCEClangFormat.cmake",
            # The locator the plan module resolves the generator through when
            # no includer did.
            "cmake/SCEFindCodegen.cmake",
            # The test tree's build, whose Mesh rule-3 override and every
            # command it declares from a plan the same suite compiles in and
            # holds to declaring what the generator plans.
            "tests/CMakeLists.txt",
            # The C11 conformance suite's build, which the same suite compiles
            # in and holds to declaring its fixtures after their plans were
            # asked together.
            "backends/c/tests/CMakeLists.txt",
            # The conformance registry, whose fixtures
            # `a_c11_machine_includes_what_it_states_it_requires` generates,
            # and the one reading of it both conformance routes register
            # their fixtures through, which `w3c_registry_drives_cmake`
            # reads.
            "tests/w3c/conformance/fixtures.json",
            "cmake/SCEW3CRegistry.cmake",
            # The C11 ctest fixture's BasicHTTP listener and the script that
            # hands its port to the runners, which
            # `the_c11_http_fixture_hands_its_runners_the_port_it_bound` runs.
            "tests/w3c/http_server_fixture.sh",
            "tests/w3c/standalone_http_server.js",
            "tests/w3c/http_endpoint.js",
            "tests/w3c/basic_http_test_endpoint.h",
            # The C11 runtime's build, whose vocabulary of what a machine
            # requires `a_c11_machine_includes_what_it_states_it_requires`
            # compiles in and holds to what the generator can state.
            "backends/c/runtime/CMakeLists.txt",
            # The host's log seams, which
            # `a_c11_machine_logs_through_the_log_seams` links a machine with
            # and reads the lines of.
            "backends/c/runtime/posix/log.c",
            # The runtime's text unit, the real seams' definitions, which the
            # same test links a machine with (docs/adr/0110, D2).
            "backends/c/runtime/text/real_text.c",
            # The C11 runtime's headers, whose includes
            # `a_c11_machines_core_reaches_only_freestanding_headers` walks from
            # every machine's core (docs/adr/0110, D1): an edit of one alone
            # has to start the suite that judges it.
            "backends/c/runtime/include/**",
            "backends/c/forge-runtime/include/**",
            # The C11 MCU gate's set, its budget, its board and the record that
            # names the set (docs/adr/0110, D3), which
            # `the_c11_mcu_gate_weighs_the_set_its_record_names` compiles in
            # and holds to one another.
            "scripts/gates/c11-mcu.sh",
            "backends/c/mcu/**",
            "docs/adr/0110-a-c11-machine-core-is-freestanding-and-a-gate-weighs-and-runs-it-on-an-mcu.md",
            # The standard library, whose statecharts that save are held to
            # their pinned shapes (`a_standard_statechart_that_saves_keeps_its_shape`,
            # docs/adr/0045): an edit of one alone has to start the suite.
            "stdlib/**",
            # The scenarios of the standard host features, which
            # `every_replay_scenario_keeps_to_its_vocabulary` holds to the
            # members their drivers read: an edit of one alone has to start
            # the suite.
            "tests/host_features/**",
            # The registry of metric names both lanes are held to (docs/adr/0109,
            # D16), which `sce-rust-metrics` reads at run time.
            "tests/metrics/**",
            # The narrow row's runtime header, which the C11 broadcast-log
            # tests read to hold the stamp width the generator judges against
            # to the width the header keeps and each name the generated header
            # calls to a declaration of it.
            "backends/c/forge-runtime/include/sce/forge/broadcast_log_narrow.h",
            # The C++ runtime's two broadcast-log headers, which the C++
            # broadcast-log tests read: each name the generated headers call
            # against a declaration of it, and the stamp width the generator
            # judges against the width the narrow row keeps.
            "backends/cpp/forge-runtime/include/sce/forge/broadcast_log.h",
            "backends/cpp/forge-runtime/include/sce/forge/broadcast_log_narrow.h",
            # The Kotlin replays of the two C ABIs (group key agreement, peer
            # core), which the lane builds with Gradle and the capi crates'
            # tests run under `SCE_REQUIRE_TOOLS`: an edit of one alone has to
            # start the suite that runs it.
            "backends/kotlin/mls-replay/**",
            "backends/kotlin/peer-replay/**",
            # The Kotlin binding of the peer core's C ABI, which the peer
            # core's Kotlin replay and the world's Kotlin device are built on.
            "backends/kotlin/peer/**",
            # The ledgers' configurations and stores, which
            # `every_ledger_binding_names_what_its_file_holds` reads: an edit
            # of a binding alone has to start the suite that holds it to its
            # file. The bound files themselves span the source trees, and an
            # edit of one that renames a bound symbol is read on the next run
            # of this lane, not on its own push.
            "docs/spec/*/mnemosyne.toml",
            "docs/spec/*/.atomic/workspace.atomic.json",
            "docs/sce-ledger/*/mnemosyne.toml",
            "docs/sce-ledger/*/.atomic/workspace.atomic.json",
            # The generated diff's renderer and the locator it resolves the
            # generator through, which
            # `a_change_shows_what_it_does_to_the_generated_code` copies into
            # a repository of its own and runs.
            "scripts/generated_diff.sh",
            "scripts/lib/sce_codegen.sh",
            # The release pipeline's declaration of its packages and its
            # reader, which `a_release_declares_its_packages_by_their_trees`
            # holds to the tree manifests.
            "scripts/release.py",
            "tools/codegen/release.toml",
        ],
        # The crate whose test suite this gate runs. `include-str-coverage`
        # in the self-test reads it: every file the sources under this root
        # `include_str!` is an input the suite asserts on, so some lane that
        # runs the suite has to start when it changes. Declaring the root is
        # the hand-written half; which paths it implies comes from the tree,
        # so an `include_str!` added tomorrow is covered the day it is
        # written rather than the day somebody remembers a list.
        "include_str_roots": ["sce-build"],
        # 1363s, and it was declared 151 until 2026-09-20. This is the densest
        # correctness net in the tree — every contract test under
        # `sce-build/tests/` runs here — and it leaves the push anyway,
        # because the budget is a ceiling on what a developer waits for rather
        # than a ranking of what is valuable.
        #
        # ⚠ NINE TIMES OFF, AND NOTHING HERE COULD SEE IT. `budget_breach`
        # compares a declaration against a measurement only for gates a push
        # SELECTS; this one is `ci_only`, so no push prices it and its figure
        # was watched by nothing. The header says as much about `w3c-kotlin`
        # and calls it the right scope for a PUSH budget, which it is — the
        # residue is that a ci_only cost can rot without limit, and this is
        # what that looks like when it does.
        #
        # ⚠⚠ Where the time actually is, measured the same day: three targets
        # own 790s of it — `diagnostic_fix_is_applicable` 469s over 5 tests,
        # `ecmascript_acceptance_parity` 187s over 4, `mutation_rounds_
        # selection` 134s over 13. 64% of the lane in 22 tests, so "the suite
        # is large" is not what makes it long, and 504 test binaries share the
        # other 36%.
        #
        # ⚠⚠⚠ A fail-fast run reads a THIRD of this and looks like the whole
        # lane: 180 of 504 binaries in 1184s, because the slow targets sort
        # early. `SCE_GATE_NO_FAIL_FAST=1` is what makes a local run comparable
        # to the CI lane's, and three rounds of fixing one failure at a time
        # were spent before that was read off the gate script.
        "ci_only": "1363s measured 2026-09-20, declared 151s until then. "
                   "Fully mirrored by rust-workspace-tests.yml, which runs "
                   "the same command. The trade is the sharpest of the four: "
                   "this is where every sce-build contract test lives, so a "
                   "broken contract now reaches main and is answered a round "
                   "later.",
        "cost_s": 1363,
        "summary": "cargo test --workspace --features cli,sce-rust-mesh/wss",
        # The gate builds the Gradle prerequisites the suites run (the Kotlin
        # runtime jars and the three JVM programs) before the suites, so a
        # pass no longer depends on what the machine built before. What is
        # left is a test that skips on a missing tool, and a landing runs the
        # suites under SCE_REQUIRE_TOOLS and reads such a miss as could not
        # run. It also reads HEAD's history (`lowering_decision_ledger`),
        # which `commits` carries.
        "cache": True,
        "kept_only_from_a_landing": "a suite test skips with a warning where a "
                                "tool or a built JVM program it runs is "
                                "missing (`ToolLocator::require_or_skip`, "
                                "`require_all_or_warn`)",
        "inputs": None,
        "commits": ["HEAD"],
        # The highest figure a run has reached (PEAK_MEASURED); the one run
        # that reached it was killed there, so the true peak may lie above.
        "peak_gb": 16,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The one reader of the content axis. The header's `source-hash` covers
    # the INPUT documents, so an edit to the templates or to the emit code
    # under `sce-build/src` that changes what comes out does not move it, and
    # the committed trees drift with every other gate green. This regenerates
    # them and compares, which is also what makes the procedure's own
    # documented claim ("regenerate and expect no diff") checked rather than
    # asserted.
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
        # Regenerates in a worktree of HEAD, with each submodule at the commit
        # HEAD records, so the unit carries HEAD's id (`commits`). The plan
        # also keeps no verdict for a tree that differs from HEAD.
        "cache": True,
        "inputs": None,
        "commits": ["HEAD"],
        "peak_gb": 6.5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # What replaces `regen-reproduces` once nothing generated is committed
    # (ADR 0097, section 5): every action of every declared tree run here
    # from an empty cache and held against the fleet's entry of the same key.
    # Declared and skipped until bx's fleet cache exists: the script passes
    # and says so when `SCE_FLEET_CACHE` names no instance, and the half that
    # needs no fleet (two runs on one machine) is a test of the workspace
    # suite, `a_declared_tree_reproduces_its_committed_tree`.
    "generated-deterministic": {
        "local": ["tools/codegen/**", "stdlib/**"],
        "builds": ["sce-codegen"],
        "no_ci_reason": "it compares against the fleet's action cache, which no "
                        "hosted runner reaches (a site's REAPI instance, read "
                        "through `bx-fleet site-tunnel`), and which does not "
                        "exist until bx's fleet cache lands (ADR 0097, "
                        "amendment 16). Without one the gate is skipped.",
        # Unmeasured: the skip is a shell start. A run that names a fleet
        # builds the generator twice (the comparer in a target directory of
        # its own) and runs every declared tree's actions, about what
        # `regen-reproduces` costs.
        "cost_s": 1,
        "summary": "every action agrees with the fleet's entry of its key",
        "cache": False,
        "no_cache_reason": "its verdict is about the fleet's entries as they "
                           "are when it runs, which move without any commit",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    "drift-suites": {
        "workflows": ["drift-verify.yml"],
        "runner_workflow": True,
        "ci_only": "146s. It is declared serial on purpose and that is what "
                   "it costs; drift-verify.yml runs it.",
        "cost_s": 146,
        "summary": "committed-tree drift + sourcemap, serial",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
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
        # 15s, timed on a warm tree on 2026-09-02 and no longer a normalised
        # figure. Seven observations that day — five `--measure` runs reading
        # 27 17 16 15 10 and two real pushes reading 10 and 12 — put the
        # settled band at 10-17s once the first run's cache-filling is
        # dropped. 15 is the worse of the last measured pair, the rule the
        # budget header sets. Still the cheapest forge arm and the only one
        # that runs before a push.
        "cost_s": 15,
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
    },
    "forge-rust": {
        "workflows": ["forge-conformance.yml"],
        "runner_workflow": True,
        "extra": ["backends/rust/**"],
        "deps": ["codegen-build"],
        # Warm reads as 0; the release profile it needs is a separate build
        # tree from every other gate, so a cold run pays that once.
        "ci_only": "96s, a release build of the Rust forge arm, and about "
                   "three minutes more for the queue runtime under loom, "
                   "Miri and ThreadSanitizer. Its siblings are already in CI "
                   "for the same reason and they share one workflow, "
                   "forge-conformance.yml.",
        "cost_s": 280,
        "summary": "Rust forge conformance (numerical + codec round-trips, release)",
        "cache": False,
        "no_cache_reason": "samples thread interleavings: its stress runs record "
                           "the histories the scheduler happened to produce and "
                           "ThreadSanitizer reports only the races it saw, so a "
                           "pass is one draw and keeping it would stop the drawing",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    "forge-python": {
        "workflows": ["forge-conformance.yml"],
        "runner_workflow": True,
        "extra": ["backends/python/**"],
        "deps": ["codegen-build"],
        "ci_only": "107s. forge-conformance.yml verifies every language arm "
                   "in parallel jobs, which is where this belongs — the arms "
                   "fire together anyway, so paying for them serially at push "
                   "time bought attribution and nothing else.",
        "cost_s": 107,
        "summary": "Python forge conformance (numerical)",
        "cache": False,
        "no_cache_reason": "samples thread interleavings: its queue and "
                           "broadcast-log stress runs record what the machine's "
                           "scheduler happened to run, so a pass is one draw and "
                           "keeping it would stop the drawing",
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
        "parallelism": PARALLELISM_SCRIPT,
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
        # The modules its projects declare every generation through, resolve
        # the generator through, and format generated C++ by, and the one that
        # says which warning classes its test programs turn off.
        #
        # And the JSON library its test programs include, vendored in the tree
        # (backends/cpp/forge-runtime/tests/conformance/CMakeLists.txt,
        # NLOHMANN_JSON_INCLUDE_DIR): `bx --checks prepush --learn forge-cpp`
        # (2026-11-24) read every header of it, and no trigger named it.
        "extra": ["backends/cpp/**", "sce/**", "cmake/SCECodegenPlan.cmake",
                  "cmake/SCEFindCodegen.cmake", "cmake/SCEClangFormat.cmake",
                  "cmake/SCETestWarnings.cmake",
                  "third_party/nlohmann_json/include/**"],
        "deps": ["codegen-build"],
        "ci_only": "59s, the last move needed to fit the ceiling. "
                   "forge-conformance.yml runs it beside the other arms.",
        "cost_s": 59,
        "summary": "C++ forge conformance build + test",
        "cache": True,
        # Keyed on its triggers: what `bx --checks prepush --learn forge-cpp`
        # (2026-11-24) read outside them, the runner's inputs and
        # `codegen-build`'s key was what the submodule preflight read deriving
        # every gate's triggers, which it no longer does, and the schema the
        # generator validates against, which forge-conformance.yml now names.
        "inputs": INPUTS_ARE_TRIGGERS,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # ⚠ The C11 arm existed as a complete standalone CMake project, a harness
    # template and ten per-kind fragments, and NOTHING RAN ANY OF IT —
    # measured 2026-09-17, there was no gate script and no job in
    # forge-conformance.yml. The four sibling arms made the set look
    # complete. Its first run was green, so what it had not been surviving
    # was change, which is exactly what a lane is for.
    #
    # ⚠⚠ THE FOLLOW-UP LANDED THE SAME DAY. This entry said `workflows` was
    # empty on purpose — "forge-conformance.yml has no C11 job today, so
    # claiming one would make `--ci-owed` report a lane that cannot pay" —
    # and carried a `no_ci_reason` naming the job as the thing that would
    # retire it. `conformance-c` is in that workflow now, so both are gone.
    #
    # The history is kept rather than deleted silently because a
    # `no_ci_reason` is a PROMISE, and this note is the only evidence it was
    # kept. An entry that simply stopped carrying one would read the same as
    # an entry that never made the promise at all.
    #
    # ⚠⚠⚠ No `ci_only`: unlike the C++ arm, this one is 17s and a push that
    # touches `backends/c/**` or the forge fixtures still selects it. It runs
    # in BOTH places on purpose — the push catches it before it leaves, and
    # the job catches the pushes that bypassed the hooks.
    #
    # Slug `forge-c` until 2026-10-20. A lane gate is named
    # `<family>-<Language::canonical_name()>`, and this backend's name is
    # `c11` (its W3C gate was already `w3c-c11`), so the old slug was the one
    # lane a backend's row could not be derived from.
    "forge-c11": {
        "workflows": ["forge-conformance.yml"],
        "runner_workflow": True,
        # The modules its projects declare every generation through, resolve
        # the generator through, and format generated code by.
        "extra": ["backends/c/**", "cmake/SCECodegenPlan.cmake",
                  "cmake/SCEFindCodegen.cmake", "cmake/SCEClangFormat.cmake"],
        "deps": ["codegen-build"],
        "cost_s": 17,
        "summary": "C11 forge conformance build + test",
        "cache": True,
        # Keyed on its triggers: what `bx --checks prepush --learn forge-c11`
        # (2026-11-24) read outside them, the runner's inputs and
        # `codegen-build`'s key was what the submodule preflight read deriving
        # every gate's triggers, which it no longer does, and the schema the
        # generator validates against, which forge-conformance.yml now names.
        "inputs": INPUTS_ARE_TRIGGERS,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # ⚠ The SIXTH arm, and it was the one the set was missing. Measured
    # 2026-09-18: `forge-conformance.yml` has six jobs — rust, python, go,
    # kotlin, cpp, c — and `scripts/gates/` mirrored five. Nothing local ran the
    # Kotlin forge harness; `w3c-kotlin` runs a different Gradle project. So a
    # break confined to the Kotlin forge lowering passed every local lane and
    # went red only after the push, which is the half of `forge-c11`'s story with
    # the sides swapped: there the code existed and nothing ran it, here CI ran
    # it and the mirror did not. Five siblings made the set look complete.
    #
    # ⚠⚠ `ci_only` on purpose. 52s is over what the push budget will spend on a
    # sixth forge arm, and its five siblings already settled that argument —
    # forge-conformance.yml runs all six in parallel jobs, so paying serially at
    # push time buys attribution and nothing else.
    "forge-kotlin": {
        "workflows": ["forge-conformance.yml"],
        "runner_workflow": True,
        "extra": ["backends/kotlin/**"],
        "deps": ["codegen-build"],
        "ci_only": "52s, the Gradle jvmTest arm. Its five siblings are in CI "
                   "for the same reason and they share one workflow, "
                   "forge-conformance.yml.",
        "cost_s": 52,
        "summary": "Kotlin forge conformance (Gradle jvmTest)",
        "cache": False,
        "no_cache_reason": "samples thread interleavings: its queue and "
                           "broadcast-log stress runs record what the JVM's "
                           "threads happened to do, so a pass is one draw and "
                           "keeping it would stop the drawing",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # ⚠ The Swift backend's own lane, and the one the family tables could not
    # hold. `<family>-<canonical name>` slugs name a backend's arm of a lane the
    # backend has: `w3c-<lang>` and `forge-<lang>` need an emitter that writes a
    # W3C statechart or a Forge kind. What Swift had first was a runtime, a
    # reference host and two hundred generated machines that replay the shared
    # scenarios, and until this entry nothing a runner ran compiled any of it:
    # the Swift lane was `bx -- scripts/test_swift_backend.sh` by hand.
    #
    # The slug is `swift-backend` and not `forge-swift` on purpose. A
    # `forge-swift` would have been a Forge conformance gate that conforms
    # nothing, and `a_backend_holds_what_the_other_backends_hold` reads the slug
    # to decide that the backend HOLDS such a gate, which would erase the `NotYet`
    # row that says it does not. The Forge and W3C arms exist now and took
    # `forge-swift` and `w3c-swift`, and this gate keeps judging the packages that
    # are neither.
    #
    # `deps` is the generator, which the script regenerates the machines with.
    "swift-backend": {
        "workflows": ["swift-backend.yml"],
        "runner_workflow": True,
        "deps": ["codegen-build"],
        # 38s and 19s, the gate script run twice end to end through `bx` on a
        # build machine on 2026-10-28 (the first run built the packages, the
        # second found them built); 38 is declared, the worse of the two. Not
        # taken by `scripts/gate --measure`, which is refused locally for this
        # repository. Both figures include the transfer.
        #
        # `ci_only` is not about the 38s, which the push budget could spend. The
        # toolchain kept it out of every push: a hand-installed pin that
        # `provision` puts on a build machine, and a push the hook ran in place
        # on a machine that lacked it would be refused for a reason that says
        # nothing about the tree. The gate fails loudly without a toolchain
        # rather than skipping. An update to main runs it all the same (see
        # "What `ci_only` means"): the pin is a tool of bx.toml's platform, so
        # bx runs the unit only on a machine that carries it.
        "ci_only": "38s, and the 6.4.0 toolchain is a hand-installed pin "
                   "(`[platform.linux.tools.swift]` in bx.toml); the gate "
                   "refuses without it, so a push of another ref run on a "
                   "machine without the pin would be refused for a reason that "
                   "says nothing about the tree. An update to main runs it "
                   "where bx placed it, on a machine that carries the pin. "
                   "swift-backend.yml installs the pin by its digest.",
        "cost_s": 38,
        # The suite's committed traces, which `ATestSuiteRunsThroughItsRunnerTests.swift`
        # replays through the function and the runner program, and the trace's wire schema,
        # whose status `TraceTests.swift` holds the runtime's to, both read at run time. The
        # workflow's `paths:` names both, as a `ci_only` gate's triggers must be
        # (`ci-only-coverage`).
        "extra": ["sce-build/tests/fixtures/test_profile/runtime/traces/**",
                  "schemas/sce-test-trace.v1.schema.json",
                  # The corpus's suites `ACorpusSuiteReplaysToItsExpectedLogsTests.swift`
                  # replays: the traces and expected logs of each, read at run time, and the
                  # documents of orders_suite and nested_suite, which the regeneration
                  # writes into the machines' target.
                  "sce-build/tests/fixtures/test_profile/runs/**",
                  "sce-build/tests/fixtures/ttcn3_export/**"],
        "summary": "Swift backend: runtime, host features and generated machines",
        # The toolchain is a pinned tool of the platform, so its version is in
        # the key, and a machine without it fails the gate loudly.
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The Swift W3C arm, the twin of `w3c-go`: `sce-codegen generate-w3c -l swift` writes the
    # suite and the engine runs it. It rides the workflow of `swift-backend` as a second step of
    # the same job, as `mutation-cases` rides `tree-hygiene.yml`, because what the two need is the
    # same: the generator built, the pinned toolchain installed and checked against its digest.
    # A job of its own would repeat those steps and pay for them twice.
    #
    # `ci_only` for the reason `swift-backend` is: the 6.4.0 toolchain is a hand-installed pin,
    # and the gate fails loudly without it. An update to main runs it where bx placed it.
    "w3c-swift": {
        "workflows": ["swift-backend.yml"],
        "runner_workflow": True,
        "deps": ["codegen-build"],
        "ci_only": "the 6.4.0 toolchain is a hand-installed pin (`[platform.linux.tools.swift]` "
                   "in bx.toml); the gate refuses without it, so a push of "
                   "another ref run on a machine without the pin would be "
                   "refused for a reason that says nothing about the tree. An "
                   "update to main runs it where bx placed it, on a machine "
                   "that carries the pin. swift-backend.yml installs the pin "
                   "by its digest.",
        # 76s, `scripts/test_swift_backend.sh w3c` through `bx` on a build machine on
        # 2026-11-01, which is what the gate runs and reads (a gate run is refused locally
        # for this repository, so the script under it was timed). 156 fixtures lowered and
        # 156 tests run. Warm, like `swift-backend`'s figure: a runner starts from nothing.
        "cost_s": 76,
        "summary": "W3C conformance, Swift AOT",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The Swift twin of `ecma262-lowered-kotlin`: one machine that asks every case of the shared
    # ECMA-262 table, generated with the build-time lowering (the only route Swift has), compiled
    # and run. A third step of the same job as `swift-backend` and `w3c-swift`, for the same
    # reason: the generator built and the pinned toolchain installed are steps it shares.
    #
    # `ci_only` for the reason `swift-backend` is: the toolchain pin.
    "ecma262-lowered-swift": {
        "workflows": ["swift-backend.yml"],
        "runner_workflow": True,
        "deps": ["codegen-build"],
        "ci_only": "the 6.4.0 toolchain is a hand-installed pin (`[platform.linux.tools.swift]` "
                   "in bx.toml); the gate refuses without it, so a push of "
                   "another ref run on a machine without the pin would be "
                   "refused for a reason that says nothing about the tree. An "
                   "update to main runs it where bx placed it, on a machine "
                   "that carries the pin. swift-backend.yml installs the pin "
                   "by its digest.",
        # 11s, `scripts/test_swift_backend.sh ecma262-lowered` through `bx` on a build machine
        # on 2026-11-01 (the script under the gate: a gate run is refused locally for this
        # repository). 107 cases, one generated machine. Warm, like its siblings: a runner
        # starts from nothing.
        "cost_s": 11,
        "summary": "ECMA-262 lowered artifact, Swift",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The Swift twin of `forge-kotlin`: the forge runtime's own tests and every unit the generator
    # writes for a forge document, held to the numerical oracle. A fourth step of the same job as
    # `swift-backend`, `w3c-swift` and `ecma262-lowered-swift`, for the same reason. It takes the
    # `forge-<canonical name>` slug the `swift-backend` entry said it would, now that Swift writes
    # forge kinds in part and holds what it writes to the oracle; the queue and broadcast-log
    # history checks the other arms make are read here when Swift has those kinds.
    #
    # `ci_only` for the reason `swift-backend` is: the toolchain pin.
    "forge-swift": {
        "workflows": ["swift-backend.yml"],
        "runner_workflow": True,
        "deps": ["codegen-build"],
        "ci_only": "the 6.4.0 toolchain is a hand-installed pin (`[platform.linux.tools.swift]` "
                   "in bx.toml); the gate refuses without it, so a push of "
                   "another ref run on a machine without the pin would be "
                   "refused for a reason that says nothing about the tree. An "
                   "update to main runs it where bx placed it, on a machine "
                   "that carries the pin. swift-backend.yml installs the pin "
                   "by its digest.",
        # 102s, `scripts/test_swift_backend.sh forge` through `bx` on a build machine on
        # 2026-11-01 (the script under the gate: a gate run is refused locally for this
        # repository). The forge runtime's 73 tests and 184 conformance tests. Warm, like its
        # siblings: a runner starts from nothing.
        "cost_s": 102,
        "summary": "Forge conformance, Swift",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The TLA+ models of docs/formal under TLC, as docs/formal/models.json declares
    # them (docs/adr/0115, D5). Nothing ran them before this gate.
    #
    # `ci_only`: the largest configuration is explored whole, which is minutes, and
    # a push has five for everything. TLC is fetched by the runner that needs it and
    # checked against the manifest's digest; Java is what every runner here has, and
    # the gate answers "could not run" (exit 3) where it has not.
    "formal-models": {
        "workflows": ["formal-models.yml"],
        "runner_workflow": True,
        "extra": ["docs/formal/**", "scripts/formal/**", "tools/git-hooks/spec_isolation.py"],
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
    },
    # The proofs of the host core under Verus, as docs/formal/proofs.json declares them
    # (docs/adr/0115, D13).
    #
    # `ci_only`: the push set runs on a developer's machine, which need not carry Verus
    # and the Rust release it runs on (bx.toml declares both for the build machines).
    "formal-proofs": {
        "workflows": ["formal-proofs.yml"],
        "runner_workflow": True,
        "extra": ["docs/formal/proofs.json", "scripts/formal/proofs.py", "backends/rust/mesh/src/https/**"],
        "ci_only": "22s measured 2026-11-23 on a host that has Verus, and the push set runs "
                   "on a developer's machine, which need not carry it or the Rust release it "
                   "runs on. formal-proofs.yml runs it on every change to a proved file or its "
                   "judge, and bx.toml declares both tools for the build machines.",
        "cost_s": 22,
        "summary": "Verus proofs of the host core, with each break of it refused",
        # Verus answers one way for one input: the file, the manifest and the pinned tool.
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 2,
        "parallelism": PARALLELISM_SCRIPT,
    },
    # The C11 and C++ queues under GenMC (SCE Protocol-Synthesis RFC §synth-5-P,
    # layer 3), and the mutants that show the check reads orderings at all. It needs
    # GenMC, which no hosted runner carries and which is built from source
    # (`scripts/install_genmc.sh`), so it runs where that was done.
    #
    # ⚠ `ci_only` because the install is the cost and the hosts do not all have
    # it: the gate fails loudly where GenMC is absent rather than skipping, and
    # a push that selected it on such a host would be refused for a reason that
    # says nothing about the tree. The workflow caches the build under a key
    # made of the installer's hash, so the pin and the cache move together.
    "genmc-queue": {
        "workflows": ["genmc-queue.yml"],
        "runner_workflow": True,
        # What the gate reads, and no wider: the runtime headers GenMC compiles,
        # the harnesses, and the hosted atomics every C harness includes whole.
        # For C++ that is the three headers a mutant rewrites (the two queue
        # headers and the broadcast log) and the
        # harnesses with their shim and support headers; every other header of
        # that runtime is not read. The same set as the workflow's `paths:`,
        # which the self-test holds.
        "extra": [
            "backends/c/forge-runtime/include/**",
            "backends/c/runtime/include/sce/text.h",
            "backends/c/forge-runtime/tests/genmc/**",
            "backends/c/forge-runtime/tests/conformance/sce_atomic_host.c",
            "backends/cpp/forge-runtime/include/sce/forge/queue.h",
            "backends/cpp/forge-runtime/include/sce/forge/queue_segmented.h",
            "backends/cpp/forge-runtime/include/sce/forge/broadcast_log.h",
            "backends/cpp/forge-runtime/tests/genmc/**",
            "scripts/install_genmc.sh",
        ],
        "ci_only": "72s measured 2026-10-10 on a host that has GenMC, and the "
                   "host is the constraint: GenMC is built from source "
                   "(scripts/install_genmc.sh), no hosted runner carries it, "
                   "and the build machines do not all have it. An update to "
                   "main runs it where bx placed it: GenMC is a tool of "
                   "bx.toml's platform, needed by this gate's runs. "
                   "genmc-queue.yml builds it once per pin and caches it.",
        "cost_s": 72,
        "summary": "C11 and C++ queues under GenMC + weakened-ordering mutants",
        # GenMC explores every interleaving its model admits rather than
        # sampling some, and its revision is pinned by the tracked installer.
        "cache": True,
        # Keyed on its triggers: `bx --checks prepush --learn genmc-queue`
        # (2026-11-24) found 55 reads and 4 listings, every one inside the
        # triggers and the runner's inputs.
        "inputs": INPUTS_ARE_TRIGGERS,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # A C11 machine's core links with no C library, is weighed against a budget
    # and runs its replay scenario on a Cortex-M4 under QEMU (docs/adr/0110, D3):
    # the set the ADR names, generated, compiled with the ADR's flags, linked
    # with `-nostdlib -lgcc` beside the board's startup, memory functions and
    # seams (backends/c/mcu/), and held to backends/c/mcu/footprint.budget.
    #
    # `cost_s` is an ESTIMATE, absent from COST_MEASURED: the gate was written
    # where no ARM compiler could run. Six generations, a few dozen small C
    # compiles and six QEMU runs of a second or two each, after the generator
    # build `codegen-build` owns. Time it on its first run and date it here.
    #
    # `ci_only` for `genmc-queue`'s reason: the ARM compiler is a tool bx.toml
    # declares (`[platform.linux.tools.arm-none-eabi-gcc]`, Arm's tarball of the
    # release scripts/install_arm_gnu_toolchain.sh pins) and QEMU a package the
    # unit requires (`packages` below), and not every build machine is verified
    # to carry them; the gate refuses (exit 3) without them, so a push that
    # selected it there would be refused for a reason that says nothing about
    # the tree. c11-mcu.yml installs both on its runner.
    "c11-mcu": {
        "workflows": ["c11-mcu.yml"],
        "runner_workflow": True,
        "deps": ["codegen-build"],
        "ci_only": "Arm's GNU Toolchain, a tool bx.toml pins by the install "
                   "script's digests, and QEMU, a package the gate's unit "
                   "requires, which not every build machine is verified to "
                   "carry; the gate ends 'could not run' without them, so a "
                   "push that selected it there would be refused for a reason "
                   "that says nothing about the tree. c11-mcu.yml installs both.",
        "cost_s": 60,
        "summary": "C11 machines linked with no libc, weighed and run on a Cortex-M4",
        # QEMU's release does not move the verdict: a run passes or fails on
        # the scenario's expectations, on a counter clock the scenario alone
        # moves. So it is a requirement of the unit, not a tool with pinned
        # bytes (docs/adr/0110, D4).
        "packages": ["qemu-system-arm"],
        # The compiler is a tool of the platform at the release bx.toml checks
        # (and the gate refuses another build of it), the budget and the
        # scenarios are tracked, and the run is on a counter clock the
        # scenario alone moves.
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
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
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
        "parallelism": PARALLELISM_SCRIPT,
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
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
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
        # The validator is the revision a tracked workflow pins. Its git reads
        # (`git diff --cached`, `checkout-index`) are the commit hook's
        # `--staged` mode; the full gate a unit runs reads the tracked tree.
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
        "parallelism": PARALLELISM_SCRIPT,
    },
    # The authoring core's claim is that it knows no subject matter. A claim
    # like that decays silently -- one helpful special case at a time -- so it
    # is a test. It was declared Python only at 0.262s; the suite since grew
    # to 413 cases, 86 of which drive a document through `sce-codegen`, and
    # the figure was never retaken.
    #
    # ⚠ What moved it: a push on 2026-09-24 was refused at 331s of the 300s
    # ceiling with every gate green, this one reading 81s paced. Timed alone
    # it was 155s with the `cargo build` (opt 0) binary `codegen-build`
    # leaves for it, and 72s once `sce-build` builds at opt 1 in the dev
    # profile (`Cargo.toml`). The run-driven 86 are also why CI now builds
    # the generator for this lane: without it they skipped, and the lane
    # passed without judging them.
    "authoring-core": {
        "workflows": ["authoring-core.yml"],
        "runner_workflow": True,
        "ci_only": "72s measured with the generator built at opt 1, 155s "
                   "at opt 0, against a declared 0.262. authoring-core.yml "
                   "runs it twice: Python only, and with sce-codegen built "
                   "so the 86 run-driven cases are judged, not skipped.",
        "cost_s": 72,
        "summary": "authoring core is domain-free, and its refusals still fire",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
        "parallelism": PARALLELISM_SCRIPT,
    },
    # The logical interface every target shares, and the library built from it. The generator and
    # a compiler are needed (the parity cases build and run C++), so unlike authoring-core's
    # Python-only job this one has no run without them: the gate refuses to start.
    "targets": {
        "workflows": ["targets.yml"],
        "runner_workflow": True,
        "ci_only": "81s with the generator built at opt 1: the interface cases are seconds each, "
                   "the three parity cases build and run a C++ library. Not taken by "
                   "`scripts/gate --measure`, which is refused locally for this repository.",
        "cost_s": 81,
        "summary": "the logical interface is platform-free, and the library judges as verify does",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The gate whose absence let a stale verifies-catalog reach CI red:
    # `ledger-citations` runs mnemosyne-cli, this workflow runs a separate
    # python generator, and the hook mirrored only the first.
    "spec-snapshot": {
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
    },
    # A compliance verdict — an AGPL-3.0 section 4 / MIT section 1 violation in a
    # release tarball — that was reachable only after a push, on a check that
    # needs nothing but bash and takes three hundredths of a second.
    "license-ssot": {
        "workflows": ["license-verify.yml"],
        "runner_workflow": True,
        "cost_s": 0,
        "summary": "license SSOT vs the tree it describes",
        # `package_embed.sh` writes `git describe` into the package's VERSION
        # file, which the licence check does not read.
        "cache": True,
        # Read by `check learn` (2026-11-24, bx 340e9f9e): the SSOT, the
        # licence texts it names, and its checker. The checker lists
        # `third_party/` and each dependency's directory and looks at every
        # file at depth two (`find -maxdepth 2 -type f`) to find a dependency
        # the SSOT does not name, which is the violation this gate exists for:
        # `third_party/` and `third_party/*/` are those listings and
        # `third_party/*/*` those files. Two notices sit deeper and are opened
        # by name (lua.h, json.hpp). `LICENSE*` covers the root's texts.
        "inputs": [
            "LICENSE*",
            "sce/sce_licenses.cmake",
            "scripts/verify_licenses.sh",
            "third_party/",
            "third_party/*/",
            "third_party/*/*",
            "third_party/lua/src/lua.h",
            "third_party/nlohmann_json/include/nlohmann/json.hpp",
        ],
        "commits": [],
        "peak_gb": 1,
        "parallelism": PARALLELISM_SCRIPT,
    },
    # The same compliance verdict for the browser visualizer, which
    # `license-ssot` cannot reach: it reads `third_party/`, and the
    # visualizer's libraries live under `web/visualizer/vendor/`.
    #
    # That gap was not hypothetical. d3 (ISC) shipped vendored and
    # unregistered for as long as the visualizer existed, and elkjs
    # (EPL-2.0 — a licence that obliges a COMMERCIAL distributor to defend
    # and indemnify every other contributor) was pulled from unpkg at
    # runtime by one `<script src>` line. Throughout, LICENSE-THIRD-PARTY.md
    # opened with "All dependencies are MIT licensed" and nothing in the
    # tree could contradict it.
    "web-vendor-licenses": {
        "workflows": ["license-verify.yml"],
        "runner_workflow": True,
        # The files the gate OPENS, not only the ones it is about. It reads
        # each vendored library's `LICENSE` and `README.md` and the registry
        # itself, so a commit that touches only one of those — deleting a
        # licence text, say, which is the violation — has to select this
        # lane. `script-read-coverage` refuses the entry without them.
        "extra": [
            "web/visualizer/vendor/**",
            "web/visualizer/*.html",
            "LICENSE-THIRD-PARTY.md",
        ],
        "cost_s": 0,
        "summary": "visualizer vendor libraries vs LICENSE-THIRD-PARTY.md",
        "cache": True,
        # Read by `check learn` (2026-11-24, bx 340e9f9e): the registry of
        # licences, each vendored library's texts and the pages that load them
        # (`web/visualizer/*.html`). It lists `web/visualizer/` for the pages
        # and `web/visualizer/vendor/` for a library nobody registered; the
        # vendor tree is named whole, which gives its listings too.
        "inputs": [
            "LICENSE-THIRD-PARTY.md",
            "web/visualizer/",
            "web/visualizer/*.html",
            "web/visualizer/vendor/**",
        ],
        "commits": [],
        "peak_gb": 1,
        "parallelism": PARALLELISM_SCRIPT,
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
        # 0.185s, and the first figure here this gate has ever been able to
        # confirm. The 4 it carried was chosen on 2026-08-24 to cover the
        # process the runner starts around a 124ms scan, because `--measure`
        # answered 0 every time — `scripts/gate` timed with whole-second
        # `SECONDS`, so a gate that finishes in a fifth of a second had no
        # readable cost at all. The runner now keeps a millisecond clock, and
        # six runs on 2026-09-02 read 0.165 0.145 0.154 0.162 0.169 0.185,
        # none of them queued behind the cargo lock. 0.185 is the worse of
        # the last pair. The process the 4 was covering turns out to be
        # ~40ms, not the seconds it was insured against.
        "cost_s": 0.185,
        "summary": "BasicHTTP fixture endpoint has one owner",
        "cache": True,
        # The whole tree, and it stays so: the scan finds its candidates with
        # `git grep` over every tracked source, and in a narrowed unit's copy
        # that search would see only the inputs, so a re-spelling in a file
        # left out would pass.
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
        "parallelism": PARALLELISM_SCRIPT,
    },
    # The generated codecs are committed goldens, not workspace members:
    # `cargo metadata` lists eight packages and none of them is a codec, so
    # `clippy --workspace` lints the generator and never its output. Both arms
    # were CI-only until the registry started asking which workflows lack a
    # gate.
    "codec-clippy": {
        "workflows": ["sce-forge-codec-clippy.yml"],
        "runner_workflow": True,
        # The goldens compile against the Rust forge runtime and what cargo
        # builds for it (its path dependencies, the workspace manifests). The
        # workflow named the runtime's directory alone, so an edit of
        # `backends/rust/portable-bytes` selected nothing that lints codecs.
        "builds": ["forge-runtime-rust"],
        "ci_only": "106s linting 95 generated codec goldens. "
                   "sce-forge-codec-clippy.yml runs it.",
        "cost_s": 106,
        "summary": "clippy generated codecs, alloc on",
        "cache": True,
        # Keyed on its triggers and the two directories the script lists for
        # `codec_*.rs`. `bx --checks prepush --learn codec-clippy` (pc2,
        # 2026-11-24): 206 reads and 5 listings, every one inside these once
        # the runtime's cargo build is among the triggers.
        "inputs": INPUTS_ARE_TRIGGERS,
        "lists": ["tests/forge/expected/", "tests/forge/expected/views/"],
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    "codec-no-alloc": {
        "workflows": ["sce-forge-codec-no-alloc.yml"],
        "runner_workflow": True,
        # As `codec-clippy`: the goldens compile against the runtime's build.
        "builds": ["forge-runtime-rust"],
        "ci_only": "90s. sce-forge-codec-no-alloc.yml runs it, next to its "
                   "alloc-on sibling.",
        # The text algorithms it compiles are read from a materialization of
        # `rust-static-datamodel` (docs/adr/0097), which sce-codegen writes.
        "deps": ["codegen-build"],
        "cost_s": 90,
        "summary": "generated codecs compile without alloc",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The two conformance surfaces nothing local ran. Both narrow their
    # workflow's catch-all (see `narrows` in `gate_triggers`) and state their
    # real inputs here: the engine, the generator, the templates that emit the
    # AOT runners, and the fixtures the runners are registered from.
    #
    # The Rust arm of the same workflow was left out here once, on the grounds
    # that `sce-rust-tests` is a workspace member and `workspace-tests` already
    # runs its cases. That run is not the lane's: it takes the committed tree
    # in the test profile, while the lane regenerates and runs in release. So
    # the lane kept its own inline spelling, and it is `w3c-rust` now.
    "w3c-cpp": {
        "workflows": ["w3c-tests.yml"],
        "builds": ["main-tree"],
        "runner_workflow": True,
        "narrows": "the lane declares no `paths:` filter, which is a statement "
                   "about a runner paid by the minute, not about what the "
                   "suite reads. Inheriting it would run the longest gate in "
                   "the set for a README edit.",
        "extra": ["sce/**", "tests/w3c/**", "tests/CMakeLists.txt",
                  "resources/**", "tools/codegen/templates/**",
                  "sce-build/src/**",
                  # The modules the route registers and plans its fixtures
                  # through, and the one the test tree takes the warning
                  # classes its test programs turn off from.
                  "cmake/SCEW3CRegistry.cmake", "cmake/SCEStaticW3CTest.cmake",
                  "cmake/SCECodegenPlan.cmake", "cmake/SCETestWarnings.cmake"],
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
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
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
    "visualizer-layout": {
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
    },
    "visualizer-wasm": {
        "workflows": ["deploy-visualizer.yml"],
        # The page's engine is the main tree built with emscripten, and the
        # codegen WASM is `sce-build-wasm` built by wasm-pack.
        "builds": ["main-tree", "codegen-wasm"],
        "runner_workflow": True,
        "ci_only": "77s measured 2026-08-22 (declared 42) building three "
                   "WASM artifacts. deploy-visualizer.yml runs it, and this "
                   "gate declares no trigger of its own, so the workflow's "
                   "filter IS the trigger — the two cannot disagree.",
        "cost_s": 77,
        "summary": "codegen + visualizer + DOOM WASM builds",
        # The SDK is bx.toml's pinned emsdk tool, wasm-pack a need of the
        # platform.
        "cache": True,
        "kept_only_from_a_landing": "the emscripten half runs only where emsdk is "
                                "found, so a pass without it judged one "
                                "artifact of three",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The Rust engine as the module a browser loads, run under Node and in
    # headless Chrome on the scenarios every backend replays. No other lane can
    # say whether the engine starts on `wasm32-unknown-unknown`: that target
    # gives a module no clock, and the failure compiles and traps only when the
    # module runs. Chrome is the web target's own host and Node the faster one.
    #
    # `ci_only` for what it needs rather than for the number alone: the target's
    # standard library, Node, Chrome, and a `wasm-bindgen` tool at exactly the
    # version `Cargo.lock` holds, none of which a push can assume. The
    # workflow's filter IS the trigger, as `visualizer-wasm`'s is.
    #
    # Measured 2026-10-08 as the gate script itself, through `bx --local`
    # (`scripts/gate --measure` is refused locally for this repository): 107s
    # with the dependencies built (the runtime and the replay crate rebuilt),
    # of which Chrome's replay is 20s. The first dev-profile build of the target
    # on that tree took 128s before Chrome was added. The same gate built in
    # release took 620s (`lto = "thin"` with one codegen unit over 44 generated
    # machines), which is why it builds the dev profile.
    "wasm32-scenarios": {
        "workflows": ["wasm32-scenarios.yml"],
        "runner_workflow": True,
        "ci_only": "107s measured 2026-10-08 with the dependencies built "
                   "(128s on a first dev-profile build before Chrome was "
                   "added, 620s in release). wasm32-scenarios.yml runs it, "
                   "and needs the wasm32 standard library, Node, headless "
                   "Chrome and a wasm-bindgen at the version Cargo.lock "
                   "holds. An update to main runs it where bx placed it, on "
                   "a machine carrying bx.toml's pinned Chrome and "
                   "wasm-bindgen tools.",
        "cost_s": 107,
        "summary": "Rust engine built for wasm32, every scenario replayed in Node and Chrome",
        # The browser and the glue tool are pinned tools of bx.toml's platform
        # (`chrome`, Chrome for Testing at the version scripts/install_chrome.sh
        # names, and `wasm-bindgen` at Cargo.lock's), and the wasm32 standard
        # library is a target of the release the plan names, declared with it
        # in `[platform.linux.tools.rust]`. In a landing
        # only the pinned Chrome judges the module, so a pass kept from one is
        # that browser's; outside a landing the gate may replay in the Chrome on
        # the machine's PATH, which no key names.
        "cache": True,
        "kept_only_from_a_landing": "outside a landing it replays in whatever "
                                    "Chrome is on PATH where the pinned Chrome "
                                    "for Testing is absent",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The other half of the main tree's ctest partition, and the half nothing
    # ran. Measured 2026-08-12 with a logging shim in place of `ctest` during a
    # full `scripts/gate --all`: 28 gates passed and the only runs against this
    # build were `w3c-c11`'s two. 159 of 382 registered cases — every `mesh_*`
    # case and every C++ unit suite — were executed by no gate and by no
    # workflow, since none configures the main tree.
    "cpp-suite": {
        "workflows": ["cpp-suite.yml"],
        "builds": ["main-tree"],
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
        # The transport SDKs' pin comes first: the build machines install them
        # with that script (`[platform.linux.tools.mesh-transports]`). Then
        # what spreads the suite over the lane's shards (`SCE_CTEST_SHARD`)
        # and the reading of the tree it plans from, which the gate script
        # runs, and the report of where the build's time went.
        "extra": ["scripts/install_mesh_transports.sh",
                  "scripts/lib/ctest_shards.py",
                  "scripts/lib/ctest_registered_commands.cmake",
                  "scripts/lib/build_time_report.py",
                  "sce/**", "tests/**", "cmake/**", "CMakeLists.txt",
                  "examples/**", "resources/**", "tools/codegen/templates/**",
                  "sce-build/src/**",
                  # The union documents every algorithm lane is held to, which
                  # the Interpreter's lowered-algorithm test lowers and runs
                  # (AnAlgorithmRunsLoweredUnderTheInterpreterTest.cpp).
                  "sce-build/tests/fixtures/union/**",
                  # The algorithms the catalog names in the statechart
                  # fixtures (a map an algorithm is lent, docs/adr/0108 step
                  # 7b), which the same test lowers and runs, and the
                  # statecharts AStaticDatamodelRunsLoweredUnderTheInterpreter
                  # Test lowers beside them.
                  "sce-build/tests/fixtures/static_datamodel/**",
                  # The test profile's documents the build generates the
                  # suites from, their traces and their expected logs, which
                  # ATestSuiteRunsEndToEndTest and
                  # ACorpusSuiteReplaysToItsExpectedLogsTest read at run time.
                  "sce-build/tests/fixtures/test_profile/**",
                  # The documents of the corpus's orders_suite and
                  # nested_suite, which the build generates.
                  "sce-build/tests/fixtures/ttcn3_export/**"],
        "deps": ["codegen-build"],
        "ci_only": "110s building and running the whole C++ ctest tree. "
                   "cpp-suite.yml runs it. Its `extra` list is wide on "
                   "purpose — sce/**, tests/**, examples/** — so it was "
                   "selected by most pushes, which is what made it a budget "
                   "problem rather than an occasional one.",
        "cost_s": 110,
        "summary": "C++ ctest suite (engine + mesh), the non-c11 half",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # ADR 0059, section 7: the C++ interface compiled at the standard a
    # consumer compiles it at. Every public header, every Forge C++ golden and
    # every C++ header the generators write for this configure, each compiled
    # ALONE with -Werror at C++17 and at C++20, beside the scan that refuses a
    # standard-selecting `#if` in the headers and the C++ templates.
    #
    # SCE's own build compiles every target at C++20, so the C++17 four
    # targets declared had never been compiled, and a downstream C++17 build
    # found the first defect under GCC 15. No other gate can see that class:
    # each builds SCE the way SCE builds itself.
    #
    # ⚠ `cost_s` is an ESTIMATE, not a stopwatch reading, and the slug is
    # absent from COST_MEASURED for that reason. Its parts: a cold scratch
    # configure and the generation rules (about two minutes), then some 2,600
    # syntax-only compiles that `scripts/lib/sce_build_jobs.sh` spreads over
    # the machine (about three on a 32-core build host). Time
    # `scripts/gate cpp-interface-standard` on such a host and date it here.
    "cpp-interface-standard": {
        "workflows": ["cpp-interface-standard.yml"],
        "builds": ["main-tree"],
        "runner_workflow": True,
        # What the gate compiles or reads to decide what to compile: the
        # engine and its headers, the C++ runtime and the C backend it
        # compiles, the C++ templates and the generator that fills them, the
        # Forge goldens, the CMake that registers the corpora, and the tool.
        # The workflow's `paths:` lists every one, which `ci-only-coverage`
        # holds.
        "extra": ["sce/**", "backends/cpp/**", "backends/c/**",
                  "tools/codegen/templates/**", "sce-build/src/**",
                  "tests/forge/expected/**", "tests/CMakeLists.txt",
                  "CMakeLists.txt", "cmake/**", "tools/interface-standard/**"],
        "deps": ["codegen-build"],
        "ci_only": "minutes long, estimated rather than measured: a cold "
                   "configure, every generation rule and about 2,600 "
                   "compiles, in a GCC 15 image the runner builds first. "
                   "cpp-interface-standard.yml runs it, and an update to main "
                   "runs it on a build machine with a rootless Docker.",
        "cost_s": 300,
        "summary": "C++ interface compiled alone at C++17 and C++20 (ADR 0059)",
        # The verdict is GCC 15's: its libstdc++ is the one that drops a
        # transitive include first. So the gate runs in one image wherever it
        # runs, the hosted lane, a build machine or a workstation alike: the
        # runner builds `container` (gcc:15.3.0 and what the configure needs)
        # and runs `scripts/gate cpp-interface-standard` inside it, generator
        # build included (see "Gates that run in an image"). A machine's own
        # GCC never judges it. A build machine takes the unit only where its
        # run account has a rootless Docker (`needs`).
        "container": "tools/interface-standard/Containerfile",
        "needs": ["docker"],
        "cache": False,
        "no_cache_reason": "the image is built from a tag (gcc:15.3.0) and the "
                           "Debian archive, and neither is fixed by the tree: "
                           "a rebuilt image can carry another compiler patch "
                           "or package, and the verdict moves with them",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # An installed SCE builds what its own generator writes. The install
    # exports `sce-codegen`, its templates and `SCECodegen.cmake`, and the C++
    # they write includes the Forge C++ runtime; until that runtime was an
    # exported target, the install shipped one of its headers and only under
    # the mesh, and `tests/install_tree_consumer` — the test that would have
    # said so — was opt-in and run by no lane. This builds a scratch tree,
    # installs it, and builds and runs four probe projects against the prefix,
    # one of them a statechart generated by the installed generator.
    #
    # ⚠ `cost_s` is an ESTIMATE, not a stopwatch reading, and the slug is
    # absent from COST_MEASURED for that reason. Its parts: a cold scratch
    # configure and a build of the SCE libraries with no tests (about the
    # 123s `ecma262-lowered-cpp` measured for its own scratch tree with ccache
    # warm, which also builds a test), the install, and four small probe
    # configures and builds. Time `scripts/gate install-consumer` on a build
    # host and date it here.
    "install-consumer": {
        "workflows": ["install-consumer.yml"],
        "builds": ["main-tree"],
        "runner_workflow": True,
        # What the gate builds, installs or runs: the CMake that defines and
        # installs the targets and the package config, the engine and its
        # headers, the Forge C++ runtime, the templates and the generator the
        # install ships, and the probes. The workflow's `paths:` lists every
        # one, which `ci-only-coverage` holds.
        "extra": ["CMakeLists.txt", "cmake/**", "sce/**",
                  "backends/cpp/forge-runtime/**", "tools/codegen/**",
                  "sce-build/src/**", "tests/install_tree_consumer/**"],
        "deps": ["codegen-build"],
        "ci_only": "minutes long, estimated rather than measured: a cold "
                   "configure and build of the SCE libraries in a tree of "
                   "its own, an install and four probe builds — the scratch "
                   "tree is the point, so a warm developer tree does not make "
                   "it cheaper. install-consumer.yml runs it.",
        "cost_s": 240,
        "summary": "generated C++ built and run against an installed SCE",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The C++ client parts built with MSVC on Windows (docs/adr/0107, D11): the
    # reference host, its COSE operations on vcpkg's libsodium and the generated
    # machines through CMake, and the Bazel consumers through both channels.
    # No other gate builds on Windows, and no push host is one.
    #
    # ⚠ `cost_s` is an ESTIMATE, absent from COST_MEASURED: the lane has no run.
    # Its parts are a cached generator build, a cold configure and a build of
    # sce_base and one test, two vcpkg ports and a cold Bazel through two
    # channels. Time the gate on a Windows host and date it here.
    "windows-msvc": {
        "workflows": ["windows-msvc.yml"],
        # Not delegated: the workflow runs the gate's script itself, not through
        # `scripts/gate`. The runner opens with this file's self-test and
        # `scripts/lib/require_submodules.sh`, which read the tree through
        # Python on POSIX paths and are held on the Linux lanes; on the Windows
        # runner they would be a second, unrelated verdict ahead of the one this
        # lane exists for. The commands live once, in the script.
        "ci_only": "a Windows host with Visual Studio 2022 and vcpkg, which no "
                   "push host is, and minutes long, estimated rather than "
                   "measured. windows-msvc.yml runs it on a hosted Windows "
                   "runner.",
        "cost_s": 1500,
        "summary": "C++ host, COSE and generated machines with MSVC; Bazel channels",
        # Off the fleet, so a landing leaves it to the hosted lane (see "Where
        # a gate can run"). The host is the platform: every build machine is
        # the Linux bx.toml declares, and the verdict is MSVC's on Windows, so
        # a landing unit could never be placed and every item reaching the
        # gate would come back unlanded. When bx.toml declares a Windows
        # platform this attribute names it, and the gate joins the judgment.
        "platform": "github-hosted-windows-2022",
        "platform_reason": "the verdict is MSVC's, with Visual Studio 2022 and "
                           "vcpkg on the hosted windows-2022 image; every build "
                           "machine is the Linux platform bx.toml declares",
        "cache": False,
        "no_cache_reason": "a host compiler and ports no declaration pins: the "
                           "verdict is MSVC's as the Windows image carries it, and "
                           "vcpkg's libxml2 and libsodium at the versions its "
                           "checkout holds",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
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
        "builds": ["main-tree"],
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
        "cache": True,
        # The whole tree: `bx --checks prepush --learn ecma262-lowered-cpp`
        # (2026-11-24) read 946 of its 1931 paths outside its triggers, nearly
        # all of them the root configure's test tree (`tests/**`,
        # `resources/**`, `integration_resources/**`, the LICENSE files,
        # `web/visualizer/test-list.js`), which `main-tree` does not name.
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
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
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    "w3c-c11": {
        "workflows": ["w3c-tests.yml"],
        "builds": ["main-tree"],
        "runner_workflow": True,
        "narrows": "same catch-all as `w3c-cpp`, same reason. This arm's real "
                   "inputs are the C backend, the C templates, the "
                   "generator, and the inbox probe it shares with the C++ "
                   "engine's inbox test (docs/adr/0084).",
        # The shared Lua the machines embed, and the shared tables every Lua
        # engine's readers ask (`system_variable_writes.json` among them).
        "extra": ["backends/c/**", "tools/codegen/templates/**",
                  "sce-build/src/**", "tests/integration/inbox_probe.scxml",
                  "sce/include/scripting/*.lua", "tests/ecmascript/**",
                  # The registry the route registers every fixture from, the
                  # documents it generates, and the modules it registers and
                  # plans them through.
                  "tests/w3c/conformance/fixtures.json", "resources/**",
                  "cmake/SCEW3CRegistry.cmake", "cmake/SCEStaticW3CTest.cmake",
                  "cmake/SCECodegenPlan.cmake",
                  # The test log every runtime writes of the generated suite,
                  # which `test_a_test_suite_runs_through_its_runner.c` reads
                  # at run time, and the wire schema whose status
                  # `test_case_run_test.c` holds the runtime's to.
                  "sce-build/tests/fixtures/test_profile/runtime/expected_logs/**",
                  "schemas/sce-test-log.v1.schema.json",
                  # The suite's committed traces, which the same test and the
                  # runner program's replay, and the trace's wire schema,
                  # whose status `input_trace_test.c` holds the runtime's to,
                  # all read at run time.
                  "sce-build/tests/fixtures/test_profile/runtime/traces/**",
                  "schemas/sce-test-trace.v1.schema.json",
                  # The documents of the corpus's boundary_suite, which the
                  # build generates, and the traces and expected logs of the
                  # corpus's other suites, which
                  # `test_a_corpus_suite_replays_to_its_expected_logs.c`
                  # replays and reads at run time.
                  "sce-build/tests/fixtures/test_profile/boundary/**",
                  "sce-build/tests/fixtures/test_profile/runs/**",
                  # The documents of job_suite the build generates, and the
                  # job's completion of the kind examples its port sends.
                  "sce-build/tests/fixtures/test_profile/*.scxml",
                  "sce-build/kind-examples/event-schema.scxml",
                  # The documents of orders_suite and nested_suite, the records
                  # and templates over a union they import, and the unions.
                  "sce-build/tests/fixtures/ttcn3_export/**",
                  "sce-build/tests/fixtures/test_profile/imports/**",
                  "sce-build/tests/fixtures/union/**",
                  # The configurations of every form the build generates,
                  # whose readers and writers
                  # `test_a_record_reads_back_the_form_it_writes.c` round-trips,
                  # and the reader of a received message's every part the
                  # build generates and
                  # `test_a_received_message_is_made_in_the_machines_types.c`
                  # runs.
                  "sce-build/tests/fixtures/test_profile/trace_forms/**",
                  "sce-build/tests/fixtures/test_profile/received/**"],
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
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    "w3c-go": {
        "workflows": ["w3c-tests.yml"],
        "runner_workflow": True,
        "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
                   "the Go backend, the Go templates and the generator.",
        # The shared Lua is copied under backends/go; the shared tables its
        # readers ask are not.
        "extra": ["backends/go/**", "tools/codegen/templates/**",
                  "sce-build/src/**", "tests/ecmascript/**",
                  # The test log's wire schema, which the runtime's
                  # `TestEveryKindIsOneTheSchemaNames` reads at run time.
                  "schemas/sce-test-log.v1.schema.json",
                  # The test log every runtime writes of the generated suite,
                  # which the Go suite runner's test of `answers_the_job`
                  # reads at run time.
                  "sce-build/tests/fixtures/test_profile/runtime/expected_logs/**",
                  # The suite's committed traces, which the same test replays,
                  # and the trace's wire schema, whose status the runtime's
                  # `TestSchemaFileDeclaresStatus` holds its own to, both read
                  # at run time.
                  "sce-build/tests/fixtures/test_profile/runtime/traces/**",
                  "schemas/sce-test-trace.v1.schema.json",
                  # The traces and expected logs of the corpus's other suites,
                  # which `a_corpus_suite_replays_to_its_expected_logs_test.go`
                  # replays and reads at run time.
                  "sce-build/tests/fixtures/test_profile/runs/**"],
        "deps": ["codegen-build"],
        # 10s, timed on a warm tree on 2026-09-02 and no longer a normalised
        # figure. Six `--measure` runs read 9 10 9 10 10 10 — none of them
        # queued behind the cargo package-cache lock, which is what makes
        # this the cleanest of the three retirements. 10 is the worse of the
        # last pair. Two pushes that day read 15s and 8s, straddling it.
        "cost_s": 10,
        "summary": "W3C conformance, Go AOT",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The Rust arm, and the last W3C lane to get a gate: `test-rust` in
    # w3c-tests.yml carried its generation, fixture server and `cargo test`
    # inline while every sibling job ran `scripts/gate w3c-<backend>`. A
    # backend's lanes are derived from this table as
    # `<family>-<Language::canonical_name()>`, so the missing slug was a lane
    # the Rust row could not be read from.
    #
    # ⚠ No `cost_s`: nobody has timed this script yet, and a figure borrowed
    # from the lane's timeout would be a guess wearing a measurement's unit.
    # `run_order` sorts an unmeasured gate last, and UNMEASURED_COST_CEILING
    # counts it, which is the honest place for it until a run is read.
    "w3c-rust": {
        "workflows": ["w3c-tests.yml"],
        "runner_workflow": True,
        "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
                   "the Rust runtime and Lua crates, the Rust templates, the "
                   "generator, the W3C fixtures it generates from and the "
                   "build fixtures its hand-written tests compile in.",
        "extra": ["backends/rust/**", "tools/codegen/templates/**",
                  "sce-build/src/**", "resources/**", "tests/w3c/**",
                  "sce-build/tests/fixtures/**",
                  # The shared Lua the Lua crate loads, and the shared tables
                  # its readers ask.
                  "sce/include/scripting/*.lua", "tests/ecmascript/**",
                  # What the crate's build script generates its trees from
                  # (docs/adr/0097): their manifests, the standard library and
                  # the documents outside the fixture directories.
                  "tools/codegen/trees/**", "stdlib/**",
                  "integration_resources/**", "tests/integration/**",
                  "examples/ai_loop/**"],
        # No `deps`: the crate's build script is the generator of its own
        # trees, built from sce-build as a build-dependency, so no gate's
        # sce-codegen is executed.
        "ci_only": "unmeasured, and the lane it mirrors says why it cannot "
                   "be a push gate: `test-rust` carries a 65-minute timeout, "
                   "the longest of the W3C jobs, because it links the "
                   "release test binaries under thin LTO with one codegen "
                   "unit. w3c-tests.yml declares no `paths:` filter, so CI "
                   "runs this arm on every push.",
        "summary": "W3C conformance, Rust AOT",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    "w3c-kotlin": {
        "workflows": ["w3c-tests.yml"],
        "runner_workflow": True,
        "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
                   "the Kotlin backend, the Kotlin templates and the "
                   "generator.",
        # The last three are what the SCRIPT opens: the refusal list it
        # holds a ceiling over, the fixture registry it walks, and the one
        # document it probes the seam with. Declared because `--ci-owed`
        # reads this list to say which lanes judge a change, and a commit
        # editing only a divergence list was being told this lane was not
        # one of them (`script-read-coverage`).
        "extra": ["backends/kotlin/**", "tools/codegen/templates/**",
                  "sce-build/src/**",
                  # How both suite modules run their tests and take a row's
                  # overlay, applied by each (it was the tests module's own
                  # build file until the W3C suite became a module).
                  "gradle/sce-kotlin-suite.gradle.kts",
                  # The shared Lua the Lua engine copies in at build time, and
                  # the shared tables its readers ask.
                  "sce/include/scripting/*.lua", "tests/ecmascript/**",
                  "tests/ecmascript/kotlin_frontend_refusals.json",
                  "tests/w3c/conformance/fixtures.json",
                  "resources/150/test150.scxml",
                  # The test log every runtime writes of the generated suite,
                  # which `ATestSuiteRunsThroughItsRunnerTest` reads at run time.
                  "sce-build/tests/fixtures/test_profile/runtime/expected_logs/**",
                  # The suite's committed traces, which the same test replays,
                  # and the trace's wire schema, whose status `TraceTest` holds
                  # the runtime's to, both read at run time.
                  "sce-build/tests/fixtures/test_profile/runtime/traces/**",
                  "schemas/sce-test-trace.v1.schema.json",
                  # The traces and expected logs of the corpus's other suites,
                  # which `ACorpusSuiteReplaysToItsExpectedLogsTest` replays and
                  # reads at run time.
                  "sce-build/tests/fixtures/test_profile/runs/**"],
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
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    "w3c-python": {
        "workflows": ["w3c-tests.yml"],
        "runner_workflow": True,
        "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
                   "the Python backend, the Python templates and the "
                   "generator.",
        # The shared Lua the engine loads, and the shared tables its readers
        # ask. The installer of the repository's Python environment, whose
        # revision names the interpreter the suite runs on wherever that
        # environment was built (`sce_gate_python` in scripts/gates/lib.sh).
        "extra": ["backends/python/**", "tools/codegen/templates/**",
                  "sce-build/src/**",
                  "sce/include/scripting/*.lua", "tests/ecmascript/**",
                  "scripts/install_sce_python.sh",
                  # The test log every runtime writes of the generated suite,
                  # which `test_a_test_suite_runs_through_its_runner.py` reads
                  # at run time.
                  "sce-build/tests/fixtures/test_profile/runtime/expected_logs/**",
                  # The suite's committed traces, which the same test replays,
                  # read at run time.
                  "sce-build/tests/fixtures/test_profile/runtime/traces/**",
                  # The trace's wire schema, whose status
                  # `test_a_recorded_trace_is_read_and_replayed.py` holds the
                  # runtime's to, read at run time.
                  "schemas/sce-test-trace.v1.schema.json",
                  # The documents of the corpus's boundary_suite, which the
                  # gate's regeneration writes the modules of (they are not
                  # committed), and the traces and expected logs of the
                  # corpus's other suites, which
                  # `test_a_corpus_suite_replays_to_its_expected_logs.py`
                  # replays and reads at run time.
                  "sce-build/tests/fixtures/test_profile/boundary/**",
                  "sce-build/tests/fixtures/test_profile/runs/**",
                  # The documents of job_suite the same regeneration writes,
                  # and the job's completion of the kind examples its port
                  # sends.
                  "sce-build/tests/fixtures/test_profile/*.scxml",
                  "sce-build/kind-examples/event-schema.scxml",
                  # The documents of orders_suite and nested_suite, the records
                  # and templates over a union they import, and the unions.
                  "sce-build/tests/fixtures/ttcn3_export/**",
                  "sce-build/tests/fixtures/test_profile/imports/**",
                  "sce-build/tests/fixtures/union/**"],
        "deps": ["codegen-build"],
        "ci_only": "72s. w3c-tests.yml is unfiltered, so CI runs this arm on "
                   "every push.",
        "cost_s": 72,
        "summary": "W3C conformance, Python AOT",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
        "parallelism": PARALLELISM_SCRIPT,
    },
    # The wrapper layer, not the engine under it: the trigger is the binding
    # sources, because `w3c-cpp` already judges the same interpreter 404 cases
    # at a time. This carried no `cost_s` for as long as no machine with the
    # Python development headers had run it, which put it last by default.
    # Measured 2026-08-12 on a machine that has them: it builds the pybind11
    # extension and runs all 202 fixtures through it in 47s.
    "w3c-python-bindings": {
        "workflows": ["w3c-tests.yml"],
        # The extension links the engine, so it configures and builds the
        # main tree (tests and examples off).
        "builds": ["main-tree"],
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
        "cache": True,
        "kept_only_from_a_landing": "it skips and exits 0 on a machine without "
                                "python3-config (python3-dev)",
        "packages": ["python3-dev"],
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The W3C runner built with emscripten and run under Node, and the
    # interactive suite beside it. Nothing ran that route from 2025-11-16 until
    # this gate, and it rotted: seven defects, each invisible to every native
    # build, repaired in b48c4f8bc1..48f14e6e6c.
    #
    # ⚠ `cost_s` is a SUM of three readings taken through `bx --local` on
    # 2026-11-04 with three build slots, before the script existed: an
    # incremental relink of both programs (60s), the W3C run (49s) and the
    # interactive run (4s). The script itself has not been timed, so the slug
    # is absent from COST_MEASURED. A cold configure and build of the tree was
    # about 600s wall (some 3000 cpu-s, the generator's build included) on the
    # same day. Its `peak_gb` was read off that cold build.
    #
    # Since 2026-11-05 it runs both programs in BOTH emscripten trees, the
    # threaded engine's and the host-driven engine's (the browser's), so a run
    # does that work twice and `cost_s` is twice the sum above: still a sum,
    # still not a timing of the script.
    "w3c-wasm": {
        "workflows": ["w3c-tests.yml"],
        "builds": ["main-tree"],
        "runner_workflow": True,
        "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
                   "what the native runner reads, plus the CMake that "
                   "configures the two emscripten trees and the Node host the "
                   "programs are linked with (tests/w3c/wasm_polyfill.js).",
        # What the two programs are built from and run against: the engine,
        # the runner and its Node host (tests/w3c/), the mock the runner
        # links, the CMake that configures the tree and registers the
        # fixtures (the root list carries the WASM link settings), the
        # generator and its templates, and the documents. And the emsdk pin
        # the build machines install it with (`scripts/install_emsdk.sh`).
        "extra": ["scripts/install_emsdk.sh",
                  "sce/**", "tests/w3c/**", "tests/mocks/**",
                  "tests/CMakeLists.txt", "CMakeLists.txt", "cmake/**",
                  "tools/codegen/templates/**", "sce-build/src/**",
                  "resources/**"],
        "deps": ["codegen-build"],
        "ci_only": "226s warm for the two trees, summed from three readings "
                   "of one tree rather than timed as a script, and about "
                   "1200s cold. No other gate a push runs builds both "
                   "emscripten trees, so the cold figure is the state a "
                   "trigger usually finds them in, and it needs emsdk, which "
                   "a push host is not required to carry. w3c-tests.yml "
                   "declares no `paths:` filter, so CI runs this arm on every "
                   "push, in a job of its own (`test-cpp-wasm`).",
        "cost_s": 226,
        "summary": "W3C conformance and the interactive suite, C++ "
                   "Interpreter + AOT built with emscripten, under Node, on "
                   "the threaded and the host-driven engine",
        # The SDK and the Node it ships are bx.toml's pinned emsdk tool.
        "cache": True,
        "kept_only_from_a_landing": "it skips and exits 0 on a machine without "
                                "emsdk (`[platform.linux.tools.emsdk]`)",
        "inputs": None,
        "commits": [],
        "peak_gb": 5.3,
        "parallelism": PARALLELISM_COMPILE,
    },
    # The emscripten tree, all of it: every target built, every registered
    # ctest case run under Node. Measured 2026-11-05: the tree declared 388
    # programs and the two gates above built three of them, and
    # `integration_tests` could not build there at all.
    #
    # ⚠ `cost_s` is HALF a reading. The first run built the tree cold in 3328s
    # on a hosted runner and failed seven links before ctest started, so the
    # Node half is not in the figure; the arrival estimate it replaces, 2400s
    # scaled from the `w3c-wasm` lane, was low by the build alone. The first
    # green run reads the whole.
    "cpp-suite-wasm": {
        "workflows": ["cpp-suite.yml"],
        "builds": ["main-tree"],
        "runner_workflow": True,
        "narrows": "the same catch-all reading as `cpp-suite`, whose "
                   "WebAssembly half this is: it builds the whole emscripten "
                   "tree, so its inputs are the engine, its tests, the CMake "
                   "that registers them (the root list carries the WASM "
                   "settings and says what the tree does not declare), the "
                   "generator and its templates, and the documents.",
        # The emsdk pin the build machines install for it comes first.
        "extra": ["scripts/install_emsdk.sh",
                  "sce/**", "tests/**", "cmake/**", "CMakeLists.txt",
                  "resources/**", "integration_resources/**",
                  "tools/codegen/templates/**", "sce-build/src/**"],
        "deps": ["codegen-build"],
        "ci_only": "the whole emscripten tree, each suite linked with "
                   "ASYNCIFY, and every case run under Node. Its first run "
                   "(cpp-suite.yml run 41956988917, 2026-11-05) configured "
                   "and built the tree cold in 3328s and stopped at seven "
                   "links, so the figure is that build alone and the ctest "
                   "half has no reading yet. It needs emsdk, which a push "
                   "host is not required to carry. cpp-suite.yml runs it in "
                   "a job of its own beside the native suite.",
        "cost_s": 3328,
        "summary": "C++ ctest suite built with emscripten, every case under Node",
        # The SDK and the Node it ships are bx.toml's pinned emsdk tool.
        "cache": True,
        "kept_only_from_a_landing": "it skips and exits 0 on a machine without "
                                "emsdk (`[platform.linux.tools.emsdk]`)",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
        "parallelism": PARALLELISM_COMPILE,
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
    # A publication, not a verification: it renders what a change does to the
    # generated code (ADR 0097, section 6) and judges nothing a push could
    # reproduce. The same diff is one `scripts/generated_diff.sh` away.
    "generated-diff.yml": "publication: scripts/generated_diff.sh renders it",
    # A publication too: on a release tag it packs the consumer packages with
    # their generated trees and provenance and pushes the release commit's
    # tags (ADR 0097, section 7). Its declaration is read by
    # `scripts/release.py --check`, which the workspace suite runs.
    "release.yml": "publication: scripts/release.py makes the release",
}


# Lines that report rather than run. A hook's `log_step "Stage 0/4 — cargo fmt
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
    # The script-engine census ledger, and the same shape as the entry above:
    # read by a gate, and inert because that gate runs for every change. Both
    # halves were checked on 2026-09-16 rather than assumed:
    #   `scripts/gates/tree-hygiene.sh` names `--test script_engine_census`,
    #     and `workflow_trigger_coverage` holds it to that.
    #   `.github/workflows/tree-hygiene.yml` declares no `paths:` filter,
    #     so CI offers it every push.
    # Editing the ledger therefore cannot escape the test that enforces it.
    # Without this entry the path is one directory below the `*.md` fnmatch
    # above, so editing the ledger alone took the Rule 1 branch and bought
    # the full run.
    "docs/SCE_SCRIPT_ENGINE_CENSUS.md",
    # The programme the census above orders. Inert on the strongest reading
    # of that claim: NOTHING reads this one. It is deliberately ungated —
    # the census test ratchets the counts through the ledger above, and no
    # gate reads the programme document itself — so "drives no path-scoped
    # gate" is literally true here rather than true-by-Rule-3. Same fnmatch
    # reason as the entry above: one directory below the `*.md` pattern, so
    # without a named entry editing it alone buys the full run.
    "docs/SCE_SCRIPT_ENGINE_PROGRAMME.md",
    # The NL->IR closure ledger, and the same shape as the entry above: read
    # by a gate, and inert for the reason at the top of this list. Both
    # halves were checked on 2026-09-14 rather than assumed, because the
    # claim is only true while both hold:
    #   python3 tools/git-hooks/gate_registry.py --changed README.md --explain
    #     -> `nl-ir-closure` is selected for a path that is already INERT,
    #        so it is a Rule 3 gate and runs for every change.
    #   .github/workflows/nl-ir-closure.yml declares no `paths:` filter,
    #        so CI offers it every push.
    # Editing the ledger therefore cannot escape the gate that measures it.
    #
    # ⚠ What this entry does NOT do is narrow `nl-ir-closure` itself. That
    # gate must stay Rule 3: it measures fourteen rows against source spread
    # over `sce-build/src/`, the C templates, `tests/`, `resources/` and the
    # visualizer, so a change that CLOSES a row without touching the ledger
    # is exactly the case that makes the ledger a stale denominator. A
    # `local` selector naming the ledger would miss every one of them.
    #
    # Before this entry the path was unclassified, so editing it took the
    # Rule 1 branch: 12 local gates and 15 lanes printed under `--ci-unowed`,
    # none of which any change to a markdown ledger can affect. That is the
    # cost the comment above prices for the other `docs/` documents, paid on
    # the one file this repository's closure work edits every round.
    "docs/SCE_NL_IR_CLOSURE.md",
    # The other documents directly under `docs/`, by the same claim and one
    # line each. Checked 2026-11-22: `git grep` for each name over
    # `sce-build/`, `scripts/`, `tools/`, `.github/`, `backends/`, `tests/` and
    # `cmake/`, every hit that is not a comment or a docstring read by hand.
    # What names one is prose in a message, a Cargo description, a schema's
    # description, the list of an evaluation tool no gate runs
    # (`tools/authoring/eval/divide_draft.py`), or a path its own test writes
    # into a temporary repository (`a_consumer_visible_change_carries_its_
    # migration_section` for the migration guide, whose content is judged by
    # `consumer-guide-gate.sh` in the hook, before any gate is chosen). What
    # sweeps every tracked file (`tree-hygiene`) runs for every change, and the
    # citation sweep of `ledger-citations` is the one it already is for
    # `docs/adr/**`. Two documents of this kind have a reader that does not run
    # for every change and are not here: `SCE_LUA_TRANSLATION_SEAM.md`
    # (`kotlin_lowering_dividend`) and `SCE_TTCN3_LANE.md`
    # (`the_corpus_covers_every_row_and_refusal_of_adr_0091`), both of which
    # `rust-workspace-tests.yml` names, so editing one selects
    # `workspace-tests`.
    #
    # ⚠ What leaving them unclassified costs since the landing queue runs the
    # `ci_only` gates: every one of them bought the whole table. The migration
    # guide alone was edited by 198 commits of main in the month before
    # 2026-11-22, and each landing of one would have held the queue for the
    # whole table. What is under `docs/formal/` is not here: `formal-models`
    # and `formal-proofs` read it.
    "docs/EMBED_MANIFEST.md",
    "docs/MESH_SCXML_COMPATIBILITY.md",
    "docs/SCE_AUDIT_LOG.md",
    "docs/SCE_AXIS_6_PATTERNS.md",
    "docs/SCE_CODEC_CONSUMER_MIGRATION.md",
    "docs/SCE_CODEGEN_DETERMINISM.md",
    "docs/SCE_FORGE_AST.md",
    "docs/SCE_HOST_FEATURES.md",
    "docs/SCE_MESH_CONFORMANCE_MATRIX.md",
    "docs/SCE_MESH_KEYING_AUDIT.md",
    "docs/SCE_RELAY_DEPLOY.md",
    "docs/SCE_SEALED_RECORD.md",
    "docs/SCE_SERVER_DEPLOY.md",
    "docs/SCE_SHARED_LIST_MLS.md",
    "docs/SCE_SHARED_LIST_PEER_ABI.md",
    "docs/SCE_SHARED_LIST_RELAY.md",
    "docs/SCE_SHARED_LIST_REVIEW_BRIEF.md",
    "docs/SCE_SHARED_LIST_TRUST.md",
    "docs/SCE_TEST_PROFILE.md",
    "docs/SCE_TEST_PROFILE_MIGRATION.md",
    "docs/SCE_TRANSITION_FUNCTION.md",
    "docs/SCE_TRANSITION_TABLE.md",
    "docs/SCE_W3C_TEST_REGISTRATION.md",
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
    # The generator of the print diagram's glyph-advance table, inert on the
    # strongest reading of the claim: NOTHING reads it. Checked 2026-10-03
    # rather than assumed — a search of the tree for its name finds the two
    # `@generated` comments it leaves in `sce-build/src/diagram/` and its own
    # usage line, and no gate script, workflow or test opens it. What the
    # build and the tests consume is the table it WRITES
    # (`sce-build/src/diagram/metrics_table.rs`, path-scoped like every
    # source under that crate and pinned to one font revision by
    # `the_table_is_pinned_to_one_font_revision`), so an edit to the
    # generator that is not followed by a regeneration changes no behaviour
    # a gate could see.
    #
    # ⚠ What this entry does NOT claim is that the pair stays in step: the
    # generator needs the font file, which no lane has, so "the table is what
    # this script writes" is held by whoever regenerates it, not by a gate.
    # Before the entry the path was unclassified and an edit to it took the
    # Rule 1 branch, buying every gate for a script none of them runs.
    "tools/diagram/gen_font_metrics.py",
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


def tracked_paths(repo_root: Path, *pathspecs: str) -> list[str]:
    """Every path git tracks that matches one of `pathspecs` — every tracked
    path when there are none — repository-relative, as git stores it.

    `-z`, always. Without it git's default `core.quotePath` writes a path
    holding any byte outside printable ASCII as a quoted octal escape,
    which names no file on disk, and a caller splitting on whitespace also
    cuts a path with a space in two — either way the file is skipped in
    silence. A git that fails is an error, not an empty tree: an empty list
    here reads as "nothing to judge" to every caller.
    """
    proc = subprocess.run(
        ["git", "-C", str(repo_root), "ls-" + "files", "-z", "--", *pathspecs],
        capture_output=True, check=True,
    )
    return [p.decode("utf-8", "surrogateescape") for p in proc.stdout.split(b"\0") if p]


def tracked_matching(repo_root: Path, glob: str) -> list[str]:
    """Tracked paths a sweep glob actually yields, asked of git itself.

    Comparing the sweep glob to a trigger glob as text would need a
    containment rule between two glob dialects. The tree answers the
    question directly and without a rule to get wrong.
    """
    return tracked_paths(repo_root, glob)


def script_read_paths(body: str, tracked: set) -> tuple[set, set]:
    """The tracked paths a gate script NAMES AS A VALUE, by how it names them.

    Returned as two sets — quoted values and bare words — because they are
    two arms of one scan and either can be blinded on its own. A caller
    that unions them without measuring both would keep reporting a healthy
    count off whichever arm still works.

    A path a script hands to a command or binds to a variable is an input:
    editing it changes what the gate answers. A path spelled inside a
    SENTENCE is the script explaining itself, and the two look identical to
    a scan that only looks for the characters.

    Both readings are live here. Measured 2026-09-03 across the 35 gate
    scripts: `w3c-kotlin` names `docs/SCE_LUA_TRANSLATION_SEAM.md` only in
    the failure message that tells a reader where to argue an entry, and
    `mutation-rounds` names `scripts/prepare_ctest_tree.sh` only in the
    sentence "Prepare one with ..." — while the same file invokes
    `scripts/mutate` twice for real. Declaring the first two as inputs
    would put a document nothing opens into a lane's triggers; missing the
    third leaves an edit to the harness judged by no lane at all.

    The discriminator is the quoted region: a value is quoted alone
    (`X="path"`) or not quoted at all, while prose is a quoted region that
    merely CONTAINS the path among other words. That is the same reading
    `common::rust_source::code_only` makes for Rust — a signature in a
    comment is a file explaining an input, not reading one — arriving here
    through failure messages instead of comments.
    """
    quoted: set = set()
    bare: set = set()
    outside: list[str] = []
    i, n = 0, len(body)
    while i < n:
        if body[i] == '"':
            j = i + 1
            while j < n and body[j] != '"':
                j += 2 if body[j] == "\\" else 1
            region = body[i + 1:j]
            if region in tracked:
                quoted.add(region)
            i = j + 1
            continue
        outside.append(body[i])
        i += 1
    for word in re.findall(r"[A-Za-z0-9_./-]+", "".join(outside)):
        if word in tracked:
            bare.add(word)
    return quoted, bare


def script_code(path: Path) -> str:
    """A gate script with its whole-line comments removed."""
    return "\n".join(
        line
        for line in path.read_text(encoding="utf-8").splitlines()
        if not line.lstrip().startswith("#")
    )


def enrolled_globs(repo_root: Path, configs: str):
    """The trees the ledgers matching `configs` enrol, as trigger globs.

    A ledger's `mnemosyne.toml` lists, under
    `[plugins.set_equality_validator]`, the paths whose citations it judges.
    They are read with the reader the citation gate itself uses
    (`paths_from_toml` in tools/mnemosyne-adoption/migrate_citations.py), so
    the two cannot parse the list differently. A directory becomes
    `<dir>/**` and a file stays itself; each ledger's own directory joins as
    well, since its config decides the set and its store is what the
    citations are judged against.

    `None` when the set cannot be read: no config matches, the reader cannot
    be loaded, or a config has no such list. The caller reads that as the
    catch-all, the safe direction, the same answer `workflow_paths` gives for
    a workflow it cannot find.

    Read once per process and root: `select` derives the triggers for every
    call, and the self-test calls it dozens of times.
    """
    key = (str(repo_root), configs)
    if key not in _ENROLLED:
        _ENROLLED[key] = _read_enrolled(repo_root, configs)
    cached = _ENROLLED[key]
    return None if cached is None else list(cached)


_ENROLLED: dict[tuple[str, str], list[str] | None] = {}


def _read_enrolled(repo_root: Path, configs: str):
    """`enrolled_globs` without the cache."""
    found = sorted(repo_root.glob(configs))
    reader = repo_root / "tools" / "mnemosyne-adoption" / "migrate_citations.py"
    if not found or not reader.is_file():
        return None
    try:
        spec = importlib.util.spec_from_file_location(
            "_sce_enrolment_reader", reader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        globs: list[str] = []
        for config in found:
            # The ledger's own directory: the config, its store and the
            # document it is made from, which the ledger's tooling reads.
            globs.append(f"{config.parent.relative_to(repo_root).as_posix()}/**")
            for absolute in module.paths_from_toml(str(config)):
                rel = Path(absolute).relative_to(module.REPO_ROOT).as_posix()
                globs.append(f"{rel}/**" if (repo_root / rel).is_dir()
                             else rel)
    except (Exception, SystemExit):
        return None
    return list(dict.fromkeys(globs))


# ── What a build reads: `builds` ──────────────────────────────────
#
# Eleven gates configure and build the main CMake tree, and every one of them
# compiles what the tree compiles whatever else it does: the engine, its
# runtimes, the submodules, and, since the ECMAScript frontend is linked into
# `sce_scripting` (sce/CMakeLists.txt, `sce_link_ecmascript_frontend`), the
# `sce-build` crate through cargo. Each gate used to write its own share of
# that into `extra`, and the shares had drifted: none named the root
# `Cargo.toml` or `Cargo.lock`, `w3c-python-bindings` and `visualizer-wasm`
# named no `sce/` at all (2026-11-16), so an edit to what they compile neither
# selected them nor moved the key of a unit kept for them.
#
# So what a shared build reads is written once, here, and a gate names it in
# `builds`; `gate_triggers` adds it to the gate's trigger, which is also the
# unit's input set (`unit_inputs`). The cargo half is DERIVED, from the files
# cargo and the crate itself declare, so a new path dependency, a new member
# or a new compiled-in file joins every such gate on the commit that adds it:
#   * the workspace: the root `Cargo.toml`, `Cargo.lock`, and every member's
#     manifest (`cargo_workspace_manifests`), which cargo reads to load the
#     workspace; a build from a crate outside it reads the root's and those
#     of the crates it builds;
#   * `.cargo/`, whose configuration every cargo run reads;
#   * each crate the build compiles: the crate named and its path
#     dependencies (`[dependencies]`, `[build-dependencies]` and their target
#     forms, never `[dev-dependencies]`), each as its `src/**`, its build
#     script and the roots its manifest declares (`cargo_build_targets`).
#     Not the whole directory: `tests/`, `benches/` and `examples/` are
#     compiled by `cargo test` or `--all-targets`, which a gate runs as its
#     own command and names its own trigger for. Taken whole, one integration
#     test of `sce-build` selected 39 gates of a landing (2026-11-24);
#   * every file a compiled module names by path, found by walking the module
#     tree as rustc does (`rust_module_closure`): a `#[path]` module and the
#     directory it owns, and each `include_str!` / `include_bytes!` /
#     `include!` target. A module behind `#[cfg(test)]` is not walked;
#   * what `sce-build`'s build script reads: the generator witness (its
#     WITNESS_FILES, WITNESS_TREES and RUN_TIME_TREE, in
#     sce-build/src/generator_witness.rs, which the script digests and embeds);
# `MAIN_TREE_CONFIGURE` is the CMake half: what the root CMakeLists.txt
# configures for every tree. A subdirectory a gate turns on with an option of
# its own (`tests/`, the examples, the Python bindings) stays in its `extra`.
#
# `every-tree-builder-names-what-it-builds` in the self-test finds the gates
# whose script configures the root tree and refuses one that does not name
# `main-tree`, and `ci-only-coverage` holds a filtered workflow's `paths:` to
# what its gate builds.
MAIN_TREE_CONFIGURE = (
    "CMakeLists.txt", "cmake/**", "sce/**", "third_party/**",
    "backends/cpp/forge-runtime/**", "backends/c/runtime/**",
    "tools/txml_converter/**",
)
# Each shared build: the CMake globs it configures, and the crates cargo
# builds for it.
# The third field says whether cargo builds the crates as members of this
# workspace (`cargo build -p`), which loads every member's manifest, or from a
# crate of the gate's own that names them as path dependencies, which reads
# their manifests and the root's (the fields they inherit) and no other.
BUILT_INPUTS: dict[str, tuple[tuple[str, ...], tuple[str, ...], bool]] = {
    "sce-codegen": ((), ("sce-build",), True),
    "main-tree": (MAIN_TREE_CONFIGURE, ("sce-build",), True),
    "codegen-wasm": ((), ("sce-build-wasm",), True),
    # The Rust forge runtime a generated Rust codec compiles against, built
    # by the codec gates in a crate of their own (scripts/check_*_codec.sh).
    # `bx --checks prepush --learn codec-clippy` (2026-11-24) read the root
    # manifest and the runtime's, and no other member's.
    "forge-runtime-rust": ((), ("backends/rust/forge-runtime",), False),
}
# The file whose constants name what `sce-build`'s build script reads.
GENERATOR_WITNESS = "sce-build/src/generator_witness.rs"

_CARGO_DEP_TABLE = re.compile(
    r"^\[(?:target\.[^\]]+\.)?(?:dependencies|build-dependencies)"
    r"(?:\.[A-Za-z0-9_-]+)?\]\s*$")
_CARGO_PATH_VALUE = re.compile(r'\bpath\s*=\s*"([^"]+)"')
# A compiled-in file (`include_str!`, `include_bytes!`, `include!`) and a
# module whose file a `#[path]` names, each by a literal path relative to the
# source file that names it.
_RUST_INCLUDE = re.compile(r'include(?:_str|_bytes)?!\(\s*"([^"]+)"\s*\)')
_RUST_PATH_ATTR = re.compile(r'^\s*#\[path\s*=\s*"([^"]+)"\s*\]')
_RUST_MOD_DECL = re.compile(
    r'^(\s*)(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;')
_RUST_TEST_ONLY = re.compile(r'^\s*#\[cfg\(test\)\]\s*$')
_RUST_ATTRIBUTE = re.compile(r'^\s*(?:#\[|///|//!)')
_CARGO_TABLE = re.compile(r'^\[\[?\s*([A-Za-z0-9_.-]+)\s*\]\]?\s*$')
_CARGO_STRING_VALUE = re.compile(r'^\s*(path|build)\s*=\s*"([^"]+)"')
_CARGO_BUILD_OFF = re.compile(r'^\s*build\s*=\s*false\b')


def cargo_path_deps(repo_root: Path, crate: str) -> list[str]:
    """The directories of the path dependencies `crate/Cargo.toml` builds.

    Read by its lines, as the registry reads every TOML file (see
    `fleet_platforms`): a table header opens a dependency table or closes
    one, and a `path = "..."` inside one names a directory relative to the
    crate. `[dev-dependencies]` is not a dependency table here: a build of
    the crate compiles none of it.
    """
    try:
        text = (repo_root / crate / "Cargo.toml").read_text(encoding="utf-8")
    except OSError:
        return []
    found: list[str] = []
    in_deps = False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            in_deps = bool(_CARGO_DEP_TABLE.match(stripped))
            continue
        if in_deps and not stripped.startswith("#"):
            found.extend(posixpath.normpath(f"{crate}/{rel}")
                         for rel in _CARGO_PATH_VALUE.findall(stripped))
    return found


def generator_witness_globs(repo_root: Path) -> list[str]:
    """What `sce-build`'s build script reads: the generator witness.

    `WITNESS_FILES` as paths, `WITNESS_TREES` and `RUN_TIME_TREE` as trees,
    read out of GENERATOR_WITNESS, the one place they are written.
    """
    try:
        text = (repo_root / GENERATOR_WITNESS).read_text(encoding="utf-8")
    except OSError:
        return []
    globs: list[str] = []
    for name, tree in (("WITNESS_FILES", False), ("WITNESS_TREES", True)):
        found = re.search(r"pub const " + name + r": &\[&str\] = &\[(.*?)\];",
                          text, re.S)
        if found:
            globs.extend(f"{v}/**" if tree else v
                         for v in re.findall(r'"([^"]+)"', found.group(1)))
    found = re.search(r'pub const RUN_TIME_TREE: &str = "([^"]+)";', text)
    if found:
        globs.append(f"{found.group(1)}/**")
    return globs


def cargo_build_targets(repo_root: Path, crate: str):
    """`(build script or None, [lib and bin roots it declares])` of `crate`.

    Cargo's own target rules (the Cargo Book, "Target auto-discovery" and
    "Configuring a target"): the build script is `build.rs` unless
    `[package] build` names another file or turns it off, and a `[lib]` or
    `[[bin]]` table may name its root with `path`. The roots cargo finds by
    itself (`src/lib.rs`, `src/main.rs`, `src/bin/`) are not listed here.
    `[[test]]`, `[[bench]]` and `[[example]]` roots are never among these:
    `cargo build` compiles none of them, and `cargo test` or `--all-targets`
    is a gate's own command. Read by its lines, as the registry reads every
    TOML file. Paths are from the repository root.
    """
    try:
        text = (repo_root / crate / "Cargo.toml").read_text(encoding="utf-8")
    except OSError:
        return None, []
    table = ""
    script = "build.rs"
    declared: list[str] = []
    for line in text.splitlines():
        header = _CARGO_TABLE.match(line.strip())
        if header:
            table = header.group(1)
            continue
        if table == "package" and _CARGO_BUILD_OFF.match(line):
            script = None
            continue
        value = _CARGO_STRING_VALUE.match(line)
        if not value:
            continue
        key, rel = value.groups()
        if table == "package" and key == "build":
            script = rel
        elif table in ("lib", "bin") and key == "path":
            declared.append(posixpath.normpath(f"{crate}/{rel}"))
    if script is not None:
        script = posixpath.normpath(f"{crate}/{script}")
        if not (repo_root / script).is_file():
            script = None
    return script, declared


def rust_module_closure(repo_root: Path, roots) -> tuple[list[str], list[str]]:
    """What a build that is not a test compiles from `roots`, by path.

    Walks the module tree the way rustc resolves it (the Rust Reference,
    "Modules"): `mod x;` in a crate root or a `mod.rs` is `x.rs` or
    `x/mod.rs` beside it, in any other file `f.rs` it is under `f/`, and a
    `#[path]` names the file relative to the directory of the file that
    declares it. A module whose attributes say `#[cfg(test)]` is not walked:
    a build that is not a test compiles none of it. Every file walked is read
    for the literal paths of `include_str!`, `include_bytes!` and `include!`,
    which resolve against that file's directory. A declaration that is
    indented sits in an inline module, whose files this does not place, so
    the walk takes every source below the directory instead.

    Returns the files named by path (`#[path]` and include targets, which
    may lie outside `src/`) and the directories a `#[path]` module owns.
    """
    root = repo_root.resolve()
    named: list[str] = []
    trees: list[str] = []
    pending = [(r, [posixpath.dirname(r)]) for r in roots]
    seen: set[str] = set()

    def resolved(base, rel):
        try:
            return (repo_root / base).joinpath(rel).resolve() \
                .relative_to(root).as_posix()
        except ValueError:
            return None

    def sources_below(folder):
        return [p for p in tracked_paths(repo_root, folder) if p.endswith(".rs")]

    while pending:
        source, owns = pending.pop()
        if source in seen:
            continue
        seen.add(source)
        try:
            text = (repo_root / source).read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        here = posixpath.dirname(source)
        # Over the whole text: rustfmt breaks a long call over three lines.
        for rel in _RUST_INCLUDE.findall(text):
            if (path := resolved(here, rel)) is not None:
                named.append(path)
                if path.endswith(".rs"):
                    pending.append((path, owns))
        lines = text.splitlines()
        for i, line in enumerate(lines):
            decl = _RUST_MOD_DECL.match(line)
            if not decl:
                continue
            indent, name = decl.groups()
            above, test_only, path_attr = i - 1, False, None
            while above >= 0 and _RUST_ATTRIBUTE.match(lines[above]):
                test_only |= bool(_RUST_TEST_ONLY.match(lines[above]))
                if (attr := _RUST_PATH_ATTR.match(lines[above])):
                    path_attr = attr.group(1)
                above -= 1
            if test_only:
                continue
            if indent:
                for folder in owns:
                    pending.extend((p, [p[:-3]]) for p in sources_below(folder))
                continue
            if path_attr is not None:
                if (path := resolved(here, path_attr)) is None:
                    continue
                folder = posixpath.dirname(path)
                named.append(path)
                trees.append(f"{folder}/**")
                pending.append((path, [folder, path[:-3]]))
                pending.extend((p, [p[:-3]]) for p in sources_below(folder))
                continue
            for folder in owns:
                flat = f"{folder}/{name}.rs"
                nested = f"{folder}/{name}/mod.rs"
                if (repo_root / flat).is_file():
                    pending.append((flat, [f"{folder}/{name}"]))
                elif (repo_root / nested).is_file():
                    pending.append((nested, [f"{folder}/{name}"]))
    return named, trees


_CARGO_MEMBERS = re.compile(r"^\s*members\s*=\s*\[(.*?)\]", re.S | re.M)


def cargo_workspace_manifests(repo_root: Path) -> list[str]:
    """The manifests cargo reads to load this workspace: the root's and each
    member's (`[workspace] members`, its globs expanded), sorted.

    A build of any member loads them all, and a crate outside the workspace
    that names a member as a path dependency reads the member's and, for the
    fields the member inherits, the root's. Named one by one rather than as
    `**/Cargo.toml`, which also names the manifests of crates no build here
    loads (a probe with a workspace of its own, a submodule's)."""
    root = repo_root / "Cargo.toml"
    try:
        text = root.read_text(encoding="utf-8")
    except OSError:
        return []
    section = text.split("[workspace]", 1)[1] if "[workspace]" in text else ""
    section = re.split(r"^\[(?!workspace\])", section, maxsplit=1, flags=re.M)[0]
    found = _CARGO_MEMBERS.search(section)
    members = re.findall(r'"([^"]+)"', re.sub(r"#[^\n]*", "", found.group(1))) \
        if found else []
    manifests = ["Cargo.toml"]
    for member in members:
        for path in sorted(repo_root.glob(member)) if "*" in member \
                else [repo_root / member]:
            if (path / "Cargo.toml").is_file():
                manifests.append(f"{path.relative_to(repo_root)}/Cargo.toml")
    return sorted(dict.fromkeys(manifests))


def cargo_build_closure(repo_root: Path, crates) -> list[str]:
    """`crates` and the path dependencies a build of them compiles
    (`cargo_path_deps`), transitively, sorted."""
    closure: list[str] = []
    pending = list(crates)
    while pending:
        crate = pending.pop()
        if crate in closure:
            continue
        closure.append(crate)
        pending.extend(cargo_path_deps(repo_root, crate))
    return sorted(closure)


def cargo_build_globs(repo_root: Path, crates, in_workspace=True) -> list[str]:
    """What a cargo build of `crates` reads, as globs: as members of this
    workspace (`in_workspace`), or as path dependencies of a crate outside it.

    See "What a build reads". Exact paths a directory glob already covers
    are left out, so each file is named once.
    """
    closure = cargo_build_closure(repo_root, crates)
    globs = [".cargo/**"] + (["Cargo.lock"] if in_workspace else [])
    named: list[str] = (cargo_workspace_manifests(repo_root) if in_workspace
                        else ["Cargo.toml"]) + [
        f"{crate}/Cargo.toml" for crate in sorted(closure)]
    if "sce-build" in closure:
        named.extend(generator_witness_globs(repo_root))
    trees: list[str] = []
    roots: list[str] = []
    for crate in sorted(closure):
        trees.append(f"{crate}/src/**")
        script, declared = cargo_build_targets(repo_root, crate)
        declared = declared + ([script] if script else [])
        named.extend(declared)
        roots.extend(declared)
        roots.extend(p for p in ([f"{crate}/src/lib.rs", f"{crate}/src/main.rs"]
                                 + tracked_paths(repo_root, f"{crate}/src/bin"))
                     if p.endswith(".rs") and p not in declared
                     and (repo_root / p).is_file())
    reached, owned = rust_module_closure(repo_root, roots)
    named.extend(reached)
    trees.extend(owned)
    for tree in dict.fromkeys(trees):
        # A tree beneath another one adds nothing to it.
        if not any(other != tree and glob_to_regex(other).match(tree[:-3])
                   for other in trees):
            globs.append(tree)
    wide = [g for g in globs + named if "*" in g]
    covered = [glob_to_regex(g) for g in wide]
    out = list(globs)
    for g in named:
        if "*" in g or not any(p.match(g) for p in covered):
            out.append(g)
    return list(dict.fromkeys(out))


_BUILT_CACHE: dict[tuple[str, str], list[str]] = {}


def built_inputs(repo_root: Path, name: str) -> list[str]:
    """The globs a gate that `builds` `name` reads for it (`BUILT_INPUTS`)."""
    key = (str(repo_root.resolve()), name)
    if key not in _BUILT_CACHE:
        configure, crates, in_workspace = BUILT_INPUTS[name]
        joined = list(dict.fromkeys(
            list(configure) + (cargo_build_globs(repo_root, crates, in_workspace)
                               if crates else [])))
        # An exact path a tree of the other half already covers is named once.
        trees = [glob_to_regex(g) for g in joined if "*" in g]
        _BUILT_CACHE[key] = [g for g in joined
                             if "*" in g or not any(p.match(g) for p in trees)]
    return list(_BUILT_CACHE[key])


# ── What a build reads that does not select it ─────────────────────
#
# A gate's trigger is what can change its verdict; its key has to hold every
# file its run looks at, or bx's trace calls the run "could not measure" (bx
# RFC 0020, 5.2). For a cargo build the two differ, and the difference is
# cargo's own, so it is derived here from the manifests as the trigger half is
# and joins the KEY of a gate that `builds` (`gate_reads`), never its
# trigger:
#   * loading the workspace (the Cargo Book, "Target auto-discovery"): every
#     package the resolver loads, which is each member and each package a
#     member names by `path` in any dependency table, dev-dependencies
#     included (`cargo_loaded_packages`). Of each, cargo reads the manifest,
#     looks for `src/lib.rs` and `src/main.rs`, looks for `build.rs` when
#     `[package] build` does not name the script, and lists `src/bin/`,
#     `tests/`, `examples/` and `benches/` where they exist, with each
#     `<dir>/<name>/main.rs` a listed directory holds
#     (`cargo_package_load_reads`). It compiles none of that unless it builds
#     the package, so an edit there changes no verdict of these gates and
#     selects none of them;
#   * a build script cargo has not run in that target directory before: no
#     earlier run has told cargo what the script reads (`rerun-if-changed`),
#     and a script that has told it nothing is fingerprinted by every file of
#     its package (the Cargo Book, "Build Scripts", "Change Detection"), so a
#     crate the build compiles that has a build script is read whole
#     (`<crate>/**`).
# Measured 2026-11-24 with `bx --checks prepush --learn` (bx 340e9f9e):
# `forge-c11` and `forge-cpp` read every member's manifest and target roots,
# their `tests/` and `examples/` listings, and every file of `sce-build`, of
# which the triggers named only what the generator compiles.
_CARGO_ANY_DEP_TABLE = re.compile(
    r"^\[(?:target\.[^\]]+\.)?(?:dependencies|build-dependencies|dev-dependencies)"
    r"(?:\.[A-Za-z0-9_-]+)?\]\s*$")
# The directories cargo lists in a package it loads, for the targets its
# manifest does not name.
CARGO_DISCOVERED_DIRS = ("src/bin", "tests", "examples", "benches")


def cargo_loaded_packages(repo_root: Path) -> list[str]:
    """The packages cargo loads to resolve this workspace, sorted: each member
    (`cargo_workspace_manifests`) and each package one names by `path` in any
    dependency table, `[dev-dependencies]` included, transitively. Read by
    lines, as `cargo_path_deps` reads a manifest; a path outside the
    repository names no tracked file and is left out."""
    members = [m[:-len("/Cargo.toml")] for m in cargo_workspace_manifests(repo_root)
               if m != "Cargo.toml"]
    seen: list[str] = []
    pending = list(members)
    while pending:
        package = pending.pop()
        if package in seen or package.startswith("../") or package == "..":
            continue
        seen.append(package)
        try:
            text = (repo_root / package / "Cargo.toml").read_text(encoding="utf-8")
        except OSError:
            continue
        in_deps = False
        for line in text.splitlines():
            stripped = line.strip()
            if stripped.startswith("["):
                in_deps = bool(_CARGO_ANY_DEP_TABLE.match(stripped))
                continue
            if in_deps and not stripped.startswith("#"):
                pending.extend(posixpath.normpath(f"{package}/{rel}")
                               for rel in _CARGO_PATH_VALUE.findall(stripped))
    return sorted(seen)


def cargo_package_load_reads(repo_root: Path, package: str) -> list[str]:
    """What cargo reads of `package` to load it, built or not, as input
    patterns: the manifest, the target roots it looks for that are tracked,
    and a listing (`<dir>/`, bx RFC 0020 4.2) of each discovery directory that
    holds a tracked file, with the `<name>/main.rs` of each target directory
    in it. See "What a build reads that does not select it"."""
    try:
        text = (repo_root / package / "Cargo.toml").read_text(encoding="utf-8")
    except OSError:
        return []
    table, script_named = "", False
    for line in text.splitlines():
        header = _CARGO_TABLE.match(line.strip())
        if header:
            table = header.group(1)
        elif table == "package" and re.match(r"^\s*build\s*=", line):
            script_named = True
    tracked = set(tracked_paths(repo_root, package))
    reads = [f"{package}/Cargo.toml"]
    looked_for = ["src/lib.rs", "src/main.rs"] + ([] if script_named else ["build.rs"])
    reads.extend(f"{package}/{rel}" for rel in looked_for
                 if f"{package}/{rel}" in tracked)
    for folder in CARGO_DISCOVERED_DIRS:
        prefix = f"{package}/{folder}/"
        held = sorted(p for p in tracked if p.startswith(prefix))
        if held:
            reads.append(prefix)
            reads.extend(p for p in held
                         if re.fullmatch(r"[^/]+/main\.rs", p[len(prefix):]))
    return reads


_READS_CACHE: dict[tuple[str, str], list[str]] = {}


def built_reads(repo_root: Path, name: str) -> list[str]:
    """What a gate that `builds` `name` reads for it beyond `built_inputs`:
    the workspace's load (`cargo_package_load_reads` of every package cargo
    loads) for a build as members of this workspace, and the whole of each
    crate the build compiles that has a build script. A pattern `built_inputs`
    or a wider one here already gives is left out. Empty for a build cargo does
    not run. See "What a build reads that does not select it"."""
    key = (str(repo_root.resolve()), name)
    if key not in _READS_CACHE:
        _, crates, in_workspace = BUILT_INPUTS[name]
        reads: list[str] = []
        if crates:
            if in_workspace:
                for package in cargo_loaded_packages(repo_root):
                    reads.extend(cargo_package_load_reads(repo_root, package))
            for crate in cargo_build_closure(repo_root, crates):
                if cargo_build_targets(repo_root, crate)[0] is not None:
                    reads.append(f"{crate}/**")
        built = built_inputs(repo_root, name)
        trees = [g for g in built + reads if g.endswith("/**")]
        # A listing is given by a tree that holds the directory's entries,
        # which is a tree that matches a name inside it.
        covering = [(t, glob_to_regex(t)) for t in trees]
        _READS_CACHE[key] = [
            p for p in dict.fromkeys(reads)
            if p not in built and not any(
                t != p and r.match(f"{p}x" if p.endswith("/") else p)
                for t, r in covering)]
    return list(_READS_CACHE[key])


def gate_triggers(repo_root: Path, slugs=None) -> dict[str, list[str]]:
    """Each gate's trigger globs, derived from its workflow and its own lists;
    those of `slugs` alone when it is given.

    A gate's globs are derived from its own entry and nothing else, so asking
    for one gate reads its workflows, its builds and its ledgers and no other
    gate's. The submodule preflight asks for the gate it runs alone, because a
    narrowed unit's copy holds that gate's inputs and no other's: deriving
    every gate's there read every workflow and ledger config of the tree, and
    each read outside the inputs is one bx calls "could not measure"
    (`bx --checks prepush --learn forge-c11`, 2026-11-24).

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
        if slugs is not None and slug not in slugs:
            continue
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
        for built in spec.get("builds", []):
            globs.extend(built_inputs(repo_root, built))
        if spec.get("enrolled_by"):
            # What the ledgers enrol, read from their configs on every call:
            # a tree enrolled tomorrow selects the gate on the same commit.
            globs.extend(enrolled_globs(repo_root, spec["enrolled_by"])
                         or ["**"])
        triggers[slug] = list(dict.fromkeys(globs))
    return triggers


def gate_reads(repo_root: Path, slugs=None) -> dict[str, list[str]]:
    """What each gate's run reads that does not select it, sorted: for each
    shared build it names (`builds`), `built_reads`. Those of `slugs` alone
    when it is given. A gate keyed on its triggers is keyed on these too
    (`gate_inputs`); see "What a build reads that does not select it"."""
    return {slug: sorted({p for built in spec.get("builds", [])
                          for p in built_reads(repo_root, built)})
            for slug, spec in GATES.items()
            if slugs is None or slug in slugs}


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


def cost_is_stale(declared, measured, no_op=None) -> bool:
    """Whether a run's timing genuinely disagrees with the declared cost.

    `no_op` is the other half of a CONDITIONAL cost. A gate reached through
    `deps` on a tree whose own sources have not moved may have nothing to
    do — `codegen-build` finds its binary already built and returns in a
    tenth of a second — while the same gate, selected by its own triggers,
    rebuilds. Both figures are true, and which one a run produces is
    decided by the change set rather than by the table.

    Without this the cheap reading is read as drift on every warm push, and
    the only way to quiet that was `PACE_NORMALISED`, which says "do not
    trust the comparison" rather than "here is the other case". Declaring
    the second number lets the comparison stay honest in both.
    """
    if declared is None:
        return True  # an unmeasured gate has just been measured
    if no_op is not None and not cost_is_stale(no_op, measured):
        return False
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
        if cost_is_stale(declared, measured[s], table[s].get("cost_s_no_op")):
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
        if cost_is_stale(declared, at_table_pace,
                         table[slug].get("cost_s_no_op")):
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
    implementation. `select` keeps what the push runs (`push_keeps`);
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


def select(repo_root: Path, changed: list[str],
           landing: bool = False) -> tuple[list[str], str]:
    """Return (gates to run in order, reason). Empty list means nothing to do.

    `landing` is the judgment of an update to main (see "What `ci_only`
    means"): every reached gate the fleet can run, `ci_only` included.
    """
    reached, reason = matched(repo_root, changed)
    return (run_order(push_keeps(repo_root, reached, landing)), reason)


def push_keeps(repo_root: Path, reached, landing: bool = False) -> set[str]:
    """The gates a push runs out of the ones its change reached.

    The one place the two kinds of push differ, so the hook, the runner and
    the plan cannot select differently: a push of another ref drops every
    `ci_only` gate (`drop_ci_only`); an update to main keeps them and drops
    only the gates hosted CI judges in its place (`drop_hosted_only`).
    """
    if landing:
        return drop_hosted_only(reached, fleet_platforms(repo_root))
    return drop_ci_only(reached)


def fleet_platforms(repo_root: Path) -> set[str]:
    """The platforms bx.toml declares (`[platform.<name>]`), which bx places on.

    Read by its table headers rather than parsed as TOML: the registry runs
    on the Python the build machines carry, and Ubuntu 22.04's 3.10 has no
    `tomllib`. A tool's table (`[platform.linux.tools.swift]`) has a dot in
    what follows `platform.` and is not a platform. A bx.toml that cannot be
    read declares nothing, so every gate that names a platform is left out
    of a landing rather than handed to a fleet that may not run it.
    """
    try:
        text = (repo_root / "bx.toml").read_text(encoding="utf-8")
    except OSError:
        return set()
    return {m.group(1) for m in map(BX_PLATFORM_HEADER.match, text.splitlines())
            if m}


def drop_hosted_only(selected, platforms, table=None) -> set[str]:
    """Remove the gates a landing leaves to hosted CI.

    Two kinds: a gate whose `platform` is not one the fleet has, and one whose
    cost the change sets (`audited_after_landing`). A gate that names no
    platform runs on bx.toml's own and stays. Its dependencies stay as well, as
    `drop_ci_only` keeps them: they are gates the change reached, whichever
    gate pulled them in.
    """
    table = GATES if table is None else table
    return {slug for slug in selected
            if (table[slug].get("platform") is None
                or table[slug]["platform"] in platforms)
            and not table[slug].get("audited_after_landing")}


def why_hosted(slug: str, table=None) -> str:
    """Why a landing leaves `slug` to hosted CI, for the line that names it.

    A gate with both reasons is named by its cost: the platform would be the
    reason only if the fleet could bear the cost.
    """
    table = GATES if table is None else table
    audited = table[slug].get("audited_after_landing")
    if audited:
        return f"is audited after landing ({audited})"
    return (f"needs platform {table[slug]['platform']!r}, which bx.toml does "
            f"not declare ({table[slug].get('platform_reason', '')})")


def container_placement(order, table=None) -> list[tuple[str, str, str]]:
    """Where each gate of a run in `order` runs: `(where, slug, containerfile)`.

    `where` is "host" (the machine the runner is on) or "container" (the
    image `containerfile` builds, where the runner runs the gate with its
    dependencies). A dependency that only gates in an image need is left out:
    it runs inside, with them. One that a gate on the host also needs stays,
    since that gate executes its output here. The order of `order` is kept.
    """
    table = GATES if table is None else table
    in_image = [s for s in order if table[s].get("container")]
    inside: set[str] = set()
    for slug in in_image:
        inside |= transitive_deps(slug, table)
    host = {s for s in order if s not in in_image and s not in inside}
    for slug in list(host):
        host |= transitive_deps(slug, table) & set(order)
    placed = []
    for slug in order:
        if slug in in_image:
            placed.append(("container", slug, table[slug]["container"]))
        elif slug in host:
            placed.append(("host", slug, ""))
    return placed


def hosted_only(repo_root: Path, selected) -> list[str]:
    """The gates of `selected` that a landing leaves to hosted CI, in order."""
    return run_order(set(selected)
                     - drop_hosted_only(selected, fleet_platforms(repo_root)))


def ci_owed(repo_root: Path, changed: list[str],
            landing: bool = False) -> list[str]:
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

    "Only CI" is whatever the push left out, so it follows `landing`: a push
    of another ref leaves out every `ci_only` gate, and an update to main
    only the gates it leaves to hosted CI (`push_keeps`). Derived from the
    selection
    rather than from the field, so the notice cannot name a gate the push ran.
    """
    return run_order({s for s in left_out(repo_root, changed, landing)
                      if any(workflow_starts_for(repo_root, wf, changed)
                             for wf in GATES[s].get("workflows", []))})


def ci_unowed(repo_root: Path, changed: list[str],
              landing: bool = False) -> list[str]:
    """Gates this change reaches that the push left out and NO workflow starts for.

    The residue of `ci_owed`, and the reason a deny-list cannot remove
    coverage in silence. A gate here is run by neither the push nor CI, so
    the change is going out unjudged by it. That is sometimes exactly right
    — a README cannot break a C++ ctest suite — and the point is that it is
    said out loud rather than discovered when a lane has been quiet for
    eight pushes.
    """
    return run_order({s for s in left_out(repo_root, changed, landing)
                      if not any(workflow_starts_for(repo_root, wf, changed)
                                 for wf in GATES[s].get("workflows", []))})


def left_out(repo_root: Path, changed: list[str],
             landing: bool = False) -> set[str]:
    """The gates this change reaches that the push does not run."""
    reached, _ = matched(repo_root, changed)
    return set(reached) - push_keeps(repo_root, reached, landing)


def drop_ci_only(selected) -> set[str]:
    """Remove the gates a push of a non-main ref does not run, keeping their deps.

    An update to main does not come here: the landing queue's judgment keeps
    every `ci_only` gate it reaches (`push_keeps`, and "What `ci_only` means"
    above GATES). Everything below is about the push of any other ref, which
    is still bounded by `PUSH_BUDGET_S`.

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


def tracked_tree_is_head(repo_root: Path) -> bool:
    """Whether every tracked file holds what HEAD holds.

    A kept verdict's key is made of the working tree's content, and two gates
    judge HEAD instead (`embed-manifest-stamp` reads its objects,
    `regen-reproduces` regenerates in a worktree of it). The two are the same
    tree only when the working tree is HEAD's, so a plan made over any other
    keeps no verdict. The push hook already refuses such a tree; this is what
    makes a set run by hand just as safe. Anything git cannot answer counts as
    a difference.
    """
    try:
        done = subprocess.run(["git", "diff", "--quiet", "HEAD", "--"],
                              cwd=repo_root, stdout=subprocess.DEVNULL,
                              stderr=subprocess.DEVNULL, check=False)
    except OSError:
        return False
    return done.returncode == 0


# What `bash scripts/gate --unit <slug>` reads before and around the gate's own
# script, read off `bx-fleet check learn` on 2026-11-23 for four gates, which
# agreed on it: the runner and the library it sources, the toolchain selectors
# (Rust's pin, Go's module for its release, the mesh transports' revision the
# library reads), the submodule preflight and the `.gitmodules` it reads, the
# registry, and `.cargo/`, whose configuration every cargo run reads. Read
# again on 2026-11-24 with bx 340e9f9e, which traces listings: seven gates
# agreed that every unit also lists the root (`./`) and `tools/git-hooks/`.
# bx's trace refuses a unit that reads a file or lists a directory outside its
# inputs, so a runner that starts reading another fails the unit ("could not
# measure"), never passes it.
RUNNER_INPUTS = (
    "./",
    ".gitmodules",
    "tools/git-hooks/",
    "scripts/gate",
    "scripts/gates/lib.sh",
    "scripts/lib/rust_toolchains.sh",
    "scripts/lib/go_toolchain.sh",
    "scripts/lib/require_submodules.sh",
    # What the gate library sources when a gate calls for it: the build's
    # parallelism, the generator, the HTTP fixture, the emscripten Rust and
    # the Swift pin. `bx --checks prepush --learn genmc-queue` (2026-11-24)
    # read the first and no input named it;
    # `every-library-the-runner-sources-is-a-runner-input` holds the list.
    "scripts/lib/sce_build_jobs.sh",
    "scripts/lib/sce_codegen.sh",
    "scripts/lib/sce_http_endpoint.sh",
    "scripts/lib/wasm_rust.sh",
    "scripts/lib/swift_pin.sh",
    "scripts/install_mesh_transports.sh",
    "tools/git-hooks/gate_registry.py",
    "rust-toolchain.toml",
    "backends/go/forge-runtime/go.mod",
    ".cargo/**",
)


def submodule_paths(repo_root: Path) -> list[str]:
    """The paths of the submodules `.gitmodules` records, sorted."""
    done = subprocess.run(
        ["git", "config", "-f", str(repo_root / ".gitmodules"),
         "--get-regexp", r"^submodule\..*\.path$"],
        capture_output=True, text=True, check=False)
    return sorted(line.split(None, 1)[1]
                  for line in done.stdout.splitlines() if " " in line)


def reaches_beneath(pattern: str, path: str) -> bool:
    """Can input pattern `pattern` name a file beneath directory `path`?

    Asked a segment at a time, so `third_party/*/*` reaches
    `third_party/spdlog` and `web/visualizer/vendor/**` does not. A
    pattern that ends in `/` gives a directory's names and nothing they
    hold (bx RFC 0020, section 4.2), so it reaches nothing beneath them.
    """
    if pattern.endswith("/"):
        return False
    parts = path.split("/")
    for i, seg in enumerate(pattern.split("/")):
        if seg == "**" or i == len(parts):
            return True
        if not glob_to_regex(seg).match(parts[i]):
            return False
    return False


def submodules_reached(patterns, paths) -> list[str]:
    """Those of the submodules at `paths` beneath which `patterns` name a file.

    A narrowed unit's copy holds a submodule checked out only when its
    inputs give the submodule's `<path>/.git`, and holds every other one as
    a bare gitlink (bx RFC 0026, section 4.1, 2026-11-23). So the submodule
    preflight asks a narrowed gate for these alone
    (`scripts/lib/require_submodules.sh`), and
    `a-narrowed-gate-holds-the-submodules-it-reads` refuses an entry that
    reaches one without its `.git`.
    """
    return [p for p in paths if any(reaches_beneath(g, p) for g in patterns)]



def gate_inputs(slug, table=None, triggers=None, reads=None):
    """What `slug`'s OWN script reads, as its key names it: `None` for the
    whole tracked tree, or a sorted list of patterns. A declared list is
    itself; `INPUTS_ARE_TRIGGERS` is the gate's trigger globs from
    `triggers` (`gate_triggers`), which a caller computes once per plan,
    the listing patterns in its `lists` (a directory the script lists,
    whose names bx keys and whose contents it does not: bx RFC 0020, 4.2),
    and what the builds it names read without selecting it, from `reads`
    (`gate_reads`). Neither a listing nor such a read is a trigger: a name
    new in such a directory is selected by the glob that matches it, or
    reads nothing the verdict depends on.

    A gate keyed on its triggers that names a build and is given no `reads`
    is refused, as one given no `triggers` is: its key would miss what its
    build reads."""
    table = GATES if table is None else table
    own = table[slug].get("inputs")
    if own != INPUTS_ARE_TRIGGERS:
        return own
    if triggers is None:
        raise ValueError(f"{slug} is keyed on its triggers and none were given")
    if table[slug].get("builds") and (reads is None or slug not in reads):
        raise ValueError(f"{slug} is keyed on its triggers and builds "
                         f"{table[slug]['builds']}, and what those read was "
                         f"not given")
    return sorted(dict.fromkeys(list(triggers[slug])
                                + list(table[slug].get("lists", []))
                                + list((reads or {}).get(slug, []))))


def unit_inputs(closure, table=None, triggers=None, reads=None):
    """The input patterns of a unit that runs every gate in `closure`.

    `None`, the whole tracked tree, as soon as one of them reads the whole
    tree: a key narrower than what the unit reads would let a pass outlive
    a change to the part left out. Otherwise the union of their patterns
    (`gate_inputs`, with `triggers` and `reads` for a gate keyed on its
    triggers), of
    `RUNNER_INPUTS`, and of each gate's script and mirrored workflows,
    sorted so that one closure always yields one key.
    """
    table = GATES if table is None else table
    patterns: set[str] = set(RUNNER_INPUTS)
    for slug in closure:
        own = gate_inputs(slug, table, triggers, reads)
        if own is None:
            return None
        patterns.update(own)
        patterns.add(f"scripts/gates/{slug}.sh")
        patterns.update(f".github/workflows/{wf}"
                        for wf in table[slug].get("workflows", []))
    return sorted(patterns)


def head_commit(repo_root: Path):
    """The full id of the commit HEAD names, or None when git cannot say."""
    try:
        done = subprocess.run(["git", "rev-parse", "--verify", "HEAD^{commit}"],
                              cwd=repo_root, capture_output=True, text=True,
                              check=False)
    except OSError:
        return None
    commit = done.stdout.strip()
    return commit if done.returncode == 0 and COMMIT_ID.match(commit) else None


# ── A gate planned as shards ──────────────────────────────────────
#
# A gate whose script runs one part of itself when a variable names `k/N`
# (`shard_env` in its entry) is planned as N units, one per part, so the parts
# run on as many machines at once (ADR 0120, D3). N is read from the workflow
# that runs the same parts as jobs (`<variable>: ${{ matrix.shard }}/N`), so
# the lane and the landing split the gate one way. A shard's unit is named
# `<slug>-shard-<k>-of-<N>` and runs `scripts/gate --shard k/N <slug>`, which
# hands the gate the variable; its kept pass is that part's, under its argv.
SHARD_UNIT = re.compile(r"^(?P<gate>.+)-shard-(?P<k>[0-9]+)-of-(?P<n>[0-9]+)$")


def shard_count(repo_root: Path, slug: str, table=None) -> int:
    """How many parts a plan runs `slug` in: 1 unless it declares
    `shard_env` and a workflow it mirrors sets that variable per shard."""
    spec = (GATES if table is None else table)[slug]
    env = spec.get("shard_env")
    if not env:
        return 1
    per_job = re.compile(r"^\s*" + re.escape(env)
                         + r":\s*\$\{\{\s*matrix\.shard\s*\}\}/([0-9]+)\s*$",
                         re.M)
    for wf in spec.get("workflows", []):
        try:
            text = (repo_root / ".github" / "workflows" / wf).read_text(
                encoding="utf-8")
        except OSError:
            continue
        found = per_job.search(text)
        if found:
            return int(found.group(1))
    return 1


def unit_gate(name: str) -> str:
    """The gate a unit of a plan runs: its name, less a shard's suffix."""
    found = SHARD_UNIT.match(name)
    return found.group("gate") if found else name


def plan_units(selected, rust=None, tree_is_head=True, table=None,
               head=None, landing=False, changes=(), shards=None,
               triggers=None, reads=None) -> list[dict]:
    """The bx check units (bx RFC 0004, section 6) that verify `selected`.

    `selected` is a set the registry already chose: `select` for a change,
    every gate for `--all`, a named closure. The plan adds no selection of
    its own, which is what lets the push run the same gates whether bx
    carries them or the runner runs them in place.

    ONE UNIT PER GATE THAT NO OTHER SELECTED GATE RUNS. A unit is
    `bash scripts/gate [--rust R] [--landing] [--at <id>]
    [--change-set <base>..<tip>...] <slug>`, and the runner gives a named
    gate its dependency closure and stops at the first failure, so a
    dependency's verdict is part of every dependent's unit. A unit of its
    own would be the same build again on another machine, measuring nothing
    new.

    A UNIT THAT READS HEAD CARRIES ITS ID. When a gate the unit runs
    declares `commits: ["HEAD"]`, the argv gets `--at <head>`, so the commit
    is in the key, and the runner refuses a checkout at any other commit.

    A UNIT THAT READS THE CHANGE SET CARRIES ITS RANGES. When a gate the unit
    runs declares `commits: ["CHANGES"]`, the argv gets `--change-set R` for
    each `<base>..<tip>` in `changes`, the ranges the push publishes, and the
    runner hands the gate the paths they change. It is not `--changed-in`,
    which selects gates: the plan has already selected, and the ranges are
    only what the gate reads. Without ranges (a plan of
    `--all`, of named gates, or of a path list) the argv carries none, and
    the gate answers for an unknown set as its script says.

    A UNIT'S VERDICT IS KEPT (`cache`) only when all of these hold:
      * the plan names the Rust release (`rust`), because every unit's argv
        then carries it and a new pinned release is a new key. Without it a
        unit runs whatever cargo its machine has on PATH, which is not in
        the key;
      * the tracked tree is HEAD's (`tree_is_head`, see
        `tracked_tree_is_head`);
      * every gate the unit runs declares `cache: True`;
      * a unit that reads HEAD has its id (`head`) to carry;
      * a unit that reads the change set has its ranges (`changes`) to carry;
      * a unit that runs a gate whose verdict outside a landing can rest on
        the machine (`kept_only_from_a_landing`) is a landing's, where a skip
        or a missing pinned tool ends 3 instead of passing.
    Each unit says `cache` either way, because the set in bx.toml keeps
    verdicts by default and a unit that is not to be kept has to say so.

    A LANDING'S UNIT SAYS SO. `landing` puts `--landing` in every unit's
    argv: the runner then counts no skip as a pass (see "Whether a gate's
    verdict may be kept"), and the flag is in the key, so a pass kept from a
    landing is never handed to a run that could have skipped, or back.

    A GATE KEYED ON ITS TRIGGERS needs `triggers` (`gate_triggers`) and, when
    it names a build, `reads` (`gate_reads`), which the caller computes once;
    see `INPUTS_ARE_TRIGGERS`.

    A GATE IN SHARDS IS N UNITS. `shards` maps a slug to its count
    (`shard_count`); a gate counted above one is a unit per part, named and
    run as "A gate planned as shards" says, its declared cost divided among
    them.

    `exclusive` is `UNIT_EXCLUSIVE` for every unit. `secs` is the declared
    cost of everything the unit runs, a first estimate that bx replaces with
    its own timings, `mem_gb` the largest `peak_gb` among them, and
    `parallelism` the largest `parallelism`. `needs` and `packages` are the union of what the
    gates it runs name in theirs, sorted, and absent when there is none, as bx
    reads an absent key.
    """
    table = GATES if table is None else table
    chosen = set(selected)
    run_by_another: set[str] = set()
    for slug in chosen:
        run_by_another |= transitive_deps(slug, table)
    units = []
    for slug in run_order(chosen - run_by_another, table):
        closure = {slug} | transitive_deps(slug, table)
        reads_head = any("HEAD" in table[s].get("commits", []) for s in closure)
        reads_changes = any("CHANGES" in table[s].get("commits", [])
                            for s in closure)
        can_skip = any(table[s].get("kept_only_from_a_landing") for s in closure)
        kept = (rust is not None and tree_is_head
                and all(table[s]["cache"] is True for s in closure)
                and (head is not None or not reads_head)
                and (bool(changes) or not reads_changes)
                and (landing or not can_skip))
        argv = ["bash", "scripts/gate"]
        if rust is not None:
            argv += ["--rust", rust]
        # A unit runs the gates the plan chose; the plan ran the registry's
        # self-test over the same tree, so the unit does not run it again
        # and its reads are the gates' own (see "--unit" in scripts/gate).
        argv.append("--unit")
        if landing:
            argv.append("--landing")
        if reads_head and head is not None:
            argv += ["--at", head]
        if reads_changes:
            for change in changes:
                argv += ["--change-set", change]
        count = (shards or {}).get(slug, 1)
        cost = sum(table[s].get("cost_s") or 0 for s in closure)
        inputs = unit_inputs(closure, table, triggers, reads)
        for part in range(1, count + 1):
            name = slug if count == 1 else f"{slug}-shard-{part}-of-{count}"
            own = argv + ([] if count == 1 else ["--shard", f"{part}/{count}"])
            unit = {
                "name": name,
                "argv": own + [slug],
                "exclusive": [UNIT_EXCLUSIVE],
                "cache": kept,
                "broke": f"gate {name} FAILED: run just this one with "
                         f"`scripts/gate {' '.join(own[len(argv):] + [slug])}`",
                "blind": f"gate {name} COULD NOT RUN on that machine: its own "
                         f"tools or packages, or the Rust release the plan "
                         f"named, are missing there. Nothing is claimed about "
                         f"the tree.",
            }
            if cost > 0:
                unit["secs"] = float(cost) / count
            unit["mem_gb"] = float(max(table[s]["peak_gb"] for s in closure))
            unit["parallelism"] = float(max(table[s]["parallelism"] for s in closure))
            for key in UNIT_REQUIRES_KEYS:
                wanted = {item for s in closure for item in table[s].get(key, [])}
                if wanted:
                    unit[key] = sorted(wanted)
            if kept and inputs:
                unit["inputs"] = inputs
            units.append(unit)
    return units


def nothing_to_verify(reason: str) -> dict:
    """The one unit of a plan whose selection is empty: a known pass."""
    return {"name": NOTHING_TO_VERIFY, "result": {"exit": 0, "note": reason}}


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


# ── The Rust toolchains bx.toml declares ──────────────────────────
#
# Every toolchain a gate runs is a `[platform.linux.tools.rust*]` table of
# bx.toml whose install is `scripts/install_rust.sh`: bx installs each once per
# build machine and places a unit only where the tables its command line names
# by `needed_by` are installed. No gate installs one there (the header of
# `scripts/lib/rust_toolchains.sh` says why). That makes the tables a claim
# with three halves, and each can go stale without a gate turning red: the
# release is pinned in `rust-toolchain.toml` and the nightly in
# `scripts/lib/rust_toolchains.sh`, the
# release each package builds with is its manifest's `rust-version`, and the
# units that need a table are the ones whose gates ask for it. A table a unit
# needs and does not name is a unit placed on a machine without its compiler,
# which ends "could not run"; one it names and does not need keeps that unit
# off every machine that lacks it, for nothing.
RUST_TOOLCHAIN_INSTALL = "scripts/install_rust.sh"
RUST_TOOLCHAIN_PINS = "scripts/lib/rust_toolchains.sh"
# The release is pinned once, in the file rustup reads for every cargo run in
# the checkout; `scripts/lib/rust_toolchains.sh` and the release table both
# read it from there, so the gates and the rest of the tree run one release.
RUST_TOOLCHAIN_FILE = "rust-toolchain.toml"
RUST_TOOLCHAIN_PLACE = "~/.local/rust/{version}"
# The manifest that pins Verus and the Rust release it runs on, and the judge of it,
# which a gate body names to need that release.
VERUS_MANIFEST = "docs/formal/proofs.json"
VERUS_JUDGE = "scripts/formal/proofs.py"
# The one other place a release is linked: the directory of the run account's
# rustup home where rustup's proxy finds the release rust-toolchain.toml pins,
# so a cargo a run starts outside a gate unit installs nothing on its first use
# (`[platform.linux.tools.rust-rustup]` in bx.toml says why). The host is the
# one scripts/install_rust.sh installs for.
RUSTUP_PROXY_PLACE = "~/.rustup/toolchains/{version}-x86_64-unknown-linux-gnu"
# How every Rust table's install line begins: the install's own rustup home and
# cargo home, then the one script, the toolchain and the store entry.
RUST_INSTALL_HEAD = (f"RUSTUP_HOME={{scratch}}/rh CARGO_HOME={{scratch}}/ch "
                     f"bash {RUST_TOOLCHAIN_INSTALL} {{version}} {{prefix}}")
# What a rustup command that adds a toolchain or a part of one looks like. No
# gate body and not the runner runs one; the gate library alone does, for a
# hosted runner (`sce_gate_require_rust` and its siblings).
RUSTUP_INSTALLS = re.compile(
    r"\brustup\s+(?:toolchain\s+install|target\s+add|component\s+add)\b")


def bx_tool_tables(repo_root: Path) -> dict[str, dict]:
    """The `[platform.linux.tools.<name>]` tables of bx.toml, by name.

    Read by its lines, as bx's shell reads a tool table: each value on one
    line, a quoted string or an array of them (bx refuses any other spelling
    in a tool table). Not a TOML parser because the one in the standard
    library arrived in Python 3.11, and a build machine runs this on 3.10.
    """
    tables: dict[str, dict] = {}
    current = None
    for raw in (repo_root / "bx.toml").read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("["):
            header = re.fullmatch(r"\[platform\.linux\.tools\.([A-Za-z0-9_.-]+)\]",
                                  line)
            current = tables.setdefault(header.group(1), {}) if header else None
            continue
        pair = re.fullmatch(r"([a-z_]+)\s*=\s*(.+)", line)
        if current is None or not pair:
            continue
        key, value = pair.group(1), pair.group(2).strip()
        if value.startswith("["):
            current[key] = [a or b for a, b in
                            re.findall(r'"([^"]*)"|\'([^\']*)\'', value)]
        elif value[:1] in ('"', "'") and value[-1:] == value[:1]:
            current[key] = value[1:-1]
    return tables


def bx_tool_version(repo_root: Path, table: dict):
    """A tool table's version as bx resolves it, or None when it has none.

    Written (`version`), or the first group of the lines of `version_file`
    that `version_pattern` matches, every one of which must say the same.
    """
    if "version" in table:
        return table["version"]
    file, pattern = table.get("version_file"), table.get("version_pattern")
    if not file or not pattern or not (repo_root / file).is_file():
        return None
    found = set()
    for line in (repo_root / file).read_text(encoding="utf-8").splitlines():
        match = re.search(pattern, line)
        if match:
            found.add(match.group(1))
    return found.pop() if len(found) == 1 else None


def workspace_rust_versions(repo_root: Path) -> dict[str, list[str]]:
    """Each `rust-version` the workspace's packages declare, with the packages.

    The members are the root manifest's `members` list and a member that says
    `rust-version.workspace = true` declares the root's, which is what
    `cargo metadata` reports and `scripts/gates/rust-version.sh` builds by.
    """
    root_text = (repo_root / "Cargo.toml").read_text(encoding="utf-8")
    workspace = re.search(r'^\[workspace\.package\][^\[]*?^rust-version\s*=\s*"([^"]+)"',
                          root_text, re.M | re.S)
    declared: dict[str, list[str]] = {}
    for member in workspace_members(repo_root):
        manifest = repo_root / member / "Cargo.toml"
        if not manifest.is_file():
            continue
        text = manifest.read_text(encoding="utf-8")
        own = re.search(r'^rust-version\s*=\s*"([^"]+)"', text, re.M)
        inherited = re.search(r"^rust-version\.workspace\s*=\s*true", text, re.M)
        if own:
            version = own.group(1)
        elif inherited and workspace:
            version = workspace.group(1)
        else:
            continue
        declared.setdefault(version, []).append(member)
    return declared


def workspace_members(repo_root: Path) -> list[str]:
    """The `members` list of the root manifest, as written there."""
    root_text = (repo_root / "Cargo.toml").read_text(encoding="utf-8")
    members_block = re.search(r"^members\s*=\s*\[(.*?)\]", root_text, re.M | re.S)
    return re.findall(r'"([^"]+)"', members_block.group(1)) if members_block else []


# A manifest that inherits the workspace's lints: a `[lints]` table whose first
# key, past blank and comment lines, is `workspace = true`. Cargo allows no
# other key beside it.
INHERITS_WORKSPACE_LINTS = re.compile(
    r"^\[lints\][ \t]*\n(?:[ \t]*(?:#[^\n]*)?\n)*workspace[ \t]*=[ \t]*true[ \t]*$", re.M)


def workspace_lint_failures(repo_root: Path) -> list[str]:
    """Every workspace member that does not inherit `[workspace.lints]`.

    The root manifest's `[workspace.lints.rust]` denies every rustc warning,
    and a member is held to it only by `[lints] workspace = true`. One without
    the table builds on rustc's defaults: a warning in it fails nothing but a
    clippy sweep that happens to compile that code, and cargo reports the
    missing table only as a warning of its own. Ten members had none until
    2026-11-14, among them every crate of the shared list and the Mesh crate,
    whose `zenoh` and `conformance` features no clippy sweep compiles.
    """
    root_text = (repo_root / "Cargo.toml").read_text(encoding="utf-8")
    if not re.search(r"^\[workspace\.lints\.rust\]", root_text, re.M):
        return ["the root Cargo.toml has no [workspace.lints.rust] table, so there "
                "is no workspace lint policy for a member to inherit"]
    members = workspace_members(repo_root)
    failures: list[str] = []
    for member in members:
        manifest = repo_root / member / "Cargo.toml"
        if not manifest.is_file():
            failures.append(f"{member} is a workspace member with no Cargo.toml")
            continue
        if not INHERITS_WORKSPACE_LINTS.search(manifest.read_text(encoding="utf-8")):
            failures.append(
                f"{member}/Cargo.toml does not inherit the workspace lints: add "
                f"`[lints]` with `workspace = true`, or a rustc warning in it fails "
                f"no build")
    # A lower bound, so a members list the parse stopped finding cannot pass.
    if len(members) < 20:
        failures.append(f"read {len(members)} workspace member(s) from the root "
                        f"Cargo.toml; a broken parse passes by reading none")
    return failures


def msrv_release(version: str):
    """The release a `rust-version` names: `1.87` is `1.87.0`, the oldest it
    admits (`sce_rust_msrv_release` in scripts/lib/rust_toolchains.sh)."""
    if re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        return version
    if re.fullmatch(r"[0-9]+\.[0-9]+", version):
        return version + ".0"
    return None


# ── The Rust a lane installs ──────────────────────────────────────
#
# Every CI lane installs the release `rust-toolchain.toml` pins, through the
# one action that reads it, and a script that needs a nightly names the dated
# one `scripts/lib/rust_toolchains.sh` pins. A channel that moves (`stable`,
# `beta`, a bare `nightly`) is whatever that channel is on the day a run
# starts: on 2026-11-14 main turned red with no commit when the runners'
# `stable` moved to 1.100 and its clippy found what 1.99 had passed. The
# sweep reads every tracked workflow, action and shell script with its
# whole-line comments removed.
#
# Two things around the install move too unless they are held. The upstream
# action is code that runs between the pin and the compiler, so it is named
# by a commit, not a branch or a tag; that holds for every action a lane
# runs, and `unpinned_action_failures` below is where it is held. And a cache
# of a Rust `target/` or of cargo's registry built by one release is restored
# into a run of the next: its key, and every restore-key prefix, carry the
# hash of `rust-toolchain.toml`, so a move starts each cache afresh instead
# of dragging the old release's artefacts along for good.
RUST_INSTALL_ACTION = ".github/actions/install-rust/action.yml"
# The upstream action the one above delegates the install to; no lane names it.
RUST_INSTALL_UPSTREAM = "dtolnay/rust-toolchain"
TOOLCHAIN_CACHE_KEY = "hashFiles('rust-toolchain.toml')"
CACHE_STEP = re.compile(
    r"^\s*(?:-\s+)?uses:\s*(actions/cache(?:/restore|/save)?|Swatinem/rust-cache)@")
MOVING_RUST_CHANNEL = re.compile(
    r"(?<![\w.+-])\+(?:stable|beta|nightly)(?![\w.-])"
    r"|\btoolchain:\s*['\"]?(?:stable|beta|nightly)['\"]?\s*$"
    r"|\brustup\s+(?:default|toolchain\s+install|install|update|run|override\s+set)\s+"
    r"(?:stable|beta|nightly)(?![\w.-])"
    r"|\bRUSTUP_TOOLCHAIN=['\"]?(?:stable|beta|nightly)(?![\w.-])")


def moving_rust_failures(repo_root: Path) -> list[str]:
    """Where a lane or a script installs or names a Rust that moves."""
    failures: list[str] = []
    action = repo_root / RUST_INSTALL_ACTION
    if not action.is_file() or RUST_TOOLCHAIN_FILE not in action.read_text(encoding="utf-8"):
        failures.append(f"{RUST_INSTALL_ACTION} does not install what "
                        f"{RUST_TOOLCHAIN_FILE} pins, and every lane installs Rust "
                        f"through it")
    swept = 0
    lanes = 0
    upstream_pins = 0
    for rel in tracked_paths(repo_root, ".github", "*.sh", "scripts"):
        path = repo_root / rel
        if Path(rel).suffix not in (".yml", ".yaml", ".sh", ""):
            continue
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeDecodeError):
            continue
        swept += 1
        for number, line in enumerate(lines, 1):
            if line.lstrip().startswith("#"):
                continue
            if RUST_INSTALL_UPSTREAM in line and rel != RUST_INSTALL_ACTION:
                failures.append(
                    f"{rel}:{number} installs Rust with {RUST_INSTALL_UPSTREAM} "
                    f"itself: a lane installs it through ./{Path(RUST_INSTALL_ACTION).parent}, "
                    f"which takes the release from {RUST_TOOLCHAIN_FILE}")
            elif RUST_INSTALL_UPSTREAM in line:
                upstream_pins += 1
            if MOVING_RUST_CHANNEL.search(line):
                failures.append(
                    f"{rel}:{number} names a Rust channel that moves "
                    f"({line.strip()[:80]!r}): the release is pinned in "
                    f"{RUST_TOOLCHAIN_FILE} and the nightly by its date in "
                    f"{RUST_TOOLCHAIN_PINS}")
            if f"uses: ./{Path(RUST_INSTALL_ACTION).parent}" in line:
                lanes += 1
    # Lower bounds, so a sweep that stopped reading cannot pass by reading nothing.
    if swept < 20 or lanes < 20:
        failures.append(
            f"the sweep read {swept} file(s) and found {lanes} lane step(s) that "
            f"install the pinned Rust; a broken enumeration passes by reading nothing")
    if upstream_pins == 0:
        failures.append(
            f"{RUST_INSTALL_ACTION} names {RUST_INSTALL_UPSTREAM} nowhere, so the "
            f"check that no lane names it but that action reads nothing and needs "
            f"re-aiming")
    failures.extend(rust_cache_failures(repo_root))
    return failures


# ── The actions a lane runs ───────────────────────────────────────
#
# A `uses:` names code that runs inside the job, with the job's token and its
# checkout. A tag or a branch is a name the action's owner can point at other
# code at any time, and a run then executes code no commit of this repository
# chose: release tags moved to a commit that printed the runner's secrets is a
# supply-chain attack that has happened on exactly this line. So an
# action from outside this repository is named by the full commit it runs, and
# the release that commit was taken from follows in a trailing comment, for a
# reader and for the tool that moves the pin
# (`uses: actions/checkout@<40 hex> # v6.1.0`). A container (`docker://`) is
# named by its digest. A local action (`./...`) is this repository's own code,
# fixed by the commit it is checked out at, and is left alone. A whole-line
# comment is prose and is not read.
#
# The sweep reads every tracked workflow and action file, and a lower bound on
# what it reads keeps a broken enumeration from passing by reading nothing.
ACTION_USES = re.compile(
    r"^\s*(?:-\s+)?uses:\s*(?P<ref>[^\s#]+)\s*(?:#\s*(?P<note>\S.*))?$")
PINNED_ACTION = re.compile(r"^[^@\s]+@(?:[0-9a-f]{40}|[0-9a-f]{64})$")
PINNED_IMAGE = re.compile(r"^docker://[^@\s]+@sha256:[0-9a-f]{64}$")


def unpinned_uses(lines: list[str]) -> list[tuple[int, str]]:
    """Each `uses:` of a workflow or action file that runs code by a name
    which moves, as its 1-based line number and what is wrong with it."""
    found: list[tuple[int, str]] = []
    for number, line in enumerate(lines, 1):
        if line.lstrip().startswith("#"):
            continue
        match = ACTION_USES.match(line)
        if not match:
            continue
        ref = match.group("ref").strip("'\"")
        if ref.startswith("./"):
            continue
        if ref.startswith("docker://"):
            if not PINNED_IMAGE.match(ref):
                found.append((number, f"names the image {ref!r} by a tag, not by "
                                      f"its sha256 digest"))
            continue
        if not PINNED_ACTION.match(ref):
            found.append((number, f"names {ref!r}, not a full commit"))
        elif not match.group("note"):
            found.append((number, f"pins {ref} with no trailing `# <release>` "
                                  f"naming the release that commit is"))
    return found


def unpinned_action_failures(repo_root: Path) -> list[str]:
    """Where a tracked workflow or action runs an action by a name that moves."""
    failures: list[str] = []
    files = 0
    actions = 0
    for rel in tracked_paths(repo_root, ".github"):
        if Path(rel).suffix not in (".yml", ".yaml"):
            continue
        try:
            lines = (repo_root / rel).read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeDecodeError):
            continue
        files += 1
        for line in lines:
            match = ACTION_USES.match(line)
            if match and not line.lstrip().startswith("#") \
                    and not match.group("ref").strip("'\"").startswith("./"):
                actions += 1
        for number, why in unpinned_uses(lines):
            failures.append(
                f"{rel}:{number} {why}: a tag or a branch is a name the action's "
                f"owner can point at other code, which then runs with this job's "
                f"token. Name the commit it resolves to and keep the release in a "
                f"trailing comment (`gh api repos/<owner>/<repo>/commits/<tag> "
                f"--jq .sha`)")
    # Lower bounds, so a sweep that stopped reading cannot pass by reading nothing.
    if files < 20 or actions < 100:
        failures.append(
            f"the sweep read {files} workflow or action file(s) and found {actions} "
            f"action(s) from outside the repository; a broken enumeration passes by "
            f"reading nothing")
    return failures


def workflow_steps(lines: list[str], uses: "re.Pattern[str]"):
    """Each step of a workflow or action whose `uses:` line matches `uses`.

    Yields the 1-based number of the `uses:` line, the action it names and
    the step's lines: from the line that opens the step (`- `) to the next
    line at that indent or less.
    """
    for i, line in enumerate(lines):
        match = uses.match(line)
        if not match:
            continue
        start = i
        while start > 0 and not re.match(r"^\s*-\s", lines[start]):
            start -= 1
        indent = len(lines[start]) - len(lines[start].lstrip())
        end = start + 1
        while end < len(lines):
            nxt = lines[end]
            if nxt.strip() and len(nxt) - len(nxt.lstrip()) <= indent:
                break
            end += 1
        yield i + 1, match.group(1), lines[start:end]


def step_values(step: list[str], key: str) -> list[str]:
    """The values of `key:` in a step: its inline value, or each line of the
    block scalar under it (`path: |`, `restore-keys: |`)."""
    for j, line in enumerate(step):
        match = re.match(r"^(\s*)" + re.escape(key) + r":\s*(.*)$", line)
        if not match:
            continue
        inline = match.group(2).strip()
        if inline and inline not in ("|", "|-", ">", ">-"):
            return [inline]
        indent = len(match.group(1))
        values: list[str] = []
        for nxt in step[j + 1:]:
            if not nxt.strip():
                continue
            if len(nxt) - len(nxt.lstrip()) <= indent:
                break
            values.append(nxt.strip())
        return values
    return []


HASHED_FILES = re.compile(r"hashFiles\(([^)]*)\)")


def key_hashed_sources(key: str) -> list[str]:
    """The source files a cache key hashes: a `.rs` glob or a `src/` tree.

    A key of Rust build state names what its dependencies are (the toolchain,
    Cargo.lock, a manifest). On 2026-11-24 five lanes hashed sce-build's
    sources into theirs, so each commit that touched them saved a new
    `target` of 380 to 620 MB, and with the repository's caches past their
    10 GB GitHub evicted the C++ compiler caches.
    """
    found = []
    for args in HASHED_FILES.findall(key):
        for glob in re.findall(r"'([^']*)'", args):
            if glob.endswith(".rs") or "/src/" in f"/{glob}":
                found.append(glob)
    return found


def rust_cache_failures(repo_root: Path) -> list[str]:
    """A cache of Rust build state whose key does not move with the release.

    A Rust cache is a `Swatinem/rust-cache` step, or an `actions/cache` step
    whose `path` holds a `target` directory or cargo's registry or git
    checkouts. Its `key`, and each of its `restore-keys`, must carry
    `hashFiles('rust-toolchain.toml')`: a restore-key prefix without it hands
    a miss the previous release's `target/`, which cargo rebuilds beside and
    the next save keeps.
    """
    failures: list[str] = []
    found = 0
    for rel in tracked_paths(repo_root, ".github"):
        if Path(rel).suffix not in (".yml", ".yaml"):
            continue
        try:
            lines = (repo_root / rel).read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeDecodeError):
            continue
        for number, action, step in workflow_steps(lines, CACHE_STEP):
            paths = step_values(step, "path")
            rust = action == "Swatinem/rust-cache" or any(
                p == "target" or p.endswith("/target")
                or ".cargo/registry" in p or ".cargo/git" in p
                for p in paths)
            if not rust:
                continue
            found += 1
            keys = step_values(step, "key")
            if not keys:
                failures.append(
                    f"{rel}:{number} caches Rust build state with no `key` that "
                    f"carries {TOOLCHAIN_CACHE_KEY}, so a release move restores the "
                    f"old release's artefacts")
            for key in keys:
                for source in key_hashed_sources(key):
                    failures.append(
                        f"{rel}:{number} keys Rust build state by the sources "
                        f"{source!r}: every commit that touches them saves another "
                        f"copy of `target`, which evicts the caches beside it; key it "
                        f"by the toolchain and Cargo.lock, cargo rebuilds a changed crate")
            for key in keys + step_values(step, "restore-keys"):
                if TOOLCHAIN_CACHE_KEY not in key:
                    failures.append(
                        f"{rel}:{number} caches Rust build state under {key!r}, "
                        f"which lacks {TOOLCHAIN_CACHE_KEY}: a release move would "
                        f"restore the old release's artefacts into the new one's run")
    # A lower bound, so a step parse that stopped finding caches cannot pass.
    if found < 30:
        failures.append(
            f"found {found} Rust cache step(s) in .github; a broken step parse "
            f"passes by finding none")
    return failures


GRADLE_CACHE_PATH = "~/.gradle/caches"


def shared_gradle_cache_keys(workflows: dict[str, list[str]]) -> tuple[list[str], int]:
    """Gradle cache keys two workflows share, and how many Gradle caches there were.

    A Gradle cache holds the dependencies of the tasks its lane ran. A key two
    lanes share is saved by whichever misses it first, and the other restores
    it, finds a hit, saves nothing, and downloads what it lacks from Maven
    Central on every run: run 45416235401 of `ecma262-lowered-kotlin` was
    refused `kotlinx-coroutines` there (429) from a cache another lane had
    saved. So each workflow keys its Gradle cache by itself. A restore-key
    prefix may be shared: it seeds a first run and is never saved under.
    """
    owners: dict[str, list[str]] = {}
    found = 0
    for rel, lines in sorted(workflows.items()):
        for number, _action, step in workflow_steps(lines, CACHE_STEP):
            if GRADLE_CACHE_PATH not in step_values(step, "path"):
                continue
            found += 1
            for key in step_values(step, "key"):
                owners.setdefault(key, []).append(f"{rel}:{number}")
    failures = []
    for key, places in sorted(owners.items()):
        files = {place.rsplit(":", 1)[0] for place in places}
        if len(files) > 1:
            failures.append(
                f"the Gradle cache key {key!r} is saved by {len(files)} lanes "
                f"({', '.join(places)}): a lane restores another's dependencies, "
                f"finds a hit, and downloads the rest on every run; key it by the lane")
    return failures, found


def gradle_cache_failures(repo_root: Path) -> list[str]:
    """`shared_gradle_cache_keys` over every workflow in the tree."""
    workflows: dict[str, list[str]] = {}
    for rel in tracked_paths(repo_root, ".github"):
        if Path(rel).suffix not in (".yml", ".yaml"):
            continue
        try:
            workflows[rel] = (repo_root / rel).read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeDecodeError):
            continue
    failures, found = shared_gradle_cache_keys(workflows)
    # A lower bound, so a step parse that stopped finding caches cannot pass.
    if found < 5:
        failures.append(
            f"found {found} Gradle cache step(s) in .github; a broken step parse "
            f"passes by finding none")
    return failures


KEEP_ONE_CACHE_STEP = re.compile(r"^\s*(?:-\s+)?uses:\s*(\./\.github/actions/keep-one-cache)\s*$")
PER_RUN_KEY = re.compile(r"\$\{\{\s*github\.(?:sha|run_id)\s*\}\}")


def per_run_cache_failures(workflows: dict[str, list[str]]) -> tuple[list[str], int]:
    """Caches saved under a key new on every run that nothing keeps to one.

    A key that carries the commit or the run (a compiler cache's) saves a new
    entry on every run; the entries before it stay until GitHub evicts the least
    recently used caches of the whole repository to stay within its 10 GB. So a
    workflow that saves one runs `./.github/actions/keep-one-cache` with the key's
    family (the key up to the commit), and no family is the beginning of
    another's, or keeping one of it would delete the other lane's.
    """
    failures: list[str] = []
    families: dict[str, str] = {}
    found = 0
    for rel, lines in sorted(workflows.items()):
        kept = {value for _n, _a, step in workflow_steps(lines, KEEP_ONE_CACHE_STEP)
                for value in step_values(step, "prefix")}
        for number, _action, step in workflow_steps(lines, CACHE_STEP):
            for key in step_values(step, "key"):
                match = PER_RUN_KEY.search(key)
                if not match:
                    continue
                found += 1
                family = key[:match.start()]
                families.setdefault(family, f"{rel}:{number}")
                if family not in kept:
                    failures.append(
                        f"{rel}:{number} saves a cache under {key!r}, new on every run, and "
                        f"the workflow keeps no one of its family {family!r} "
                        f"(./.github/actions/keep-one-cache): each run's entry would stay "
                        f"until GitHub evicts other lanes' caches for room")
    for family, place in sorted(families.items()):
        for other, other_place in sorted(families.items()):
            if other != family and other.startswith(family):
                failures.append(
                    f"the cache family {family!r} ({place}) begins {other!r} "
                    f"({other_place}): keeping one of the first deletes the second's")
    return failures, found


def per_run_cache_sweep(repo_root: Path) -> list[str]:
    """`per_run_cache_failures` over every workflow in the tree."""
    workflows: dict[str, list[str]] = {}
    for rel in tracked_paths(repo_root, ".github"):
        if Path(rel).suffix not in (".yml", ".yaml"):
            continue
        try:
            workflows[rel] = (repo_root / rel).read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeDecodeError):
            continue
    failures, found = per_run_cache_failures(workflows)
    if found < 5:
        failures.append(
            f"found {found} cache step(s) keyed by the run in .github; a broken step "
            f"parse passes by finding none")
    return failures


def rust_toolchain_file(repo_root: Path) -> dict:
    """`channel`, `components` and `targets` of rust-toolchain.toml.

    Read by its lines, each value on one line, as the shell readers read it
    (`scripts/lib/rust_toolchains.sh`, `.github/actions/install-rust`), and for
    the reason `bx_tool_tables` gives for not parsing TOML. A key written any
    other way is absent here, which the checks that use it report.
    """
    path = repo_root / RUST_TOOLCHAIN_FILE
    if not path.is_file():
        return {}
    found: dict = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        scalar = re.fullmatch(r'channel = "([^"]+)"', line)
        if scalar:
            found["channel"] = scalar.group(1)
        listed = re.fullmatch(r'(components|targets) = \[(.*)\]', line)
        if listed:
            found[listed.group(1)] = re.findall(r'"([^"]+)"', listed.group(2))
    return found


def rust_install_parts(install: str) -> dict[str, set[str]]:
    """The components (`-c`) and targets (`-t`) on an install line."""
    return {"components": set(re.findall(r"(?:^|\s)-c\s+(\S+)", install)),
            "targets": set(re.findall(r"(?:^|\s)-t\s+(\S+)", install))}


def rustup_proxy_failures(repo_root: Path, proxied: dict[str, dict],
                          pinned: dict, install_head: str) -> list[str]:
    """What is wrong with the release bx.toml links into rustup's home.

    One table, the release rust-toolchain.toml pins, read from that file;
    installed as every Rust table is, with exactly the components and targets
    that file lists, so rustup's proxy finds all it asks for and nothing else;
    and needed by every run, since any run may start cargo through the proxy.
    """
    failures: list[str] = []
    if len(proxied) != 1:
        return [f"{len(proxied)} [platform.linux.tools.*] table(s) link a Rust "
                f"toolchain at {RUSTUP_PROXY_PLACE} ({sorted(proxied)}), and one "
                f"does: without it a cargo a run starts outside a gate unit "
                f"installs the release through the account's rustup proxy on first "
                f"use, and two such runs race in that install"]
    name, spec = next(iter(proxied.items()))
    install = spec.get("install", "")
    shape = {
        "version_file": spec.get("version_file") == RUST_TOOLCHAIN_FILE,
        "install": install.startswith(install_head),
        "path": spec.get("path") == RUSTUP_PROXY_PLACE + "/bin/cargo",
        "check": spec.get("check") == "cargo {version}",
        "install_inputs": spec.get("install_inputs") == [RUST_TOOLCHAIN_INSTALL],
        "needed_by": not spec.get("needed_by"),
    }
    for key, ok in shape.items():
        if not ok:
            failures.append(
                f"[platform.linux.tools.{name}] {key} is {spec.get(key)!r}: the "
                f"release rustup's proxy finds is read from {RUST_TOOLCHAIN_FILE}, "
                f"installed by `{install_head} ...`, linked at {RUSTUP_PROXY_PLACE} "
                f"with its cargo checked there, keyed by {RUST_TOOLCHAIN_INSTALL}, "
                f"and needed by every run")
    channel = pinned.get("channel")
    version = bx_tool_version(repo_root, spec)
    if version != channel:
        failures.append(
            f"[platform.linux.tools.{name}] resolves to Rust {version!r} and "
            f"{RUST_TOOLCHAIN_FILE} pins {channel!r}: the proxy would still install "
            f"the pinned release on first use")
    on_line = rust_install_parts(install)
    for key in ("components", "targets"):
        want = set(pinned.get(key) or [])
        if on_line[key] != want:
            failures.append(
                f"[platform.linux.tools.{name}] installs the {key} "
                f"{sorted(on_line[key])} and {RUST_TOOLCHAIN_FILE} lists "
                f"{sorted(want)}: the toolchain rustup's proxy finds is the one that "
                f"file asks for, so the proxy installs no missing part on first use "
                f"and the toolchain is the one any other checkout's rustup installs")
    return failures


def verus_rust_release(repo_root: Path) -> str | None:
    """The Rust release docs/formal/proofs.json says Verus runs on."""
    manifest = repo_root / VERUS_MANIFEST
    if not manifest.is_file():
        return None
    return json.loads(manifest.read_text(encoding="utf-8")).get("verus", {}).get("toolchain")


def rust_toolchain_failures(repo_root: Path) -> list[str]:
    """What is wrong with the Rust toolchains bx.toml declares, as sentences."""
    failures: list[str] = []
    installed = {name: spec for name, spec in bx_tool_tables(repo_root).items()
                 if RUST_TOOLCHAIN_INSTALL in spec.get("install", "")}
    # The table linked into rustup's home is the proxy's release, held on its
    # own below; every other one is a toolchain a gate runs.
    proxied = {name: spec for name, spec in installed.items()
               if spec.get("prefix_link") == RUSTUP_PROXY_PLACE}
    tables = {name: spec for name, spec in installed.items() if name not in proxied}
    if not tables:
        return [f"bx.toml declares no Rust toolchain: no [platform.linux.tools.*] "
                f"table installs with {RUST_TOOLCHAIN_INSTALL}"]

    pin_text = (repo_root / RUST_TOOLCHAIN_PINS).read_text(encoding="utf-8")
    pins: dict[str, str] = {}
    nightly_pin = re.search(r'^SCE_RUST_NIGHTLY="([^"]+)"$', pin_text, re.M)
    if nightly_pin:
        pins["SCE_RUST_NIGHTLY"] = nightly_pin.group(1)
    else:
        failures.append(f"{RUST_TOOLCHAIN_PINS} has no SCE_RUST_NIGHTLY=\"...\" line")

    pinned = rust_toolchain_file(repo_root)
    channel = pinned.get("channel")
    if channel is None or not RUST_RELEASE.match(channel):
        failures.append(
            f"{RUST_TOOLCHAIN_FILE} pins {channel!r} on a `channel = \"...\"` line, "
            f"not a release such as 1.91.0: a channel that moves is a build no "
            f"one can repeat")
    else:
        pins["SCE_RUST_RELEASE"] = channel
    if re.search(r'^SCE_RUST_RELEASE="[0-9]', pin_text, re.M) \
            or RUST_TOOLCHAIN_FILE not in pin_text:
        failures.append(
            f"{RUST_TOOLCHAIN_PINS} does not read SCE_RUST_RELEASE from "
            f"{RUST_TOOLCHAIN_FILE}: a release written there too is a second pin, "
            f"and the gates would run one release while the tree builds with another")

    install_head = RUST_INSTALL_HEAD
    versions: dict[str, str] = {}
    for name, spec in sorted(tables.items()):
        version = bx_tool_version(repo_root, spec)
        if version is None:
            failures.append(f"[platform.linux.tools.{name}] resolves to no one version")
            continue
        versions[name] = version
        shape = {
            "install": spec.get("install", "").startswith(install_head),
            "path": spec.get("path") == RUST_TOOLCHAIN_PLACE + "/bin/cargo",
            "prefix_link": spec.get("prefix_link") == RUST_TOOLCHAIN_PLACE,
            "install_inputs": spec.get("install_inputs") == [RUST_TOOLCHAIN_INSTALL],
            "needed_by": bool(spec.get("needed_by")),
        }
        for key, ok in shape.items():
            if not ok:
                failures.append(
                    f"[platform.linux.tools.{name}] {key} is {spec.get(key)!r}: every "
                    f"Rust toolchain is installed by `{install_head} ...`, linked at "
                    f"{RUST_TOOLCHAIN_PLACE} where the gates look, keyed by "
                    f"{RUST_TOOLCHAIN_INSTALL}, and needed by the units named")
    failures.extend(rustup_proxy_failures(repo_root, proxied, pinned, install_head))

    by_version: dict[str, list[str]] = {}
    for name, version in versions.items():
        by_version.setdefault(version, []).append(name)
    for version, names in sorted(by_version.items()):
        if len(names) > 1:
            failures.append(
                f"Rust {version} is declared by {sorted(names)}: two tables of one "
                f"toolchain are two store entries linked at one place")

    def table_of(var: str):
        names = by_version.get(pins.get(var, ""), [])
        if not names:
            failures.append(f"no [platform.linux.tools.rust*] table declares "
                            f"{var} ({pins.get(var)}), so bx installs it nowhere")
        return names[0] if names else None

    release = table_of("SCE_RUST_RELEASE")
    nightly = table_of("SCE_RUST_NIGHTLY")
    if release and tables[release].get("version_file") != RUST_TOOLCHAIN_FILE:
        failures.append(f"[platform.linux.tools.{release}] does not read the release "
                        f"from {RUST_TOOLCHAIN_FILE}, the one line that pins it")
    if release:
        # What a checkout's rustup installs with the release, a build
        # machine's release has as well, or a gate passes on one and cannot
        # run on the other.
        on_line = rust_install_parts(tables[release].get("install", ""))
        for key, flag in (("components", "-c"), ("targets", "-t")):
            for item in sorted(set(pinned.get(key) or []) - on_line[key]):
                failures.append(
                    f"{RUST_TOOLCHAIN_FILE} lists {key[:-1]} {item!r} and the install "
                    f"line of [platform.linux.tools.{release}] has no `{flag} {item}`, "
                    f"so a build machine's release lacks what every other checkout's has")
    # The release Verus runs on (docs/formal/proofs.json), which is no package's
    # and not this tree's: one table, read from the manifest that pins Verus.
    pins["VERUS_RUST"] = verus_rust_release(repo_root) or ""
    verus = table_of("VERUS_RUST")
    if verus and tables[verus].get("version_file") != VERUS_MANIFEST:
        failures.append(f"[platform.linux.tools.{verus}] does not read the release "
                        f"Verus runs on from {VERUS_MANIFEST}, which pins Verus")
    if nightly and tables[nightly].get("version_file") != RUST_TOOLCHAIN_PINS:
        failures.append(f"[platform.linux.tools.{nightly}] does not read the nightly "
                        f"from {RUST_TOOLCHAIN_PINS}, the one line that pins it")

    # The release each package declares, and a table for each, in both directions.
    declared = workspace_rust_versions(repo_root)
    if not declared:
        failures.append("no workspace package declares a rust-version, so the "
                        "MSRV tables below are held to nothing")
    wanted: dict[str, list[str]] = {}
    for version, packages in declared.items():
        release_of = msrv_release(version)
        if release_of is None:
            failures.append(f"rust-version {version!r} of {packages} is not a release")
        else:
            wanted.setdefault(release_of, []).extend(packages)
    msrv = {name: version for name, version in versions.items()
            if name not in (release, nightly, verus)}
    for version in sorted(set(wanted) - set(msrv.values())):
        failures.append(
            f"{sorted(wanted[version])} declare rust-version {version} and no "
            f"[platform.linux.tools.rust*] table declares Rust {version}, so "
            f"`rust-version` could not run on any build machine")
    for name in sorted(n for n, v in msrv.items() if v not in wanted):
        failures.append(
            f"[platform.linux.tools.{name}] declares Rust {msrv[name]}, which no "
            f"workspace package names as its rust-version: a toolchain installed "
            f"for nothing, and a unit kept off every machine without it")

    # The units each table must be named by: the plan's unit for every gate,
    # and the gates whose scripts ask for a toolchain beyond the release.
    def asks(slug: str) -> str:
        script = gate_script(repo_root, slug)
        if not script.is_file():
            return ""
        code = script_code(script)
        if "sce_rust_msrv_release" in code:
            return "msrv"
        if VERUS_JUDGE in code:
            return "verus"
        if "sce_gate_require_rust " in code or "sce_gate_configure_wasm_tree" in code:
            return "nightly"
        return ""

    kind = {slug: asks(slug) for slug in GATES}
    lines = {}
    triggers = gate_triggers(repo_root)
    reads = gate_reads(repo_root)
    for slug in GATES:
        unit = plan_units({slug}, rust=pins.get("SCE_RUST_RELEASE", "0.0.0"),
                          head="0" * 40, triggers=triggers, reads=reads)
        lines[slug] = " ".join(unit[0]["argv"]) if unit else ""

    def named(name: str, line: str) -> bool:
        return any(e and e in line for e in tables[name].get("needed_by", []))

    for name in sorted(versions):
        if name == release:
            want_kind = None
        elif name == nightly:
            want_kind = "nightly"
        elif name == verus:
            want_kind = "verus"
        else:
            want_kind = "msrv"
        for slug in sorted(GATES):
            closure = {slug} | transitive_deps(slug)
            needs = want_kind is None or any(kind[s] == want_kind for s in closure)
            if needs and not named(name, lines[slug]):
                failures.append(
                    f"the unit `{lines[slug]}` needs [platform.linux.tools.{name}] "
                    f"and its needed_by {tables[name].get('needed_by')} does not "
                    f"name it, so bx may place it on a machine without that Rust")
            if not needs and named(name, lines[slug]):
                failures.append(
                    f"[platform.linux.tools.{name}]'s needed_by "
                    f"{tables[name].get('needed_by')} names the unit `{lines[slug]}`, "
                    f"which needs no such Rust, and keeps it off machines without it")
    if release and named(release, "bash scripts/gate --plan --all"):
        failures.append(f"[platform.linux.tools.{release}]'s needed_by names the plan "
                        f"command, which reads the pin and runs no compiler")

    # Nothing in the runner or a gate body installs any part of a toolchain.
    bodies = [repo_root / "scripts" / "gate"] + [gate_script(repo_root, s) for s in GATES]
    for body in bodies:
        if body.is_file() and RUSTUP_INSTALLS.search(script_code(body)):
            failures.append(
                f"{body.relative_to(repo_root)} installs a toolchain, a target or a "
                f"component through rustup; a build machine runs units side by side "
                f"in one rustup home, so what a gate needs is declared in bx.toml "
                f"and asked for through scripts/gates/lib.sh")
    return failures


# ── The Go release bx.toml declares ───────────────────────────────
#
# The Go modules declare a release (`go x.y.z` in each backends/go/*/go.mod)
# that the build machines' distribution Go is older than, and a run there
# relied on GOTOOLCHAIN=auto fetching it from the network at run time, which a
# machine that cannot fetch fails. So bx.toml declares the release as
# `[platform.linux.tools.go]`, installed by `scripts/install_go.sh` and chosen
# for a run by `sce_go_select` (scripts/lib/go_toolchain.sh). The claim has
# four halves that can each go stale without a gate turning red: the table
# reads the one go.mod line every module states, the install pins a digest for
# that release, the runs that start go choose it, and `needed_by` names exactly
# the units that start go. A unit it misses goes to a machine without the
# release; one it names for nothing is kept off such machines for nothing.
GO_TOOLCHAIN_INSTALL = "scripts/install_go.sh"
GO_TOOLCHAIN_LIB = "scripts/lib/go_toolchain.sh"
GO_VERSION_FILE = "backends/go/forge-runtime/go.mod"
GO_TOOLCHAIN_PLACE = "~/.local/share/go/{version}"
# The gates that start go, each with how. A gate whose script runs go itself is
# found by GO_COMMAND as well, so one added tomorrow cannot be missed here; the
# others start it through a program the scan cannot follow.
GO_GATES = {
    "forge-go": "runs `go test` over the forge runtime, and its generate.sh "
                "runs `go build`",
    "w3c-go": "runs `go test` over the generated W3C suite and the Go modules",
    "workspace-tests": "runs the sce-build tests and the C ABI suites that "
                       "build, vet and run Go (`forge_conformance` among them)",
    "mutation-rounds": "scripts/mutate builds a Go casefile's tests with "
                       "`go test -c`",
    "regen-reproduces": "runs every regeneration step, `go-round-trip` and "
                        "`forge-goldens` among them",
}
# The actions of bx.toml whose run starts go, each with how.
GO_ACTIONS = {
    "capture-cbor-pin": "`go run` of the Go CBOR driver",
    "regen-go-round-trip": "round_trip/generate.sh runs `go build`",
    "regen-forge-goldens": "`forge_conformance` builds and vets the Go it writes",
}
# The named commands of bx.toml whose run starts go, each with how. Such a
# command chooses the release on its own line (`sce_go_select`), which is what
# needed_by names it by.
GO_COMMANDS = {
    "verify": "`cargo test --workspace` runs the sce-build tests and the C ABI "
              "suites that build, vet and run Go",
}
# A go command in a script's code: a subcommand that builds or runs Go, or gofmt.
GO_COMMAND = re.compile(
    r"(?:^|[\s;&|(])go\s+(?:build|test|run|vet|generate|install|tool)\b|\bgofmt\b",
    re.M)


def bx_action_runs(repo_root: Path, kind: str = "actions") -> dict[str, str]:
    """The `run` line of each `[<kind>.<name>]` table of bx.toml, by name:
    the actions, or with `kind="commands"` the named commands.

    Read by its lines, as `bx_tool_tables` reads the tool tables: a run is one
    quoted string on one line.
    """
    runs: dict[str, str] = {}
    current = None
    for raw in (repo_root / "bx.toml").read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("["):
            header = re.fullmatch(rf"\[{re.escape(kind)}\.([A-Za-z0-9_.-]+)\]", line)
            current = header.group(1) if header else None
            continue
        pair = re.fullmatch(r"run\s*=\s*(['\"])(.*)\1", line)
        if current is not None and pair:
            runs[current] = pair.group(2)
    return runs


def go_gates_by_scan(repo_root: Path) -> set[str]:
    """The gates whose own script runs a go command (GO_COMMAND)."""
    return {slug for slug in GATES
            if gate_script(repo_root, slug).is_file()
            and GO_COMMAND.search(script_code(gate_script(repo_root, slug)))}


def setup_go_failures(lines: list[str], where: str) -> list[str]:
    """Each `actions/setup-go` step of a workflow's `lines` that does not read
    the release from GO_VERSION_FILE (`go-version-file:`), as sentences.

    A step's inputs are the lines after its `uses:` that are indented more
    deeply, or its `with:` at the same depth.
    """
    failures: list[str] = []
    for number, line in enumerate(lines):
        stripped = line.strip()
        if not re.match(r"(?:-\s+)?uses:\s*['\"]?actions/setup-go@", stripped):
            continue
        indent = len(line) - len(line.lstrip())
        inputs: list[str] = []
        for after in lines[number + 1:]:
            text = after.strip()
            if not text or text.startswith("#"):
                continue
            depth = len(after) - len(after.lstrip())
            if depth < indent or (depth == indent and not text.startswith("with:")):
                break
            inputs.append(text)
        if f"go-version-file: {GO_VERSION_FILE}" not in inputs \
                or any(i.startswith("go-version:") for i in inputs):
            failures.append(
                f"{where}:{number + 1} sets Go up without `go-version-file: "
                f"{GO_VERSION_FILE}`, so the lane runs another release than the one "
                f"the build machines run ([platform.linux.tools.go] in bx.toml)")
    return failures


def go_toolchain_failures(repo_root: Path, tables=None, actions=None,
                          commands=None) -> list[str]:
    """What is wrong with the Go release bx.toml declares, as sentences.

    `tables`, `actions` and `commands` replace bx.toml's tool tables, action
    run lines and named commands' run lines, so the self-test can show a
    drifted declaration refused.
    """
    tables = bx_tool_tables(repo_root) if tables is None else tables
    actions = bx_action_runs(repo_root) if actions is None else actions
    commands = bx_action_runs(repo_root, "commands") if commands is None else commands
    declared = {name: spec for name, spec in tables.items()
                if GO_TOOLCHAIN_INSTALL in spec.get("install", "")}
    if len(declared) != 1:
        return [f"{len(declared)} [platform.linux.tools.*] table(s) install with "
                f"{GO_TOOLCHAIN_INSTALL} ({sorted(declared)}), and one does: without "
                f"it a Go unit goes to any machine with some go, which fetches the "
                f"release the modules declare at run time or fails"]
    name, spec = next(iter(declared.items()))
    failures: list[str] = []
    shape = {
        "version_file": spec.get("version_file") == GO_VERSION_FILE,
        "path": spec.get("path") == GO_TOOLCHAIN_PLACE + "/sce-go-version",
        "prefix_link": spec.get("prefix_link") == GO_TOOLCHAIN_PLACE,
        "check": spec.get("check") == "go version go{version} linux",
        "install_inputs": spec.get("install_inputs") == [GO_TOOLCHAIN_INSTALL],
        "needed_by": bool(spec.get("needed_by")),
    }
    for key, ok in shape.items():
        if not ok:
            failures.append(
                f"[platform.linux.tools.{name}] {key} is {spec.get(key)!r}: the Go "
                f"release is read from {GO_VERSION_FILE}, linked at "
                f"{GO_TOOLCHAIN_PLACE} where {GO_TOOLCHAIN_LIB} looks, checked by the "
                f"`go version` its sce-go-version prints, keyed by "
                f"{GO_TOOLCHAIN_INSTALL}, and needed by the units named")

    version = bx_tool_version(repo_root, spec)
    if version is None or not RUST_RELEASE.match(version):
        failures.append(f"[platform.linux.tools.{name}] resolves to {version!r}, not "
                        f"one release such as 1.23.0 read from {GO_VERSION_FILE}")
        return failures

    # Every module builds on the one release, under GOTOOLCHAIN=local.
    for mod in sorted((repo_root / "backends" / "go").glob("*/go.mod")):
        lines = re.findall(r"^go (\S+)$", mod.read_text(encoding="utf-8"), re.M)
        if lines != [version]:
            failures.append(
                f"{mod.relative_to(repo_root)} states go {lines} and the declared "
                f"release is {version}: every Go run uses that one release with "
                f"GOTOOLCHAIN=local, so a module that asks for another fails there")

    # The install pins the digest of that release's archive.
    install = repo_root / GO_TOOLCHAIN_INSTALL
    install_text = install.read_text(encoding="utf-8") if install.is_file() else ""
    if not re.search(rf'"go{re.escape(version)}\.linux-amd64\.tar\.gz [0-9a-f]{{64}}"',
                     install_text):
        failures.append(
            f"{GO_TOOLCHAIN_INSTALL} pins no SHA-256 for go{version}.linux-amd64.tar.gz, "
            f"so provision refuses to install the release the modules declare")

    # The hosted lanes set up the release from the same line.
    for workflow in sorted((repo_root / ".github" / "workflows").glob("*.yml")):
        failures.extend(setup_go_failures(
            workflow.read_text(encoding="utf-8").splitlines(),
            str(workflow.relative_to(repo_root))))

    # What chooses the release reads it where the table does, and the runs that
    # start go choose it.
    lib = repo_root / GO_TOOLCHAIN_LIB
    lib_text = lib.read_text(encoding="utf-8") if lib.is_file() else ""
    if GO_VERSION_FILE not in lib_text or ".local/share/go/" not in lib_text:
        failures.append(f"{GO_TOOLCHAIN_LIB} does not read the release from "
                        f"{GO_VERSION_FILE} or look for it under ~/.local/share/go/")
    choosers = ["scripts/gate"] + sorted({
        actions[a].split()[0] for a in GO_ACTIONS if a in actions and actions[a].split()})
    for rel in choosers:
        path = repo_root / rel
        if not path.is_file() or "sce_go_select" not in script_code(path):
            failures.append(f"{rel} starts go and does not choose the declared release "
                            f"(`sce_go_select` of {GO_TOOLCHAIN_LIB})")

    def named(line: str) -> bool:
        return any(e and e in line for e in spec.get("needed_by", []))

    for slug in sorted(go_gates_by_scan(repo_root) - set(GO_GATES)):
        failures.append(f"scripts/gates/{slug}.sh runs go and GO_GATES does not "
                        f"name it, so needed_by is held to nothing for it")
    triggers = gate_triggers(repo_root)
    reads = gate_reads(repo_root)
    for slug in sorted(GATES):
        unit = plan_units({slug}, rust="0.0.0", head="0" * 40,
                          triggers=triggers, reads=reads)
        line = " ".join(unit[0]["argv"]) if unit else ""
        needs = bool(({slug} | transitive_deps(slug)) & set(GO_GATES))
        if needs and not named(line):
            failures.append(f"the unit `{line}` starts go and needed_by "
                            f"{spec.get('needed_by')} does not name it, so bx may "
                            f"place it on a machine without Go {version}")
        if not needs and named(line):
            failures.append(f"needed_by {spec.get('needed_by')} names the unit "
                            f"`{line}`, which starts no go, and keeps it off "
                            f"machines without Go {version}")
    for action in sorted(set(GO_ACTIONS) - set(actions)):
        failures.append(f"GO_ACTIONS names `{action}`, which bx.toml does not declare")
    for action, run in sorted(actions.items()):
        needs = action in GO_ACTIONS
        if needs and not named(run):
            failures.append(f"the action `{action}` starts go and needed_by "
                            f"{spec.get('needed_by')} does not name its run `{run}`")
        if not needs and named(run):
            failures.append(f"needed_by {spec.get('needed_by')} names the run `{run}` "
                            f"of `{action}`, which starts no go")
    for command in sorted(set(GO_COMMANDS) - set(commands)):
        failures.append(f"GO_COMMANDS names `{command}`, which bx.toml does not declare")
    for command, run in sorted(commands.items()):
        needs = command in GO_COMMANDS
        if needs and "sce_go_select" not in run:
            failures.append(f"[commands.{command}] starts go and does not choose the "
                            f"declared release on its line (`sce_go_select` of "
                            f"{GO_TOOLCHAIN_LIB})")
        if needs and not named(run):
            failures.append(f"[commands.{command}] starts go and needed_by "
                            f"{spec.get('needed_by')} does not name its run")
        if not needs and named(run):
            failures.append(f"needed_by {spec.get('needed_by')} names the run of "
                            f"[commands.{command}], which starts no go")
    if named("bash scripts/gate --plan --all"):
        failures.append(f"needed_by {spec.get('needed_by')} names the plan command, "
                        f"which runs no go")
    return failures


# ── The ARM compiler bx.toml declares ─────────────────────────────
#
# `c11-mcu` holds each machine's bytes to backends/c/mcu/footprint.budget, and
# those figures are the compiler's, so the compiler is a tool with pinned bytes
# (bx RFC 0009, section 4.8): Arm's tarball of the release
# `scripts/install_arm_gnu_toolchain.sh` pins with a digest per host. The claim
# has five halves that can each go stale without a gate turning red: the table
# reads the script's release line and checks Arm's build by name, the script pins
# a digest for every host it accepts, the gate puts that release first on PATH
# and refuses another build, the hosted lane installs it with the same script,
# and the budget names the build its figures are measured with. QEMU is no tool
# here: its release does not move the verdict, so it is `packages` of the gate.
ARM_TOOLCHAIN_INSTALL = "scripts/install_arm_gnu_toolchain.sh"
ARM_TOOLCHAIN_PLACE = "~/.local/share/arm-gnu-toolchain/{version}"
ARM_TOOLCHAIN_GATE = "c11-mcu"
ARM_TOOLCHAIN_BUDGET = "backends/c/mcu/footprint.budget"
# The hosts the install accepts, as Arm names them in its archives.
ARM_TOOLCHAIN_HOSTS = ("x86_64", "aarch64")
# The shape of an Arm GNU Toolchain release (`13.2.rel1`).
ARM_TOOLCHAIN_RELEASE = re.compile(r"^[0-9]+\.[0-9]+\.rel[0-9]+$")


def arm_toolchain_failures(repo_root: Path, tables=None) -> list[str]:
    """What is wrong with the ARM compiler bx.toml declares, as sentences.

    `tables` replaces bx.toml's tool tables, so the self-test can show a
    drifted declaration refused.
    """
    tables = bx_tool_tables(repo_root) if tables is None else tables
    declared = {name: spec for name, spec in tables.items()
                if ARM_TOOLCHAIN_INSTALL in spec.get("install", "")}
    if len(declared) != 1:
        return [f"{len(declared)} [platform.linux.tools.*] table(s) install with "
                f"{ARM_TOOLCHAIN_INSTALL} ({sorted(declared)}), and one does: without "
                f"it `{ARM_TOOLCHAIN_GATE}` goes to a machine whose compiler is "
                f"whatever release it carries, and its budget is one build's"]
    name, spec = next(iter(declared.items()))
    failures: list[str] = []
    shape = {
        "version_file": spec.get("version_file") == ARM_TOOLCHAIN_INSTALL,
        "path": spec.get("path") == ARM_TOOLCHAIN_PLACE + "/bin/arm-none-eabi-gcc",
        "prefix_link": spec.get("prefix_link") == ARM_TOOLCHAIN_PLACE,
        "check": spec.get("check") == "Arm GNU Toolchain {version}",
        "install_inputs": spec.get("install_inputs") == [ARM_TOOLCHAIN_INSTALL],
        "needed_by": spec.get("needed_by") == [ARM_TOOLCHAIN_GATE],
    }
    for key, ok in shape.items():
        if not ok:
            failures.append(
                f"[platform.linux.tools.{name}] {key} is {spec.get(key)!r}: the "
                f"release is read from {ARM_TOOLCHAIN_INSTALL}, linked at "
                f"{ARM_TOOLCHAIN_PLACE} where the gate looks, checked by the name "
                f"Arm's build prints, keyed by {ARM_TOOLCHAIN_INSTALL}, and needed "
                f"by `{ARM_TOOLCHAIN_GATE}` alone")

    version = bx_tool_version(repo_root, spec)
    if version is None or not ARM_TOOLCHAIN_RELEASE.match(version):
        failures.append(f"[platform.linux.tools.{name}] resolves to {version!r}, not "
                        f"one release such as 13.2.rel1 read from {ARM_TOOLCHAIN_INSTALL}")
        return failures
    build = f"Arm GNU Toolchain {version}"

    # The install pins the digest of that release's archive for every host.
    install = repo_root / ARM_TOOLCHAIN_INSTALL
    install_text = install.read_text(encoding="utf-8") if install.is_file() else ""
    for host in ARM_TOOLCHAIN_HOSTS:
        archive = f"arm-gnu-toolchain-{version}-{host}-arm-none-eabi.tar.xz"
        if not re.search(rf'"{re.escape(archive)} [0-9a-f]{{64}}"', install_text):
            failures.append(f"{ARM_TOOLCHAIN_INSTALL} pins no SHA-256 for {archive}, "
                            f"so provision refuses to install {build} on a {host} host")

    # The gate reads the release from the script's line, puts it first on PATH
    # and refuses another build.
    script = gate_script(repo_root, ARM_TOOLCHAIN_GATE)
    code = script_code(script) if script.is_file() else ""
    for needle, what in (
            (ARM_TOOLCHAIN_INSTALL, f"read the release from {ARM_TOOLCHAIN_INSTALL}"),
            (".local/share/arm-gnu-toolchain/", "put the release's bin/ on PATH"),
            ("Arm GNU Toolchain $ARM_GNU_TOOLCHAIN_REV", "refuse another build")):
        if needle not in code:
            failures.append(f"scripts/gates/{ARM_TOOLCHAIN_GATE}.sh does not {what} "
                            f"(no `{needle}` in its code)")

    # The hosted lane installs the same build with the same script, and not the
    # distribution's.
    for workflow in GATES.get(ARM_TOOLCHAIN_GATE, {}).get("workflows", []):
        path = repo_root / ".github" / "workflows" / workflow
        lines = [line.strip() for line in
                 (path.read_text(encoding="utf-8").splitlines() if path.is_file() else [])
                 if not line.strip().startswith("#")]
        if not any("run:" in line and ARM_TOOLCHAIN_INSTALL in line for line in lines):
            failures.append(f".github/workflows/{workflow} runs `{ARM_TOOLCHAIN_GATE}` and "
                            f"does not install {build} with {ARM_TOOLCHAIN_INSTALL}")
        if any("apt-get install" in line and "gcc-arm-none-eabi" in line.split()
               for line in lines):
            failures.append(f".github/workflows/{workflow} installs the distribution's "
                            f"gcc-arm-none-eabi, another build than {build}")

    # The budget names the build its figures are measured with.
    budget = repo_root / ARM_TOOLCHAIN_BUDGET
    measured = [line for line in (budget.read_text(encoding="utf-8").splitlines()
                                  if budget.is_file() else [])
                if line.startswith("MEASURED_WITH=")]
    if len(measured) != 1 or f"({build} " not in measured[0]:
        failures.append(f"{ARM_TOOLCHAIN_BUDGET} states MEASURED_WITH {measured}, which "
                        f"does not name {build}, the build the gate runs")
    return failures


def distribution_link_failures(tables: dict[str, dict]) -> list[str]:
    """Each tool table whose install links a file of the distribution, as a
    sentence.

    bx keys a tool by its declared version and checks it by what `path`
    prints, and a link into /usr is a file apt can replace under the same key
    (bx RFC 0009, section 4.8). A program whose release moves the verdict is a
    tool with pinned bytes; one whose release does not is a requirement of the
    unit (`packages` of a gate), not a tool.
    """
    return [f"[platform.linux.tools.{name}] installs by linking a file of the "
            f"distribution ({spec.get('install')!r}): its bytes are apt's and not "
            f"the declaration's; pin a tarball, or make it `packages` of the gate"
            for name, spec in sorted(tables.items())
            if re.search(r"\bln\s+-[a-zA-Z]*s[a-zA-Z]*\s+/usr/", spec.get("install", ""))]


def print_self_test_failures(failures: list[str]) -> int:
    """Print what the self-test found wrong, and the exit status it ends with."""
    for f in failures:
        print(f"SELF-TEST FAIL: {f}", file=sys.stderr)
    return 1


def self_test(repo_root: Path) -> int:
    """Cases that pin the rules the registry must not break."""
    failures = []
    cases = 0

    def check(label, changed, want_full=None, want_has=(), want_lacks=(),
              landing=False):
        nonlocal cases
        cases += 1
        keys, reason = select(repo_root, changed, landing=landing)
        # "Full" is every gate the push can run. `select` is what the hook
        # calls, so for a push of another ref a `ci_only` gate is not missing
        # from its answer — it was never on offer, and counting it would make
        # rule 1 unprovable here. For an update to main (`landing`) the offer
        # is every gate the fleet can run (`push_keeps`).
        offered = push_keeps(repo_root, set(GATES), landing)
        full = len(keys) == len(offered)
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
          want_has=["codegen-build", "clippy"],
          want_lacks=["workspace-tests"])
    # The same edit on an update to main: the landing queue's judgment runs
    # the `ci_only` gates it reaches, so the workspace suite joins clippy in
    # its answer, and it is still scoped (not every gate). A landing that
    # dropped a `ci_only` gate for a Rust edit would land what it judges
    # unjudged.
    check("rust-landing", ["sce-build/src/mesh/deploy.rs"], want_full=False,
          want_has=["codegen-build", "workspace-tests", "clippy"],
          landing=True)
    # An integration test of the generator crate is compiled by the workspace
    # suite and clippy, and by no build of the generator or the C++ tree.
    check("crate-test-landing", ["sce-build/tests/gate_registry_contract.rs"],
          want_full=False, want_has=["workspace-tests", "clippy"],
          want_lacks=["cpp-suite-wasm", "w3c-cpp", "cpp-suite", "w3c-c11",
                      "w3c-wasm", "install-consumer", "cpp-interface-standard",
                      "forge-rust", "drift-suites", "c11-mcu",
                      "example-codegen", "targets", "doc-content",
                      "generated-deterministic"],
          landing=True)
    # Rule 1 on an update to main: every gate the fleet can run, `ci_only`
    # included, and never a gate hosted CI audits in its place.
    check("unclassified-landing", ["some_new_top_level_dir/thing.txt"],
          want_full=True, want_has=["cpp-suite-wasm", "clippy"],
          want_lacks=["windows-msvc", "mutation-rounds"], landing=True)
    # A document under `docs/` that no path-scoped gate reads does not buy the
    # whole table on an update to main, and the two that a suite reads select
    # that suite.
    for doc in ("docs/SCE_CODEC_CONSUMER_MIGRATION.md",
                "docs/SCE_TEST_PROFILE.md"):
        check("document-landing", [doc], want_full=False,
              want_has=["tree-hygiene"], want_lacks=["workspace-tests"],
              landing=True)
    for doc in ("docs/SCE_LUA_TRANSLATION_SEAM.md",
                "docs/SCE_TTCN3_LANE.md"):
        check("read-document-landing", [doc], want_full=False,
              want_has=["tree-hygiene", "workspace-tests"], landing=True)
    # Dependency closure: an example-only change still builds sce-codegen.
    # `example-codegen` is `ci_only` now, but the closure it pulls in is not:
    # the dependency must survive its dependent leaving, or a gate that
    # remains local loses the binary it executes.
    check("example-dep", ["examples/smart_light/smart_light.scxml"],
          want_has=["codegen-build"], want_lacks=["example-codegen"])
    check("example-dep-landing", ["examples/smart_light/smart_light.scxml"],
          want_has=["codegen-build", "example-codegen"], landing=True)
    # A template edit must reach the committed-tree drift gate.
    check("template", ["tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2"],
          want_full=False, want_has=["codegen-build"],
          want_lacks=["drift-suites"])
    check("template-landing",
          ["tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2"],
          want_full=False, want_has=["codegen-build", "drift-suites"],
          landing=True)
    # doc-check.yml edits must reach the gate that mirrors it. Under the
    # numbered table this gate was mapped to the no_std workflow, so an edit
    # to doc-check.yml alone did not select it.
    cases += 1
    keys, _ = select(repo_root, [".github/workflows/doc-check.yml"])
    if "rustdoc-links" not in keys:
        failures.append("doc-check-self: rustdoc-links missing from " + str(keys))

    # ── The judgment of an update to main ───────────────────────────
    #
    # a-landing-runs-the-ci-only-gates-it-reaches. The landing queue judges a
    # candidate by the hook's selection for `refs/heads/main`, and that
    # selection keeps every reached `ci_only` gate (see "What `ci_only`
    # means"). A Rust source edit reaches `workspace-tests`, which is
    # `ci_only`: the landing must run it and a push of another ref must not.
    # (`clippy` was the second witness until 848cafed39 took it out of
    # `ci_only`.) A selection that went on dropping `ci_only` for main is the
    # state that let cdba8e0eb0 land on 7 gates while 34 it reached ran after.
    cases += 1
    rust_path = ["sce-build/src/mesh/deploy.rs"]
    landing_keys, _ = select(repo_root, rust_path, landing=True)
    branch_keys, _ = select(repo_root, rust_path)
    for slug in ("workspace-tests",):
        if not GATES[slug].get("ci_only"):
            failures.append(
                f"landing: {slug} is no longer ci_only, so this case cannot "
                f"tell the two selections apart; pick a ci_only witness")
        if slug not in landing_keys:
            failures.append(
                f"landing: an update to main that edits {rust_path[0]} does "
                f"not run {slug}, a ci_only gate the change reaches; the "
                f"queue's judgment would land it unjudged: {landing_keys}")
        if slug in branch_keys:
            failures.append(
                f"landing: a push of another ref runs {slug}, which is "
                f"ci_only; only an update to main keeps those gates")
    # Still path-scoped: the landing keeps what the change REACHES, so a
    # docs-only edit does not buy the Rust workspace or the C++ suite, and it
    # still runs the always-on tree-wide gates.
    cases += 1
    docs_keys, _ = select(repo_root, ["README.md"], landing=True)
    for slug in ("workspace-tests", "cpp-suite"):
        if slug in docs_keys:
            failures.append(
                f"landing: a README edit selects {slug} on an update to main; "
                f"the landing keeps ci_only gates, it does not stop scoping")
    if "tree-hygiene" not in docs_keys:
        failures.append(
            "landing: a README edit does not run tree-hygiene on an update to "
            "main, and that gate rides an unfiltered workflow: " + str(docs_keys))
    # The notice follows the selection: a gate the landing ran is not "handed
    # to CI", and on a push of another ref the same gate is.
    cases += 1
    if "workspace-tests" in ci_owed(repo_root, rust_path, landing=True):
        failures.append(
            "landing: --ci-owed names workspace-tests as handed to CI on an "
            "update to main, which runs it")
    if "workspace-tests" not in ci_owed(repo_root, rust_path) \
            and "workspace-tests" not in ci_unowed(repo_root, rust_path):
        failures.append(
            "landing: on a push of another ref, workspace-tests is reached and "
            "left out, and neither --ci-owed nor --ci-unowed names it")

    # a-landing-leaves-out-only-the-gates-hosted-ci-audits. An unclassified
    # path reaches every gate (rule 1), so the landing's answer there is exactly
    # the table less the gates whose platform bx.toml does not declare and the
    # ones that say they are audited after landing, and what it leaves out is
    # exactly those.
    cases += 1
    platforms = fleet_platforms(repo_root)
    if not platforms:
        failures.append(
            "platform: no `[platform.<name>]` table was read from bx.toml, so "
            "every gate that names a platform reads as off the fleet")
    off_fleet = {s for s in GATES
                 if GATES[s].get("platform") is not None
                 and GATES[s]["platform"] not in platforms}
    audited = {s for s in GATES if GATES[s].get("audited_after_landing")}
    left_to_ci = off_fleet | audited
    unclassified = ["some_new_top_level_dir/thing.txt"]
    full_keys, _ = select(repo_root, unclassified, landing=True)
    if set(full_keys) != set(GATES) - left_to_ci:
        failures.append(
            f"landing: an unclassified path on an update to main runs "
            f"{len(full_keys)} gate(s), not the {len(GATES) - len(left_to_ci)} "
            f"the fleet can run and bound; missing="
            f"{sorted(set(GATES) - left_to_ci - set(full_keys))} "
            f"extra={sorted(set(full_keys) & left_to_ci)}")
    if left_out(repo_root, unclassified, landing=True) != left_to_ci:
        failures.append(
            f"landing: what an update to main leaves out is "
            f"{sorted(left_out(repo_root, unclassified, landing=True))}, not "
            f"the gates off the fleet and the audited ones {sorted(left_to_ci)}")
    # The filter itself, on a fixture, so it keeps measuring when the live
    # table has no gate off the fleet.
    cases += 1
    platform_fixture = {
        "anywhere": {},
        "on-linux": {"platform": "linux"},
        "on-macos": {"platform": "macos"},
        "audited": {"audited_after_landing": "its cost has no bound"},
    }
    kept = drop_hosted_only(set(platform_fixture), {"linux"},
                            table=platform_fixture)
    if kept != {"anywhere", "on-linux"}:
        failures.append(
            f"platform-fixture: with only `linux` declared the fleet keeps "
            f"{sorted(kept)}; a gate naming no platform and a gate naming a "
            f"declared one stay, a gate naming another goes, and so does one "
            f"audited after landing")

    # a-gate-audited-after-landing-is-ci-only-and-says-why. It is left out of
    # the landing for its cost, so it has to be `ci_only` (no push of another
    # ref can run it either), it has to name the workflow that audits it, and
    # the judgment prints the reason, so it has to be one.
    cases += 1
    for slug, spec in sorted(GATES.items()):
        if "audited_after_landing" not in spec:
            continue
        if not spec.get("ci_only"):
            failures.append(
                f"audited: {slug} is left out of a landing and is not "
                f"ci_only; a push of another ref would run it")
        if not spec.get("workflows"):
            failures.append(
                f"audited: {slug} is left to hosted CI after landing and "
                f"names no workflow, so nothing judges it")
        why = spec["audited_after_landing"]
        if not isinstance(why, str) or len(why.strip()) < 40:
            failures.append(
                f"audited: {slug} is audited after landing with no reason "
                f"saying what bounds its cost in no landing")
    if not audited:
        failures.append(
            "audited: no gate says it is audited after landing, so the rule "
            "above is held by nothing")

    # a-gate-off-the-fleet-is-ci-only-and-says-why. Such a gate is left out of
    # the landing, so it has to be `ci_only` (no push of another ref can run
    # it on the fleet either), it has to name the workflow that audits it,
    # and the reason is printed in the judgment, so it has to be one.
    cases += 1
    for slug, spec in sorted(GATES.items()):
        if "platform" not in spec:
            continue
        if not isinstance(spec["platform"], str) or not spec["platform"]:
            failures.append(
                f"platform: {slug} declares platform={spec['platform']!r}; it "
                f"is the name of a platform, as bx.toml spells one")
            continue
        if spec["platform"] in platforms:
            continue
        if not spec.get("ci_only"):
            failures.append(
                f"platform: {slug} runs on {spec['platform']!r}, which bx.toml "
                f"does not declare, and is not ci_only; no push can run it")
        if not spec.get("workflows"):
            failures.append(
                f"platform: {slug} is off the fleet and names no workflow, so "
                f"nothing judges it before or after landing")
        why = spec.get("platform_reason")
        if not isinstance(why, str) or len(why.strip()) < 40:
            failures.append(
                f"platform: {slug} is off the fleet with no `platform_reason` "
                f"saying what that runner has that the fleet does not")

    # a-gate-in-an-image-runs-there-with-what-only-it-needs. On a fixture: a
    # dependency only the gate in an image needs goes in with it, and one a
    # gate on the host also needs stays on the host.
    cases += 1
    image_fixture = {
        "build": {},
        "on-host": {"deps": ["build"]},
        "boxed": {"deps": ["build"], "container": "x/Containerfile"},
    }
    alone = container_placement(["build", "boxed"], table=image_fixture)
    if alone != [("container", "boxed", "x/Containerfile")]:
        failures.append(
            f"placement-fixture: a gate in an image and its own dependency were "
            f"placed as {alone}; the dependency runs inside, with it")
    mixed = container_placement(["build", "on-host", "boxed"],
                                table=image_fixture)
    if mixed != [("host", "build", ""), ("host", "on-host", ""),
                 ("container", "boxed", "x/Containerfile")]:
        failures.append(
            f"placement-fixture: with a gate on the host that needs the same "
            f"dependency, the run was placed as {mixed}")

    # a-gate-in-an-image-says-so-everywhere. Its Containerfile is a tracked
    # file, an edit of it selects the gate, and its unit asks bx for docker,
    # which a build machine has only under a rootless daemon.
    cases += 1
    image_gates = 0
    image_triggers = gate_triggers(repo_root)
    for slug, spec in sorted(GATES.items()):
        containerfile = spec.get("container")
        if containerfile is None:
            continue
        image_gates += 1
        if not isinstance(containerfile, str) \
                or not (repo_root / containerfile).is_file():
            failures.append(
                f"container: {slug} runs in the image {containerfile!r} "
                f"builds, and there is no such file")
            continue
        pats = [glob_to_regex(g) for g in image_triggers[slug]]
        if not any(p.match(containerfile) for p in pats):
            failures.append(
                f"container: {slug} runs in the image {containerfile} builds, "
                f"and no trigger of its own selects an edit of that file")
        if CONTAINER_NEED not in spec.get("needs", []):
            failures.append(
                f"container: {slug} runs in an image and its unit does not ask "
                f"bx for {CONTAINER_NEED!r}, so it could go to a machine that "
                f"cannot start one")
    if image_gates == 0:
        failures.append(
            "container: no gate runs in an image, so this case read nothing; "
            "cpp-interface-standard does")

    # a-gate-image-builds-on-the-selected-rust. No image installs Rust: a
    # toolchain installed in the image is no release the unit's key names. The
    # runner hands its own selection in instead, mounted read-only where the
    # runner inside looks, with the release as `--rust`.
    cases += 1
    for slug, spec in sorted(GATES.items()):
        containerfile = spec.get("container")
        if not isinstance(containerfile, str) \
                or not (repo_root / containerfile).is_file():
            continue
        for line in script_code(repo_root / containerfile).splitlines():
            if IMAGE_INSTALLS_RUST.search(line):
                failures.append(
                    f"container: {containerfile} (the image of {slug}) "
                    f"installs or runs Rust ({line.strip()[:70]!r}); the "
                    f"runner mounts the release it selected, and an image's "
                    f"own would be a compiler the unit's key does not name")
                break
    runner_file = repo_root / "scripts" / "gate"
    runner_text = script_code(runner_file) if runner_file.is_file() else ""
    in_container = re.search(r"^gate_in_container\(\)\s*\{\n(.*?)^\}",
                             runner_text, re.M | re.S)
    if in_container is None:
        failures.append(
            "container: scripts/gate defines no `gate_in_container`, so no gate "
            "in an image can run, and this case reads nothing")
    else:
        for mark in CONTAINER_RUNNER_MARKS:
            if mark not in in_container.group(1):
                failures.append(
                    f"container: `gate_in_container` in scripts/gate no longer "
                    f"holds {mark!r}: the gate in the image would not run on "
                    f"the Rust this run selected")

    # the-fleet-and-the-lanes-install-one-emsdk. The build machines install
    # the release `scripts/install_emsdk.sh` pins, and the hosted lanes name
    # one in their own `emsdk install` steps. A landing judged on one release
    # and audited on another would compare two compilers, so every step names
    # the script's.
    cases += 1
    emsdk_script = repo_root / "scripts" / "install_emsdk.sh"
    pin = None
    if emsdk_script.is_file():
        found = re.search(r'^EMSDK_REV="([0-9.]+)"$',
                          emsdk_script.read_text(encoding="utf-8"), re.M)
        pin = found.group(1) if found else None
    if pin is None:
        failures.append(
            "emsdk: no EMSDK_REV line in scripts/install_emsdk.sh, the pin "
            "bx.toml's emsdk tool reads")
    else:
        named = 0
        for workflow in sorted((repo_root / ".github" / "workflows").glob("*.yml")):
            text = workflow.read_text(encoding="utf-8", errors="replace")
            for step in re.finditer(r"emsdk (?:install|activate) (\S+)", text):
                named += 1
                if step.group(1) != pin:
                    failures.append(
                        f"emsdk: {workflow.name} names emsdk {step.group(1)} "
                        f"and the build machines install {pin} "
                        f"(scripts/install_emsdk.sh)")
        if named == 0:
            failures.append(
                "emsdk: no workflow names an `emsdk install` release, so this "
                "case read nothing")

    # the-fleet-runs-the-wasm-bindgen-cargo-lock-holds. The tool refuses a
    # module another version of the crate built. The build machines install
    # the version scripts/install_wasm_bindgen.sh names (bx reads a version
    # from one line, which Cargo.lock does not give), and the hosted lane the
    # one Cargo.lock holds, so the two are held to one here, in the commit that
    # moves either.
    cases += 1
    bindgen_script = repo_root / "scripts" / "install_wasm_bindgen.sh"
    bindgen_pin = None
    if bindgen_script.is_file():
        found = re.search(r'^WASM_BINDGEN_REV="([0-9.]+)"$',
                          bindgen_script.read_text(encoding="utf-8"), re.M)
        bindgen_pin = found.group(1) if found else None
    lock = (repo_root / "Cargo.lock").read_text(encoding="utf-8")
    locked = re.findall(r'^name = "wasm-bindgen"\nversion = "([^"]+)"$', lock,
                        re.M)
    if bindgen_pin is None:
        failures.append(
            "wasm-bindgen: no WASM_BINDGEN_REV line in "
            "scripts/install_wasm_bindgen.sh, the pin bx.toml's tool reads")
    elif len(locked) != 1:
        failures.append(
            f"wasm-bindgen: Cargo.lock holds {len(locked)} wasm-bindgen "
            f"package(s), not one, so the tool's version is not defined")
    elif locked[0] != bindgen_pin:
        failures.append(
            f"wasm-bindgen: Cargo.lock holds {locked[0]} and the build "
            f"machines install {bindgen_pin} (scripts/install_wasm_bindgen.sh); "
            f"raise the script's WASM_BINDGEN_REV with the lock")

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
    # A sweep extractor that stopped matching would leave the loop above
    # reading nothing and still passing, so the extractor answers for
    # itself here rather than being taken on the tree's word.
    #
    # ⚠ THIS USED TO REQUIRE A LIVE GATE TO CARRY ONE (`sweeps_seen >= 1`),
    # and that is a lower bound a repair can break: on 2026-09-22 the last
    # such loop left `example-codegen` for `sce-build/tests/cli_lint_sweep.rs`,
    # where the same sweep is derived and runs on every push rather than in
    # one workflow — and this case failed, reporting a broken extractor for
    # a tree that had simply stopped containing the shape. The four gates
    # that do enumerate the tree (`tree-hygiene`, `mutation-cases`,
    # `mutation-rounds`, `ledger-citations`) reach it through forms this
    # extractor deliberately does not read, and they are registered
    # unfilterable instead, which is the other half of the same contract.
    probe = "git ls-" + "files 'examples/*.scxml' 'integration_resources/*/*.scxml'"
    extracted = swept_globs(probe)
    cases += 1
    if extracted != ["examples/*.scxml", "integration_resources/*/*.scxml"]:
        failures.append(
            "trigger: the sweep extractor no longer reads a quoted-glob "
            f"`ls-files` call — it answered {extracted} for a call carrying "
            "two globs, so the loop above says nothing about any gate")

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

    # How much of that ceiling is declared in figures no run can check.
    #
    # A `PACE_NORMALISED` cost is a share of a full push, and `budget_breach`
    # can only compare a declaration against a measurement of the same kind —
    # so those seconds sit inside the ceiling with nothing able to say they
    # are still true. `drift_report_lines` already calls an empty map this
    # work's success condition; this is that sentence as a number, so the
    # share moves in the direction the prose named instead of drifting.
    cases += 1
    # This fails in BOTH directions, for the reason UNMEASURED_COST_CEILING
    # below spells out: a ratchet that only catches growth is one nobody ever
    # lowers, and the hole it guards then sits at whatever size it reached.
    # Growth means another cost became uncheckable; a drop means somebody
    # re-timed a gate honestly and left the ceiling behind.
    paced_local = sum((GATES[s].get("cost_s") or 0)
                      for s in local if s in PACE_NORMALISED)
    share = paced_local / worst if worst else 0.0
    if abs(share - PACED_BUDGET_SHARE_CEILING) > 0.005:
        carrying = sorted((((GATES[s].get("cost_s") or 0), s) for s in local
                           if s in PACE_NORMALISED), reverse=True)
        grew = share > PACED_BUDGET_SHARE_CEILING
        failures.append(
            f"paced-share: {paced_local}s of the {worst}s a push can select "
            f"is declared in pace-normalised figures — {share:.1%} against a "
            f"{PACED_BUDGET_SHARE_CEILING:.1%} ratchet. Those costs are "
            f"shares of a full push, so no run can check them; carrying it: "
            + ", ".join(f"{s} ({c}s)" for c, s in carrying[:3])
            + (". Another cost became uncheckable — a paced entry is a last "
               "resort, not a place to put a figure that was awkward to take."
               if grew else
               ". The share dropped and the ceiling was left behind — LOWER "
               "PACED_BUDGET_SHARE_CEILING to what it now is, which is what "
               "makes the shrinking visible instead of silent."))

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
        # What the gate builds (`builds`) is its trigger as much as `extra`
        # is, so a filtered workflow is held to the derived list too.
        built = [g for b in spec.get("builds") or []
                 for g in built_inputs(repo_root, b)]
        for own in (spec.get("local") or []) + (spec.get("extra") or []) + built:
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
                    # `as_posix`: a trigger is a glob written with `/`, and a path
                    # spelled with the platform's own separator (`schemas\x.json` on
                    # Windows) matches none of them, which read as a hundred inputs
                    # that no lane starts for (measured 2026-10-03, the first run of
                    # this runner on Windows).
                    path = resolved.relative_to(repo_root).as_posix()
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

    # The same question one layer down, for the gates that ARE a shell
    # script. `include-str-coverage` above asks it of a Rust suite's
    # compiled-in inputs; a gate script's inputs are the tracked files it
    # opens, and nothing asked whether those are declared.
    #
    # What it costs when they are not is not hypothetical. `w3c-kotlin`
    # reads `tests/ecmascript/kotlin_frontend_refusals.json` and declares
    # neither it nor the fixture registry it walks, so a commit editing only
    # a divergence list was told — by `--ci-owed`, the report a round reads
    # to know which lanes will judge it — that w3c-tests.yml was not one of
    # them. The lane still ran, because that workflow declares no filter, so
    # the answer was wrong in the one direction nothing notices: a reader
    # looking at the wrong lane for a verdict that did exist.
    #
    # ⚠ The registry's own memory of this defect said the push hook was
    # failing to SELECT the gate. Measured 2026-09-03, that was false —
    # `w3c-kotlin` is `ci_only`, so no push selects it whatever it declares.
    # The report is what the missing declaration breaks.
    #
    # Three of the four gates this finds are `ci_only`, so declaring their
    # inputs costs a push nothing and buys the report its answer.
    cases += 1
    tracked_all = set(tracked_paths(repo_root))
    scripts_seen = 0
    quoted_seen = 0
    bare_seen = 0
    triggers_by_slug = gate_triggers(repo_root)
    for slug in sorted(GATES):
        script = gate_script(repo_root, slug)
        if not script.is_file():
            continue
        scripts_seen += 1
        pats = [glob_to_regex(g) for g in triggers_by_slug.get(slug, [])]
        quoted, bare = script_read_paths(script_code(script), tracked_all)
        quoted_seen += len(quoted)
        bare_seen += len(bare)
        for path in sorted(quoted | bare):
            if any(p.match(path) for p in pats):
                continue
            failures.append(
                f"script-read-coverage: {slug} reads '{path}' and declares no "
                f"trigger matching it, so a commit touching only that file is "
                f"not told this gate judges it. Add the path to the gate's "
                f"`local`/`extra`, or stop naming it as a value.")
    # A floor per arm, not one over the union. Blinding either half leaves
    # the other filling the count, and a scan that stopped looking then
    # wears the shape of a scan that found nothing to report — which is the
    # same failure `cli_feature_gating`'s sweeping arm was caught by.
    if scripts_seen < 1:
        failures.append(
            "script-read-coverage: no gate script was found — the case read "
            "nothing")
    if quoted_seen < 1:
        failures.append(
            "script-read-coverage: no gate script named a tracked file as a "
            "quoted value — that arm of the reader is blind")
    if bare_seen < 1:
        failures.append(
            "script-read-coverage: no gate script named a tracked file as a "
            "bare word — that arm of the reader is blind")

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
        if not (spec.get("local") or spec.get("extra") or spec.get("builds")
                or spec.get("ci_only")):
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
    # `cost_s_no_op` is a claim with two halves, and both are checkable.
    #
    # It says: this gate can be reached without its own triggers firing, and
    # when that happens it costs nearly nothing. A gate nothing pulls in
    # through `deps` cannot be reached that way, so the field would be
    # describing a case that never occurs — and a cheaper-than-cost_s claim
    # that is not cheaper is just a second cost_s.
    for slug in sorted(GATES):
        no_op = GATES[slug].get("cost_s_no_op")
        if no_op is None:
            continue
        declared = GATES[slug].get("cost_s")
        if declared is None or no_op >= declared:
            failures.append(
                f"conditional cost: {slug} declares cost_s_no_op={no_op} "
                f"against cost_s={declared} — the no-op case has to be the "
                f"cheaper one, or the gate has one cost written twice")
        pullers = sorted(o for o in GATES
                         if slug in (GATES[o].get("deps") or []))
        if not pullers:
            failures.append(
                f"conditional cost: {slug} declares a no-op cost but no gate "
                f"lists it in `deps`, so it is only ever selected by its own "
                f"triggers — the case that figure describes cannot arise")

    cases += 1
    # The EFFECT, not the data: drive the two functions that consume the
    # field and check the answer changes.
    #
    # Calling `cost_is_stale` directly here would pass while every CALL SITE
    # ignored the field — measured 2026-09-02, when exactly that mutation
    # (dropping the argument in `order_drift`) left this case green. What
    # the field is for is the report, so the report is what gets driven.
    conditional = sorted(s for s in GATES if GATES[s].get("cost_s_no_op"))
    if not conditional:
        failures.append(
            "conditional cost: no gate declares cost_s_no_op, so this case "
            "is measuring nothing — drop it or restore the declaration")
    baseline = {s: (GATES[s].get("cost_s") or 0)
                for s in GATES if not GATES[s].get("ci_only")}
    for slug in conditional:
        spec = GATES[slug]
        # `order_drift` cannot answer for a gate other gates depend on — the
        # order is dependency-first, so its cost never moves its position.
        # `budget_breach` names the gates it blamed, so drive that, and put
        # an unrelated gate over the ceiling so there is a verdict to read
        # rather than a None that would pass by measuring nothing.
        filler = max((s for s in baseline if s != slug),
                     key=lambda s: (baseline[s], s))

        no_op_run = dict(baseline)
        no_op_run[slug] = spec["cost_s_no_op"]
        no_op_run[filler] = PUSH_BUDGET_S * 3
        breach = budget_breach(no_op_run)
        if breach is None:
            failures.append(
                f"conditional cost: forcing a breach with {filler} did not "
                f"produce one, so the {slug} check below reads nothing")
        elif any(row[0] == slug for row in breach["grown"]):
            failures.append(
                f"conditional cost: a run where {slug} did its no-op counts "
                f"it as a GROWN gate against its {spec['cost_s']}s — "
                f"cost_s_no_op is not reaching `budget_breach`, so a warm "
                f"push is priced as though the gate had rebuilt")

        # The other direction, or the branch would excuse everything.
        slow_run = dict(baseline)
        slow_run[slug] = PUSH_BUDGET_S * 3
        breach = budget_breach(slow_run)
        if breach is None or not any(row[0] == slug for row in breach["grown"]):
            failures.append(
                f"conditional cost: {slug} at {PUSH_BUDGET_S * 3}s is not "
                f"blamed for the breach — the no-op branch is swallowing "
                f"real growth")

    cases += 1
    # A cost below a second is a claim about the RUNNER, so check the runner.
    #
    # `scripts/gate` timed with whole-second `SECONDS` until 2026-09-02, and
    # the consequence was not a rounding error: `http-endpoint-ssot` read 0
    # on every `--measure` run, its declared 4s could never be confirmed
    # against anything, and it sat in `PACE_NORMALISED` for that reason
    # alone. Its cost is now 0.185 — a number the old clock could not
    # produce. If the clock goes back, that figure silently becomes
    # unconfirmable again and the entry drifts back into the map.
    #
    # This is the effect, not the edit: it asks whether a sub-second cost
    # still has an instrument that can read it.
    sub_second = sorted(s for s in GATES
                        if 0 < (GATES[s].get("cost_s") or 0) < 1)
    if sub_second:
        runner_path = repo_root / "scripts" / "gate"
        runner = runner_path.read_text(encoding="utf-8", errors="replace")
        if re.search(r"^\s*start=\$SECONDS\b", runner, re.M):
            failures.append(
                f"sub-second cost: {sub_second} declare a cost under one "
                f"second, but {runner_path} times gates with whole-second "
                f"`SECONDS` — those figures cannot be re-measured, which is "
                f"how such a gate ends up in PACE_NORMALISED with a chosen "
                f"number instead of a read one")
        if "EPOCHREALTIME" not in runner:
            failures.append(
                f"sub-second cost: {sub_second} need a sub-second clock and "
                f"{runner_path} no longer has one")

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

    cases += 1
    # A gate that obtains the generator runs its output, so it must declare
    # `codegen-build` and let `run_order` build it first. Without the edge the
    # gate is free to sort ahead of the build and `sce_codegen_require` pays
    # the rebuild inside it: `nl-ir-closure`, declared 1.3s, ran 288s on the
    # 2026-09-24 push that changed `sce-build`'s dev profile, and the budget
    # refused a push whose gates had all passed. Derived from the scripts, so
    # a new gate that calls the helper cannot join without the edge.
    gates_dir = repo_root / "scripts" / "gates"
    for script in sorted(gates_dir.glob("*.sh")):
        slug = script.stem
        if slug not in GATES or slug == "codegen-build":
            continue
        text = script.read_text(encoding="utf-8", errors="replace")
        if "sce_gate_codegen" not in text and "sce_codegen_require" not in text:
            continue
        if "codegen-build" not in GATES[slug].get("deps", []):
            failures.append(
                f"{slug}: its script obtains sce-codegen but the gate does "
                f"not declare deps=['codegen-build'], so it can run before "
                f"the build and pay for it")

    # ── The gates as bx check units ─────────────────────────────────
    #
    # every-gate-says-whether-its-verdict-is-kept. A kept pass is reused for
    # every later run with the same key, so a gate whose verdict reads
    # something the key does not hold would pass forever on one reading.
    # Nothing defaults: a gate that does not say is refused, `cache: False`
    # carries the reason, and `cache: True` carries none (a reason beside a
    # True is a doubt somebody wrote down and did not act on).
    cases += 1
    for slug, spec in sorted(GATES.items()):
        if spec.get("cache") not in (True, False):
            failures.append(
                f"verdict: {slug} does not say `cache: True` or `cache: False`; "
                f"a gate whose verdict bx may keep has to claim it, and one "
                f"that reads the clock, the network or the machine has to say "
                f"so (see 'Whether a gate's verdict may be kept')")
        why = spec.get("no_cache_reason")
        if spec.get("cache") is False and (not isinstance(why, str)
                                           or len(why.strip()) < 40):
            failures.append(
                f"verdict: {slug} is `cache: False` with no `no_cache_reason` "
                f"saying what it reads that the key does not hold")
        if spec.get("cache") is True and why is not None:
            failures.append(
                f"verdict: {slug} is `cache: True` and still carries a "
                f"`no_cache_reason`; one of the two is wrong")
        commits = spec.get("commits")
        if not isinstance(commits, list) \
                or any(c not in CARRIED_COMMITS for c in commits):
            failures.append(
                f"verdict: {slug} declares commits={commits!r}; it is a list "
                f"of the commits its verdict reads, each one the plan carries "
                f"({', '.join(CARRIED_COMMITS)}), and `[]` when it reads none")
        peak = spec.get("peak_gb")
        if isinstance(peak, bool) or not isinstance(peak, (int, float)) \
                or peak <= 0:
            failures.append(
                f"verdict: {slug} declares peak_gb={peak!r}; it is the memory "
                f"its own script peaks at, in GiB, a number above 0")
        cores = spec.get("parallelism")
        if isinstance(cores, bool) or not isinstance(cores, int) or cores < 1:
            failures.append(
                f"verdict: {slug} declares parallelism={cores!r}; it is how many "
                f"processes its own script can keep busy, a whole number above 0")
        if "inputs" not in spec:
            failures.append(
                f"verdict: {slug} does not declare `inputs` (`None` for the "
                f"whole tracked tree, or a list of patterns)")
            continue
        inputs = spec["inputs"]
        if inputs is None:
            continue
        lists = spec.get("lists", [])
        if lists and inputs != INPUTS_ARE_TRIGGERS:
            failures.append(
                f"verdict: {slug} declares lists and is not keyed on its "
                f"triggers; a declared list of inputs names its listings itself")
        if not isinstance(lists, list) or any(
                not isinstance(p, str) or not p.endswith("/") or p.startswith("/")
                or ".." in p.split("/") for p in lists):
            failures.append(
                f"verdict: {slug} declares lists={lists!r}; each is a directory "
                f"under the root ending in `/`")
        if inputs == INPUTS_ARE_TRIGGERS:
            if spec.get("cache") is not True:
                failures.append(
                    f"verdict: {slug} is keyed on its triggers and its verdict "
                    f"is not kept (cache={spec.get('cache')!r}); a key that "
                    f"keeps nothing is no narrowing")
            continue
        if not isinstance(inputs, list) or not inputs:
            failures.append(
                f"verdict: {slug} declares inputs={inputs!r}; it is `None`, "
                f"{INPUTS_ARE_TRIGGERS!r} or a non-empty list of patterns")
            continue
        for pattern in inputs:
            if (not isinstance(pattern, str) or not pattern
                    or pattern.startswith("/")
                    or ".." in pattern.split("/")):
                failures.append(
                    f"verdict: {slug} declares the input {pattern!r}, which is "
                    f"not a pattern under the repository root")
    # The cases below plan units, and a plan reads these fields of every gate:
    # one that is missing is said here, not by a traceback of the planner.
    if any(f.startswith("verdict: ") for f in failures):
        return print_self_test_failures(failures)

    # every-tree-builder-names-what-it-builds. A gate whose script configures
    # the root CMake tree compiles the engine and, through it, the generator
    # crate; its trigger has to hold what that reads, and `builds` is where it
    # gets it. Derived from the scripts, so a new gate that builds the tree
    # and names nothing fails here, the state `w3c-python-bindings` and
    # `visualizer-wasm` were in until 2026-11-16.
    #
    # A gate off the fleet (`platform`, "Where a gate can run") is not asked:
    # no push and no landing selects it and no unit keys it, so what its
    # trigger holds decides neither. Its workflow's own filter is what starts
    # it, and that is the lane's choice (`windows-msvc` names the submodules
    # its MSVC build compiles, not every tree the Linux build reads).
    cases += 1
    configures_root = re.compile(
        r"sce_main_build_dir|prepare_ctest_tree\.sh|sce_gate_configure_wasm"
        r"|cmake\s+-S\s+(?:\"\$SCE_REPO_ROOT\"|\.)\s|^\s*(?:if\s+!\s+)?cmake\s+-B\b",
        re.M)
    fleet_now = fleet_platforms(repo_root)
    tree_builders = 0
    for slug, spec in sorted(GATES.items()):
        for built in spec.get("builds") or []:
            if built not in BUILT_INPUTS:
                failures.append(
                    f"builds: {slug} builds {built!r}, which BUILT_INPUTS does "
                    f"not declare")
        script = gate_script(repo_root, slug)
        if not script.is_file() or not configures_root.search(script_code(script)):
            continue
        tree_builders += 1
        if spec.get("platform") is not None and spec["platform"] not in fleet_now:
            continue
        if "main-tree" not in (spec.get("builds") or []):
            failures.append(
                f"builds: scripts/gates/{slug}.sh configures the root CMake "
                f"tree and {slug} does not name `main-tree` in `builds`, so "
                f"what the tree compiles neither selects it nor keys its unit")
    if tree_builders < 8:
        failures.append(
            f"builds: {tree_builders} gate script(s) were found configuring the "
            f"root tree, and eleven did when this case was written; the "
            f"pattern no longer reads them")

    # the-cargo-half-is-derived. What a build of the generator crate reads is
    # read off the manifests and the witness, never written: the workspace
    # files, the crate and each path dependency it builds, and every tree the
    # build script digests. A parse that stopped reading any of them fails
    # here rather than narrowing every key built on it.
    cases += 1
    main_tree = built_inputs(repo_root, "main-tree")
    expected = ["Cargo.lock", ".cargo/**", "sce-build/src/**",
                "sce-build/build.rs"] + cargo_workspace_manifests(repo_root)
    if len(cargo_workspace_manifests(repo_root)) < 10:
        failures.append(
            f"builds: the workspace manifests read as "
            f"{cargo_workspace_manifests(repo_root)}; the root lists 32 members")
    expected += [f"{d}/src/**" for d in cargo_path_deps(repo_root, "sce-build")]
    witness = generator_witness_globs(repo_root)
    if not cargo_path_deps(repo_root, "sce-build"):
        failures.append(
            "builds: no path dependency was read from sce-build/Cargo.toml, "
            "and it builds the Rust runtime; the manifest reader reads nothing")
    if not any(g.endswith("/**") for g in witness) or \
            not any("*" not in g for g in witness):
        failures.append(
            f"builds: the generator witness ({GENERATOR_WITNESS}) gave "
            f"{witness}; its WITNESS_FILES, WITNESS_TREES and RUN_TIME_TREE "
            f"are no longer read")
    covered = [glob_to_regex(g) for g in main_tree if "*" in g]
    for want in expected + list(MAIN_TREE_CONFIGURE) + witness:
        if want not in main_tree and not any(p.match(want) for p in covered):
            failures.append(
                f"builds: `main-tree` does not hold {want!r}, which a build "
                f"of the tree reads")

    # What a build does not read stays out. A crate's `tests/`, `benches/`
    # and `examples/` are compiled by `cargo test` or `--all-targets`, never
    # by the build these gates share, and a module behind `#[cfg(test)]` is
    # no part of it either. Measured 2026-11-24: with each crate taken whole,
    # an edit to one integration test of `sce-build` selected 39 gates of a
    # landing, 9536 s by their declared costs, `cpp-suite-wasm` (3353 s) among
    # them. A module named by `#[path]` and a file named by an `include_str!`
    # broken over lines are what the walk has to keep finding.
    for want in ("backends/rust/runtime/generated/mod.rs",
                 "backends/swift/forge-runtime/Sources/SceForgeRuntime/Codec.swift"):
        if not any(glob_to_regex(g).match(want) for g in main_tree):
            failures.append(
                f"builds: `main-tree` does not hold {want!r}, which the crate "
                f"compiles in by path; the module walk no longer finds it")
    for unread in ("sce-build/tests/gate_registry_contract.rs",
                   "sce-build/examples/gen_any.rs",
                   "sce-build/tests/common/executable.rs",
                   "tests/forge/expected/views/codec_tlv_entry.rs"):
        if any(glob_to_regex(g).match(unread) for g in main_tree):
            failures.append(
                f"builds: `main-tree` holds {unread!r}, which only a test "
                f"build compiles, so editing it selects every gate that builds "
                f"the tree")
    # The one build script whose reads are derived is `sce-build`'s (the
    # generator witness). Another crate the shared builds compile growing one
    # would read files nothing here names.
    for name, (_, crates, _) in BUILT_INPUTS.items():
        for crate in crates:
            closure, pending = [], [crate]
            while pending:
                c = pending.pop()
                if c not in closure:
                    closure.append(c)
                    pending.extend(cargo_path_deps(repo_root, c))
            for c in closure:
                script, _ = cargo_build_targets(repo_root, c)
                if script and c != "sce-build":
                    failures.append(
                        f"builds: {c} has a build script ({script}) and "
                        f"`{name}` compiles it, but only sce-build's reads are "
                        f"derived (the generator witness); name what it reads")

    # what-a-build-reads-without-selecting-it. Read off the manifests, as the
    # trigger half is, and held to what `bx --checks prepush --learn forge-c11`
    # read (2026-11-24): a member that is not compiled is looked at for its
    # target roots and listed for its tests, a build script named by
    # `[package] build` is not looked for, one left to discovery is, and the
    # generator crate, whose build script the build runs, is read whole. None
    # of it selects: the triggers of the gate that builds the generator hold
    # none of it, its key holds all of it.
    cases += 1
    generator_reads = built_reads(repo_root, "sce-codegen")
    for want in ("sce-build/**", "sce-revision/src/lib.rs", "sce-specs/src/bin/",
                 "sce-specs/tests/", "backends/rust/scenarios/build.rs",
                 "backends/rust/runtime/tests/", "tests/server/src/main.rs"):
        if want not in generator_reads:
            failures.append(
                f"build-reads: what a build of the generator reads beside what "
                f"it compiles does not hold {want!r}, which cargo reads to load "
                f"the workspace or to run the crate's build script")
    for unread in ("backends/rust/deploy-conformance/build.rs",
                   "sce-revision/tests/the_cases_the_python_gives.rs"):
        if any(glob_to_regex(g).match(unread) for g in generator_reads
               if not g.endswith("/")):
            failures.append(
                f"build-reads: what a build of the generator reads holds "
                f"{unread!r}, which cargo neither looks for nor compiles there")
    if built_reads(repo_root, "forge-runtime-rust"):
        failures.append(
            f"build-reads: the forge runtime built from a crate of a gate's own "
            f"reads {built_reads(repo_root, 'forge-runtime-rust')} beside what it "
            f"compiles, and `bx --checks prepush --learn codec-clippy` read "
            f"nothing there")
    building_triggers = [glob_to_regex(g)
                         for g in gate_triggers(repo_root)["codegen-build"]]
    for read in ("sce-revision/src/lib.rs", "sce-build/tests/common/executable.rs"):
        if any(p.match(read) for p in building_triggers):
            failures.append(
                f"build-reads: `codegen-build` is selected by {read!r}, which "
                f"its build reads and does not compile")
    if GATES["codegen-build"].get("inputs") == INPUTS_ARE_TRIGGERS:
        building_key = gate_inputs(
            "codegen-build",
            triggers=gate_triggers(repo_root, ["codegen-build"]),
            reads=gate_reads(repo_root, ["codegen-build"]))
        for read in generator_reads:
            if read not in building_key:
                failures.append(
                    f"build-reads: `codegen-build` is keyed on its triggers and "
                    f"its key does not hold {read!r}, which its build reads")

    # the-preflight-derives-the-gate-it-runs-alone. A narrowed unit's copy
    # holds that gate's inputs and no other's, so the submodule preflight
    # (scripts/lib/require_submodules.sh) derives the triggers of the gate it
    # runs and of no other: every gate's read every workflow and ledger
    # config, which `bx --checks prepush --learn forge-c11` (2026-11-24) found
    # outside its inputs. Asked of the derivation and of the call.
    cases += 1
    alone = gate_triggers(repo_root, ["forge-c11"])
    if set(alone) != {"forge-c11"} \
            or alone["forge-c11"] != gate_triggers(repo_root)["forge-c11"]:
        failures.append(
            f"preflight: the triggers of forge-c11 alone read as "
            f"{sorted(alone)}, and are its entry of every gate's")
    if set(gate_reads(repo_root, ["codegen-build"])) != {"codegen-build"}:
        failures.append("preflight: what the builds of codegen-build alone read "
                        "was derived for other gates too")
    try:
        preflight = (repo_root / "scripts" / "lib" / "require_submodules.sh") \
            .read_text(encoding="utf-8")
    except OSError:
        preflight = ""
    calls = re.findall(r'mod\["(gate_triggers|gate_reads)"\]\(([^)]*)\)', preflight)
    if not calls or any("[slug]" not in args for _, args in calls):
        failures.append(
            f"preflight: scripts/lib/require_submodules.sh derives {calls}; it "
            f"asks gate_triggers and gate_reads for the gate it runs alone")

    # a-workflow-that-follows-the-generator-holds-its-build. A workflow that
    # starts for every source of the generator (`sce-build/src/**`) but not for
    # the rest of what cargo reads to build it (its manifest, its build script,
    # the trees the script digests, the files its sources include) runs the
    # lane for some changes to the generator and not others. Measured
    # 2026-11-24: three lanes named `sce-build/src/**` and none of the build
    # script or `kind-examples/`. Compared over tracked files of the crate.
    cases += 1
    built_pats = [glob_to_regex(g) for g in built_inputs(repo_root, "sce-codegen")]
    in_crate = [p for p in tracked_paths(repo_root, "sce-build")
                if any(b.match(p) for b in built_pats)]
    following = 0
    for wf in sorted((repo_root / ".github" / "workflows").glob("*.yml")):
        wf_globs = workflow_paths(repo_root, wf.name)
        if "sce-build/src/**" not in wf_globs or "sce-build/**" in wf_globs:
            continue
        following += 1
        wf_pats = [glob_to_regex(g) for g in wf_globs]
        missing = [p for p in in_crate if not any(x.match(p) for x in wf_pats)]
        if missing:
            failures.append(
                f"builds: {wf.name} starts for every source of the generator "
                f"(sce-build/src/**) but not for {missing[0]!r}"
                f"{f' and {len(missing) - 1} more' if len(missing) > 1 else ''}, "
                f"which a build of it reads; name what `sce-codegen` reads of "
                f"the crate")
    if following < 10:
        failures.append(
            f"builds: {following} workflow(s) were found following the "
            f"generator's sources, and sixteen did when this case was written; "
            f"the case no longer reads them")

    # A measured peak names a gate that exists, so the note beside it cannot
    # outlive the gate it was read off.
    cases += 1
    for slug in sorted(set(PEAK_MEASURED) - set(GATES)):
        failures.append(f"verdict: PEAK_MEASURED names {slug}, which is no gate")

    # every-unit-names-the-packages-its-gates-need. A gate's `packages` is
    # absent or a non-empty list of distinct package names, and each name is in
    # the code of the gate's script or of a script it runs, which is what runs
    # the package's program: a name neither mentions any longer would keep the
    # unit off machines for nothing.
    cases += 1
    declaring = 0
    for slug, spec in sorted(GATES.items()):
        if "packages" not in spec:
            continue
        declaring += 1
        packages = spec["packages"]
        if not isinstance(packages, list) or not packages \
                or len(set(packages)) != len(packages) \
                or any(not isinstance(p, str) or not PACKAGE_NAME.match(p)
                       for p in packages):
            failures.append(
                f"packages: {slug} declares packages={packages!r}; it is a "
                f"non-empty list of distinct package names, or absent for none")
            continue
        script = gate_script(repo_root, slug)
        code = script_code(script) if script.is_file() else ""
        run = set().union(*script_read_paths(code, set(tracked_paths(
            repo_root, "scripts/*.sh", "scripts/**/*.sh"))))
        code += "\n".join(script_code(repo_root / path) for path in sorted(run))
        for package in packages:
            if package not in code:
                failures.append(
                    f"packages: {slug} requires the package {package}, which "
                    f"neither scripts/gates/{slug}.sh nor a script it runs "
                    f"names in its code, so the requirement keeps its unit "
                    f"off machines for nothing")
    if declaring == 0:
        failures.append(
            "packages: no gate declares a package, so the check above read "
            "nothing; `c11-mcu` requires qemu-system-arm")

    # a-verdict-that-reads-head-carries-it. A gate script that hands git the
    # name HEAD (`--rev HEAD`, `git worktree add … HEAD`, `git show HEAD:…`)
    # reads a commit the key does not hold, so a gate whose pass is kept has
    # to declare it and let the plan carry HEAD's id. Derived from the code
    # lines of each script, so a new reader cannot join without the
    # declaration. What a script cannot show (a Rust target that asks git
    # about HEAD's history) is declared by hand, beside the gate.
    cases += 1
    head_reader = re.compile(r"(\bgit\b.*\bHEAD\b|--rev\s+HEAD\b)")
    readers_seen = 0
    for slug, spec in sorted(GATES.items()):
        script = gate_script(repo_root, slug)
        if not script.is_file():
            continue
        if not any(head_reader.search(line)
                   for line in script_code(script).splitlines()):
            continue
        readers_seen += 1
        if spec.get("cache") is True and "HEAD" not in spec.get("commits", []):
            failures.append(
                f"commits: scripts/gates/{slug}.sh hands git HEAD, and the gate "
                f"keeps its pass without declaring commits=['HEAD']; the pass "
                f"would be reused for a commit it never read")
    if readers_seen == 0:
        failures.append(
            "commits: no gate script hands git HEAD, so the scan above read "
            "nothing; `embed-manifest-stamp` and `regen-reproduces` do")

    # a-verdict-that-reads-the-change-set-carries-it. A gate script that
    # reads `SCE_GATE_CHANGED_FILE` does its work per changed path, and a
    # unit is a build machine's checkout whose refs say nothing about the
    # push, so the plan has to carry the push's ranges to it. Unlike HEAD this
    # holds whether or not the pass is kept: without the ranges the gate
    # answers for an unknown set. Derived from the code lines of each script,
    # in both directions, so a declaration cannot outlive its reader either.
    cases += 1
    change_reader = re.compile(r"\bSCE_GATE_CHANGED_FILE\b")
    readers_seen = 0
    for slug, spec in sorted(GATES.items()):
        script = gate_script(repo_root, slug)
        reads = script.is_file() and any(
            change_reader.search(line)
            for line in script_code(script).splitlines())
        declared = "CHANGES" in spec.get("commits", [])
        readers_seen += reads
        if reads and not declared:
            failures.append(
                f"commits: scripts/gates/{slug}.sh reads SCE_GATE_CHANGED_FILE "
                f"and the gate does not declare commits=['CHANGES']; a "
                f"landing's unit of it would be handed no change set")
        if declared and not reads:
            failures.append(
                f"commits: {slug} declares commits=['CHANGES'] and "
                f"scripts/gates/{slug}.sh never reads SCE_GATE_CHANGED_FILE")
    if readers_seen == 0:
        failures.append(
            "commits: no gate script reads SCE_GATE_CHANGED_FILE, so the scan "
            "above read nothing; `mutation-rounds` does")

    # a-narrowed-gate-reads-no-commit. bx runs a unit with input patterns in a
    # repository of one commit that holds them (bx RFC 0026). That commit is
    # not HEAD and carries no change set, so a gate that reads either would
    # be refused by the runner (`--at`) or handed an empty set there. Such a
    # gate keeps the whole tree until it is split into a part that reads the
    # tree and a part that reads the commits (ADR 0120, D2).
    cases += 1
    for slug, spec in sorted(GATES.items()):
        if spec.get("inputs") is not None and spec.get("commits"):
            failures.append(
                f"narrowed: {slug} declares input patterns and reads "
                f"{spec['commits']}; the copy bx runs a narrowed unit in has "
                f"one commit, which is neither HEAD nor the change set")

    # a-narrowed-gate-holds-the-submodules-it-reads. A narrowed unit's copy
    # holds a submodule checked out only when the inputs give its
    # `<path>/.git` (bx RFC 0026, section 4.1); a gate whose inputs reach
    # files beneath one without it would find them in a submodule git calls
    # not checked out, and the preflight would refuse the unit.
    cases += 1
    submodules_now = submodule_paths(repo_root)
    if not submodules_now:
        failures.append(
            "submodules: .gitmodules records none, so the check below read "
            "nothing; third_party/ holds four")
    triggers_of_inputs = gate_triggers(repo_root)
    reads_of_inputs = gate_reads(repo_root)
    for slug, spec in sorted(GATES.items()):
        own = gate_inputs(slug, triggers=triggers_of_inputs, reads=reads_of_inputs)
        if own is None:
            continue
        for sub in submodules_reached(own, submodules_now):
            if not any(glob_to_regex(p).match(f"{sub}/.git")
                       for p in own if not p.endswith("/")):
                failures.append(
                    f"narrowed: {slug} reads beneath the submodule {sub} and "
                    f"its inputs do not give {sub}/.git, so its unit's copy "
                    f"holds that submodule as not checked out")

    # every-library-the-runner-sources-is-a-runner-input. A narrowed unit's copy
    # holds its inputs alone, so a library `scripts/gate` or the gate library
    # sources that `RUNNER_INPUTS` does not name is missing there, and a change
    # to it moves no narrowed key. Read off the `source` lines themselves.
    cases += 1
    sourced = set()
    for runner in ("scripts/gate", "scripts/gates/lib.sh"):
        try:
            text = (repo_root / runner).read_text(encoding="utf-8")
        except OSError:
            failures.append(f"runner-inputs: {runner} could not be read")
            continue
        sourced |= set(re.findall(
            r'^\s*source\s+"\$(?:SCE_)?REPO_ROOT/(scripts/lib/[A-Za-z0-9_]+\.sh)"',
            text, re.M))
    if len(sourced) < 5:
        failures.append(f"runner-inputs: the runner and the gate library source "
                        f"{sorted(sourced)}; the reader found too few to be reading")
    for lib in sorted(sourced - set(RUNNER_INPUTS)):
        failures.append(f"runner-inputs: {lib} is sourced by the runner or the gate "
                        f"library and RUNNER_INPUTS does not name it")

    # a-gate-keyed-on-its-triggers-is-narrowed-by-them. A gate whose key is its
    # trigger globs (`INPUTS_ARE_TRIGGERS`) gains nothing when a glob is the
    # whole tree, and its unit would claim a narrowed key while bx keys it on
    # every file; such a gate says `None` instead, which is what it is.
    cases += 1
    for slug, spec in sorted(GATES.items()):
        if spec.get("inputs") != INPUTS_ARE_TRIGGERS:
            continue
        whole = [g for g in triggers_of_inputs[slug] if glob_to_regex(g).match(
            "a-path-no-glob-names/at/any/depth.txt")]
        if whole:
            failures.append(
                f"narrowed: {slug} is keyed on its triggers and {whole} names "
                f"every path; declare `inputs: None`")

    # The trees and triggers the two checks below read, as they stand.
    tracked_now = tracked_paths(repo_root)
    triggers_now = gate_triggers(repo_root)

    # every-enrolled-tree-triggers-the-gate-that-judges-it. `spec-snapshot`'s
    # tooling suite judges the citations of every path a ledger enrols, so a
    # change to any tracked file under one must select it. Asked of the
    # tracked files rather than of the globs, so a directory that lost its
    # `/**`, a file that gained one, or a config the glob stopped matching
    # each fail here.
    cases += 1
    configs = sorted(repo_root.glob(GATES["spec-snapshot"]["enrolled_by"]))
    reader_path = (repo_root / "tools" / "mnemosyne-adoption"
                   / "migrate_citations.py")
    if len(configs) < 5 or not reader_path.is_file():
        failures.append(
            f"enrolment: {len(configs)} ledger config(s) matched "
            f"{GATES['spec-snapshot']['enrolled_by']!r} and the reader is "
            f"{'present' if reader_path.is_file() else 'missing'}; there were "
            f"five when spec-snapshot's trigger was derived from them")
    else:
        reader_spec = importlib.util.spec_from_file_location(
            "_sce_enrolment_self_test", reader_path)
        reader = importlib.util.module_from_spec(reader_spec)
        reader_spec.loader.exec_module(reader)
        snapshot_regexes = [glob_to_regex(g)
                            for g in triggers_now["spec-snapshot"]]
        if "**" in triggers_now["spec-snapshot"]:
            failures.append(
                "enrolment: spec-snapshot's trigger fell back to the "
                "catch-all, so its enrolled trees were not read")
        judged = 0
        for config in configs:
            enrolled = [Path(p).relative_to(reader.REPO_ROOT).as_posix()
                        for p in reader.paths_from_toml(str(config))]
            ledger_dir = config.parent.relative_to(repo_root).as_posix()
            for rel in enrolled + [ledger_dir]:
                under = [p for p in tracked_now
                         if p == rel or p.startswith(rel.rstrip("/") + "/")]
                judged += len(under)
                missing = [p for p in under
                           if not any(r.match(p) for r in snapshot_regexes)]
                if missing:
                    failures.append(
                        f"enrolment: {config.relative_to(repo_root)} enrols "
                        f"{rel!r}, and {missing[:3]} reach no trigger of "
                        f"spec-snapshot, which judges their citations")
        if judged == 0:
            failures.append(
                "enrolment: the ledgers enrol no tracked file, so the check "
                "that spec-snapshot is triggered by them reads nothing")
    # A set that cannot be read is the catch-all, never an empty trigger.
    cases += 1
    if enrolled_globs(repo_root, "docs/no-such-ledger/*/mnemosyne.toml") \
            is not None:
        failures.append(
            "enrolment: a ledger glob matching no config yielded globs; the "
            "trigger has to fall back to the catch-all instead")

    # the-mesh-catalog's-witnesses-trigger-its-guard. `spec-snapshot` checks
    # the mesh verifies-catalog against the `SCE-VERIFIES:` markers in the
    # files `TEST_TREES` names in its generator, so each of those files is a
    # reason to run it. The tuple is that list's one home; read here rather
    # than restated, so a witness added there and not to the workflow's
    # `paths:` fails this instead of leaving a catalog edit unguarded, the
    # state bx-fleet check learn found on 2026-11-09 (five of them missing).
    cases += 1
    generator = (repo_root / "tools" / "mnemosyne-adoption"
                 / "gen_mesh_verifies_catalog.py")
    witness_globs: list[str] = []
    if generator.is_file():
        tree = ast.parse(generator.read_text(encoding="utf-8"))
        for node in tree.body:
            if (isinstance(node, ast.Assign)
                    and any(isinstance(t, ast.Name) and t.id == "TEST_TREES"
                            for t in node.targets)
                    and isinstance(node.value, (ast.Tuple, ast.List))):
                witness_globs = [e.value for e in node.value.elts
                                 if isinstance(e, ast.Constant)
                                 and isinstance(e.value, str)]
    if not witness_globs:
        failures.append(
            "spec-snapshot: no TEST_TREES tuple was read from "
            "tools/mnemosyne-adoption/gen_mesh_verifies_catalog.py, so the "
            "check that its witnesses trigger the gate reads nothing")
    snapshot_trigger = [glob_to_regex(g) for g in
                        triggers_now.get("spec-snapshot", [])]
    for witness in witness_globs:
        regex = glob_to_regex(witness)
        unguarded = [p for p in tracked_now if regex.match(p)
                     and not any(t.match(p) for t in snapshot_trigger)]
        if unguarded:
            failures.append(
                f"spec-snapshot: TEST_TREES names {witness!r}, and "
                f"{unguarded[:3]} reach no trigger of the gate; add it to "
                f"spec-snapshot-drift.yml's `paths:`")

    # the-plan-is-the-selection. The push used to run exactly the gates
    # `select` chose; it now hands bx the units `plan_units` makes of that
    # choice. Each path set below must come back as units that cover every
    # selected gate, name no gate the selection left out, and never make a
    # unit of a gate another unit already runs. A plan that drifted from the
    # selection would be the push verifying something else than it says.
    for label, selection, landing in (
            ("rust-source", select(repo_root, ["sce-build/src/mesh/deploy.rs"])[0], False),
            ("rust-source-landing", select(repo_root, ["sce-build/src/mesh/deploy.rs"],
                                           landing=True)[0], True),
            ("docs-only", select(repo_root, ["README.md"])[0], False),
            ("go-backend", select(repo_root, ["backends/go/forge-runtime/go.mod"])[0], False),
            ("unclassified", select(repo_root, ["some_new_top_level_dir/x.txt"])[0], False),
            ("every-gate", list(GATES), False),
            ("every-gate-landing", list(GATES), True),
    ):
        cases += 1
        head = "0" * 40
        counts = {s: shard_count(repo_root, s) for s in selection}
        units = plan_units(set(selection), rust="1.0.0", head=head,
                           landing=landing, shards=counts,
                           triggers=triggers_of_inputs, reads=reads_of_inputs)
        names = [u["name"] for u in units]
        parts: dict[str, list[int]] = {}
        if len(names) != len(set(names)):
            failures.append(f"plan[{label}]: a unit is named twice: {names}")
        covered: set[str] = set()
        for unit in units:
            gate = unit_gate(unit["name"])
            closure = {gate} | transitive_deps(gate)
            covered |= closure
            shard = SHARD_UNIT.match(unit["name"])
            if shard:
                parts.setdefault(gate, []).append(int(shard.group("k")))
            if gate not in selection:
                failures.append(
                    f"plan[{label}]: unit {unit['name']} is not in the "
                    f"selection {sorted(selection)}")
            reads_head = any("HEAD" in GATES[s]["commits"] for s in closure)
            want = (["bash", "scripts/gate", "--rust", "1.0.0", "--unit"]
                    + (["--landing"] if landing else [])
                    + (["--at", head] if reads_head else [])
                    + (["--shard", f"{shard.group('k')}/{shard.group('n')}"]
                       if shard else []) + [gate])
            if unit["argv"] != want:
                failures.append(
                    f"plan[{label}]: unit {unit['name']} runs {unit['argv']}, "
                    f"not the runner on its own slug at the planned release "
                    f"(as a landing's when it is one, and at HEAD's id when "
                    f"it reads HEAD): {want}")
            can_skip = any(GATES[s].get("kept_only_from_a_landing") for s in closure)
            if can_skip and not landing and unit["cache"]:
                failures.append(
                    f"plan[{label}]: unit {unit['name']} runs a gate that can "
                    f"skip and keeps its verdict outside a landing, where the "
                    f"pass could be a skip")
            for key in UNIT_REQUIRES_KEYS:
                wanted = sorted({item for s in closure
                                 for item in GATES[s].get(key, [])})
                if unit.get(key, []) != wanted:
                    failures.append(
                        f"plan[{label}]: unit {unit['name']} asks bx for "
                        f"{key}={unit.get(key, [])}, and what it runs requires "
                        f"{wanted}")
            peak = max(GATES[s]["peak_gb"] for s in closure)
            if unit.get("mem_gb") != float(peak):
                failures.append(
                    f"plan[{label}]: unit {unit['name']} asks for "
                    f"{unit.get('mem_gb')} GiB, and the largest peak of what "
                    f"it runs is {peak}")
            cores = max(GATES[s]["parallelism"] for s in closure)
            if unit.get("parallelism") != float(cores):
                failures.append(
                    f"plan[{label}]: unit {unit['name']} asks for "
                    f"parallelism {unit.get('parallelism')}, and the most of "
                    f"what it runs is {cores}")
            if unit["exclusive"] != [UNIT_EXCLUSIVE]:
                failures.append(
                    f"plan[{label}]: unit {unit['name']} may share a checkout "
                    f"with another unit ({unit['exclusive']})")
            others = {unit_gate(n) for n in names} - {gate}
            if others & transitive_deps(gate):
                failures.append(
                    f"plan[{label}]: unit {unit['name']} runs "
                    f"{sorted(others & transitive_deps(gate))}, which "
                    f"are units of their own as well")
        # A gate in shards is every one of its parts, each once.
        for gate in selection:
            want = list(range(1, counts[gate] + 1)) if counts[gate] > 1 else []
            if sorted(parts.get(gate, [])) != want:
                failures.append(
                    f"plan[{label}]: {gate} is planned in parts "
                    f"{sorted(parts.get(gate, []))}, not 1..{counts[gate]}")
        missing = set(selection) - covered
        if missing:
            failures.append(
                f"plan[{label}]: selected and run by no unit: {sorted(missing)}")
        if selection and not units:
            failures.append(f"plan[{label}]: a selection of "
                            f"{len(selection)} gate(s) planned no unit")

    # a-sharded-gate-is-split-the-way-its-lane-splits-it. A gate that declares
    # `shard_env` reads it in its script and has a lane that sets it per job
    # with a count above one; otherwise the plan would run it whole while the
    # entry says it is split, or split it on a variable the script ignores and
    # run the whole suite N times.
    cases += 1
    sharded = 0
    for slug, spec in sorted(GATES.items()):
        env = spec.get("shard_env")
        if not env:
            continue
        sharded += 1
        if shard_count(repo_root, slug) < 2:
            failures.append(
                f"shards: {slug} reads its shard from {env}, and no workflow "
                f"it mirrors sets {env}: ${{{{ matrix.shard }}}}/N with N above 1")
        script = gate_script(repo_root, slug)
        if not script.is_file() or env not in script_code(script):
            failures.append(
                f"shards: {slug} declares shard_env {env}, which "
                f"scripts/gates/{slug}.sh does not read")
    if sharded < 1:
        failures.append("shards: no gate declares shard_env; the case read "
                        "nothing")

    # a-kept-verdict-needs-a-release-a-clean-tree-and-a-kept-closure. Driven
    # from a fixture so that it keeps measuring whatever the live table
    # says: `child` keeps its verdict and runs `parent`, which does not.
    cases += 1
    fixture = {
        "parent": {"cost_s": 2, "cache": False, "inputs": None,
                   "no_cache_reason": "x" * 40, "commits": [], "peak_gb": 5,
                   "parallelism": 1},
        "child": {"cost_s": 3, "deps": ["parent"], "cache": True,
                  "inputs": None, "commits": [], "peak_gb": 1, "parallelism": 1},
        "alone": {"cost_s": 1, "cache": True, "inputs": ["docs/**"],
                  "commits": [], "peak_gb": 1, "parallelism": 1},
        "at-head": {"cost_s": 1, "cache": True, "inputs": None,
                    "commits": ["HEAD"], "peak_gb": 1, "parallelism": 1},
        "emulated": {"cost_s": 1, "cache": True, "inputs": None,
                     "commits": [], "peak_gb": 1, "parallelism": 1,
                     "packages": ["qemu-system-arm"]},
        "over-emulated": {"cost_s": 1, "deps": ["emulated"], "cache": True,
                          "inputs": None, "commits": [], "peak_gb": 1,
                          "parallelism": 1,
                          "packages": ["binutils", "qemu-system-arm"]},
        "on-changes": {"cost_s": 1, "cache": True, "inputs": None,
                       "commits": ["CHANGES"], "peak_gb": 1, "parallelism": 1},
        "after-changes": {"cost_s": 1, "deps": ["on-changes"],
                          "cache": True, "inputs": None, "commits": [],
                          "peak_gb": 1, "parallelism": 1},
        "by-triggers": {"cost_s": 1, "cache": True,
                        "inputs": INPUTS_ARE_TRIGGERS, "commits": [],
                        "peak_gb": 1, "parallelism": 1},
    }
    # a-gate-keyed-on-its-triggers-is-keyed-on-what-selects-it: its unit's key
    # is its trigger globs, given once per plan, and the runner's inputs.
    cases += 1
    by_triggers = plan_units({"by-triggers"}, rust="1.0.0", table=fixture,
                             triggers={"by-triggers": ["docs/b/**", "docs/a.md",
                                                       "docs/b/**"]})[0]
    if by_triggers.get("inputs") != sorted(
            {"docs/a.md", "docs/b/**", "scripts/gates/by-triggers.sh",
             *RUNNER_INPUTS}):
        failures.append(
            f"plan-fixture: `by-triggers` is keyed on its triggers, its "
            f"script and the runner's inputs, and was planned as {by_triggers}")
    try:
        plan_units({"by-triggers"}, rust="1.0.0", table=fixture)
        failures.append("plan-fixture: a gate keyed on its triggers was planned "
                        "with no triggers given, so its key named nothing")
    except ValueError:
        pass
    # a-gate-keyed-on-its-triggers-is-keyed-on-what-its-build-reads: a gate
    # that names a build is keyed on what the build reads without selecting
    # it (`gate_reads`) as well, and is refused a plan that was not given it.
    cases += 1
    building = {"building": dict(fixture["by-triggers"], builds=["sce-codegen"])}
    built_unit = plan_units({"building"}, rust="1.0.0", table=building,
                            triggers={"building": ["sce-build/src/**"]},
                            reads={"building": ["sce-revision/src/lib.rs",
                                                "sce-specs/tests/"]})[0]
    if built_unit.get("inputs") != sorted(
            {"sce-build/src/**", "sce-revision/src/lib.rs", "sce-specs/tests/",
             "scripts/gates/building.sh", *RUNNER_INPUTS}):
        failures.append(
            f"plan-fixture: `building` is keyed on its triggers, what its build "
            f"reads beside them, its script and the runner's inputs, and was "
            f"planned as {built_unit}")
    try:
        plan_units({"building"}, rust="1.0.0", table=building,
                   triggers={"building": ["sce-build/src/**"]})
        failures.append("plan-fixture: a gate keyed on its triggers that names "
                        "a build was planned with no reads given, so its key "
                        "missed what cargo reads to load the workspace")
    except ValueError:
        pass
    by_name = {u["name"]: u for u in plan_units(
        {"parent", "child", "alone"}, rust="1.0.0", table=fixture)}
    if set(by_name) != {"child", "alone"}:
        failures.append(
            f"plan-fixture: units {sorted(by_name)}; `parent` is run by "
            f"`child`'s unit and is no unit of its own")
    elif by_name["child"]["cache"] is not False:
        failures.append(
            "plan-fixture: `child` runs `parent`, whose verdict is not kept, "
            "and its own unit kept a verdict anyway")
    elif by_name["alone"]["cache"] is not True \
            or by_name["alone"].get("inputs") != sorted(
                {"docs/**", "scripts/gates/alone.sh", *RUNNER_INPUTS}):
        failures.append(
            f"plan-fixture: `alone` keeps its verdict over its own inputs, "
            f"its script's and the runner's, and was planned as "
            f"{by_name['alone']}")
    elif by_name["child"].get("secs") != 5.0:
        failures.append(
            f"plan-fixture: `child` runs 3s of its own and 2s of `parent`, "
            f"and was planned at {by_name['child'].get('secs')}s")
    elif by_name["child"].get("mem_gb") != 5.0:
        failures.append(
            f"plan-fixture: `child` peaks at 1 GiB and runs `parent`, which "
            f"peaks at 5, and asked for {by_name['child'].get('mem_gb')}")
    # A unit that reads HEAD carries its id and keeps its pass only then.
    cases += 1
    commit = "a" * 40
    with_head = plan_units({"at-head"}, rust="1.0.0", table=fixture,
                           head=commit)
    without = plan_units({"at-head"}, rust="1.0.0", table=fixture, head=None)
    if with_head[0]["argv"] != ["bash", "scripts/gate", "--rust", "1.0.0", "--unit",
                                "--at", commit, "at-head"] \
            or with_head[0]["cache"] is not True:
        failures.append(
            f"plan-fixture: a gate that reads HEAD was planned as "
            f"{with_head[0]}, not at HEAD's id with its pass kept")
    if "--at" in without[0]["argv"] or without[0]["cache"] is not False:
        failures.append(
            f"plan-fixture: with no HEAD id to carry, a gate that reads HEAD "
            f"was planned as {without[0]}; it keeps nothing and names no id")
    if "--at" in plan_units({"alone"}, rust="1.0.0", table=fixture,
                            head=commit)[0]["argv"]:
        failures.append(
            "plan-fixture: a gate that reads no commit carried HEAD's id, "
            "which would make its key a commit's for nothing")
    # A unit requires every package of what it runs, once each, and a unit
    # whose gates require none carries no `packages` key.
    cases += 1
    emulated = plan_units({"emulated", "over-emulated"}, rust="1.0.0", table=fixture)
    if [(u["name"], u.get("packages")) for u in emulated] \
            != [("over-emulated", ["binutils", "qemu-system-arm"])]:
        failures.append(
            f"plan-fixture: `over-emulated` runs `emulated`, and its unit should "
            f"require binutils and qemu-system-arm once each; planned as "
            f"{[(u['name'], u.get('packages')) for u in emulated]}")
    if "packages" in plan_units({"alone"}, rust="1.0.0", table=fixture)[0]:
        failures.append(
            "plan-fixture: a unit whose gates require no package carried a "
            "`packages` key")
    # A unit that reads the change set carries every range the push handed
    # the plan, through a gate it depends on as well, and keeps its pass only
    # then; a gate that reads none carries none. Measured 2026-11-20: the
    # landing unit of `mutation-rounds` carried no range and its gate chose
    # 578 casefiles from a build machine's upstream where the push chose 14.
    cases += 1
    ranges = ("b" * 40 + ".." + "c" * 40, "d" * 40 + ".." + "e" * 40)
    carried = ["--change-set", ranges[0], "--change-set", ranges[1]]
    for slug in ("on-changes", "after-changes"):
        planned = plan_units({slug}, rust="1.0.0", table=fixture,
                             landing=True, changes=ranges)[0]
        if planned["argv"] != ["bash", "scripts/gate", "--rust", "1.0.0", "--unit",
                               "--landing", *carried, slug] \
                or planned["cache"] is not True:
            failures.append(
                f"plan-fixture: `{slug}` runs a gate that reads the change set "
                f"and was planned as {planned}, not with both ranges and its "
                f"pass kept")
        bare = plan_units({slug}, rust="1.0.0", table=fixture)[0]
        if "--change-set" in bare["argv"] or bare["cache"] is not False:
            failures.append(
                f"plan-fixture: with no range to carry, `{slug}` was planned "
                f"as {bare}; it keeps nothing and names no range")
    if "--change-set" in plan_units({"alone"}, rust="1.0.0", table=fixture,
                                    changes=ranges)[0]["argv"]:
        failures.append(
            "plan-fixture: a gate that reads no change set carried the push's "
            "ranges, which would make its key a push's for nothing")
    for rust, clean, why in ((None, True, "no Rust release was named"),
                             ("1.0.0", False, "the tree differs from HEAD")):
        cases += 1
        if any(u["cache"] for u in plan_units({"alone"}, rust=rust,
                                              tree_is_head=clean,
                                              table=fixture)):
            failures.append(
                f"plan-fixture: a unit kept its verdict when {why}")
    # a-pass-that-could-be-a-skip-is-kept-only-from-a-landing. A gate that
    # skips where a tool is missing keeps its pass only from a unit that runs
    # with `--landing`, where the skip ends 3; and a unit asks bx for what the
    # gates it runs require, and for the most cores among them, through what
    # they depend on.
    cases += 1
    skip_fixture = {
        "prereq": {"cost_s": 1, "cache": True, "inputs": None, "commits": [],
                   "peak_gb": 1, "parallelism": 4,
                   "packages": ["pkg-b"]},
        "skipper": {"cost_s": 1, "cache": True, "inputs": None, "commits": [],
                    "peak_gb": 1, "parallelism": 1, "deps": ["prereq"],
                    "kept_only_from_a_landing": "x" * 40,
                    "needs": ["tool-a"], "packages": ["pkg-a"]},
    }
    branch_unit = plan_units({"skipper"}, rust="1.0.0", table=skip_fixture)[0]
    landing_unit = plan_units({"skipper"}, rust="1.0.0", table=skip_fixture,
                              landing=True)[0]
    if branch_unit["cache"] is not False or "--landing" in branch_unit["argv"]:
        failures.append(
            f"plan-fixture: a gate that can skip was planned outside a landing "
            f"as {branch_unit}; its pass could be a skip and is not kept")
    if landing_unit["cache"] is not True \
            or landing_unit["argv"] != ["bash", "scripts/gate", "--rust",
                                        "1.0.0", "--unit", "--landing", "skipper"]:
        failures.append(
            f"plan-fixture: a landing's unit of a gate that can skip was "
            f"planned as {landing_unit}, not with --landing and its pass kept")
    if landing_unit.get("needs") != ["tool-a"] \
            or landing_unit.get("packages") != ["pkg-a", "pkg-b"]:
        failures.append(
            f"plan-fixture: a unit asks bx for needs={landing_unit.get('needs')} "
            f"packages={landing_unit.get('packages')}, not the union of what "
            f"it runs requires")
    if landing_unit.get("parallelism") != 4.0:
        failures.append(
            f"plan-fixture: a unit asks bx for parallelism "
            f"{landing_unit.get('parallelism')}, not the most among what it "
            f"runs (4, a dependency's)")

    # every-skip-and-requirement-is-declared-in-its-shape. A gate that can skip
    # names what it skips without, and claims `cache: True` (its pass is kept
    # only from a landing, which `plan_units` decides); `needs` is a list of
    # names, as `packages` is held to be above.
    cases += 1
    for slug, spec in sorted(GATES.items()):
        why = spec.get("kept_only_from_a_landing")
        if why is not None:
            if not isinstance(why, str) or len(why.strip()) < 30:
                failures.append(
                    f"skip: {slug} declares kept_only_from_a_landing={why!r}; it "
                    f"says what the verdict rests on outside a landing")
            if spec.get("cache") is not True:
                failures.append(
                    f"skip: {slug} can skip and is `cache: False`; a landing "
                    f"closes the skip, so say what else keeps the pass from "
                    f"being kept, or claim it")
        if "needs" in spec:
            names = spec["needs"]
            if not isinstance(names, list) or not names \
                    or not all(isinstance(n, str) and n.strip() for n in names):
                failures.append(
                    f"needs: {slug} declares needs={names!r}; it is a "
                    f"non-empty list of names")

    cases += 1
    unpinned = plan_units({"alone"}, rust=None, table=fixture)
    if unpinned and "--rust" in unpinned[0]["argv"]:
        failures.append(
            "plan-fixture: a plan that named no release still put --rust in "
            "a unit's argv")

    # an-empty-selection-is-said. bx refuses a plan that lists no unit, so a
    # change that selects nothing is answered by one unit whose result is
    # already a pass, and that is the only shape it takes.
    cases += 1
    empty = nothing_to_verify("nothing")
    if plan_units(set()) or set(empty) != {"name", "result"} \
            or empty["result"].get("exit") != 0:
        failures.append(
            f"plan-empty: an empty selection is planned as {plan_units(set())} "
            f"and said as {empty}")

    # every-rust-toolchain-is-declared-pinned-and-named-by-its-units. The
    # tables, the pins, the packages' rust-version lines and the units that
    # need each table, held to one another (see "The Rust toolchains bx.toml
    # declares" above).
    cases += 1
    failures.extend(f"rust-toolchains: {f}" for f in rust_toolchain_failures(repo_root))

    # the-go-release-is-declared-pinned-and-named-by-its-units. The table, the
    # go.mod lines, the install's digest, the runs that choose the release and
    # the units that start go, held to one another (see "The Go release bx.toml
    # declares" above).
    cases += 1
    failures.extend(f"go-toolchain: {f}" for f in go_toolchain_failures(repo_root))

    # the-go-check-refuses-a-drifted-declaration. Its silence above is a reading
    # only if it refuses a table that misses a unit, names one for nothing, or
    # reads another file, and the absence of the table; and only if the scan
    # finds the gates whose scripts run go.
    cases += 1
    go_tables = bx_tool_tables(repo_root)
    go_names = [n for n, s in go_tables.items()
                if GO_TOOLCHAIN_INSTALL in s.get("install", "")]

    def go_drifted(edit):
        return {n: (edit(dict(s)) if n in go_names else s)
                for n, s in go_tables.items()}

    for label, edit_spec, want_refused in (
            ("as declared", lambda s: s, False),
            ("without w3c-go",
             lambda s: {**s, "needed_by": [e for e in s.get("needed_by", [])
                                           if e != "w3c-go"]}, True),
            ("naming clippy", lambda s: {**s, "needed_by": s.get("needed_by", [])
                                         + ["clippy"]}, True),
            ("without the CBOR pin's run",
             lambda s: {**s, "needed_by": [e for e in s.get("needed_by", [])
                                           if "capture_cbor_pin" not in e]}, True),
            ("reading another go.mod",
             lambda s: {**s, "version_file": "backends/go/runtime/go.mod"}, True)):
        refused = bool(go_toolchain_failures(repo_root, tables=go_drifted(edit_spec)))
        if refused != want_refused:
            failures.append(f"go-toolchain-refuses: the table {label} was "
                            f"{'refused' if refused else 'passed'}")
    go_commands = bx_action_runs(repo_root, "commands")
    unchosen = {name: (run.replace("sce_go_select verify || true; ", "")
                       if name in GO_COMMANDS else run)
                for name, run in go_commands.items()}
    if unchosen == go_commands \
            or not go_toolchain_failures(repo_root, commands=unchosen):
        failures.append("go-toolchain-refuses: a [commands.verify] that does not "
                        "choose the declared Go on its line was passed")
    absent = {n: s for n, s in go_tables.items() if n not in go_names}
    if not go_toolchain_failures(repo_root, tables=absent):
        failures.append("go-toolchain-refuses: no Go table was passed")
    setup_go_fixture = [
        "    - name: Setup Go",
        "      uses: actions/setup-go@" + "e" * 40 + " # v7.0.0",
        "      with:",
        f"        go-version-file: {GO_VERSION_FILE}",
        "    - name: Setup Go",
        "      uses: actions/setup-go@" + "e" * 40 + " # v7.0.0",
        "      with:",
        "        go-version: '1.23'",
        "    - name: Setup Go",
        "      uses: actions/setup-go@" + "e" * 40 + " # v7.0.0",
        "    - name: Next",
        "      run: echo uses: actions/setup-go@v5",
    ]
    flagged = [re.match(r"f:(\d+) ", f).group(1)
               for f in setup_go_failures(setup_go_fixture, "f")]
    if flagged != ["6", "10"]:
        failures.append(f"go-toolchain-refuses: of the fixture's setup-go steps, "
                        f"lines 6 and 10 read no go-version-file, line 2 does and "
                        f"line 12 is a command, not a step; the reader flagged "
                        f"{flagged}")
    scanned = go_gates_by_scan(repo_root)
    if not {"forge-go", "w3c-go"} <= scanned:
        failures.append(f"go-toolchain-refuses: the scan for go commands found "
                        f"{sorted(scanned)}, not forge-go and w3c-go, whose scripts "
                        f"run `go test`")

    # the-arm-compiler-is-declared-pinned-and-run-by-its-gate. The table, the
    # install's digests, the gate's PATH, the hosted lane and the budget, held
    # to one another (see "The ARM compiler bx.toml declares" above).
    cases += 1
    failures.extend(f"arm-toolchain: {f}" for f in arm_toolchain_failures(repo_root))

    # the-arm-check-refuses-a-drifted-declaration. Its silence above is a
    # reading only if it refuses a table needed by more than the gate, one that
    # checks a version any build prints, one that reads no release, and the
    # absence of the table.
    cases += 1
    arm_tables = bx_tool_tables(repo_root)
    arm_names = [n for n, s in arm_tables.items()
                 if ARM_TOOLCHAIN_INSTALL in s.get("install", "")]

    def arm_drifted(edit):
        return {n: (edit(dict(s)) if n in arm_names else s)
                for n, s in arm_tables.items()}

    for label, edit_spec, want_refused in (
            ("as declared", lambda s: s, False),
            ("needed by every gate", lambda s: {**s, "needed_by": ["scripts/gate"]}, True),
            ("checking the version any build prints",
             lambda s: {**s, "check": "13.2.1"}, True),
            ("reading another line",
             lambda s: {**s, "version_pattern": '^GO_REV="([0-9.]+)"'}, True)):
        refused = bool(arm_toolchain_failures(repo_root, tables=arm_drifted(edit_spec)))
        if refused != want_refused:
            failures.append(f"arm-toolchain-refuses: the table {label} was "
                            f"{'refused' if refused else 'passed'}")
    if not arm_toolchain_failures(
            repo_root, tables={n: s for n, s in arm_tables.items() if n not in arm_names}):
        failures.append("arm-toolchain-refuses: no ARM compiler table was passed")

    # a-link-to-the-distribution-is-not-a-tool. No tool table installs by
    # linking a file under /usr (bx RFC 0009, section 4.8): its bytes are apt's.
    # The fixture shows the reader refuses the shape the ARM tools had until
    # 2026-11-21 and passes a pinned install, so its silence on bx.toml is a
    # reading.
    cases += 1
    failures.extend(f"distribution-link: {f}"
                    for f in distribution_link_failures(bx_tool_tables(repo_root)))
    linked = distribution_link_failures({
        "linked": {"install": "mkdir -p {prefix}/bin && ln -s /usr/bin/qemu-system-arm "
                              "{prefix}/bin/qemu-system-arm"},
        "pinned": {"install": "ARM_GNU_TOOLCHAIN_ROOT={prefix} bash "
                              "scripts/install_arm_gnu_toolchain.sh"},
    })
    if len(linked) != 1 or "[platform.linux.tools.linked]" not in linked[0]:
        failures.append(f"distribution-link: of a table linking /usr/bin and a pinned "
                        f"one, the reader refused {linked}")

    # the-release-rustups-proxy-finds-is-installed-once. The check of the table
    # linked into rustup's home answers for the tree above; here it is shown
    # to refuse a table that drifted from rust-toolchain.toml or that only some
    # runs need, and the absence of the table, so its silence above is a reading.
    cases += 1
    proxy_tables = {name: spec for name, spec in bx_tool_tables(repo_root).items()
                    if spec.get("prefix_link") == RUSTUP_PROXY_PLACE}
    proxy_head = RUST_INSTALL_HEAD
    pinned_now = rust_toolchain_file(repo_root)
    for label, edit_spec, want_refused in (
            ("as declared", lambda s: s, False),
            ("a target rust-toolchain.toml does not list",
             lambda s: {**s, "install": s.get("install", "") + " -t aarch64-unknown-none"},
             True),
            ("needed by some runs only", lambda s: {**s, "needed_by": ["scripts/gate"]},
             True)):
        drifted = {name: edit_spec(dict(spec)) for name, spec in proxy_tables.items()}
        refused = bool(rustup_proxy_failures(repo_root, drifted, pinned_now, proxy_head))
        if refused != want_refused:
            failures.append(
                f"rustup-proxy-release: the table {label} was "
                f"{'refused' if refused else 'passed'}")
    if not rustup_proxy_failures(repo_root, {}, pinned_now, proxy_head):
        failures.append("rustup-proxy-release: no table linked into rustup's home "
                        "was passed")

    # every-lane-installs-the-pinned-rust. No lane or script installs or
    # names a Rust channel that moves (see "The Rust a lane installs" above).
    cases += 1
    failures.extend(f"every-lane-installs-the-pinned-rust: {f}"
                    for f in moving_rust_failures(repo_root))
    for key, want in (
            ("x-${{ hashFiles('Cargo.lock', 'sce-build/**/*.rs') }}", ["sce-build/**/*.rs"]),
            ("x-${{ hashFiles('sce-build/src/**') }}", ["sce-build/src/**"]),
            ("x-${{ hashFiles('rust-toolchain.toml') }}-${{ hashFiles('Cargo.lock', 'sce-build/Cargo.toml') }}", [])):
        if key_hashed_sources(key) != want:
            failures.append(f"every-lane-installs-the-pinned-rust: the sources {key!r} "
                            f"hashes read as {key_hashed_sources(key)!r}, not {want!r}")

    # every-lane-keys-its-own-gradle-cache. No two workflows save a Gradle
    # cache under one key (`shared_gradle_cache_keys`). The fixture holds the
    # reader to a shared key it refuses and a shared restore prefix it leaves.
    cases += 1
    gradle_step = lambda key: [
        "    - name: Cache Gradle",
        "      uses: actions/cache@" + "e" * 40,
        "      with:",
        "        path: |",
        "          ~/.gradle/caches",
        f"        key: {key}",
        "        restore-keys: |",
        "          Linux-gradle-",
    ]
    shared, seen = shared_gradle_cache_keys({
        "a.yml": gradle_step("Linux-gradle-h"), "b.yml": gradle_step("Linux-gradle-h")})
    if seen != 2 or len(shared) != 1:
        failures.append(f"every-lane-keys-its-own-gradle-cache: two lanes saving "
                        f"under one key gave {shared!r} from {seen} step(s)")
    apart, _ = shared_gradle_cache_keys({
        "a.yml": gradle_step("Linux-gradle-a-h"), "b.yml": gradle_step("Linux-gradle-b-h")})
    if apart:
        failures.append(f"every-lane-keys-its-own-gradle-cache: two lanes with keys "
                        f"of their own were refused: {apart!r}")
    failures.extend(f"every-lane-keys-its-own-gradle-cache: {f}"
                    for f in gradle_cache_failures(repo_root))

    # a-cache-keyed-by-the-run-is-kept-to-one. A cache saved under a key new on
    # every run is kept to one entry of its family, and no family begins another
    # (`per_run_cache_failures`). The fixture holds the reader to each refusal.
    cases += 1
    def per_run_job(key, prefix=None):
        lines = ["    - name: Restore ccache",
                 "      uses: actions/cache/restore@" + "e" * 40,
                 "      with:",
                 "        path: ~/.ccache",
                 f"        key: {key}"]
        if prefix is not None:
            lines += ["    - name: Keep",
                      "      uses: ./.github/actions/keep-one-cache",
                      "      with:",
                      f"        prefix: {prefix}",
                      "        keep: x"]
        return lines
    sha = "${{ github.sha }}"
    for label, workflows, want in (
            ("kept", {"a.yml": per_run_job(f"L-ccache-a-{sha}", "L-ccache-a-")}, 0),
            ("not kept", {"a.yml": per_run_job(f"L-ccache-a-{sha}")}, 1),
            ("a family that begins another",
             {"a.yml": per_run_job(f"L-ccache-{sha}", "L-ccache-"),
              "b.yml": per_run_job(f"L-ccache-b-{sha}", "L-ccache-b-")}, 1)):
        got, _ = per_run_cache_failures(workflows)
        if len(got) != want:
            failures.append(f"a-cache-keyed-by-the-run-is-kept-to-one: {label} gave "
                            f"{len(got)} refusal(s), not {want}: {got!r}")
    failures.extend(f"a-cache-keyed-by-the-run-is-kept-to-one: {f}"
                    for f in per_run_cache_sweep(repo_root))

    # every-action-is-named-by-its-commit. No workflow or action runs an
    # action from outside the repository by a tag or a branch (see "The
    # actions a lane runs" above). The fixture holds the line reader to each
    # shape it refuses and each it leaves alone, so a reader that stopped
    # matching cannot pass the sweep below by finding nothing.
    cases += 1
    pin, image_digest = "c" * 40, "d" * 64
    fixture_lines = [
        "      - uses: actions/checkout@v6",
        "        uses: actions/cache@main",
        "    uses: org/repo/.github/workflows/build.yml@v1",
        "      - uses: docker://alpine:3.20",
        f"      - uses: actions/checkout@{pin}",
        '      - uses: "actions/checkout@v6"',
        f"      - uses: actions/checkout@{pin} # v6.1.0",
        f"        uses: actions/cache/save@{pin}  # v5.1.0",
        "      - uses: ./.github/actions/install-rust",
        "      # uses: actions/checkout@v6",
        f"      - uses: docker://alpine@sha256:{image_digest}",
        "        run: echo uses: actions/checkout@v6",
    ]
    refused = [number for number, _ in unpinned_uses(fixture_lines)]
    if refused != [1, 2, 3, 4, 5, 6]:
        failures.append(
            f"every-action-is-named-by-its-commit: the fixture's lines 1-6 name "
            f"an action or an image by a name that moves, or pin one with no "
            f"release beside it, and 7-12 do not; the reader refused {refused}")
    failures.extend(f"every-action-is-named-by-its-commit: {f}"
                    for f in unpinned_action_failures(repo_root))

    # every-workspace-member-inherits-the-workspace-lints. The clippy gate's
    # `-D warnings` reaches only what its sweep compiles; the workspace's own
    # deny reaches every build of a member that inherits it.
    cases += 1
    failures.extend(f"every-workspace-member-inherits-the-workspace-lints: {f}"
                    for f in workspace_lint_failures(repo_root))

    if failures:
        return print_self_test_failures(failures)
    print(f"gate_registry self-test: {cases} cases OK", file=sys.stderr)
    return 0


def print_plan(args, repo_root: Path) -> int:
    """`--plan`: the units of the selection, one JSON object a line.

    What a plan is FOR is said in `plan_units`. Here is only how the three
    selections reach it, each through the function that already answers it
    for the runner: `select` for a change, the whole table for `--all`, and
    the named gates with their dependencies. The reason goes to stderr, where
    bx shows a plan command's own words; stdout carries nothing but units.

    A refusal exits 2 and prints no unit: bx runs no part of a plan whose
    command failed, which is the right answer to a selection that could not
    be made.
    """
    if args.rust is not None and not RUST_RELEASE.match(args.rust):
        print(f"gate_registry: --rust takes a release such as 1.91.0, got "
              f"{args.rust!r}", file=sys.stderr)
        return 2
    picked = [bool(args.all), bool(args.named),
              args.changed is not None or args.changed_from is not None]
    if sum(picked) != 1:
        print("gate_registry: --plan takes exactly one selection: --all, "
              "--named, or a change (--changed / --changed-from)",
              file=sys.stderr)
        return 2
    if args.landing and args.named:
        print("gate_registry: --landing selects for an update to main, and "
              "named gates are not a selection; pass --all or a change",
              file=sys.stderr)
        return 2
    # What a landing leaves to hosted CI, named on stderr below so the
    # judgment says which reached gates it did not measure.
    off_fleet: list[str] = []
    if args.all:
        selected, reason = set(GATES), "every gate (--all)"
        if args.landing:
            off_fleet = hosted_only(repo_root, selected)
            selected = drop_hosted_only(selected, fleet_platforms(repo_root))
            reason = "every gate the fleet can run (--all --landing)"
    elif args.named:
        unknown = sorted(set(args.named) - set(GATES))
        if unknown:
            print("gate_registry: unknown slug(s): " + ", ".join(unknown),
                  file=sys.stderr)
            return 2
        selected = set(args.named)
        for slug in args.named:
            selected |= transitive_deps(slug)
        reason = "the named gate(s): " + " ".join(args.named)
    else:
        if args.changed is not None:
            changed = list(args.changed)
        else:
            text = sys.stdin.read() if str(args.changed_from) == "-" \
                else args.changed_from.read_text(encoding="utf-8")
            changed = [ln.strip() for ln in text.splitlines() if ln.strip()]
        keys, reason = select(repo_root, changed, landing=args.landing)
        selected = set(keys)
        if args.landing:
            off_fleet = hosted_only(repo_root, matched(repo_root, changed)[0])
            reason += ", ci_only included (--landing)"

    tree_is_head = tracked_tree_is_head(repo_root)
    head = head_commit(repo_root)
    units = plan_units(selected, rust=args.rust, tree_is_head=tree_is_head,
                       head=head, landing=bool(args.landing),
                       changes=tuple(args.changed_in or ()),
                       shards={s: shard_count(repo_root, s) for s in selected},
                       triggers=gate_triggers(repo_root),
                       reads=gate_reads(repo_root))
    print(f"  selection: {reason}", file=sys.stderr)
    for slug in off_fleet:
        print(f"  NOT JUDGED HERE, hosted CI audits it after landing: {slug} "
              f"{why_hosted(slug)}", file=sys.stderr)
    if head is None:
        print("  git names no HEAD commit, so no unit that reads it keeps a "
              "verdict", file=sys.stderr)
    if args.rust is None:
        print("  no Rust release was named, so no unit's verdict is kept",
              file=sys.stderr)
    if not tree_is_head:
        print("  the tracked tree differs from HEAD, so no unit's verdict is "
              "kept", file=sys.stderr)
    if not units:
        units = [nothing_to_verify(reason)]
    for unit in units:
        print(json.dumps(unit, sort_keys=True))
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo-root", default=".", type=Path)
    ap.add_argument("--changed-from", type=Path,
                    help="file with one changed path per line ('-' for stdin)")
    ap.add_argument("--changed", nargs="*", default=None)
    ap.add_argument("--changed-in", action="append", metavar="BASE..TIP",
                    help="with --plan: a range the change set was read from, "
                         "carried in the argv of every unit whose gate reads "
                         "the change set (`commits: [\"CHANGES\"]`)")
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
    ap.add_argument("--plan", action="store_true",
                    help="print the bx check units (one JSON object a line) "
                         "that verify the selection: the change's gates "
                         "(--changed / --changed-from), every gate (--all) or "
                         "the named gates and their dependencies (--named)")
    ap.add_argument("--placement", nargs="+", metavar="SLUG",
                    help="for gates in run order, print where each runs, one "
                         "'host <slug>' or 'container <slug> <containerfile>' "
                         "a line, leaving out what runs inside an image")
    ap.add_argument("--landing", action="store_true",
                    help="select as the judgment of an update to main: every "
                         "reached gate the fleet can run, ci_only included "
                         "(with a change, --plan, --ci-owed or --ci-unowed)")
    ap.add_argument("--all", action="store_true",
                    help="with --plan: every gate, ci_only included")
    ap.add_argument("--named", nargs="+", metavar="SLUG",
                    help="with --plan: these gates and what they depend on")
    ap.add_argument("--rust", metavar="RELEASE",
                    help="with --plan: the Rust release every unit runs, as "
                         "`rustc --version` prints it (1.91.0); without it no "
                         "unit's verdict is kept")
    ap.add_argument("--shard-env", metavar="SLUG",
                    help="print the variable SLUG's script reads its shard "
                         "from (`shard_env`), or nothing when it has none")
    args = ap.parse_args()

    repo_root = args.repo_root.resolve()

    if args.shard_env is not None:
        if args.shard_env not in GATES:
            print(f"gate_registry: unknown slug: {args.shard_env}",
                  file=sys.stderr)
            return 2
        print(GATES[args.shard_env].get("shard_env", ""))
        return 0

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

    if args.placement:
        unknown = sorted(set(args.placement) - set(GATES))
        if unknown:
            print("gate_registry: unknown slug(s): " + ", ".join(unknown),
                  file=sys.stderr)
            return 2
        for where, slug, containerfile in container_placement(args.placement):
            print(f"{where} {slug} {containerfile}".rstrip())
        return 0

    if args.plan:
        return print_plan(args, repo_root)
    if args.all or args.named or args.rust is not None or args.changed_in:
        ap.error("--all, --named, --rust and --changed-in belong to --plan")
        return 2

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
        for slug in ci_owed(repo_root, changed, landing=args.landing):
            workflows = ",".join(GATES[slug].get("workflows") or ["?"])
            print(f"{slug} {workflows}")
        return 0

    if args.ci_unowed:
        for slug in ci_unowed(repo_root, changed, landing=args.landing):
            workflows = ",".join(GATES[slug].get("workflows") or ["?"])
            print(f"{slug} {workflows}")
        return 0

    keys, reason = select(repo_root, changed, landing=args.landing)
    if args.explain:
        print(f"  selection: {reason}"
              + (", ci_only included (--landing)" if args.landing else ""),
              file=sys.stderr)
        if args.landing:
            for slug in hosted_only(repo_root, matched(repo_root, changed)[0]):
                print(f"  NOT JUDGED HERE, hosted CI audits it after landing: "
                      f"{slug} {why_hosted(slug)}", file=sys.stderr)
    for k in keys:
        print(k)
    return 0


if __name__ == "__main__":
    sys.exit(main())
