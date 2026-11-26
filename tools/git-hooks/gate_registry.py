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
import contextlib
import importlib.util
import io
import json
import pathlib
import re
import subprocess
import sys
import types
from pathlib import Path


def _gate_table() -> types.ModuleType:
    """`gate_table.py`: the entries' reader, and the derivations a gate's run
    shares with this file ("One file per gate" there).

    Compiled from its source and kept under the name it gives itself, the
    three steps its `sibling` takes for every other module, which cannot
    load the module that defines it. Not imported, for the reason `sibling`
    gives: an import reads and writes `__pycache__/`.
    """
    if "gate_table" not in sys.modules:
        path = Path(__file__).resolve().with_name("gate_table.py")
        module = types.ModuleType("gate_table")
        module.__file__ = str(path)
        sys.modules["gate_table"] = module
        exec(compile(path.read_text(encoding="utf-8"), str(path), "exec"),
             module.__dict__)
    return sys.modules["gate_table"]


gate_table = _gate_table()
trigger_globs = gate_table.sibling("trigger_globs")

from gate_table import (  # noqa: E402  (loaded by path, above)
    INPUTS_ARE_TRIGGERS,
    glob_to_regex,
    submodule_paths,
    submodules_reached,
)
from trigger_globs import (  # noqa: E402
    BUILT_INPUTS,
    GENERATOR_WITNESS,
    MAIN_TREE_CONFIGURE,
    built_inputs,
    built_reads,
    cargo_build_targets,
    cargo_path_deps,
    cargo_workspace_manifests,
    enrolled_globs,
    generator_witness_globs,
    tracked_paths,
    workflow_paths,
)


# ── Gate table ────────────────────────────────────────────────────
#
# `workflows` names the CI workflow(s) a gate mirrors; its `paths:` filter
# is the trigger. `extra` adds locally-known triggers for what the
# workflow's filter cannot express, and `local` is for gates CI has no
# counterpart for. `deps` names gates whose output this one executes.
# `builds` names what the gate builds that other gates build alike (the main
# CMake tree, the generator); what each of those reads is derived once, in
# `BUILT_INPUTS`, and joins the trigger of every gate that names it (see
# "What a build reads" in trigger_globs.py).
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
# takes under five minutes. It bounds every push, an update to main
# included: the landing queue's judgment selects what any push selects, and
# the gates that do not fit are `ci_only`, which hosted CI audits after they
# land (see "What `ci_only` means" above GATES).
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
#
# 33 -> 35 on 2026-11-26, the same direction: `clang-format` and
# `generated-diff` arrived as the gate forms of two lanes that ran their
# scripts inline, so that the lanes could run them in the host image
# (docs/adr/0145). Each entry derives its estimate; its first run reads it.
# 35 -> 36 on 2026-11-27, the same direction: `mutation-swift` arrived when the
# Swift rounds left `mutation-rounds` for an image of their own (ADR 0145), and
# declares that gate's figure until a run of its own is timed.
UNMEASURED_COST_CEILING = 36

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

# `parallelism` and `inputs` take their marks from `gate_table.py`, where
# their reasons are written: `PARALLELISM_COMPILE`, `PARALLELISM_SCRIPT` and
# `INPUTS_ARE_TRIGGERS` are the names an entry may use for a value.

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

# ── What `ci_only` means: audited after landing, never judged before ──
#
# `ci_only` says that no push runs the gate, an update to main included: it
# does not fit `PUSH_BUDGET_S`, its cost has no bound in the tree, or it needs
# a platform the fleet lacks, and hosted CI runs it after the change lands.
# Every push selects one set, `drop_ci_only` of what its change reaches
# (`select`), so the hook, the runner and the plan cannot select differently.
# A push that updates main, which only the landing queue's integrator can make
# (bx RFC 0024), differs from a push of another ref in how its units run, not
# in which gates it selects: each unit carries `--landing`, so a gate that
# would skip for want of a tool ends 3, could not run, instead of passing
# (`kept_only_from_a_landing`, "Whether a gate's verdict may be kept").
#
# The owner's decision of 2026-11-25, which replaced ADR 0120's D1: "before
# landing, only fast, deterministic tests that finish in a few minutes; no
# automatic revert; a red found after landing is repaid by a follow-up
# commit". From 2026-11-08 an update to main had run every `ci_only` gate it
# reached as well (item 395), after cdba8e0eb0 landed on 7 gates while 34
# `ci_only` gates it reached ran only after it and main went red. A judgment
# of every reached gate then took about 1.5 h on the fleet, and the queue
# held every item behind it for that long. The trade is now the one a push of
# any ref always made: a red of a `ci_only` gate arrives after landing, on
# hosted CI's run of main, which the next round reads first and repays with a
# commit of its own. `--ci-owed` names, for the change at hand, the gates
# that audit is to judge, and the workflow that owns each.
#
# ── Where a gate can run: `platform` ─────────────────────────────────
#
# A gate runs on the platform bx.toml declares (`[platform.<name>]`) unless it
# names another in `platform`, with `platform_reason` saying what that runner
# has that the fleet does not. Such a gate is `ci_only`, so no push selects it,
# and hosted CI audits it after landing like every other `ci_only` gate. The
# attribute decides one selection still: a full run on the fleet (`--all
# --landing`, which `PREPUSH_FULL=1` asks for on an update to main) leaves out
# a gate whose `platform` bx.toml does not declare, and names it
# (`drop_hosted_only`). A unit bx cannot place on any machine ends "not
# measured" (exit 2), and the integrator sends an item judged that way back
# unlanded, so a full run that kept such a gate could never land. One
# attribute, read against bx.toml: when the fleet gains that platform, the
# gate joins a full fleet run without an edit here. A toolchain the fleet
# merely lacks is not a platform: it is a tool of bx.toml's platform, with an
# install, so bx provisions it and places the unit where it is (Swift, GenMC,
# the Python environment, emsdk, the mesh SDKs, Chrome for Testing,
# wasm-bindgen), and a compiler a gate's verdict belongs to is an image the
# gate runs in (`container`). One gate names a platform the fleet lacks:
# `windows-msvc`, whose verdict is MSVC's on a hosted Windows runner, and
# which every build machine, being the Linux platform bx.toml declares,
# cannot give. `a-gate-off-the-fleet-is-ci-only-and-says-why` in the
# self-test holds that such a gate is `ci_only`, names a workflow, and gives
# its reason.
BX_PLATFORM_HEADER = re.compile(r"^\[platform\.([A-Za-z0-9_-]+)\]\s*$")

# ── A gate whose cost the change sets: `audited_after_landing` ─────────
#
# Every `ci_only` gate is audited after landing ("What `ci_only` means"). This
# attribute marks the ones whose cost is the change's rather than the tree's,
# which even a deliberate full run on the fleet (`--all --landing`) cannot
# bound. The cost of a gate that does one build per mutation case its change
# reaches grows with the change, and nothing bounds it. Measured 2026-11-21 on
# item 346: its change set reaches 15 casefiles and 152 cases, and in its
# second attempt the one unit that ran them held one machine 10367s of the
# judgment's four hours (named by elimination: the account times the units it
# does not name). The unit keeps no verdict, so every attempt begins it again.
# Over the sixteen commits of main before 2026-11-22 the change set reaches 188
# casefiles and 1207 cases. No shard count makes that a run of the fleet's: at
# a minute a case it is a machine-day, and a ctest case is tens of minutes. The
# textbook place for it is an audit of what landed, where hosted CI already
# shards it and defers what is past its own ceiling (`SCE_PUSH_ROUND_JOBS`).
#
# So a full run on the fleet leaves out a gate that declares
# `audited_after_landing`, with the reason as its value, and names it beside
# the gates off the fleet (`drop_hosted_only`, `why_audited`). It is for a
# gate whose cost has no bound in the tree, never for one that is merely slow:
# a slow gate is `ci_only` and stays in a full run.
# `a-gate-audited-after-landing-is-ci-only-and-says-why` in the self-test holds
# that such a gate is `ci_only`, names the workflow that audits it, and gives
# its reason.

# ── Gates that run in an image: `container` ───────────────────────────
#
# A gate names, in `container`, the tracked Containerfile of the image it runs
# in (docs/adr/0145). The runner then never runs it on the machine it is on: it
# makes the image present and runs `scripts/gate <slug>` inside it, with the
# checkout mounted at its own path, so the gate and everything it depends on
# (`deps`, the generator build among them) run there and nowhere else. Hosted
# CI, a build machine and a workstation therefore judge it with the same
# system packages and toolchains, and the file is the one place they are named;
# no workflow installs a package at job time
# (sce-build/tests/no_workflow_installs_a_package_at_job_time.rs). The images
# are shared by environment, not made per gate (D4): a Containerfile's
# directory is its family, `tools/ci-images/host` for the Linux lanes'
# toolchains, `tools/ci-images/swift` for the Swift toolchain, and
# `tools/interface-standard` for the newest GCC that `cpp-interface-standard`
# measures with. `container_placement` says which gates of a run go into an
# image and which run where the runner is; the dependencies only a gate in an
# image needs go in with it.
#
# An image is addressed by its content and pulled (D2). Its tag is the digest
# of its build context, the Containerfile's directory, every file of it by path
# and content (`gate_image_digest` in scripts/gate). The runner takes the
# image this machine holds under that tag, else pulls
# `ghcr.io/newmassrael/sce-gate-<family>:<digest>` (three tries), else builds
# it: only a change that moved the context builds one. A run on main that
# built one pushes it, so the next run pulls (`SCE_GATE_IMAGE_PUSH`, which
# only `.github/actions/gate-image` sets, on a push to main); a tag the
# registry holds is never pushed again. A lane's job that runs such a gate
# uses that action and declares `packages: write`
# (`a-lane-of-a-gate-in-an-image-pulls-it`).
#
# A rebuild of one context yields the same packages (D3): the base image named
# by digest, the package archive read from a dated snapshot, and each
# downloaded toolchain by its release and checksum
# (`a-gate-image-is-pinned`). A release a script or bx.toml pins already is
# written in the Containerfile again, since nothing outside the context reaches
# the build, and `an-image-installs-the-releases-the-tree-pins` holds every
# such copy to its source. `IMAGES_NOT_YET_PINNED` names an image that does
# not yet name its base and archive so, with the reason, and is refused once
# the image does.
#
# The image starts from an environment of its own, so what a lane sets for the
# gate crosses by name: the variables its entry names in `container_env`
# (below), and its part as `--shard`. A variable a lane sets in the job that
# runs the gate that is neither would be read nowhere
# (`a-lane-sets-only-what-its-image-is-handed`), and a name no lane sets hands
# nothing in (`a-gate-in-an-image-is-handed-what-its-lane-sets`).
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
# `a-gate-image-builds-on-the-selected-rust` holds both halves: no line of a
# Containerfile's last stage, the image the gate runs in, installs Rust
# (`IMAGE_INSTALLS_RUST`; an earlier stage may build a dependency with one and
# hand the image what it installed, as the host image's zenoh-c stage does),
# and `gate_in_container` mounts the selected toolchain and passes its release
# in. The compile cache's sccache, the other Rust toolchains the machine
# declares (the pinned nightly among them) and the rev-pinned Mnemosyne build
# are handed in the same way, since their pins are the tree's and the runner
# provisions them.
#
# An image starts from an environment of its own, so a variable a lane sets
# for its gate does not reach the run inside unless the runner hands it in. A
# gate whose script, or a tool it runs, reads such a variable names it in
# `container_env`, and `gate_in_container` hands exactly those in
# (`gate_table.py container-env <slug>`). The part a sharded lane runs is not
# one of them: it goes in as `--shard`, which the runner inside checks.
# `a-gate-in-an-image-is-handed-what-its-lane-sets` holds every name to a
# gate in an image, to a variable's spelling, and to a lane that sets it.
#
# Every gate that runs on Linux names an image (ADR 0145, D1), one of a few
# shared by environment (D4): `tools/ci-images/host/Containerfile` for the C,
# C++, Rust, JVM, Go and Python lanes, `tools/ci-images/swift/Containerfile`
# for Swift, and `tools/interface-standard/Containerfile` for the newest GCC.
# The image is built from the directory that holds its Containerfile and
# addressed by the digest of those files, so that directory is one of the
# gate's triggers and is in the key of every unit that runs the gate
# (`image_context` in trigger_globs.py): a version moved in the image is a
# change the gate judges (D3). A gate in an image names no `packages`: what
# its script runs is the image's, and the machine its unit goes to needs
# docker alone. A pin the gate reads from an install script has a second copy
# in the image that installs the tool, and the two are held to one
# (`a-gate-image-holds-the-pins-its-gate-reads`).
CONTAINER_NEED = "docker"
CONTAINER_ENV_NAME = re.compile(r"^[A-Z][A-Z0-9_]*$")
IMAGE_INSTALLS_RUST = re.compile(
    r"\brustup\b|sh\.rustup\.rs|static\.rust-lang\.org|\bcargo\s+install\b"
    r"|\brustc\b|\bcargo\b")
