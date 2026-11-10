#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
PACED_BUDGET_SHARE_CEILING = 0.237

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
# The residue, stated rather than hidden: a tool bx.toml requires by name only
# (`needs`, `packages`) is whatever version the machine carries, and a verdict
# that moved with that version would be kept across it. bx runs one kept pass in
# sixteen again and blocks a key whose verdict changed (RFC 0020, section 9),
# which is the instrument that finds such a gate.
#
# `inputs` is the other half of the key. `None` is the whole tracked tree; a list
# of patterns (bx RFC 0003, section 6) narrows the key to those files, and bx
# then runs the unit in a copy of them alone, under strace, and calls a read
# outside them "could not measure" (RFC 0020, section 5.2). Every gate is `None`
# today, and not by default: that copy holds no `.git`, and the runner and the
# gate library both ask git for the repository root before anything else, so a
# narrowed unit could only ever end "could not measure".
# `narrowed-units-need-a-git-free-root` in the self-test refuses a list until
# that changes.
#
# What whole-tree keys buy, measured against what was asked of them: one tree
# is judged once (a push retried over the same commits, a full run after a
# scoped one, two sessions verifying the same tree). A rebase over commits that
# touch nothing a gate reads still moves the tree, so it is a new key for every
# unit; that reuse needs narrowed inputs, which needs the root above. bx has
# proposed making that copy a git repository whose HEAD holds the inputs (not
# decided on 2026-11-04); this guard is the line to revisit when it is.
#
# `commits` is what a verdict reads of git beyond the tree: the commits whose
# ids, ancestry or objects it asks about. The key holds the tree's content and
# no commit, so a verdict that reads one is a function of the key only when the
# commit's id is in the unit's argv. `"HEAD"` is the one name the plan carries:
# it resolves HEAD once and puts `--at <id>` in the argv of every unit that runs
# such a gate, and the runner refuses (exit 3, "could not run") a checkout that
# is at any other commit. HEAD's id fixes its ancestry and the submodule commits
# its tree records, so one id covers what these gates read. A gate that reads a
# ref the plan cannot carry (an upstream branch, which moves) is `cache: False`
# and says so. `[]` is a verdict that reads no commit: `git ls-files` and
# `git grep` read the tracked tree, which is the key. The generator stamps the
# commit it was built at into what it says about itself (`GENERATOR_COMMIT`);
# no gate compares that stamp with anything, since every commit would then
# fail it, so building the generator is not reading a commit.
# `a-verdict-that-reads-head-carries-it` derives the HEAD readers a script
# shows and refuses one that does not declare it.
#
# `peak_gb` is the memory one run of the gate's OWN script takes at its peak,
# in GiB; a unit asks bx for the largest figure among the gates it runs
# (`mem_gb`, bx RFC 0004, section 6). bx starts units on a machine only while
# their figures fit what it has free, and gives a unit no hard limit by it. The
# figures in `PEAK_MEASURED` were read off a run; every other one is the figure
# of its class, and is a scheduling estimate until a run is read: 5 for a gate
# that compiles (cargo, C and C++, Gradle, Go, Swift), 1 for a script that runs
# built tools over the tree.
PEAK_MEASURED: dict[str, str] = {
    "codegen-build": "4.98 GB at the peak of an sce-codegen build (bx, "
                     "2026-11-04)",
    "regen-reproduces": "5 to 6.3 GB over a full regeneration of the "
                        "committed trees (bx, 2026-11-04)",
    "w3c-wasm": "5.3 GB at the peak of a cold emscripten configure and build "
                "of both programs with three build slots (bx --local, "
                "2026-11-04), before the gate script existed",
}

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

# The shape of a full commit id a plan carries (SHA-1, or SHA-256).
COMMIT_ID = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$")

# The commit names a gate's `commits` may hold: the ones the plan carries.
CARRIED_COMMITS = ("HEAD",)

