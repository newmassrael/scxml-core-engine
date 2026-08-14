#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: tree-hygiene.yml
#
# Every mutation casefile still applies to the tree it studies.
#
# `sce-build/tests/mutations/*.cases` is the only source in this repository
# that nothing compiles and nothing ran. Each case is written against a
# literal anchor in a file that goes on changing, and when the anchor moves
# or splits the case stops testing what its comment says it tests. The
# harness reports that honestly — to whoever runs it by hand, which between
# two commits is nobody.
#
# Measured, twice. `parallel_microstep_owns_exit_and_entry.cases` stood in
# the debt registry as CAUGHT 1/1 for a day while it was in fact
# INCONCLUSIVE, a commit having added an `else` next to its anchor; it
# surfaced only because someone went back to run it. And the first run of
# this gate found three more, in two files: an anchor that had come to match
# four places, so `edit()`'s "replace the first" was aiming at whichever
# backend appeared earliest in the file, and a pair whose anchor closed two
# `format!` calls where the cases were written to mean one each.
#
# What this gate asks is the half of a mutation round that costs nothing:
# does every case still find its anchor, is that anchor unambiguous, and
# does applying it still change the tree. It does not build and does not run
# a test, so it says "applies" and never "CAUGHT" — the verdict about
# whether a suite still turns red belongs to the full round,
# `scripts/mutate <casefile>`, which is a rebuild per case and stays a
# deliberate command.
#
# The trigger is the whole tree, and that is not laziness. What this gate
# reads is the union of every `mutation_targets` declaration inside the
# casefiles — sources under `sce/`, `sce-build/`, `backends/`, generator
# templates, a CMakeLists, two workflows and a gate script. A `paths:` list
# restating that union would be a second copy of a declaration the casefiles
# already carry, free to drift the moment a case names a new file. So this
# runs from tree-hygiene.yml, which declares no filter, for the same reason
# the gates already there do.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

CORPUS="sce-build/tests/mutations"

# Lower bounds on the sweep, not on the corpus. They fail when the
# enumeration stops finding the tree — the failure mode that makes a gate
# report green for having examined nothing. Both sit well under today's
# reading (22 files, 124 cases) so that deleting a case is an ordinary edit
# and losing the directory is not.
MIN_CASEFILES=20
MIN_CASES=100

mapfile -t CASEFILES < <(git ls-files "$CORPUS/*.cases")
if (( ${#CASEFILES[@]} < MIN_CASEFILES )); then
    sce_gate_fail "found ${#CASEFILES[@]} casefile(s) under $CORPUS, expected at least $MIN_CASEFILES — the sweep is not reaching the corpus"
fi

stale=()
cases=0
for casefile in "${CASEFILES[@]}"; do
    if output="$(scripts/mutate --check "$casefile" 2>&1)"; then
        # `|| true` because grep exits 1 on no match, and a casefile that
        # reported success with no `applies` line is a condition for the
        # floor below to judge, not one for `set -e` to end the sweep on.
        applied="$(grep -c '^applies' <<<"$output" || true)"
        cases=$(( cases + applied ))
    else
        stale+=("$casefile")
        printf '\n  FAIL: %s\n' "$casefile" >&2
        printf '%s\n' "$output" >&2
    fi
done

if (( ${#stale[@]} > 0 )); then
    sce_gate_fail "${#stale[@]} casefile(s) no longer apply to the tree: ${stale[*]}
Each one is a mutation that reads as evidence and produces none. Re-aim the
anchor at the site the case comment describes — widening it with a line of
surrounding context is what makes an ambiguous one say which site it means."
fi

# After the failure check, so a corpus that is failing gets the reason that
# explains it rather than a count that follows from it.
if (( cases < MIN_CASES )); then
    sce_gate_fail "only $cases case(s) checked across ${#CASEFILES[@]} casefile(s), expected at least $MIN_CASES — the harness is reporting fewer cases than the corpus holds"
fi

sce_gate_step "$cases case(s) in ${#CASEFILES[@]} casefile(s) still apply"