CONTAINER_RUNNER_MARKS = ('--rust "$release"', '.local/rust/$release:ro')

# The action a lane's job runs before `scripts/gate` runs a gate in an image,
# and the permission a push to main needs of the job's token.
GATE_IMAGE_ACTION = "./.github/actions/gate-image"
GATE_IMAGE_PERMISSION = "packages: write"

# What `a-gate-image-is-pinned` reads: a `FROM` that names its base by digest,
# and the dated snapshot an image that installs packages reads them from.
IMAGE_BASE_BY_DIGEST = re.compile(r"^FROM\s+\S+@sha256:[0-9a-f]{64}(\s+AS\s+\S+)?\s*$", re.I)
IMAGE_INSTALLS_PACKAGES = re.compile(r"\bapt-get\b.*\binstall\b|\bapt\s+install\b")
IMAGE_SNAPSHOT = re.compile(r"^ARG\s+UBUNTU_SNAPSHOT=\d{8}T\d{6}Z\s*$", re.M)
IMAGE_SNAPSHOT_SOURCE = "https://snapshot.ubuntu.com/ubuntu/${UBUNTU_SNAPSHOT}"

# An image that does not yet name its base by digest and its archive by date,
# with why; refused once it does, so the list is no longer than what is true.
IMAGES_NOT_YET_PINNED: dict[str, str] = {
    "tools/interface-standard/Containerfile":
        "cpp-interface-standard's GCC 15 image is built from the tag gcc:15.3.0 "
        "and reads Debian's live archive; its gate keeps no verdict for that "
        "reason (its entry's no_cache_reason) until the file names both",
}


def image_last_stage(code: str) -> list[str]:
    """The lines of a Containerfile's last stage (from its last `FROM`), the
    image a gate runs in; every line when it names no `FROM`."""
    lines = code.splitlines()
    starts = [i for i, line in enumerate(lines) if re.match(r"^\s*FROM\s", line, re.I)]
    return lines[starts[-1]:] if starts else lines


def image_args(text: str) -> dict[str, str]:
    """The `ARG NAME=value` lines of a Containerfile, by name."""
    return dict(re.findall(r"^ARG\s+([A-Z0-9_]+)=(\S+)\s*$", text, re.M))


def image_runs(text: str) -> list[str]:
    """Each `RUN` of a Containerfile as one line, its continuations joined and
    its whole-line comments dropped."""
    runs: list[str] = []
    current: list[str] | None = None
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("#"):
            continue
        if current is None:
            if not re.match(r"^RUN\s", stripped):
                continue
            current = []
        current.append(stripped.rstrip("\\").strip())
        if not stripped.endswith("\\"):
            runs.append(" ".join(current))
            current = None
    if current is not None:
        runs.append(" ".join(current))
    return runs


def image_pin_failures(path: str, text: str) -> list[str]:
    """What `a-gate-image-is-pinned` refuses in the Containerfile `text` at
    `path`: a base not named by digest, packages read from an archive that is
    not a dated snapshot, a download not checked against a checksum in the
    same step, and a repository cloned without a commit checked out."""
    failures: list[str] = []
    code = [line.strip() for line in text.splitlines()
            if line.strip() and not line.lstrip().startswith("#")]
    froms = [line for line in code if re.match(r"^FROM\s", line, re.I)]
    if not froms:
        failures.append(f"{path} names no base image")
    # A stage built from an earlier one (`FROM base`) names its base by that
    # stage, which is pinned where it is made.
    stages = {match.group(1).lower() for match in
              (re.search(r"\sAS\s+(\S+)\s*$", line, re.I) for line in froms) if match}
    for line in froms:
        named = line.split()[1].lower() if len(line.split()) > 1 else ""
        if named in stages:
            continue
        if not IMAGE_BASE_BY_DIGEST.match(line):
            failures.append(f"{path} names its base {line!r} by a tag, which the "
                            f"registry can move; name it by its digest (@sha256:...)")
    runs = image_runs(text)
    if any(IMAGE_INSTALLS_PACKAGES.search(run) for run in runs) and not (
            IMAGE_SNAPSHOT.search(text) and IMAGE_SNAPSHOT_SOURCE in text):
        failures.append(f"{path} installs packages from an archive that is not a "
                        f"dated snapshot (ARG UBUNTU_SNAPSHOT and "
                        f"{IMAGE_SNAPSHOT_SOURCE}), so a rebuild reads whatever "
                        f"the archive holds that day")
    for run in runs:
        if re.search(r"\b(?:curl|wget)\s+-", run) and "sha256sum --check" not in run:
            failures.append(f"{path} downloads in a step that checks no SHA-256: "
                            f"{run[:90]!r}")
        if re.search(r"\bgit\s+clone\b", run) \
                and not re.search(r"\bcheckout\b|\brev-parse\s+HEAD\b", run):
            failures.append(f"{path} clones a repository and neither checks out a "
                            f"commit nor holds the clone to one: {run[:90]!r}")
    return failures


def image_release_failures(repo_root: Path) -> list[str]:
    """What `an-image-installs-the-releases-the-tree-pins` refuses: a release or
    checksum written in an image's Containerfile that is not the one the tree
    pins it to, read from the pin's own file. The releases a gate reads from
    an install script's `X_REV` line are held by
    `a-gate-image-holds-the-pins-its-gate-reads` too; this holds the checksums
    beside them and the releases no gate reads."""
    failures: list[str] = []

    def read(rel: str) -> str:
        try:
            return (repo_root / rel).read_text(encoding="utf-8")
        except OSError:
            failures.append(f"image-releases: {rel} could not be read")
            return ""

    def first(pattern: str, text: str) -> str:
        found = re.search(pattern, text, re.M)
        return found.group(1) if found else ""

    def held(image: str, args: dict[str, str], name: str, want: str, source: str):
        if not want:
            failures.append(f"image-releases: {source} names no value for {name} "
                            f"to hold {image} to")
        elif args.get(name) != want:
            failures.append(f"image-releases: {image} has {name}={args.get(name)!r} "
                            f"and {source} pins {want!r}")

    host = "tools/ci-images/host/Containerfile"
    args = image_args(read(host))
    go = first(r"^go ([0-9]+\.[0-9]+\.[0-9]+)\s*$", read("backends/go/forge-runtime/go.mod"))
    held(host, args, "GO_RELEASE", go, "backends/go/forge-runtime/go.mod")
    held(host, args, "GO_SHA256",
         first(r'"go' + re.escape(go) + r'\.linux-amd64\.tar\.gz ([0-9a-f]{64})"',
               read("scripts/install_go.sh")) if go else "", "scripts/install_go.sh")
    arm_script = read("scripts/install_arm_gnu_toolchain.sh")
    arm = first(r'^ARM_GNU_TOOLCHAIN_REV="([^"]+)"', arm_script)
    held(host, args, "ARM_GNU_TOOLCHAIN_REV", arm, "scripts/install_arm_gnu_toolchain.sh")
    held(host, args, "ARM_GNU_TOOLCHAIN_SHA256",
         first(r'"arm-gnu-toolchain-' + re.escape(arm)
               + r'-x86_64-arm-none-eabi\.tar\.xz ([0-9a-f]{64})"', arm_script) if arm else "",
         "scripts/install_arm_gnu_toolchain.sh")
    held(host, args, "GENMC_REV", first(r'^GENMC_REV="([^"]+)"', read("scripts/install_genmc.sh")),
         "scripts/install_genmc.sh")
    chrome = read("scripts/install_chrome.sh")
    held(host, args, "CHROME_REV", first(r'^CHROME_REV="([^"]+)"', chrome), "scripts/install_chrome.sh")
    held(host, args, "CHROME_ZIP_SHA256", first(r'^CHROME_ZIP_SHA256="([0-9a-f]{64})"', chrome),
         "scripts/install_chrome.sh")
    held(host, args, "WASM_BINDGEN_REV",
         first(r'^WASM_BINDGEN_REV="([^"]+)"', read("scripts/install_wasm_bindgen.sh")),
         "scripts/install_wasm_bindgen.sh")
    bazel = first(r'^SCE_BAZEL_VERSION="([^"]+)"', read("scripts/lib/bazel_pins.sh"))
    held(host, args, "BAZEL_RELEASE", bazel, "scripts/lib/bazel_pins.sh")
    held(host, args, "BAZEL_SHA256",
         first(r'"bazel-' + re.escape(bazel) + r'-linux-x86_64 ([0-9a-f]{64})"',
               read("scripts/install_bazel.sh")) if bazel else "", "scripts/install_bazel.sh")
    mesh = read("scripts/install_mesh_transports.sh")
    for name in ("CYCLONEDDS_TAG", "CYCLONEDDS_CXX_TAG", "VSOMEIP_TAG", "ZENOH_C_TAG",
                 "ZENOH_CPP_TAG", "MESH_TRANSPORTS_REV"):
        held(host, args, name, first(r'^' + name + r'="([^"]+)"', mesh),
             "scripts/install_mesh_transports.sh")

    def canonical(name: str) -> str:
        return re.sub(r"[-_.]+", "-", name).lower()

    def version_key(version: str) -> list[int]:
        return [int(part) for part in re.findall(r"[0-9]+", version)]

    pinned = {canonical(name): version for name, version in re.findall(
        r"^([A-Za-z0-9_.-]+)==([^\s\\]+)", read("tools/ci-images/host/python-requirements.txt"), re.M)}
    suite = first(r"^SUITE_PACKAGES=\(([^)]*)\)", read("scripts/install_sce_python.sh"))
    wanted = [(canonical(name), floor, "scripts/install_sce_python.sh")
              for name, floor in re.findall(r'"([A-Za-z0-9_.-]+)(?:>=([0-9.]+))?"', suite)]
    runtime = first(r"^dependencies = \[([^\]]*)\]", read("backends/python/runtime/pyproject.toml"))
    wanted += [(canonical(name), floor, "backends/python/runtime/pyproject.toml")
               for name, floor in re.findall(r'"([A-Za-z0-9_.-]+)(?:>=([0-9.]+))?"', runtime)]
    if len(wanted) < 4:
        failures.append(f"image-releases: read only {len(wanted)} Python requirement(s) from "
                        f"scripts/install_sce_python.sh and pyproject.toml; the reader is broken")
    for name, floor, source in wanted:
        if name not in pinned:
            failures.append(f"image-releases: {source} installs {name}, and the host "
                            f"image's python-requirements.txt does not")
        elif floor and version_key(pinned[name]) < version_key(floor):
            failures.append(f"image-releases: the host image pins {name}=={pinned[name]}, "
                            f"under the floor {floor} {source} names")

    swift = "tools/ci-images/swift/Containerfile"
    swift_args = image_args(read(swift))
    bx = read("bx.toml")
    table = re.search(r"^\[platform\.linux\.tools\.swift\]\n(.*?)(?=^\[|\Z)", bx, re.M | re.S)
    held(swift, swift_args, "SWIFT_RELEASE",
         first(r'^version = "([^"]+)"', table.group(1)) if table else "",
         "bx.toml [platform.linux.tools.swift]")
    held(swift, swift_args, "SWIFT_SHA256", first(r"ubuntu24\.04\) sum=([0-9a-f]{64})", bx),
         "bx.toml [platform.linux.tools.swift] (ubuntu24.04)")
    return failures


# A variable a lane sets in a job of its own, read by the job's other steps
# and never by the gate: the runner reads them outside the image.
LANE_ENV_OUTSIDE = ("SHARDS_RESULT",)
# What the runner hands every image whatever `container_env` says (RUNNER_ENV
# in scripts/gate): the switch of the runner itself, which a lane may set.
LANE_ENV_RUNNER = ("SCE_GATE_NO_FAIL_FAST",)
LANE_ENV_KEY = re.compile(r"^\s+(?:-\s+)?(?:run:\s+)?([A-Z][A-Z0-9_]*)[:=]")