# The name of the one unit a plan carries when the change selects no gate: bx
# refuses a plan that lists nothing ("a plan that lists nothing has measured
# nothing"), and a selection that is empty is an answer, so it is said as a
# unit whose result is already known.
NOTHING_TO_VERIFY = "no-gate-applies"

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
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
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
        "cache": False,
        "no_cache_reason": "skip-capable: `scripts/test_emit_manifest_fail_fast.sh` "
                           "exits 0 on a machine without clang++-19, which bx.toml "
                           "does not require, so a pass there is the machine's and "
                           "not the tree's",
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
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
        # The harness itself. The gate invokes `scripts/mutate` twice — once
        # to ask a casefile what it declares, once to run the round — so an
        # edit to it changes every verdict this lane produces. It was not
        # declared, and `--ci-owed` therefore did not name this lane for a
        # commit that changed nothing else (`script-read-coverage`).
        #
        # The cost model, because the gate sizes every job from it and judges
        # every job's duration against it: an edit to a runner's row changes
        # how many jobs the lane schedules and which of them turn red.
        "extra": ["scripts/mutate", "scripts/lib/mutation_rounds_cost.sh"],
        "cost_s": 877,
        "summary": "changed mutation targets still turn their suites red",
        "cache": False,
        "no_cache_reason": "reads the clock and git refs: it judges each round's "
                           "duration against the cost model (`date +%s`), and with "
                           "no change set it takes its base from the upstream ref",
        "inputs": None,
        # The upstream ref it reads moves, and the gate resolves it itself, so
        # there is no commit the plan could carry for it.
        "commits": [],
        "peak_gb": 5,
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
        # The lint set moves with each stable release, which is why the plan
        # names the release in every unit's argv: a new one is a new key.
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
        # `tests/json_text/**` and `tests/durations/**` are tables the runtime's
        # and sce-build's unit tests read at run time — every engine is held to
        # them — so an edit to a table alone has to start the suite that reads it.
        "extra": [
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
            # The C11 runtime's build, whose vocabulary of what a machine
            # requires `a_c11_machine_includes_what_it_states_it_requires`
            # compiles in and holds to what the generator can state.
            "backends/c/runtime/CMakeLists.txt",
            # The standard library, whose statecharts that save are held to
            # their pinned shapes (`a_standard_statechart_that_saves_keeps_its_shape`,
            # docs/adr/0045): an edit of one alone has to start the suite.
            "stdlib/**",
            # The scenarios of the standard host features, which
            # `every_replay_scenario_keeps_to_its_vocabulary` holds to the
            # members their drivers read: an edit of one alone has to start
            # the suite.
            "tests/host_features/**",
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
        "cache": False,
        "no_cache_reason": "skip-capable on an artifact: `forge_conformance` "
                           "skips its Kotlin compile and run cases where the "
                           "gitignored forge-runtime jar was never built "
                           "(`KOTLIN_JAR_HINT`), so a pass depends on what that "
                           "machine built before. It also reads HEAD's history "
                           "(`lowering_decision_ledger`), which `commits` carries",
        "inputs": None,
        "commits": ["HEAD"],
        "peak_gb": 5,
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
        "extra": ["backends/cpp/**", "sce/**", "cmake/SCECodegenPlan.cmake",
                  "cmake/SCEFindCodegen.cmake", "cmake/SCEClangFormat.cmake",
                  "cmake/SCETestWarnings.cmake"],
        "deps": ["codegen-build"],
        "ci_only": "59s, the last move needed to fit the ceiling. "
                   "forge-conformance.yml runs it beside the other arms.",
        "cost_s": 59,
        "summary": "C++ forge conformance build + test",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
        # toolchain is what keeps it out: a hand-installed pin that `provision`
        # puts on a build machine, and a push the hook handed to a machine that
        # lacks it would be refused for a reason that says nothing about the
        # tree. That is `genmc-queue`'s argument, and the gate fails loudly
        # without a toolchain rather than skipping, so the lane is the place it
        # runs until every build machine is verified to carry the pin.
        "ci_only": "38s, and the 6.4.0 toolchain is a hand-installed pin "
                   "(`[platform.linux.tools.swift]` in bx.toml) that not every build machine "
                   "is verified to carry; the gate refuses without it, so a "
                   "push that selected it there would be refused for a reason "
                   "that says nothing about the tree. swift-backend.yml "
                   "installs the pin by its digest.",
        "cost_s": 38,
        "summary": "Swift backend: runtime, host features and generated machines",
        # The toolchain is a pinned tool of the platform, so its version is in
        # the key, and a machine without it fails the gate loudly.
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
    },
    # The Swift W3C arm, the twin of `w3c-go`: `sce-codegen generate-w3c -l swift` writes the
    # suite and the engine runs it. It rides the workflow of `swift-backend` as a second step of
    # the same job, as `mutation-cases` rides `tree-hygiene.yml`, because what the two need is the
    # same: the generator built, the pinned toolchain installed and checked against its digest.
    # A job of its own would repeat those steps and pay for them twice.
    #
    # `ci_only` for the reason `swift-backend` is: the 6.4.0 toolchain is a hand-installed pin
    # that not every build machine is verified to carry, and the gate fails loudly without it.
    "w3c-swift": {
        "workflows": ["swift-backend.yml"],
        "runner_workflow": True,
        "deps": ["codegen-build"],
        "ci_only": "the 6.4.0 toolchain is a hand-installed pin (`[platform.linux.tools.swift]` "
                   "in bx.toml) that not every build machine is verified to "
                   "carry; the gate refuses without it, so a push that "
                   "selected it there would be refused for a reason that says "
                   "nothing about the tree. swift-backend.yml installs the pin "
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
                   "in bx.toml) that not every build machine is verified to "
                   "carry; the gate refuses without it, so a push that "
                   "selected it there would be refused for a reason that says "
                   "nothing about the tree. swift-backend.yml installs the pin "
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
                   "in bx.toml) that not every build machine is verified to "
                   "carry; the gate refuses without it, so a push that "
                   "selected it there would be refused for a reason that says "
                   "nothing about the tree. swift-backend.yml installs the pin "
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
                   "and the build machines do not all have it. "
                   "genmc-queue.yml builds it once per pin and caches it.",
        "cost_s": 72,
        "summary": "C11 and C++ queues under GenMC + weakened-ordering mutants",
        # GenMC explores every interleaving its model admits rather than
        # sampling some, and its revision is pinned by the tracked installer.
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
    },
    # The gate whose absence let a stale verifies-catalog reach CI red:
    # `ledger-citations` runs mnemosyne-cli, this workflow runs a separate
    # python generator, and the hook mirrored only the first.
    "spec-snapshot": {
        "workflows": ["spec-snapshot-drift.yml"],
        "runner_workflow": True,
        "cost_s": 13,
        "summary": "spec snapshot integrity + verifies-catalog drift",
        # `check_spec_drift.py --mode integrity` is the offline mode: it hashes
        # the snapshot against its recorded provenance. The `upstream` mode,
        # which fetches the published specification, is not what this runs.
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
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
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
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
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
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
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
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
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
    },
    "codec-no-alloc": {
        "workflows": ["sce-forge-codec-no-alloc.yml"],
        "runner_workflow": True,
        "ci_only": "90s. sce-forge-codec-no-alloc.yml runs it, next to its "
                   "alloc-on sibling.",
        "cost_s": 90,
        "summary": "generated codecs compile without alloc",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
        "cache": False,
        "no_cache_reason": "reads an artifact another gate leaves: the codegen "
                           "and engine WASMs `visualizer-wasm` writes into "
                           "gitignored directories, which the key does not hold "
                           "and nothing here rebuilds",
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
    },
    "visualizer-wasm": {
        "workflows": ["deploy-visualizer.yml"],
        "runner_workflow": True,
        "ci_only": "77s measured 2026-08-22 (declared 42) building three "
                   "WASM artifacts. deploy-visualizer.yml runs it, and this "
                   "gate declares no trigger of its own, so the workflow's "
                   "filter IS the trigger — the two cannot disagree.",
        "cost_s": 77,
        "summary": "codegen + visualizer + DOOM WASM builds",
        "cache": False,
        "no_cache_reason": "skip-capable: the emscripten half runs only where "
                           "emsdk is found (on PATH or under the home), which "
                           "bx.toml does not require, so a pass without it judged "
                           "one artifact of three",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
                   "holds, which a push hook cannot assume.",
        "cost_s": 107,
        "summary": "Rust engine built for wasm32, every scenario replayed in Node and Chrome",
        "cache": False,
        "no_cache_reason": "a host browser: the scenarios replay in whatever "
                           "headless Chrome the machine carries, and no "
                           "declaration names its version",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
                   "compiles. cpp-interface-standard.yml runs it in a pinned "
                   "GCC 15 container, because the newest libstdc++ is the one "
                   "that drops a transitive include first and a push host's "
                   "compiler is whatever that host carries.",
        "cost_s": 300,
        "summary": "C++ interface compiled alone at C++17 and C++20 (ADR 0059)",
        "cache": False,
        "no_cache_reason": "a host compiler: it compiles every header alone with "
                           "-Werror under whatever GCC and libstdc++ the machine "
                           "carries, and its lane pins a GCC 15 container because "
                           "that verdict moves with the compiler",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
    },
    "w3c-c11": {
        "workflows": ["w3c-tests.yml"],
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
                  "schemas/sce-test-log.v1.schema.json"],
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
                  "sce-build/tests/fixtures/test_profile/runtime/expected_logs/**"],
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
                  "sce/include/scripting/*.lua", "tests/ecmascript/**"],
        "deps": ["codegen-build"],
        "ci_only": "unmeasured, and the lane it mirrors says why it cannot "
                   "be a push gate: `test-rust` carries a 65-minute timeout, "
                   "the longest of the W3C jobs, because it links some 200 "
                   "release test binaries under thin LTO with one codegen "
                   "unit. w3c-tests.yml declares no `paths:` filter, so CI "
                   "runs this arm on every push.",
        "summary": "W3C conformance, Rust AOT",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
                  # The shared Lua the Lua engine copies in at build time, and
                  # the shared tables its readers ask.
                  "sce/include/scripting/*.lua", "tests/ecmascript/**",
                  "tests/ecmascript/kotlin_frontend_refusals.json",
                  "tests/w3c/conformance/fixtures.json",
                  "resources/150/test150.scxml",
                  # The test log every runtime writes of the generated suite,
                  # which `ATestSuiteRunsThroughItsRunnerTest` reads at run time.
                  "sce-build/tests/fixtures/test_profile/runtime/expected_logs/**"],
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
                  "sce-build/tests/fixtures/test_profile/runtime/expected_logs/**"],
        "deps": ["codegen-build"],
        "ci_only": "72s. w3c-tests.yml is unfiltered, so CI runs this arm on "
                   "every push.",
        "cost_s": 72,
        "summary": "W3C conformance, Python AOT",
        "cache": True,
        "inputs": None,
        "commits": [],
        "peak_gb": 1,
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
        "cache": False,
        "no_cache_reason": "skip-capable: it exits 0 on a machine without "
                           "python3-config, which bx.toml requires by no name, so "
                           "a pass there is the machine's and not the tree's",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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
        "runner_workflow": True,
        "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
                   "what the native runner reads, plus the CMake that "
                   "configures the two emscripten trees and the Node host the "
                   "programs are linked with (tests/w3c/wasm_polyfill.js).",
        # What the two programs are built from and run against: the engine,
        # the runner and its Node host (tests/w3c/), the mock the runner
        # links, the CMake that configures the tree and registers the
        # fixtures (the root list carries the WASM link settings), the
        # generator and its templates, and the documents.
        "extra": ["sce/**", "tests/w3c/**", "tests/mocks/**",
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
        "cache": False,
        "no_cache_reason": "skip-capable: it exits 0 on a machine without "
                           "emsdk, which bx.toml requires by no name, and the "
                           "Node it runs on is the one that emsdk ships, at no "
                           "version a declaration pins",
        "inputs": None,
        "commits": [],
        "peak_gb": 5.3,
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
        "runner_workflow": True,
        "narrows": "the same catch-all reading as `cpp-suite`, whose "
                   "WebAssembly half this is: it builds the whole emscripten "
                   "tree, so its inputs are the engine, its tests, the CMake "
                   "that registers them (the root list carries the WASM "
                   "settings and says what the tree does not declare), the "
                   "generator and its templates, and the documents.",
        "extra": ["sce/**", "tests/**", "cmake/**", "CMakeLists.txt",
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
        "cache": False,
        "no_cache_reason": "skip-capable: it exits 0 on a machine without "
                           "emsdk, which bx.toml requires by no name, and the "
                           "Node it runs on is the one that emsdk ships, at no "
                           "version a declaration pins",
        "inputs": None,
        "commits": [],
        "peak_gb": 5,
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


def unit_inputs(closure, table=None):
    """The input patterns of a unit that runs every gate in `closure`.

    `None`, the whole tracked tree, as soon as one of them reads the whole
    tree: a key narrower than what the unit reads would let a pass outlive
    a change to the part left out. Otherwise the union of their patterns,
    sorted so that one closure always yields one key.
    """
    table = GATES if table is None else table
    patterns: set[str] = set()
    for slug in closure:
        own = table[slug].get("inputs")
        if own is None:
            return None
        patterns.update(own)
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


def plan_units(selected, rust=None, tree_is_head=True, table=None,
               head=None) -> list[dict]:
    """The bx check units (bx RFC 0004, section 6) that verify `selected`.

    `selected` is a set the registry already chose: `select` for a change,
    every gate for `--all`, a named closure. The plan adds no selection of
    its own, which is what lets the push run the same gates whether bx
    carries them or the runner runs them in place.

    ONE UNIT PER GATE THAT NO OTHER SELECTED GATE RUNS. A unit is
    `bash scripts/gate [--rust R] <slug>`, and the runner gives a named gate
    its dependency closure and stops at the first failure, so a dependency's
    verdict is part of every dependent's unit. A unit of its own would be
    the same build again on another machine, measuring nothing new.

    A UNIT THAT READS HEAD CARRIES ITS ID. When a gate the unit runs
    declares `commits: ["HEAD"]`, the argv gets `--at <head>`, so the commit
    is in the key, and the runner refuses a checkout at any other commit.

    A UNIT'S VERDICT IS KEPT (`cache`) only when all of these hold:
      * the plan names the Rust release (`rust`), because every unit's argv
        then carries it and a new pinned release is a new key. Without it a
        unit runs whatever cargo its machine has on PATH, which is not in
        the key;
      * the tracked tree is HEAD's (`tree_is_head`, see
        `tracked_tree_is_head`);
      * every gate the unit runs declares `cache: True`;
      * a unit that reads HEAD has its id (`head`) to carry.
    Each unit says `cache` either way, because the set in bx.toml keeps
    verdicts by default and a unit that is not to be kept has to say so.

    `exclusive` is `UNIT_EXCLUSIVE` for every unit. `secs` is the declared
    cost of everything the unit runs, a first estimate that bx replaces with
    its own timings, and `mem_gb` the largest `peak_gb` among them.
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
        kept = (rust is not None and tree_is_head
                and all(table[s]["cache"] is True for s in closure)
                and (head is not None or not reads_head))
        argv = ["bash", "scripts/gate"]
        if rust is not None:
            argv += ["--rust", rust]
        if reads_head and head is not None:
            argv += ["--at", head]
        argv.append(slug)
        unit = {
            "name": slug,
            "argv": argv,
            "exclusive": [UNIT_EXCLUSIVE],
            "cache": kept,
            "broke": f"gate {slug} FAILED: run just this one with "
                     f"`scripts/gate {slug}`",
            "blind": f"gate {slug} COULD NOT RUN on that machine: its own "
                     f"tools, or the Rust release the plan named, are missing "
                     f"there. Nothing is claimed about the tree.",
        }
        cost = sum(table[s].get("cost_s") or 0 for s in closure)
        if cost > 0:
            unit["secs"] = float(cost)
        unit["mem_gb"] = float(max(table[s]["peak_gb"] for s in closure))
        inputs = unit_inputs(closure, table)
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
# release and the nightly are pinned in `scripts/lib/rust_toolchains.sh`, the
# release each package builds with is its manifest's `rust-version`, and the
# units that need a table are the ones whose gates ask for it. A table a unit
# needs and does not name is a unit placed on a machine without its compiler,
# which ends "could not run"; one it names and does not need keeps that unit
# off every machine that lacks it, for nothing.
RUST_TOOLCHAIN_INSTALL = "scripts/install_rust.sh"
RUST_TOOLCHAIN_PINS = "scripts/lib/rust_toolchains.sh"
RUST_TOOLCHAIN_PLACE = "~/.local/rust/{version}"
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
    members_block = re.search(r"^members\s*=\s*\[(.*?)\]", root_text, re.M | re.S)
    members = re.findall(r'"([^"]+)"', members_block.group(1)) if members_block else []
    workspace = re.search(r'^\[workspace\.package\][^\[]*?^rust-version\s*=\s*"([^"]+)"',
                          root_text, re.M | re.S)
    declared: dict[str, list[str]] = {}
    for member in members:
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


def msrv_release(version: str):
    """The release a `rust-version` names: `1.87` is `1.87.0`, the oldest it
    admits (`sce_rust_msrv_release` in scripts/lib/rust_toolchains.sh)."""
    if re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        return version
    if re.fullmatch(r"[0-9]+\.[0-9]+", version):
        return version + ".0"
    return None


def rust_toolchain_failures(repo_root: Path) -> list[str]:
    """What is wrong with the Rust toolchains bx.toml declares, as sentences."""
    failures: list[str] = []
    tables = {name: spec for name, spec in bx_tool_tables(repo_root).items()
              if RUST_TOOLCHAIN_INSTALL in spec.get("install", "")}
    if not tables:
        return [f"bx.toml declares no Rust toolchain: no [platform.linux.tools.*] "
                f"table installs with {RUST_TOOLCHAIN_INSTALL}"]

    pin_text = (repo_root / RUST_TOOLCHAIN_PINS).read_text(encoding="utf-8")
    pins: dict[str, str] = {}
    for var in ("SCE_RUST_RELEASE", "SCE_RUST_NIGHTLY"):
        pin = re.search(rf'^{var}="([^"]+)"$', pin_text, re.M)
        if pin:
            pins[var] = pin.group(1)
        else:
            failures.append(f"{RUST_TOOLCHAIN_PINS} has no {var}=\"...\" line")

    install_head = (f"RUSTUP_HOME={{scratch}}/rh CARGO_HOME={{scratch}}/ch "
                    f"bash {RUST_TOOLCHAIN_INSTALL} {{version}} {{prefix}}")
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
    if release and tables[release].get("version_file") != RUST_TOOLCHAIN_PINS:
        failures.append(f"[platform.linux.tools.{release}] does not read the release "
                        f"from {RUST_TOOLCHAIN_PINS}, the one line that pins it")
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
            if name not in (release, nightly)}
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
        if "sce_gate_require_rust " in code or "sce_gate_configure_wasm_tree" in code:
            return "nightly"
        return ""

    kind = {slug: asks(slug) for slug in GATES}
    lines = {}
    for slug in GATES:
        unit = plan_units({slug}, rust=pins.get("SCE_RUST_RELEASE", "0.0.0"),
                          head="0" * 40)
        lines[slug] = " ".join(unit[0]["argv"]) if unit else ""

    def named(name: str, line: str) -> bool:
        return any(e and e in line for e in tables[name].get("needed_by", []))

    for name in sorted(versions):
        if name == release:
            want_kind = None
        elif name == nightly:
            want_kind = "nightly"
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
        if "inputs" not in spec:
            failures.append(
                f"verdict: {slug} does not declare `inputs` (`None` for the "
                f"whole tracked tree, or a list of patterns)")
            continue
        inputs = spec["inputs"]
        if inputs is None:
            continue
        if not isinstance(inputs, list) or not inputs:
            failures.append(
                f"verdict: {slug} declares inputs={inputs!r}; it is `None` or "
                f"a non-empty list of patterns")
            continue
        for pattern in inputs:
            if (not isinstance(pattern, str) or not pattern
                    or pattern.startswith("/")
                    or ".." in pattern.split("/")):
                failures.append(
                    f"verdict: {slug} declares the input {pattern!r}, which is "
                    f"not a pattern under the repository root")

    # A measured peak names a gate that exists, so the note beside it cannot
    # outlive the gate it was read off.
    cases += 1
    for slug in sorted(set(PEAK_MEASURED) - set(GATES)):
        failures.append(f"verdict: PEAK_MEASURED names {slug}, which is no gate")

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

    # narrowed-units-need-a-git-free-root. bx runs a unit with input patterns
    # in a copy of those files alone, with no `.git` (bx RFC 0020, section
    # 5.2). A unit is the runner, and every gate sources its library; both
    # ask git for the repository root before anything else, so a narrowed
    # unit there would end "could not measure" on every push, and the push
    # would fail on a property of the copy, not of the tree. Derived from
    # the two files, so the refusal lifts the day neither asks git.
    cases += 1
    narrowed = sorted(s for s, spec in GATES.items() if spec.get("inputs"))
    if narrowed:
        asks_git = [rel for rel in ("scripts/gate", "scripts/gates/lib.sh")
                    if "rev-parse --show-toplevel" in
                    (repo_root / rel).read_text(encoding="utf-8",
                                                errors="replace")]
        if asks_git:
            failures.append(
                f"narrowed: {', '.join(narrowed)} declare input patterns, and "
                f"{' and '.join(asks_git)} ask git for the repository root; "
                f"bx runs a narrowed unit in a copy of its inputs that holds "
                f"no .git, so the unit could only end 'could not measure'")

    # the-plan-is-the-selection. The push used to run exactly the gates
    # `select` chose; it now hands bx the units `plan_units` makes of that
    # choice. Each path set below must come back as units that cover every
    # selected gate, name no gate the selection left out, and never make a
    # unit of a gate another unit already runs. A plan that drifted from the
    # selection would be the push verifying something else than it says.
    for label, selection in (
            ("rust-source", select(repo_root, ["sce-build/src/mesh/deploy.rs"])[0]),
            ("docs-only", select(repo_root, ["README.md"])[0]),
            ("go-backend", select(repo_root, ["backends/go/forge-runtime/go.mod"])[0]),
            ("unclassified", select(repo_root, ["some_new_top_level_dir/x.txt"])[0]),
            ("every-gate", list(GATES)),
    ):
        cases += 1
        head = "0" * 40
        units = plan_units(set(selection), rust="1.0.0", head=head)
        names = [u["name"] for u in units]
        if len(names) != len(set(names)):
            failures.append(f"plan[{label}]: a unit is named twice: {names}")
        covered: set[str] = set()
        for unit in units:
            closure = {unit["name"]} | transitive_deps(unit["name"])
            covered |= closure
            if unit["name"] not in selection:
                failures.append(
                    f"plan[{label}]: unit {unit['name']} is not in the "
                    f"selection {sorted(selection)}")
            reads_head = any("HEAD" in GATES[s]["commits"] for s in closure)
            want = (["bash", "scripts/gate", "--rust", "1.0.0"]
                    + (["--at", head] if reads_head else []) + [unit["name"]])
            if unit["argv"] != want:
                failures.append(
                    f"plan[{label}]: unit {unit['name']} runs {unit['argv']}, "
                    f"not the runner on its own slug at the planned release "
                    f"(and at HEAD's id when it reads HEAD): {want}")
            peak = max(GATES[s]["peak_gb"] for s in closure)
            if unit.get("mem_gb") != float(peak):
                failures.append(
                    f"plan[{label}]: unit {unit['name']} asks for "
                    f"{unit.get('mem_gb')} GiB, and the largest peak of what "
                    f"it runs is {peak}")
            if unit["exclusive"] != [UNIT_EXCLUSIVE]:
                failures.append(
                    f"plan[{label}]: unit {unit['name']} may share a checkout "
                    f"with another unit ({unit['exclusive']})")
            others = set(names) - {unit["name"]}
            if others & transitive_deps(unit["name"]):
                failures.append(
                    f"plan[{label}]: unit {unit['name']} runs "
                    f"{sorted(others & transitive_deps(unit['name']))}, which "
                    f"are units of their own as well")
        missing = set(selection) - covered
        if missing:
            failures.append(
                f"plan[{label}]: selected and run by no unit: {sorted(missing)}")
        if selection and not units:
            failures.append(f"plan[{label}]: a selection of "
                            f"{len(selection)} gate(s) planned no unit")

    # a-kept-verdict-needs-a-release-a-clean-tree-and-a-kept-closure. Driven
    # from a fixture so that it keeps measuring whatever the live table
    # says: `child` keeps its verdict and runs `parent`, which does not.
    cases += 1
    fixture = {
        "parent": {"cost_s": 2, "cache": False, "inputs": None,
                   "no_cache_reason": "x" * 40, "commits": [], "peak_gb": 5},
        "child": {"cost_s": 3, "deps": ["parent"], "cache": True,
                  "inputs": None, "commits": [], "peak_gb": 1},
        "alone": {"cost_s": 1, "cache": True, "inputs": ["docs/**"],
                  "commits": [], "peak_gb": 1},
        "at-head": {"cost_s": 1, "cache": True, "inputs": None,
                    "commits": ["HEAD"], "peak_gb": 1},
    }
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
            or by_name["alone"].get("inputs") != ["docs/**"]:
        failures.append(
            f"plan-fixture: `alone` keeps its verdict over its own inputs and "
            f"was planned as {by_name['alone']}")
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
    if with_head[0]["argv"] != ["bash", "scripts/gate", "--rust", "1.0.0",
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
    for rust, clean, why in ((None, True, "no Rust release was named"),
                             ("1.0.0", False, "the tree differs from HEAD")):
        cases += 1
        if any(u["cache"] for u in plan_units({"alone"}, rust=rust,
                                              tree_is_head=clean,
                                              table=fixture)):
            failures.append(
                f"plan-fixture: a unit kept its verdict when {why}")
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

    if failures:
        for f in failures:
            print(f"SELF-TEST FAIL: {f}", file=sys.stderr)
        return 1
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
    if args.all:
        selected, reason = set(GATES), "every gate (--all)"
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
        keys, reason = select(repo_root, changed)
        selected = set(keys)

    tree_is_head = tracked_tree_is_head(repo_root)
    head = head_commit(repo_root)
    units = plan_units(selected, rust=args.rust, tree_is_head=tree_is_head,
                       head=head)
    print(f"  selection: {reason}", file=sys.stderr)
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
    ap.add_argument("--all", action="store_true",
                    help="with --plan: every gate, ci_only included")
    ap.add_argument("--named", nargs="+", metavar="SLUG",
                    help="with --plan: these gates and what they depend on")
    ap.add_argument("--rust", metavar="RELEASE",
                    help="with --plan: the Rust release every unit runs, as "
                         "`rustc --version` prints it (1.91.0); without it no "
                         "unit's verdict is kept")
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

    if args.plan:
        return print_plan(args, repo_root)
    if args.all or args.named or args.rust is not None:
        ap.error("--all, --named and --rust belong to --plan")
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