def image_lane_failures(repo_root: Path, table=None) -> list[str]:
    """What `a-lane-sets-only-what-its-image-is-handed` and
    `a-lane-of-a-gate-in-an-image-pulls-it` refuse, for every gate of `table`
    that runs in an image and every job of its workflows that runs it: a
    variable of this repository's (`SCE_`, `SPDLOG_`) the job sets, as an
    `env:` key or a `NAME=value` before a command, that the run inside is not
    handed (the gate's `container_env` or `shard_env`), a job that does not
    use the gate-image action, and one that does not declare the permission a
    push of its image needs."""
    table = GATES if table is None else table
    failures: list[str] = []
    jobs_read = 0
    for slug, spec in sorted(table.items()):
        if not spec.get("container"):
            continue
        handed = set(spec.get("container_env") or []) | set(LANE_ENV_RUNNER)
        if spec.get("shard_env"):
            handed.add(spec["shard_env"])
        for workflow in spec.get("workflows", []):
            path = repo_root / ".github" / "workflows" / workflow
            if not path.is_file():
                continue
            lines = path.read_text(encoding="utf-8").splitlines()
            for job, start, end in workflow_jobs(lines):
                code = [line for line in lines[start:end]
                        if not line.lstrip().startswith("#")]
                if not gate_table.runs_slug("\n".join(code), slug):
                    continue
                jobs_read += 1
                where = f"{workflow} job `{job}`"
                for line in code:
                    key = LANE_ENV_KEY.match(line)
                    if not key or key.group(1) in LANE_ENV_OUTSIDE \
                            or not re.match(r"^(SCE_|SPDLOG_)", key.group(1)):
                        continue
                    name = key.group(1)
                    if name not in handed:
                        failures.append(f"container_env: {where} sets {name} for "
                                        f"{slug}, which runs in an image and is not "
                                        f"handed it (container_env of its entry)")
                if not any(re.match(r"^\s*(?:-\s+)?uses:\s*" + re.escape(GATE_IMAGE_ACTION)
                                    + r"\s*$", line) for line in code):
                    failures.append(f"gate-image: {where} runs {slug} in its image and "
                                    f"does not use {GATE_IMAGE_ACTION}, so its run "
                                    f"neither logs in to pull the image nor pushes one "
                                    f"it built")
                if not any(line.strip() == GATE_IMAGE_PERMISSION for line in code):
                    failures.append(f"gate-image: {where} runs {slug} in its image and "
                                    f"does not declare {GATE_IMAGE_PERMISSION!r}, which "
                                    f"a run on main needs to push the image it built")
    if jobs_read == 0:
        failures.append("container_env: no job of any workflow runs a gate in an image, so "
                        "the case read nothing")
    return failures

# Every gate's entry is a file of its own, `tools/git-hooks/gates/<slug>.py`,
# read by `gate_table.load_table`: the fields above, with the reasons for
# their values beside them. A gate's run reads its own entry and those of the
# gates it depends on, and not this file ("One file per gate" in
# gate_table.py).
GATES: dict[str, dict] = gate_table.load_table()

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
    # It runs at commit time instead of push time, which is earlier than a
    # gate would catch it: a formatting fix-up never becomes its own commit.
    # The hook's stage comments named `rust-ci.yml` for years — a workflow
    # this repository does not have — which is exactly the drift
    # `rustdoc-links` had and the reason these claims are now checked.
    # `clang-format-check.yml` stood beside it until its check needed the
    # host image's clang-format-19, which a lane reaches through a gate
    # (`clang-format`, docs/adr/0145); the commit hook still runs it on the
    # staged files.
    "fmt-check.yml": "commit-hook: cargo fmt --all -- --check",
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
    # ⚠ What leaving them unclassified costs: rule 1 hands back every gate a
    # push can run. While a landing also ran the `ci_only` gates (2026-11-08
    # to 2026-11-25) every one of them bought the whole table: the migration
    # guide alone was edited by 198 commits of main in the month before
    # 2026-11-22, and each landing of one would have held the queue for all of
    # it. What is under `docs/formal/` is not here: `formal-models` and
    # `formal-proofs` read it.
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


SOURCED_GATE_SCRIPT = re.compile(
    r'^\s*source\s+"\$\(dirname\s+"\$\{BASH_SOURCE\[0\]\}"\)/([A-Za-z0-9_-]+)\.sh"')


def gate_script_code(path: Path) -> str:
    """A gate script's code (`script_code`), followed by the code of any other
    gate's script it sources beside it: `mutation-swift` is `mutation-rounds`'s
    script run over the Swift share of the corpus, so what that script reads,
    this gate reads. `lib.sh`, which every gate script sources, is a library
    and not followed."""
    code = script_code(path)
    parts = [code]
    for line in code.splitlines():
        found = SOURCED_GATE_SCRIPT.match(line)
        if found and found.group(1) != "lib":
            sourced = path.parent / f"{found.group(1)}.sh"
            if sourced.is_file() and sourced != path:
                parts.append(script_code(sourced))
    return "\n".join(parts)


def gate_triggers(repo_root: Path, slugs=None) -> dict[str, list[str]]:
    """Each gate's trigger globs, derived from its workflow and its own lists
    (`triggers_of` in trigger_globs.py, which says how); those of `slugs`
    alone when it is given.

    A gate's globs are derived from its own entry and nothing else, so asking
    for one gate reads its workflows, its builds and its ledgers and no other
    gate's. The submodule preflight derives the gate it runs alone
    (`submodule_needs` in gate_table.py), because a narrowed unit's copy holds
    that gate's inputs and no other's: deriving every gate's there read every
    workflow and ledger config of the tree, and each read outside the inputs
    is one bx calls "could not measure" (`bx --checks prepush --learn
    forge-c11`, 2026-11-24).
    """
    return {slug: trigger_globs.triggers_of(repo_root, spec)
            for slug, spec in GATES.items()
            if slugs is None or slug in slugs}


def gate_reads(repo_root: Path, slugs=None) -> dict[str, list[str]]:
    """What each gate's run reads that does not select it (`reads_of` in
    trigger_globs.py); those of `slugs` alone when it is given. A gate keyed
    on its triggers is keyed on these too (`gate_inputs`); see "What a build
    reads that does not select it" in trigger_globs.py."""
    return {slug: trigger_globs.reads_of(repo_root, spec)
            for slug, spec in GATES.items()
            if slugs is None or slug in slugs}


def run_order(slugs, table=None) -> list[str]:
    """Dependencies first, then cheapest first (`run_order` in
    gate_table.py), over `table`, every gate's entry unless given."""
    return gate_table.run_order(slugs, GATES if table is None else table)


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
    return gate_table.transitive_deps(slug, GATES if table is None else table)


def matched(repo_root: Path, changed: list[str]) -> tuple[set[str], str]:
    """Every gate the change set reaches, BEFORE `ci_only` is applied.

    Split out of `select` so the two readers of this answer share one
    implementation. `select` keeps what the push runs (`drop_ci_only`);
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
    """Return (gates to run in order, reason). Empty list means nothing to do.

    The one selection every push makes, an update to main included (see
    "What `ci_only` means"): every gate the change reaches, less the
    `ci_only` ones (`drop_ci_only`). What a landing changes is how its units
    run (`plan_units`, `--landing`), never which gates it selects, so the
    hook, the runner and the plan cannot select differently.
    """
    reached, reason = matched(repo_root, changed)
    return (run_order(drop_ci_only(reached)), reason)


def fleet_platforms(repo_root: Path) -> set[str]:
    """The platforms bx.toml declares (`[platform.<name>]`), which bx places on.

    Read by its table headers rather than parsed as TOML: the registry runs
    on the Python the build machines carry, and Ubuntu 22.04's 3.10 has no
    `tomllib`. A tool's table (`[platform.linux.tools.swift]`) has a dot in
    what follows `platform.` and is not a platform. A bx.toml that cannot be
    read declares nothing, so every gate that names a platform is left out
    of a full fleet run rather than handed to a fleet that may not run it.
    """
    try:
        text = (repo_root / "bx.toml").read_text(encoding="utf-8")
    except OSError:
        return set()
    return {m.group(1) for m in map(BX_PLATFORM_HEADER.match, text.splitlines())
            if m}


def drop_hosted_only(selected, platforms, table=None) -> set[str]:
    """Remove the gates a full run on the fleet (`--all --landing`) leaves to
    hosted CI.

    Two kinds: a gate whose `platform` is not one the fleet has, and one whose
    cost the change sets (`audited_after_landing`). A gate that names no
    platform runs on bx.toml's own and stays. Its dependencies stay as well, as
    `drop_ci_only` keeps them: they are gates the run selected, whichever
    gate pulled them in. A selection by change never comes here: `select`
    has already dropped every `ci_only` gate, and both kinds are `ci_only`.
    """
    table = GATES if table is None else table
    return {slug for slug in selected
            if (table[slug].get("platform") is None
                or table[slug]["platform"] in platforms)
            and not table[slug].get("audited_after_landing")}


def why_audited(slug: str, platforms, table=None) -> str:
    """Why a landing leaves `slug` to hosted CI's audit, for the line naming it.

    A gate whose cost has no bound is named by that cost, a gate off the
    fleet by its platform (the platform would be the reason only if the
    fleet could bear the cost), and any other `ci_only` gate by the
    workflows that audit it after landing.
    """
    table = GATES if table is None else table
    spec = table[slug]
    audited = spec.get("audited_after_landing")
    if audited:
        return f"is audited after landing ({audited})"
    platform = spec.get("platform")
    if platform is not None and platform not in platforms:
        return (f"needs platform {platform!r}, which bx.toml does not declare "
                f"({spec.get('platform_reason', '')})")
    workflows = ", ".join(spec.get("workflows") or ["no workflow"])
    return f"is ci_only, audited after landing by {workflows}"


def name_audited(repo_root: Path, slugs) -> None:
    """Say on stderr, a line a gate, which gates a landing leaves to hosted CI.

    The judgment's own account of what it did not measure, so the log of a
    landing names the verdicts the next round reads off hosted CI's run of
    main. A landing by change names `ci_owed`, the reached `ci_only` gates
    whose workflow starts for it; the ones none starts for are the hook's
    `--ci-unowed` notice, since no audit after landing judges them either.
    """
    platforms = fleet_platforms(repo_root)
    for slug in run_order(set(slugs)):
        print(f"  NOT JUDGED HERE, hosted CI audits it after landing: {slug} "
              f"{why_audited(slug, platforms)}", file=sys.stderr)


def container_placement(order, table=None) -> list[tuple[str, str, str]]:
    """Where each gate of a run in `order` runs (`container_placement` in
    gate_table.py, which the runner asks)."""
    return gate_table.container_placement(order, GATES if table is None else table)


def hosted_only(repo_root: Path, selected) -> list[str]:
    """The gates of `selected` a full fleet run leaves to hosted CI, in order."""
    return run_order(set(selected)
                     - drop_hosted_only(selected, fleet_platforms(repo_root)))


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

    "Only CI" is whatever the push left out, every reached `ci_only` gate,
    and it is the same for every push: on an update to main it is the list
    of what hosted CI audits after landing. Derived from the selection
    rather than from the field, so the notice cannot name a gate the push ran.
    """
    return run_order({s for s in left_out(repo_root, changed)
                      if any(workflow_starts_for(repo_root, wf, changed)
                             for wf in GATES[s].get("workflows", []))})


def ci_unowed(repo_root: Path, changed: list[str]) -> list[str]:
    """Gates this change reaches that the push left out and NO workflow starts for.

    The residue of `ci_owed`, and the reason a deny-list cannot remove
    coverage in silence. A gate here is run by neither the push nor CI, so
    the change is going out unjudged by it. That is sometimes exactly right
    — a README cannot break a C++ ctest suite — and the point is that it is
    said out loud rather than discovered when a lane has been quiet for
    eight pushes.
    """
    return run_order({s for s in left_out(repo_root, changed)
                      if not any(workflow_starts_for(repo_root, wf, changed)
                                 for wf in GATES[s].get("workflows", []))})


def left_out(repo_root: Path, changed: list[str]) -> set[str]:
    """The gates this change reaches that no push runs: its `ci_only` ones."""
    reached, _ = matched(repo_root, changed)
    return set(reached) - drop_ci_only(reached)


def drop_ci_only(selected) -> set[str]:
    """Remove the gates no push runs, keeping their deps.

    Every push comes here, an update to main included: the landing queue's
    judgment is the set that fits `PUSH_BUDGET_S`, by the owner's decision of
    2026-11-25 ("What `ci_only` means" above GATES), and hosted CI audits
    the rest after landing.

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

    The trade recorded here is the owner's: a red arrives one round later
    (for an update to main, after landing, repaid by a commit of its own),
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
# gate table's reader, and `.cargo/`, whose configuration every cargo run
# reads. Read again on 2026-11-24 with bx 340e9f9e, which traces listings:
# seven gates agreed that every unit also lists the root (`./`) and
# `tools/git-hooks/`. bx's trace refuses a unit that reads a file or lists a
# directory outside its inputs, so a runner that starts reading another fails
# the unit ("could not measure"), never passes it.
#
# The runner read this file then, and lists `tools/git-hooks/` and the root
# because it ran it with its directory, or the working one, first on
# `sys.path`. It reads `gate_table.py` instead since 2026-11-25, as
# `python3 -I`, which lists neither, and `unit_inputs` adds the entries of
# the gates a unit runs ("One file per gate" in gate_table.py);
# `a-gate-run-reads-its-own-entries` holds that reading. The root stays: what
# else in the runner lists it was never traced apart from Python.
RUNNER_INPUTS = (
    "./",
    ".gitmodules",
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
    # What the gate library sources for every gate: the mapping that keeps
    # Cargo's home out of what a Rust build writes.
    "scripts/lib/reproducible_paths.sh",
    # What the gate library runs around every suite a gate runs, and when one
    # fails, to name the smallest command that reproduces it
    # (`sce_gate_tested`), and the reading of a ctest tree it asks then.
    "scripts/lib/repro.py",
    "scripts/lib/ctest_shards.py",
    "scripts/install_mesh_transports.sh",
    "tools/git-hooks/gate_table.py",
    "rust-toolchain.toml",
    "backends/go/forge-runtime/go.mod",
    ".cargo/**",
)

# The trigger derivation as a unit's inputs name it: read by a run only for a
# gate keyed on its triggers (`submodule_needs` in gate_table.py).
TRIGGER_GLOBS_INPUT = "tools/git-hooks/trigger_globs.py"


def gate_inputs(slug, table=None, triggers=None, reads=None):
    """What `slug`'s OWN script reads, as its key names it (`gate_inputs` in
    gate_table.py)."""
    return gate_table.gate_inputs(slug, GATES if table is None else table,
                                  triggers, reads)


def unit_inputs(closure, table=None, triggers=None, reads=None):
    """The input patterns of a unit that runs every gate in `closure`.

    `None`, the whole tracked tree, as soon as one of them reads the whole
    tree: a key narrower than what the unit reads would let a pass outlive
    a change to the part left out. Otherwise the union of their patterns
    (`gate_inputs`, with `triggers` and `reads` for a gate keyed on its
    triggers), of `RUNNER_INPUTS`, and of each gate's script, entry and
    mirrored workflows, sorted so that one closure always yields one key. A
    gate keyed on its triggers adds `trigger_globs.py`, which the runner's
    submodule preflight reads to learn what those triggers reach. A gate in an
    image adds the image's build context, which the runner reads to address
    and build the image the gate runs in.
    """
    table = GATES if table is None else table
    patterns: set[str] = set(RUNNER_INPUTS)
    for slug in closure:
        own = gate_inputs(slug, table, triggers, reads)
        if own is None:
            return None
        patterns.update(own)
        patterns.add(f"scripts/gates/{slug}.sh")
        patterns.add(gate_table.entry_input(slug))
        if table[slug].get("inputs") == INPUTS_ARE_TRIGGERS:
            patterns.add(TRIGGER_GLOBS_INPUT)
        if table[slug].get("container"):
            patterns.add(trigger_globs.image_context(table[slug]["container"]))
        patterns.update(f".github/workflows/{wf}"
                        for wf in table[slug].get("workflows", []))
    return sorted(patterns)


# The tracer `a-gate-run-reads-its-own-entries` runs a question of the runner
# under: `gate_table.py` as `python3 -I` starts it (no directory first on
# `sys.path`; `runpy` adds none for a file), with Python's audit hook
# writing every file the run opened and every directory it listed, one a line,
# to the file its first argument names.
RUN_TRACER = r"""
import os, runpy, sys
log, target = sys.argv[1], sys.argv[2]
sys.argv = [target] + sys.argv[3:]
seen, live = [], [True]
def hook(event, args):
    if not live[0]:
        return
    if event == "open" and isinstance(args[0], (str, bytes)):
        seen.append("open " + os.path.abspath(os.fsdecode(args[0])))
    elif event in ("os.listdir", "os.scandir"):
        path = args[0] if args and args[0] is not None else "."
        if isinstance(path, (str, bytes)):
            seen.append("list " + os.path.abspath(os.fsdecode(path)))
sys.addaudithook(hook)
code = 0
try:
    runpy.run_path(target, run_name="__main__")
except SystemExit as exc:
    code = exc.code if isinstance(exc.code, int) else 1
live[0] = False
with open(log, "w", encoding="utf-8") as out:
    out.write("\n".join(seen))
sys.exit(code)
"""


def traced_run_reads(question: list[str]) -> tuple[int, list[tuple[str, str]]]:
    """The status of `gate_table.py <question>` and what its run read of this
    checkout: `(kind, path)`, `kind` "open" or "list", the path relative to
    the checkout `gate_table.py` belongs to."""
    import tempfile

    root = gate_table.HERE.parent.parent
    with tempfile.TemporaryDirectory() as scratch:
        log = Path(scratch) / "reads"
        done = subprocess.run(
            [sys.executable, "-I", "-c", RUN_TRACER, str(log),
             str(gate_table.HERE / "gate_table.py"), *question],
            cwd=root, capture_output=True, text=True, check=False)
        lines = log.read_text(encoding="utf-8").splitlines() \
            if log.is_file() else []
    reads = []
    for line in lines:
        kind, _, path = line.partition(" ")
        full = Path(path)
        if full == root or root in full.parents:
            reads.append((kind, full.relative_to(root).as_posix() if full != root
                          else "."))
    return done.returncode, reads


def unit_read_failures(repo_root: Path, table=None) -> list[str]:
    """What `a-gate-run-reads-its-own-entries` refuses, for the gates of
    `table` it picks by what they declare: one alone, one with a dependency,
    one in an image, one in shards, the first whose unit is narrowed, and the
    first keyed on its triggers that needs a submodule, whose preflight reads
    `trigger_globs.py` to derive its reach.

    Each question the runner asks of a unit's gates (`order`, `placement`,
    `submodules`, `shard-env` for a gate in shards) is run under
    `RUN_TRACER`, and its reads are held to three things: of
    `tools/git-hooks/` it opens `gate_table.py` and the entries of the gates
    the unit runs (and `trigger_globs.py` when one is keyed on its
    triggers), it lists no directory of the checkout, and every file it
    opens is among the inputs the plan gives the unit (`unit_inputs`) when
    those are narrowed. A run that opened neither `gate_table.py` nor its
    gate's entry traced nothing, and is refused as a reading that did not
    happen.
    """
    table = GATES if table is None else table
    triggers = gate_triggers(repo_root)
    reads = gate_reads(repo_root)

    def first(wanted):
        return next((s for s in sorted(table) if wanted(s, table[s])), None)

    picks = {
        "a gate alone": first(lambda s, e: not e.get("deps")
                              and not e.get("container")),
        "a gate with a dependency": first(lambda s, e: bool(e.get("deps"))
                                          and not e.get("container")),
        "a gate in an image": first(lambda s, e: bool(e.get("container"))),
        "a gate in shards": first(lambda s, e: bool(e.get("shard_env"))),
        "a narrowed gate": first(lambda s, e: unit_inputs(
            {s} | transitive_deps(s, table), table, triggers, reads) is not None),
        "a gate keyed on its triggers that needs a submodule": first(
            lambda s, e: e.get("inputs") == INPUTS_ARE_TRIGGERS and any(
                gate_table.submodule_needs(repo_root, [s], table, which=True))),
    }
    failures = [f"no {what} in the table to trace" for what in
                ("a gate alone", "a gate with a dependency")
                if picks[what] is None]
    for what, slug in picks.items():
        if slug is None:
            continue
        closure = {slug} | transitive_deps(slug, table)
        order = run_order(closure, table)
        allowed = {"tools/git-hooks/gate_table.py"} | {
            gate_table.entry_input(s) for s in closure}
        if any(table[s].get("inputs") == INPUTS_ARE_TRIGGERS for s in closure):
            allowed.add(TRIGGER_GLOBS_INPUT)
        inputs = unit_inputs(closure, table, triggers, reads)
        files = [glob_to_regex(p) for p in inputs or [] if not p.endswith("/")]
        questions = [["order", slug], ["placement", *order],
                     ["submodules", *order]]
        if table[slug].get("shard_env"):
            questions.append(["shard-env", slug])
        for question in questions:
            asked = f"{what} ({slug}): gate_table.py {' '.join(question)}"
            status, seen = traced_run_reads(question)
            if status != 0:
                failures.append(f"{asked} ended {status}")
                continue
            opened = {path for kind, path in seen if kind == "open"}
            for path in sorted(path for kind, path in seen if kind == "list"):
                failures.append(f"{asked} listed {path}/, which no unit's "
                                f"inputs give and no entry read needs")
            for path in sorted(p for p in opened if p.startswith("tools/git-hooks/")
                               and p not in allowed):
                failures.append(f"{asked} opened {path}, which the verdict of "
                                f"{sorted(closure)} does not rest on")
            if inputs is not None:
                for path in sorted(p for p in opened
                                   if not any(r.match(p) for r in files)):
                    failures.append(f"{asked} opened {path}, outside the "
                                    f"inputs its unit is planned with")
            if not {"tools/git-hooks/gate_table.py",
                    gate_table.entry_input(slug)} <= opened:
                failures.append(f"{asked} was traced opening neither "
                                f"gate_table.py nor the gate's entry, so the "
                                f"trace read nothing: {sorted(opened)}")
    return failures

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
                "broke": f"gate {name} FAILED: its log ends with the smallest "
                         f"command that reproduces each failure, and the whole "
                         f"gate is `scripts/gate {' '.join(own[len(argv):] + [slug])}`",
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
# of cargo's registry downloaded by one release is restored into a run of the
# next (Rust build state is kept by no cache: "The compile cache" below): its
# key, and every restore-key prefix, carry the hash of `rust-toolchain.toml`,
# so a move starts the cache afresh instead of dragging the old release's
# index along for good.
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


def step_bounds(lines: list[str], index: int) -> tuple[int, int]:
    """The step of a workflow or action that holds line `index` (0-based):
    from the line that opens it (`- `) to the next line at that indent or
    less, as a 0-based start and end."""
    start = index
    while start > 0 and not re.match(r"^\s*-\s", lines[start]):
        start -= 1
    indent = len(lines[start]) - len(lines[start].lstrip())
    end = start + 1
    while end < len(lines):
        nxt = lines[end]
        if nxt.strip() and len(nxt) - len(nxt.lstrip()) <= indent:
            break
        end += 1
    return start, end


def workflow_steps(lines: list[str], uses: "re.Pattern[str]"):
    """Each step of a workflow or action whose `uses:` line matches `uses`.

    Yields the 1-based number of the `uses:` line, the action it names and
    the step's lines (`step_bounds`).
    """
    for i, line in enumerate(lines):
        match = uses.match(line)
        if not match:
            continue
        start, end = step_bounds(lines, i)
        yield i + 1, match.group(1), lines[start:end]


def workflow_jobs(lines: list[str]) -> list[tuple[str, int, int]]:
    """Each job of a workflow, as its name and the 0-based start and end of
    its lines: the keys two spaces in under the top-level `jobs:`."""
    heads: list[tuple[str, int]] = []
    end = len(lines)
    inside = False
    for i, line in enumerate(lines):
        if not inside:
            inside = bool(re.match(r"^jobs:\s*$", line))
            continue
        if line.strip() and not line[0].isspace() and not line.startswith("#"):
            end = i
            break
        match = re.match(r"^  ([A-Za-z0-9_-]+):\s*$", line)
        if match:
            heads.append((match.group(1), i))
    return [(name, start, heads[k + 1][1] if k + 1 < len(heads) else end)
            for k, (name, start) in enumerate(heads)]


def workflow_texts(repo_root: Path) -> dict[str, list[str]]:
    """Every tracked workflow and action file under .github, by its path, as
    its lines."""
    workflows: dict[str, list[str]] = {}
    for rel in tracked_paths(repo_root, ".github"):
        if Path(rel).suffix not in (".yml", ".yaml"):
            continue
        try:
            workflows[rel] = (repo_root / rel).read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeDecodeError):
            continue
    return workflows


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

    A key of cargo's registry download names what it holds (the toolchain
    and Cargo.lock), never the sources: a key that moves with every commit
    saves a new entry for each. On 2026-11-24 five lanes hashed sce-build's
    sources into the key of their `target`, so each commit that touched them
    saved a new one of 380 to 620 MB, and with the repository's caches past
    their 10 GB GitHub evicted the C++ compiler caches. Rust build state is now
    kept by no key at all (`compile_cache_failures`).
    """
    found = []
    for args in HASHED_FILES.findall(key):
        for glob in re.findall(r"'([^']*)'", args):
            if glob.endswith(".rs") or "/src/" in f"/{glob}":
                found.append(glob)
    return found


CARGO_DOWNLOAD = re.compile(r"(?:^|/)\.?cargo/(?:registry|git)(?:/|$)")
SAVED_KEY = re.compile(r"^\$\{\{\s*steps\.([\w-]+)\.outputs\.cache-primary-key\s*\}\}$")


def rust_cache_failures(repo_root: Path) -> list[str]:
    """A cache of cargo's registry download whose key does not move with the
    release, or moves with the sources.

    Rust build state is kept by no snapshot (`compile_cache_failures`): what a
    lane compiles goes through the shared compile cache, and the one Rust
    cache kept is cargo's registry download, which
    `.github/actions/compile-cache` restores. A cache whose `path` holds
    cargo's registry or git checkouts carries `hashFiles('rust-toolchain.toml')`
    in its `key` and each of its `restore-keys`: a prefix without it hands a
    miss the previous release's index. A save that names its restore's key
    (`steps.<id>.outputs.cache-primary-key`) is read as that key.
    """
    failures: list[str] = []
    found = 0
    for rel, lines in sorted(workflow_texts(repo_root).items()):
        restored: dict[str, list[str]] = {}
        for _number, _action, step in workflow_steps(lines, CACHE_STEP):
            ids = step_values(step, "id")
            if ids:
                restored[ids[0]] = step_values(step, "key")
        for number, _action, step in workflow_steps(lines, CACHE_STEP):
            paths = step_values(step, "path")
            if not any(CARGO_DOWNLOAD.search(p) for p in paths):
                continue
            found += 1
            keys: list[str] = []
            for key in step_values(step, "key"):
                saved = SAVED_KEY.match(key)
                keys.extend(restored.get(saved.group(1), [key]) if saved else [key])
            if not keys:
                failures.append(
                    f"{rel}:{number} caches cargo's registry with no `key` that "
                    f"carries {TOOLCHAIN_CACHE_KEY}, so a release move restores the "
                    f"old release's index")
            for key in keys:
                for source in key_hashed_sources(key):
                    failures.append(
                        f"{rel}:{number} keys cargo's registry by the sources "
                        f"{source!r}: every commit that touches them saves another "
                        f"copy, which evicts the caches beside it; key it by the "
                        f"toolchain and Cargo.lock, which name what it holds")
            for key in keys + step_values(step, "restore-keys"):
                if TOOLCHAIN_CACHE_KEY not in key:
                    failures.append(
                        f"{rel}:{number} caches cargo's registry under {key!r}, "
                        f"which lacks {TOOLCHAIN_CACHE_KEY}: a release move would "
                        f"restore the old release's index into the new one's run")
    # A lower bound, so a step parse that stopped finding caches cannot pass:
    # the compile cache's restore and save, which also serve a gate in an image
    # (`gate_in_container` mounts the download into the image's cargo home).
    if found < 2:
        failures.append(
            f"found {found} cargo registry cache step(s) in .github; a broken step "
            f"parse passes by finding none")
    return failures


# A Gradle cache's `path`: the runner's own home, or the home a gate in an
# image keeps Gradle in on the runner (`~/.cache/sce-gate-containers/<slug>/home`,
# `gate_in_container` in scripts/gate).
GRADLE_CACHE_PATH = re.compile(r"^~/(?:\.cache/sce-gate-containers/[\w.-]+/home/)?\.gradle/caches$")


def is_gradle_cache(path: str) -> bool:
    """Whether a cache step's `path` entry is a Gradle home's caches: the
    runner's (`~/.gradle/caches`), or the home of an image a gate runs in
    (`~/.cache/sce-gate-containers/<slug>/home/.gradle/caches`, ADR 0145),
    which a lane keeps across runs the same way."""
    return GRADLE_CACHE_PATH.match(path) is not None


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
            if not any(is_gradle_cache(p) for p in step_values(step, "path")):
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
    failures, found = shared_gradle_cache_keys(workflow_texts(repo_root))
    # A lower bound, so a step parse that stopped finding caches cannot pass.
    if found < 5:
        failures.append(
            f"found {found} Gradle cache step(s) in .github; a broken step parse "
            f"passes by finding none")
    return failures


KEEP_ONE_CACHE_STEP = re.compile(r"^\s*(?:-\s+)?uses:\s*(\./\.github/actions/keep-one-cache)\s*$")
PER_RUN_KEY = re.compile(r"\$\{\{\s*github\.(?:sha|run_id)\s*\}\}")


def per_run_cache_failures(workflows: dict[str, list[str]]) -> tuple[list[str], int]:
    """Caches saved under a key new on every run, and families of which
    `keep-one-cache` keeps one that begin one another; with how many cache
    steps were read.

    A key that carries the commit or the run saves a new entry on every run.
    It is the shape a snapshot of build state takes (a ccache or sccache
    directory saved after each build), and every run's entry stayed until
    GitHub evicted the least recently used caches of the whole repository to
    stay within its 10 GB; such state now goes through the shared compile
    cache (`compile_cache_failures`), so no key carries the run. A family
    whose key moves with what it holds (vcpkg's packages, the generator's
    plans) is kept to one entry by `./.github/actions/keep-one-cache`, and no
    family it keeps is the beginning of another's, or keeping one of it would
    delete the other lane's.
    """
    failures: list[str] = []
    families: dict[str, str] = {}
    found = 0
    for rel, lines in sorted(workflows.items()):
        for number, _action, step in workflow_steps(lines, CACHE_STEP):
            found += 1
            for key in step_values(step, "key"):
                if PER_RUN_KEY.search(key):
                    failures.append(
                        f"{rel}:{number} saves a cache under {key!r}, new on every "
                        f"run: each run's entry stays until GitHub evicts other "
                        f"lanes' caches for room. Compile output goes through "
                        f"./.github/actions/compile-cache; any other cache is keyed "
                        f"by what it holds")
        for number, _action, step in workflow_steps(lines, KEEP_ONE_CACHE_STEP):
            for family in step_values(step, "prefix"):
                families.setdefault(family, f"{rel}:{number}")
    for family, place in sorted(families.items()):
        for other, other_place in sorted(families.items()):
            if other != family and other.startswith(family):
                failures.append(
                    f"the cache family {family!r} ({place}) begins {other!r} "
                    f"({other_place}): keeping one of the first deletes the second's")
    return failures, found


def per_run_cache_sweep(repo_root: Path) -> list[str]:
    """`per_run_cache_failures` over every workflow in the tree."""
    failures, found = per_run_cache_failures(workflow_texts(repo_root))
    # A bound on the parse, not on the tree: 28 steps were read on 2026-11-26,
    # once the toolchains the lanes cached moved into the gates' images
    # (ADR 0145), and the lanes still migrating remove more of them.
    if found < 10:
        failures.append(
            f"found {found} cache step(s) in .github; a broken step parse passes "
            f"by finding none")
    return failures


# ── The compile cache ─────────────────────────────────────────────
#
# What a lane compiles is kept by its content, not by the lane:
# `.github/actions/compile-cache` installs sccache, pinned by release and
# checksum, on the GitHub Actions cache, and compiles Rust (RUSTC_WRAPPER),
# CMake's C and C++ (CMAKE_<LANG>_COMPILER_LAUNCHER) and emcc
# (EM_COMPILER_WRAPPER) through it: one cache entry per compilation, keyed by
# its inputs, so every lane and shard that compiles a unit alike shares the
# one entry, and the eviction that makes room drops one object, not a lane's
# whole build. The structure it replaced, a snapshot per Rust lane of its
# whole `target` and per C/C++ job of its ccache directory, held 10.14 GB in
# 31 entries on 2026-11-25 against the repository's 10 GB, most of it the
# same dependency crates stored once per lane.
#
# So no cache step holds compile output (a Rust `target`, which
# Swatinem/rust-cache saves too; a compiler cache's directory; a whole CMake
# build tree), no lane installs or names a compiler cache past that action,
# and a job that uses it prints `sccache --show-stats` after it, whatever the
# job's outcome, so its log says what the cache answered and what it could
# not. The sweep reads every tracked workflow and action file; whole-line
# comments are prose and are not read.
COMPILE_CACHE_ACTION = ".github/actions/compile-cache/action.yml"
COMPILE_CACHE_USES = re.compile(r"^\s*(?:-\s+)?uses:\s*\./\.github/actions/compile-cache\s*$")
COMPILER_CACHE_ACTIONS = ("mozilla-actions/sccache-action", "hendrikmuhs/ccache-action")
CCACHE_WORD = re.compile(r"(?<![\w.~/-])ccache(?![\w-])")
COMPILER_CACHE_DIRS = frozenset({"ccache", ".ccache", "sccache", ".sccache"})


def compile_snapshot(path: str) -> str:
    """What compile output a cache step's `path` entry holds, or "" for none."""
    parts = [part for part in re.split(r"[\\/]", path.strip()) if part not in ("", ".", "~")]
    if not parts:
        return ""
    if parts[-1] == "target":
        return "a Rust `target`"
    if COMPILER_CACHE_DIRS.intersection(parts):
        return "a compiler cache's directory"
    if len(parts) == 1 and re.fullmatch(r"build(?:_[\w-]*)?", parts[0]):
        return "a whole CMake build tree"
    return ""


def compile_cache_failures(workflows: dict[str, list[str]]) -> tuple[list[str], int]:
    """Where a workflow or action keeps compile output as a snapshot, compiles
    through a cache past `./.github/actions/compile-cache`, or uses that action
    and never says what it answered; with how many jobs use the action."""
    failures: list[str] = []
    users = 0
    for rel, lines in sorted(workflows.items()):
        for number, action, step in workflow_steps(lines, CACHE_STEP):
            if action == "Swatinem/rust-cache":
                failures.append(
                    f"{rel}:{number} caches with Swatinem/rust-cache, a snapshot of "
                    f"`target` for this lane alone: Rust compiles through "
                    f"./.github/actions/compile-cache, which every lane shares")
                continue
            for path in step_values(step, "path"):
                what = compile_snapshot(path)
                if what:
                    failures.append(
                        f"{rel}:{number} saves {what} ({path!r}), compile output "
                        f"stored again for one lane: compilations go through "
                        f"./.github/actions/compile-cache, one entry per compilation "
                        f"for every lane")
        for number, line in enumerate(lines, 1):
            if line.lstrip().startswith("#"):
                continue
            for action in COMPILER_CACHE_ACTIONS:
                if re.match(r"^\s*(?:-\s+)?uses:\s*" + re.escape(action) + r"@", line):
                    failures.append(
                        f"{rel}:{number} installs a compiler cache with {action}: a "
                        f"lane compiles through ./.github/actions/compile-cache, which "
                        f"pins sccache by release and checksum")
            if CCACHE_WORD.search(line):
                failures.append(
                    f"{rel}:{number} names ccache ({line.strip()[:80]!r}): a lane "
                    f"compiles through ./.github/actions/compile-cache")
        for job, start, end in workflow_jobs(lines):
            opened = next((i for i in range(start, end)
                           if COMPILE_CACHE_USES.match(lines[i])), None)
            if opened is None:
                continue
            users += 1
            reported = False
            for i in range(opened + 1, end):
                if lines[i].lstrip().startswith("#") or "sccache --show-stats" not in lines[i]:
                    continue
                step_start, step_end = step_bounds(lines, i)
                if any(re.match(r"^\s*(?:-\s+)?if:.*\balways\(\)", step_line)
                       for step_line in lines[step_start:step_end]):
                    reported = True
            if not reported:
                failures.append(
                    f"{rel}:{opened + 1} job `{job}` compiles through "
                    f"./.github/actions/compile-cache and prints no "
                    f"`sccache --show-stats` after it in a step that runs "
                    f"`if: always()`: its log would not say what the cache answered")
    return failures, users


def compile_cache_sweep(repo_root: Path) -> list[str]:
    """`compile_cache_failures` over every workflow and action in the tree,
    and the action's own pin: a release, and a checksum checked for every
    archive it downloads."""
    failures, users = compile_cache_failures(workflow_texts(repo_root))
    action = repo_root / COMPILE_CACHE_ACTION
    text = action.read_text(encoding="utf-8") if action.is_file() else ""
    targets = re.findall(r"^\s*target=\S+", text, re.M)
    sums = re.findall(r"^\s*sha256=[0-9a-f]{64}\s*$", text, re.M)
    if not re.search(r"^\s*SCCACHE_RELEASE:\s*v\d+\.\d+\.\d+\s*$", text, re.M) \
            or not targets or len(targets) != len(sums) or "sha256sum --check" not in text:
        failures.append(
            f"{COMPILE_CACHE_ACTION} does not pin sccache: a release (vX.Y.Z) in "
            f"SCCACHE_RELEASE and a sha256 checked for each archive it names "
            f"({len(targets)} target(s), {len(sums)} checksum(s))")
    # A lower bound, so a job parse that stopped finding the action cannot pass.
    if users < 35:
        failures.append(
            f"found {users} job(s) that compile through ./.github/actions/compile-cache; "
            f"a broken job parse passes by finding none")
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
# and refuses another build, the image the gate runs in installs it by the same
# pin, and the budget names the build its figures are measured with. QEMU is no tool
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

    # The image the gate runs in, on the hosted lane as on every other machine
    # (ADR 0145, D1), installs the same build: with the same script, or by the
    # release and the digest the script pins for the hosted runners' x86_64
    # archive (D3). Never the distribution's.
    containerfile = GATES.get(ARM_TOOLCHAIN_GATE, {}).get("container")
    image = repo_root / containerfile if isinstance(containerfile, str) else None
    if image is None or not image.is_file():
        failures.append(f"`{ARM_TOOLCHAIN_GATE}` names no image it runs in "
                        f"({containerfile!r}), so the hosted lane judges it with "
                        f"whatever compiler its runner carries")
    else:
        image_code = script_code(image)
        x86 = re.search(rf'"arm-gnu-toolchain-{re.escape(version)}-x86_64-arm-none-eabi'
                        rf'\.tar\.xz ([0-9a-f]{{64}})"', install_text)
        by_script = ARM_TOOLCHAIN_INSTALL in image_code
        by_pin = x86 is not None and version in image_code \
            and x86.group(1) in image_code
        if not (by_script or by_pin):
            failures.append(f"{containerfile}, the image `{ARM_TOOLCHAIN_GATE}` runs in, "
                            f"installs no {build}: it runs {ARM_TOOLCHAIN_INSTALL}, or "
                            f"names the release and the x86_64 digest that script pins")
        if "gcc-arm-none-eabi" in image_code.split():
            failures.append(f"{containerfile} installs the distribution's "
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

    ap = build_parser()

    def cli(*argv):
        """The registry asked in process, as the hook asks it: (status,
        stdout, stderr)."""
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            status = run_cli(ap, ap.parse_args(["--repo-root", str(repo_root),
                                                *argv]))
        return status, out.getvalue(), err.getvalue()

    def check(label, changed, want_full=None, want_has=(), want_lacks=(),
              want_unreached=(), want_owed=()):
        nonlocal cases
        cases += 1
        keys, reason = select(repo_root, changed)
        # "Full" is every gate a push can run. `select` is what the hook
        # calls, so a `ci_only` gate is not missing from its answer — it was
        # never on offer, and counting it would make rule 1 unprovable here.
        offered = drop_ci_only(set(GATES))
        full = len(keys) == len(offered)
        if want_full is not None and full != want_full:
            failures.append(f"{label}: full={full}, wanted {want_full} ({reason})")
        for k in want_has:
            if k not in keys:
                failures.append(f"{label}: gate {k} missing from {keys}")
        for k in want_lacks:
            if k in keys:
                failures.append(f"{label}: gate {k} unexpectedly selected")
        # What the change must not even reach, for a `ci_only` gate no
        # selection holds: a trigger that widened would show only here.
        reached = matched(repo_root, changed)[0]
        for k in want_unreached:
            if k in reached:
                failures.append(f"{label}: gate {k} is reached by {changed}")
        # An update to main selects what any push selects (the owner's
        # decision of 2026-11-25, "What `ci_only` means"). The hook asks for
        # it with `--landing`, so the case asks the same command line; a
        # selection that branched on a landing again turns this red.
        _, landed, _ = cli("--changed", *changed, "--landing")
        if landed.splitlines() != keys:
            failures.append(
                f"{label}: an update to main selects {landed.split()}, not "
                f"what any push selects: {keys}")
        # And what it leaves out, every reached `ci_only` gate, is named as
        # hosted CI's to audit after landing (`--ci-owed`, as the hook asks).
        if want_owed:
            _, owed_text, _ = cli("--changed", *changed, "--ci-owed", "--landing")
            owed = {ln.split()[0] for ln in owed_text.splitlines() if ln.strip()}
            for k in want_owed:
                if k not in owed:
                    failures.append(
                        f"{label}: --ci-owed on an update to main does not "
                        f"name {k}, a ci_only gate the change reaches, as "
                        f"audited after landing: {sorted(owed)}")

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
    # The same edit on an update to main: every `check` asks the hook's
    # `--landing` too, so this case says what the landing leaves to hosted
    # CI. The workspace suite is reached and `ci_only`: the landing does not
    # run it, and `--ci-owed` names it as audited after landing.
    check("rust-landing", ["sce-build/src/mesh/deploy.rs"], want_full=False,
          want_has=["codegen-build", "clippy"], want_lacks=["workspace-tests"],
          want_owed=["workspace-tests"])
    # An integration test of the generator crate is compiled by the workspace
    # suite and clippy, and by no build of the generator or the C++ tree. The
    # gates it must not reach are `ci_only` and in no selection, so the case
    # asks what the change reaches.
    check("crate-test-landing", ["sce-build/tests/gate_registry_contract.rs"],
          want_full=False, want_has=["clippy"],
          want_lacks=["workspace-tests"],
          want_unreached=["cpp-suite-wasm", "w3c-cpp", "cpp-suite", "w3c-c11",
                          "w3c-wasm", "install-consumer",
                          "cpp-interface-standard", "forge-rust",
                          "drift-suites", "c11-mcu", "example-codegen",
                          "targets", "doc-content", "generated-deterministic"],
          want_owed=["workspace-tests"])
    # Rule 1 on an update to main: every gate a push can run, which is the
    # set `PUSH_BUDGET_S` bounds, and no `ci_only` gate.
    check("unclassified-landing", ["some_new_top_level_dir/thing.txt"],
          want_full=True, want_has=["clippy"],
          want_lacks=["cpp-suite-wasm", "windows-msvc", "mutation-rounds"])
    # A document under `docs/` that no path-scoped gate reads does not buy
    # rule 1's answer, and the two that a suite reads reach that suite. The
    # tree-wide sweep (`tree-hygiene`) and the suite are `ci_only`: hosted
    # CI audits them after landing, and `--ci-owed` says so.
    for doc in ("docs/SCE_CODEC_CONSUMER_MIGRATION.md",
                "docs/SCE_TEST_PROFILE.md"):
        check("document-landing", [doc], want_full=False,
              want_has=["http-endpoint-ssot"], want_lacks=["tree-hygiene"],
              want_unreached=["workspace-tests"], want_owed=["tree-hygiene"])
    for doc in ("docs/SCE_LUA_TRANSLATION_SEAM.md",
                "docs/SCE_TTCN3_LANE.md"):
        check("read-document-landing", [doc], want_full=False,
              want_lacks=["tree-hygiene", "workspace-tests"],
              want_owed=["tree-hygiene", "workspace-tests"])
    # Dependency closure: an example-only change still builds sce-codegen.
    # `example-codegen` is `ci_only` now, but the closure it pulls in is not:
    # the dependency must survive its dependent leaving, or a gate that
    # remains local loses the binary it executes.
    check("example-dep", ["examples/smart_light/smart_light.scxml"],
          want_has=["codegen-build"], want_lacks=["example-codegen"])
    check("example-dep-landing", ["examples/smart_light/smart_light.scxml"],
          want_has=["codegen-build"], want_lacks=["example-codegen"],
          want_owed=["example-codegen"])
    # A template edit must reach the committed-tree drift gate.
    check("template", ["tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2"],
          want_full=False, want_has=["codegen-build"],
          want_lacks=["drift-suites"])
    check("template-landing",
          ["tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2"],
          want_full=False, want_has=["codegen-build"],
          want_lacks=["drift-suites"], want_owed=["drift-suites"])
    # doc-check.yml edits must reach the gate that mirrors it. Under the
    # numbered table this gate was mapped to the no_std workflow, so an edit
    # to doc-check.yml alone did not select it.
    cases += 1
    keys, _ = select(repo_root, [".github/workflows/doc-check.yml"])
    if "rustdoc-links" not in keys:
        failures.append("doc-check-self: rustdoc-links missing from " + str(keys))

    # ── The judgment of an update to main ───────────────────────────
    #
    # a-landing-leaves-every-reached-ci-only-gate-to-the-audit. The owner's
    # decision of 2026-11-25 ("What `ci_only` means"): a landing runs the
    # gates any push runs, each unit with `--landing`, and names the reached
    # `ci_only` gates as hosted CI's to audit after landing. A Rust source
    # edit reaches `workspace-tests`, which is `ci_only`, so the landing's
    # plan, asked as the hook asks it, has no unit of it, every unit it has
    # says `--landing`, and its account names the suite. A plan that branched
    # on a landing again (a unit of a `ci_only` gate), lost the flag
    # (`plan_units`) or stopped naming what it left (`name_audited`) turns
    # this red.
    cases += 1
    rust_path = ["sce-build/src/mesh/deploy.rs"]
    witness = "workspace-tests"
    if not GATES[witness].get("ci_only") \
            or witness not in matched(repo_root, rust_path)[0]:
        failures.append(
            f"landing: {witness} is no longer a ci_only gate {rust_path[0]} "
            f"reaches, so this case reads nothing; pick another witness")
    status, plan_text, plan_err = cli("--plan", "--rust", "1.0.0", "--changed",
                                      *rust_path, "--landing")
    landing_units = [json.loads(ln) for ln in plan_text.splitlines()
                     if ln.strip()]
    planned = {unit_gate(u["name"]) for u in landing_units}
    if status != 0 or not landing_units:
        failures.append(
            f"landing: the plan of an update to main that edits {rust_path[0]} "
            f"ended {status} with {len(landing_units)} unit(s): {plan_err}")
    if planned - set(select(repo_root, rust_path)[0]):
        failures.append(
            f"landing: the plan of an update to main runs "
            f"{sorted(planned - set(select(repo_root, rust_path)[0]))}, which "
            f"no push selects; a landing selects what any push selects")
    for unit in landing_units:
        if "--landing" not in unit.get("argv", []):
            failures.append(
                f"landing: unit {unit['name']} of an update to main runs "
                f"{unit.get('argv')}, without --landing, so a skip would pass")
    if f"after landing: {witness} is ci_only, audited after landing by " \
            not in plan_err:
        failures.append(
            f"landing: the plan of an update to main does not name {witness} "
            f"as left to hosted CI's audit: {plan_err}")
    # The notice is the same on every push: `--ci-owed` names what hosted CI
    # audits, and on an update to main that is what it audits after landing.
    cases += 1
    owed_main = cli("--changed", *rust_path, "--ci-owed", "--landing")[1]
    owed_branch = cli("--changed", *rust_path, "--ci-owed")[1]
    if owed_main != owed_branch:
        failures.append(
            f"landing: --ci-owed names {owed_main.split()} on an update to "
            f"main and {owed_branch.split()} on a push of another ref; both "
            f"leave out the same gates")
    if witness not in {ln.split()[0] for ln in owed_main.splitlines()
                       if ln.strip()}:
        failures.append(
            f"landing: --ci-owed does not name {witness} on an update to main, "
            f"which reaches it and leaves it to hosted CI: {owed_main!r}")

    # a-landing-leaves-out-exactly-the-ci-only-gates. An unclassified path
    # reaches every gate (rule 1), so the landing's answer there is the table
    # less every `ci_only` gate, the set `budget_is_not_exceeded` holds to
    # `PUSH_BUDGET_S`, and what it leaves out is exactly the `ci_only` gates.
    cases += 1
    ci_only_gates = {s for s in GATES if GATES[s].get("ci_only")}
    unclassified = ["some_new_top_level_dir/thing.txt"]
    full_keys = cli("--changed", *unclassified, "--landing")[1].split()
    if set(full_keys) != set(GATES) - ci_only_gates:
        failures.append(
            f"landing: an unclassified path on an update to main runs "
            f"{len(full_keys)} gate(s), not the {len(GATES) - len(ci_only_gates)} "
            f"that are not ci_only; missing="
            f"{sorted(set(GATES) - ci_only_gates - set(full_keys))} "
            f"extra={sorted(set(full_keys) & ci_only_gates)}")
    if left_out(repo_root, unclassified) != ci_only_gates:
        failures.append(
            f"landing: what an update to main leaves out is "
            f"{sorted(left_out(repo_root, unclassified))}, not the ci_only "
            f"gates {sorted(ci_only_gates)}")

    # a-full-fleet-run-leaves-out-what-the-fleet-cannot-place-or-bound. The
    # one selection that still reads `platform` and `audited_after_landing`:
    # `--plan --all --landing`, which `PREPUSH_FULL=1` asks for on an update
    # to main. It keeps every other gate, `ci_only` included, and names the
    # ones it left.
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
    status, full_text, full_err = cli("--plan", "--rust", "1.0.0", "--all",
                                      "--landing")
    full_covered: set[str] = set()
    for unit in (json.loads(ln) for ln in full_text.splitlines() if ln.strip()):
        gate = unit_gate(unit["name"])
        full_covered |= {gate} | transitive_deps(gate)
    if status != 0 or full_covered != set(GATES) - off_fleet - audited:
        failures.append(
            f"full-landing: --plan --all --landing ended {status} covering "
            f"{len(full_covered)} gate(s), not every gate but the ones off the "
            f"fleet and the audited ones; missing="
            f"{sorted(set(GATES) - off_fleet - audited - full_covered)} "
            f"extra={sorted(full_covered & (off_fleet | audited))}")
    for slug in sorted(off_fleet | audited):
        if f"after landing: {slug} " not in full_err:
            failures.append(
                f"full-landing: --plan --all --landing leaves out {slug} "
                f"without naming it: {full_err}")
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
    # The reason a landing gives for each gate it leaves, one per kind: an
    # unbounded cost wins over a platform, a platform the fleet lacks is
    # named with its reason, and any other gate by its workflows.
    cases += 1
    why_fixture = {
        "unbounded": {"audited_after_landing": "its cost has no bound",
                      "platform": "macos"},
        "elsewhere": {"platform": "macos", "platform_reason": "a Mac"},
        "on-linux": {"platform": "linux", "workflows": ["l.yml"]},
        "slow": {"workflows": ["a.yml", "b.yml"]},
        "orphan": {},
    }
    for slug, want in (
            ("unbounded", "is audited after landing (its cost has no bound)"),
            ("elsewhere", "needs platform 'macos', which bx.toml does not "
                          "declare (a Mac)"),
            ("on-linux", "is ci_only, audited after landing by l.yml"),
            ("slow", "is ci_only, audited after landing by a.yml, b.yml"),
            ("orphan", "is ci_only, audited after landing by no workflow")):
        got = why_audited(slug, {"linux"}, table=why_fixture)
        if got != want:
            failures.append(f"why-audited: {slug} reads {got!r}, not {want!r}")

    # a-gate-audited-after-landing-is-ci-only-and-says-why. It is left out of
    # even a full fleet run for its cost, so it has to be `ci_only` (no push
    # can run it either), it has to name the workflow that audits it, and the
    # judgment prints the reason, so it has to be one.
    cases += 1
    for slug, spec in sorted(GATES.items()):
        if "audited_after_landing" not in spec:
            continue
        if not spec.get("ci_only"):
            failures.append(
                f"audited: {slug} is left out of a full fleet run and is not "
                f"ci_only; a push would run it")
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
    # even a full fleet run, so it has to be `ci_only` (no push can run it on
    # the fleet either), it has to name the workflow that audits it, and the
    # reason is printed in the judgment, so it has to be one.
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
        for line in image_last_stage(script_code(repo_root / containerfile)):
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

    # a-gate-in-an-image-is-handed-what-its-lane-sets. A name in
    # `container_env` is a variable spelled as one, of a gate that runs in an
    # image, other than its part, and set by a workflow the gate mirrors (a
    # key of an `env:` mapping, or an assignment its steps make, to a command
    # or to `$GITHUB_ENV`): a name no lane sets hands nothing in and reads as
    # if it did.
    cases += 1
    for slug, spec in sorted(GATES.items()):
        names = spec.get("container_env")
        if names is None:
            continue
        if not spec.get("container"):
            failures.append(
                f"container_env: {slug} names variables to hand into an image "
                f"and runs in none")
        if not isinstance(names, list) or not names:
            failures.append(
                f"container_env: {slug} declares {names!r}; it is a list of "
                f"variable names, absent when there are none")
            continue
        lanes = "\n".join(
            (repo_root / ".github" / "workflows" / wf).read_text(encoding="utf-8")
            for wf in spec.get("workflows", [])
            if (repo_root / ".github" / "workflows" / wf).is_file())
        for name in names:
            if not isinstance(name, str) or not CONTAINER_ENV_NAME.match(name):
                failures.append(
                    f"container_env: {slug} names {name!r}, which is not a "
                    f"variable's name")
                continue
            if name == spec.get("shard_env"):
                failures.append(
                    f"container_env: {slug} names its part's variable {name}, "
                    f"which goes into the image as `--shard`")
            if not re.search(rf"^\s*{name}:|\b{name}=", lanes, re.M):
                failures.append(
                    f"container_env: {slug} hands {name} into its image and "
                    f"no workflow it mirrors ({spec.get('workflows')}) sets it")

    # a-gate-image-is-pinned. Every image a gate runs in names its base by
    # digest, reads its packages from a dated snapshot and checks every
    # download against a checksum in the step that downloads it, so a rebuild
    # of one context yields the same environment (docs/adr/0145, D3). An image
    # in IMAGES_NOT_YET_PINNED is read the other way: once it is pinned, the
    # entry is stale. The fixture holds the reader to each refusal.
    cases += 1
    digest = "a" * 64
    pinned_fixture = "\n".join([
        f"FROM ubuntu:24.04@sha256:{digest}",
        "ARG UBUNTU_SNAPSHOT=20261126T000000Z",
        f'RUN echo "{IMAGE_SNAPSHOT_SOURCE}" && apt-get install -y git',
        "ARG X_SHA256=" + digest,
        'RUN curl -fsSL -o /tmp/x "https://x" \\',
        '    && echo "${X_SHA256}  /tmp/x" | sha256sum --check --strict -',
        "RUN git clone https://x /opt/x && git -C /opt/x checkout --quiet abc"])
    for label, text, want in (
            ("pinned", pinned_fixture, 0),
            ("a base by tag", pinned_fixture.replace(f"@sha256:{digest}", ""), 1),
            ("a live archive", pinned_fixture.replace(IMAGE_SNAPSHOT_SOURCE, "http://archive"), 1),
            ("an unchecked download", pinned_fixture.replace(" | sha256sum --check --strict -", ""), 1),
            ("a clone of no commit", pinned_fixture.replace(" && git -C /opt/x checkout --quiet abc", ""), 1)):
        got = image_pin_failures("x/Containerfile", text)
        if len(got) != want:
            failures.append(f"image-pinned: the fixture {label} gave {len(got)} "
                            f"refusal(s), not {want}: {got!r}")
    image_files = sorted({spec["container"] for spec in GATES.values()
                          if isinstance(spec.get("container"), str)})
    for containerfile in image_files:
        path = repo_root / containerfile
        if not path.is_file():
            continue
        found = image_pin_failures(containerfile, path.read_text(encoding="utf-8"))
        if containerfile in IMAGES_NOT_YET_PINNED:
            if not found:
                failures.append(f"image-pinned: {containerfile} names its base by digest "
                                f"and its archive by date now; drop it from "
                                f"IMAGES_NOT_YET_PINNED")
            continue
        failures.extend(f"image-pinned: {f}" for f in found)
    for containerfile in sorted(set(IMAGES_NOT_YET_PINNED) - set(image_files)):
        failures.append(f"image-pinned: IMAGES_NOT_YET_PINNED names {containerfile}, "
                        f"which no gate runs in")

    # an-image-installs-the-releases-the-tree-pins. A release a script or
    # bx.toml pins is written in an image's Containerfile again, since nothing
    # outside its build context reaches the build; each copy is held to the
    # pin's own file, so the image and a build machine run one release.
    cases += 1
    failures.extend(image_release_failures(repo_root))

    # a-lane-sets-only-what-its-image-is-handed and
    # a-lane-of-a-gate-in-an-image-pulls-it. The image starts from an
    # environment of its own, so a variable a lane sets for a gate in an image
    # is in its `container_env`, or its part, or read nowhere; and the job that
    # runs it logs in to the registry and may push what it built
    # (.github/actions/gate-image).
    cases += 1
    failures.extend(image_lane_failures(repo_root))

    # the-fleet-and-the-lanes-install-one-emsdk. The build machines install
    # the release `scripts/install_emsdk.sh` pins, and the image a gate that
    # builds with emscripten runs in names one on its own `emsdk install`
    # line, as a workflow's step did before the lanes ran their gates in
    # images. A landing judged on one release and audited on another would
    # compare two compilers, so every such line names the script's, and every
    # image an emscripten gate runs in has one.
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
        installing: set[str] = set()
        sources = [str(w.relative_to(repo_root)) for w in
                   sorted((repo_root / ".github" / "workflows").glob("*.yml"))]
        sources += sorted({spec["container"] for spec in GATES.values()
                           if isinstance(spec.get("container"), str)
                           and (repo_root / spec["container"]).is_file()})
        for rel in sources:
            text = (repo_root / rel).read_text(encoding="utf-8", errors="replace")
            for step in re.finditer(r"emsdk (?:install|activate) (\S+)", text):
                named += 1
                installing.add(rel)
                if step.group(1) != pin:
                    failures.append(
                        f"emsdk: {rel} names emsdk {step.group(1)} "
                        f"and the build machines install {pin} "
                        f"(scripts/install_emsdk.sh)")
        emscripten = re.compile(
            r"\bemsdk\b|sce_gate_emscripten_node|sce_gate_configure_wasm_tree")
        for slug, spec in sorted(GATES.items()):
            containerfile = spec.get("container")
            script = gate_script(repo_root, slug)
            if not isinstance(containerfile, str) or not script.is_file() \
                    or not emscripten.search(script_code(script)):
                continue
            if containerfile not in installing:
                failures.append(
                    f"emsdk: {slug} builds with emscripten in the image "
                    f"{containerfile} builds, which has no `emsdk install "
                    f"{pin}` line: the release it builds with is then no "
                    f"one the build machines install")
        if named == 0:
            failures.append(
                "emsdk: no workflow or gate image names an `emsdk install` "
                "release, so this case read nothing")

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
        # The image a gate runs in selects it as its sources do
        # (`image_context`), so the lane runs for an edit of the image too.
        image = ([trigger_globs.image_context(spec["container"])]
                 if spec.get("container") else [])
        owns = ((spec.get("local") or []) + (spec.get("extra") or [])
                + built + image)
        for own in owns:
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
    # Reading the entry's own source for the note is what keeps the note
    # from being optional; the same shape the `local`-only gates' reasons
    # take. The note sits in the first lines of the entry's dict, which the
    # entry's file opens on a line of its own. Read from this checkout's
    # entries (`gate_table.entry_source`), never from `--repo-root`, so one
    # checkout's comments are not judged against another checkout's table.
    cases += 1
    for slug, spec in sorted(GATES.items()):
        if not spec.get("workflows") or spec.get("runner_workflow"):
            continue
        entry_src = gate_table.entry_source(slug)
        entry = next((i for i, line in enumerate(entry_src) if line == "{"), None)
        if entry is None:
            failures.append(f"delegation: {gate_table.entry_input(slug)} opens "
                            f"no dict on a line of its own")
            continue
        window = "\n".join(entry_src[entry : entry + 12])
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
    # name the same slugs. Six comments in the entries said a cost was
    # pace-normalised, and until 2026-09-02 that was the only place it was
    # written — so the drift report told readers to "update cost_s" on
    # every one of them. A new normalisation that lands as a comment and
    # not as an entry here puts us straight back there, and nothing would
    # notice. Rule 10 of this repository's round rules, applied to the
    # registry's own files.
    #
    # Each entry is a file of its own, so a mention belongs to the gate whose
    # file holds it. While the entries were one literal in this file the
    # scanner had to find where one ended, and caught itself on 2026-09-02
    # attributing the mentions in this function and in the drift printer to
    # whichever slug happened to be last.
    prose_says = {slug for slug in GATES
                  if any("pace-normalis" in line
                         for line in gate_table.entry_source(slug))}
    if prose_says != set(PACE_NORMALISED):
        only_prose = sorted(prose_says - set(PACE_NORMALISED))
        only_data = sorted(set(PACE_NORMALISED) - prose_says)
        failures.append(
            f"PACE_NORMALISED disagrees with the comments in the entries: "
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
    # outside its inputs. Asked of the derivation and of the call: the
    # preflight asks gate_table.py, whose answer for forge-c11 alone is the one
    # every gate's derivation gives (what its run opens is traced by
    # `a-gate-run-reads-its-own-entries`).
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
    if '"$TABLE" submodules' not in preflight or "gate_registry.py" in preflight:
        failures.append(
            "preflight: scripts/lib/require_submodules.sh does not ask "
            "gate_table.py for the gates it runs, so a unit's run would read "
            "the registry and every gate's derivation")
    whole = gate_inputs("forge-c11", triggers=gate_triggers(repo_root),
                        reads=gate_reads(repo_root))
    scope = "*" if whole is None else " ".join(
        submodules_reached(whole, submodule_paths(repo_root)))
    asked = list(gate_table.submodule_needs(repo_root, ["forge-c11"], GATES))
    if asked not in ([], [f"forge-c11\t{scope}"]):
        failures.append(
            f"preflight: gate_table.py answers {asked} for forge-c11 alone, and "
            f"its derivation among every gate's reaches {scope!r}")

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
    # unit off machines for nothing. A gate in an image names none: its script
    # runs the image's programs, and its unit asks the machine for docker alone.
    cases += 1
    read_gates = 0
    for slug, spec in sorted(GATES.items()):
        if spec.get("container"):
            read_gates += 1
            if "packages" in spec:
                failures.append(
                    f"packages: {slug} runs in the image {spec['container']} "
                    f"builds and asks the machine for the package(s) "
                    f"{spec['packages']!r}; the image holds what its script "
                    f"runs, so its unit asks for {CONTAINER_NEED!r} alone")
            continue
        if "packages" not in spec:
            continue
        read_gates += 1
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
    if read_gates == 0:
        failures.append(
            "packages: no gate declares a package and none runs in an image, "
            "so the check above read nothing")

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
            for line in gate_script_code(script).splitlines())
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

    # a-gate-run-reads-its-own-entries. What a unit's run reads of
    # tools/git-hooks/ is what its verdict rests on ("One file per gate" in
    # gate_table.py): the runner's questions open gate_table.py and the
    # entries of the gates the unit runs, list nothing, and stay inside the
    # inputs the plan gives the unit. Traced as the runner starts them, with
    # Python's audit hook (`unit_read_failures`).
    cases += 1
    failures.extend(f"a-gate-run-reads-its-own-entries: {f}"
                    for f in unit_read_failures(repo_root))

    # an-entry-is-data. An entry is read, never run: literals and the names of
    # `ENTRY_NAMES` are its values, and anything else is refused, so an
    # entry can neither execute code in the runner nor name what
    # gate_table.py does not define. Every file in the entries' directory is
    # an entry, named by its gate's slug.
    cases += 1
    good = gate_table.parse_entry(
        '{\n    # a reason\n    "parallelism": PARALLELISM_SCRIPT,\n'
        '    "cost_s": -1.5, "inputs": INPUTS_ARE_TRIGGERS,\n'
        '    "deps": ["a", "b"], "summary": "x" "y",\n}\n', "fixture")
    if good != {"parallelism": gate_table.PARALLELISM_SCRIPT, "cost_s": -1.5,
                "inputs": INPUTS_ARE_TRIGGERS, "deps": ["a", "b"],
                "summary": "xy"}:
        failures.append(f"an-entry-is-data: the fixture read as {good}")
    for text in ('{"inputs": open("x")}', '{"x": __import__("os")}',
                 '{"x": [i for i in ()]}', '["not", "a", "dict"]',
                 '{"x": UNDEFINED}', '{1: 2}', '{"x": 1 + 2}', '{**{}}'):
        try:
            gate_table.parse_entry(text, "fixture")
            failures.append(f"an-entry-is-data: {text!r} was read as an entry")
        except gate_table.EntryError:
            pass
    for path in sorted(gate_table.ENTRIES.iterdir()):
        if path.suffix != ".py" or not gate_table.SLUG.match(path.stem):
            failures.append(f"an-entry-is-data: {path.name} in the entries' "
                            f"directory names no gate, so nothing reads it")

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
            ("rust-source-landing", select(repo_root, ["sce-build/src/mesh/deploy.rs"])[0],
             True),
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
             gate_table.entry_input("by-triggers"), TRIGGER_GLOBS_INPUT,
             *RUNNER_INPUTS}):
        failures.append(
            f"plan-fixture: `by-triggers` is keyed on its triggers, its "
            f"script, its entry, the derivation of its triggers and the "
            f"runner's inputs, and was planned as {by_triggers}")
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
             "scripts/gates/building.sh", gate_table.entry_input("building"),
             TRIGGER_GLOBS_INPUT, *RUNNER_INPUTS}):
        failures.append(
            f"plan-fixture: `building` is keyed on its triggers, what its build "
            f"reads beside them, its script, its entry, the derivation of both "
            f"and the runner's inputs, and was planned as {built_unit}")
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
                {"docs/**", "scripts/gates/alone.sh",
                 gate_table.entry_input("alone"), *RUNNER_INPUTS}):
        failures.append(
            f"plan-fixture: `alone` keeps its verdict over its own inputs, "
            f"its script's, its entry's and the runner's, and was planned as "
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
    # install's digests, the gate's PATH, the gate's image and the budget, held
    # to one another (see "The ARM compiler bx.toml declares" above).
    cases += 1
    failures.extend(f"arm-toolchain: {f}" for f in arm_toolchain_failures(repo_root))

    # a-gate-image-holds-the-pins-its-gate-reads. A gate reads a tool's release
    # from the line of an install script that pins it (`X_REV="..."` in
    # `scripts/install_<tool>.sh`), and in an image the tool is the image's,
    # named there by version (ADR 0145, D3). The pin then has two homes, and a
    # release moved in one alone would be a gate that looks for one build and
    # runs another; the image's copy is held to the script's here. Derived from
    # the gate's own code, so a gate that starts reading a pin tomorrow is held
    # without an edit here.
    cases += 1
    pins_held = 0
    for slug, spec in sorted(GATES.items()):
        containerfile = spec.get("container")
        script = gate_script(repo_root, slug)
        if not isinstance(containerfile, str) or not script.is_file() \
                or not (repo_root / containerfile).is_file():
            continue
        code = script_code(script)
        image_code = script_code(repo_root / containerfile)
        for install in sorted(set(re.findall(r"scripts/install_\w+\.sh", code))):
            install_path = repo_root / install
            if not install_path.is_file():
                continue
            pins = re.findall(r'^([A-Z][A-Z0-9_]*_REV)="([^"]+)"$',
                              install_path.read_text(encoding="utf-8"), re.M)
            for var, value in pins:
                if var not in code:
                    continue
                pins_held += 1
                if value not in image_code:
                    failures.append(
                        f"image-pins: {slug} reads {var}={value} from {install} "
                        f"and runs in the image {containerfile} builds, which "
                        f"names no {value}: the gate would look for one release "
                        f"and run whatever the image installed")
    if pins_held == 0:
        failures.append(
            "image-pins: no gate in an image reads a pin from an install script, "
            "so this case read nothing; c11-mcu reads ARM_GNU_TOOLCHAIN_REV")

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
    gradle_step = lambda key, home="~": [
        "    - name: Cache Gradle",
        "      uses: actions/cache@" + "e" * 40,
        "      with:",
        "        path: |",
        f"          {home}/.gradle/caches",
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
    # The Gradle home of an image a gate runs in is read as one too: a lane
    # that keeps it under another lane's key is the same sharing.
    image_home = "~/.cache/sce-gate-containers/x/home"
    imaged, imaged_seen = shared_gradle_cache_keys({
        "a.yml": gradle_step("Linux-gradle-h", image_home),
        "b.yml": gradle_step("Linux-gradle-h")})
    if imaged_seen != 2 or len(imaged) != 1:
        failures.append(f"every-lane-keys-its-own-gradle-cache: an image's Gradle "
                        f"home under another lane's key gave {imaged!r} from "
                        f"{imaged_seen} step(s)")
    failures.extend(f"every-lane-keys-its-own-gradle-cache: {f}"
                    for f in gradle_cache_failures(repo_root))

    # a-cache-keyed-by-the-run-is-refused. No cache is saved under a key new
    # on every run, and no family keep-one-cache keeps begins another
    # (`per_run_cache_failures`). The fixture holds the reader to each refusal.
    cases += 1
    def cache_job(key, prefix=None):
        lines = ["    - name: Restore",
                 "      uses: actions/cache/restore@" + "e" * 40,
                 "      with:",
                 "        path: ~/.local/thing",
                 f"        key: {key}"]
        if prefix is not None:
            lines += ["    - name: Keep",
                      "      uses: ./.github/actions/keep-one-cache",
                      "      with:",
                      f"        prefix: {prefix}",
                      "        keep: x"]
        return lines
    sha = "${{ github.sha }}"
    content = "${{ hashFiles('a') }}"
    for label, workflows, want in (
            ("keyed by what it holds and kept", {"a.yml": cache_job(f"L-a-{content}", "L-a-")}, 0),
            ("keyed by the run, though kept", {"a.yml": cache_job(f"L-a-{sha}", "L-a-")}, 1),
            ("keyed by the run", {"a.yml": cache_job(f"L-a-{sha}")}, 1),
            ("a family that begins another",
             {"a.yml": cache_job(f"L-{content}", "L-"),
              "b.yml": cache_job(f"L-b-{content}", "L-b-")}, 1)):
        got, _ = per_run_cache_failures(workflows)
        if len(got) != want:
            failures.append(f"a-cache-keyed-by-the-run-is-refused: {label} gave "
                            f"{len(got)} refusal(s), not {want}: {got!r}")
    failures.extend(f"a-cache-keyed-by-the-run-is-refused: {f}"
                    for f in per_run_cache_sweep(repo_root))

    # compile-cache-is-content-addressed. No cache step saves compile output, no
    # lane compiles through a cache past ./.github/actions/compile-cache, and a
    # job that uses it prints what it answered (see "The compile cache"
    # above). The fixture holds the reader to each refusal and to each shape it
    # leaves alone, so the sweep's silence is a reading.
    cases += 1
    def cached(*paths, action="actions/cache"):
        return (["jobs:", "  a:", "    steps:", "    - name: Cache",
                 f"      uses: {action}@" + "e" * 40, "      with:", "        path: |"]
                + [f"          {p}" for p in paths] + ["        key: k"])
    def compiled(*report):
        return (["jobs:", "  a:", "    steps:",
                 "    - name: Compile through the shared compile cache",
                 "      id: compile-cache",
                 "      uses: ./.github/actions/compile-cache",
                 "    - name: Build",
                 "      run: cargo build"] + list(report))
    reported = ["    - name: Report what the compile cache answered",
                "      if: always() && steps.compile-cache.outcome == 'success'",
                "      run: sccache --show-stats"]
    for label, lines, want in (
            ("cargo's registry download",
             cached("~/.cargo/registry/index", "~/.cargo/registry/cache", "~/.cargo/git/db"), 0),
            ("configure state of a CMake tree",
             cached("build/CMakeCache.txt", "build/CMakeFiles", "build_python/_deps"), 0),
            ("a Rust target", cached("~/.cargo/registry", "target"), 1),
            ("an image's Rust target", cached("~/.cache/sce-gate-containers/x/target"), 1),
            ("Swatinem/rust-cache", cached(action="Swatinem/rust-cache"), 1),
            ("a ccache directory", cached("~/.ccache"), 1),
            ("an sccache directory", cached("${{ runner.temp }}/sccache"), 1),
            ("a whole CMake build tree", cached("build_wasm"), 1),
            ("sccache installed past the action",
             ["      - uses: mozilla-actions/sccache-action@" + "e" * 40 + " # v0.0.11"], 1),
            ("ccache installed", ["          sudo apt-get install -y cmake ccache ninja-build"], 1),
            ("ccache in prose", ["      # ccache and ~/.ccache are what this replaced"], 0),
            ("the action, reported", compiled(*reported), 0),
            ("the action, never reported", compiled(), 1),
            ("the action, reported only on success",
             compiled(*[line for line in reported if "if:" not in line]), 1)):
        got, _ = compile_cache_failures({"a.yml": lines})
        if len(got) != want:
            failures.append(f"compile-cache-is-content-addressed: {label} gave "
                            f"{len(got)} refusal(s), not {want}: {got!r}")
    _, users = compile_cache_failures({"a.yml": compiled(*reported),
                                       "b.yml": compiled(*reported)})
    if users != 2:
        failures.append(f"compile-cache-is-content-addressed: two jobs that use the "
                        f"action read as {users}")
    failures.extend(f"compile-cache-is-content-addressed: {f}"
                    for f in compile_cache_sweep(repo_root))

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
    # judgment says which gates it did not measure.
    audited: list[str] = []
    if args.all:
        selected, reason = set(GATES), "every gate (--all)"
        if args.landing:
            audited = hosted_only(repo_root, selected)
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
        keys, reason = select(repo_root, changed)
        selected = set(keys)
        if args.landing:
            audited = ci_owed(repo_root, changed)
            reason += ", as a landing's (--landing)"

    tree_is_head = tracked_tree_is_head(repo_root)
    head = head_commit(repo_root)
    units = plan_units(selected, rust=args.rust, tree_is_head=tree_is_head,
                       head=head, landing=bool(args.landing),
                       changes=tuple(args.changed_in or ()),
                       shards={s: shard_count(repo_root, s) for s in selected},
                       triggers=gate_triggers(repo_root),
                       reads=gate_reads(repo_root))
    print(f"  selection: {reason}", file=sys.stderr)
    name_audited(repo_root, audited)
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


def build_parser() -> argparse.ArgumentParser:
    """The command line, built once for `main` and for the self-test, which
    asks the registry what the hook asks it, in process."""
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
    ap.add_argument("--landing", action="store_true",
                    help="as the judgment of an update to main: a change "
                         "selects what any push selects, every unit of a plan "
                         "counts no skip as a pass, and the gates left to "
                         "hosted CI's audit are named; with --plan --all, "
                         "every gate the fleet can run")
    ap.add_argument("--all", action="store_true",
                    help="with --plan: every gate, ci_only included")
    ap.add_argument("--named", nargs="+", metavar="SLUG",
                    help="with --plan: these gates and what they depend on")
    ap.add_argument("--rust", metavar="RELEASE",
                    help="with --plan: the Rust release every unit runs, as "
                         "`rustc --version` prints it (1.91.0); without it no "
                         "unit's verdict is kept")
    return ap


def run_cli(ap: argparse.ArgumentParser, args) -> int:
    """What the command line asked for, answered; `ap` reports a misuse."""
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
        print(f"  selection: {reason}"
              + (", as a landing's (--landing)" if args.landing else ""),
              file=sys.stderr)
        if args.landing:
            name_audited(repo_root, ci_owed(repo_root, changed))
    for k in keys:
        print(k)
    return 0


def main() -> int:
    ap = build_parser()
    return run_cli(ap, ap.parse_args())


if __name__ == "__main__":
    sys.exit(main())
