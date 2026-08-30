# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Read a test runner's own output and say which tests turned red, and what the
# runner printed about them.
#
# `scripts/mutate` prints a count with every verdict — `CAUGHT (1/287 red)`
# — and a count does not say which assertion earned it. Attribution was
# therefore obtained by re-running the whole harness under
# `GTEST_FILTER=<suite>` until only one suite was left, which is a workaround
# for a fact the runner had already printed and the harness discarded.
#
# The NAME is only half of that fact, and the half that stops one question
# short. Measured 2026-08-30: a red baseline reported
# `test_oracle.py::test_value_is_two` and nothing else, while the runner had
# printed the assertion, the values it compared and the line it stood on — all
# of it thrown away one pipe later. That silence is what kept a leaked
# `SCE_MUTATION_SHARD` unreadable for six days: the CI job's log carried zero
# occurrences of `panicked at` or `assertion`, because the harness parsed the
# names out of the runner's output and dropped the output.
#
# So each format has TWO parsers here: one for the names, one for what the
# runner said about them. They sit in pairs, because a format that drifts
# drifts for both.
#
# Sourced rather than inlined so the parsers can be exercised against captured
# output without a mutation round: they are the part most likely to drift,
# since every one of them reads a format owned by somebody else.
#
# All of them read stdin. The name parsers write one name per line; the detail
# parsers write the runner's own lines, unedited.

# gtest's verdict lines out of a ctest run.
#
# `ctest -j` prefixes every line of a test's output with its job number, so
# the marker is not at the start of the line; that prefix is stripped first.
# The name filter is `contains a dot`, which is what separates
# `[  FAILED  ] Suite.Case (2 ms)` from gtest's own summary line
# `[  FAILED  ] 1 test, listed below:` — without it the summary reads as a
# test named `1`. Names are de-duplicated because gtest prints each failure
# twice: once where it happens and once in that summary.
mutation_failures_from_gtest() {
    awk '{
        sub(/^[0-9]+: /, "")
        if ($0 ~ /^\[  FAILED  \]/ && $4 ~ /\./) print $4
    }' | sort -u
}

# What gtest printed between starting a test and failing it.
#
# gtest brackets every test with `[ RUN      ]` and closes it with one of
# `[       OK ]` or `[  FAILED  ]`, and the diagnosis — the file and line, the
# value, the expectation — is what it prints in between. Buffering to the
# closing bracket and emitting only the failing brackets is what separates the
# diagnosis from the flood a passing test may also have logged.
#
# The summary repeat at the end of a run is NOT a bracket and so never opens a
# buffer: `[  FAILED  ] 1 test, listed below:` arrives with no `[ RUN      ]`
# in front of it, and is dropped along with everything else outside a bracket.
mutation_detail_from_gtest() {
    awk '{
        sub(/^[0-9]+: /, "")
        if ($0 ~ /^\[ RUN +\]/) { buf = $0 "\n"; open = 1; next }
        if (!open) next
        if ($0 ~ /^\[ +FAILED +\]/) { printf "%s%s\n", buf, $0; open = 0; next }
        if ($0 ~ /^\[ +OK +\]/ || $0 ~ /^\[ +SKIPPED +\]/) { open = 0; next }
        buf = buf $0 "\n"
    }'
}

# libtest's verdict lines out of a cargo test binary.
#
# The per-test line only, not the `failures:` block that repeats the names
# further down — one is a verdict and the other is an index of them, and
# reading both would double every count a caller derived from this.
mutation_failures_from_cargo() {
    sed -n 's/^test \(.*\) \.\.\. FAILED$/\1/p'
}

# The captured output libtest replays for each test it failed.
#
# libtest holds a test's stdout and its panic message and prints them after the
# run under `---- <name> stdout ----`, one block per failing test, closed by
# the `failures:` index that `mutation_failures_from_cargo` deliberately
# ignores. That index is where a block ends: a panic's own text can hold
# anything, including a line of dashes, so the terminator is the one marker
# libtest owns rather than a guess about what a panic will not say.
mutation_detail_from_cargo() {
    awk '
        /^failures:$/ { open = 0; next }
        /^---- .+ ----$/ { open = 1; print; next }
        open { print }
    '
}

# Go's verdict lines out of a compiled `go test -c` binary run with `-test.v`.
#
# Anchored at the start of the line, which is what separates a test from a
# SUBTEST: Go indents a subtest's verdict by four spaces per level, and a
# parent whose subtest failed prints its own `--- FAIL` too. Counting both
# would report two red where the runner reports one, and the count is what the
# harness compares against its baseline.
mutation_failures_from_go() {
    sed -n 's/^--- FAIL: \([^ ]*\).*/\1/p'
}

# What a Go test logged between starting and failing.
#
# `-test.v` opens each test with `=== RUN   <name>` and closes it with
# `--- PASS`, `--- FAIL` or `--- SKIP`; `t.Errorf` writes its indented
# `file.go:12: ...` line in between. The closing verdict is matched with a
# leading-space tolerance rather than anchored, because a subtest's verdict is
# indented and a subtest is exactly where a table-driven Go test says which row
# broke.
#
# The INDENTED verdicts that follow a parent's are kept for the same reason. Go
# reports a failing table as `--- FAIL: TestRed` and then one indented line per
# failing row, and those row names are the whole answer to "which case of the
# table did the mutation reach" — the name parser above cannot carry them,
# because it deliberately counts the parent only.
mutation_detail_from_go() {
    awk '
        /^=== RUN/ { buf = $0 "\n"; open = 1; trailing = 0; next }
        open && /^ *--- FAIL: / { printf "%s%s\n", buf, $0; open = 0; trailing = 1; next }
        open && /^ *--- (PASS|SKIP): / { open = 0; next }
        open { buf = buf $0 "\n"; next }
        trailing && /^ +--- (FAIL|PASS|SKIP): / { print; next }
        { trailing = 0 }
    '
}

# pytest's short summary, which it prints for a failing run under `-q`.
#
# The node id alone — `path::test_name` — because the reason follows it after
# a dash and is one line of an assertion that may be many. `ERROR` lines are
# read as well: a fixture or an import that refused is a test that did not
# run, and a mutation that breaks collection is exactly the shape a Python
# round has to be able to name (the machine this suite imports is generated,
# so a mutation upstream of it fails at import rather than at an assertion).
mutation_failures_from_pytest() {
    sed -n -e 's/^FAILED \([^ ]*\).*/\1/p' -e 's/^ERROR \([^ ]*\).*/\1/p'
}

# pytest's own report on the failures it summarised.
#
# The `FAILURES` and `ERRORS` banners open the section where pytest prints the
# source line, the `E` line and the traceback; the `short test summary info`
# banner closes it, and what follows is the one-line-per-failure list the name
# parser above already reads. Both banners are taken because both are verdicts
# about a mutation: an ERROR is a test that could not even be collected, which
# is what an edit upstream of an import looks like.
mutation_detail_from_pytest() {
    awk '
        /^=+ (FAILURES|ERRORS) =+$/ { open = 1; print; next }
        /^=+ short test summary info =+$/ { open = 0; next }
        open { print }
    '
}

# How many failing baseline tests to list under the refusal.
MUTATION_BASELINE_FAILURE_LINES="${MUTATION_BASELINE_FAILURE_LINES:-20}"

# The lines a red BASELINE is reported with, read from the names the run
# already collected.
#
# The same silence this file exists to end, one branch further up. A round
# whose baseline is red stops with `baseline is not green (2 failing)` — a
# COUNT, and a count names no test, so the reader's next question ("which
# two") had no answer in the output. Measured 2026-08-24: the first
# whole-corpus sweep reported exactly that for `ci_lane_gate_selection.cases`
# and the log carried nothing else to act on, while the same suite passed
# locally. Attribution then costs a round-trip through CI, which is the
# workaround `mutation_failures_from_*` was written to remove for the CAUGHT
# path and which the baseline path never got.
#
# Capped, because a baseline that broke wholesale can name hundreds and the
# refusal above it is what the reader must not lose. An empty input says so
# out loud rather than printing nothing: a parser that has drifted off the
# runner's format would otherwise restore the exact silence being repaired,
# and "the runner named none" is a different fact from "there were none".
mutation_baseline_failures() {
    awk -v limit="$MUTATION_BASELINE_FAILURE_LINES" '
        NF { n++; if (n <= limit) print }
        END {
            if (n == 0) {
                print "(the run named no failing test — the parser may have drifted)"
            } else if (n > limit) {
                printf "(+%d more)\n", n - limit
            }
        }
    '
}

# How many lines of the runner's own account to print under those names.
#
# Larger than the name cap by an order of magnitude because the unit is
# different: one name is one line, one failure's account is a panic, a
# traceback or a gtest expectation block. Small enough that a baseline broken
# wholesale still leaves the refusal above it on the screen.
MUTATION_BASELINE_DETAIL_LINES="${MUTATION_BASELINE_DETAIL_LINES:-160}"

# What the runner printed about those failures, capped the same way and for the
# same reason.
#
# The empty case is the one that matters, and it says so out loud. A detail
# parser that drifts off its runner's format produces nothing, which would
# print as an absence of trouble rather than as an absence of reading — the
# precise shape of the defect this repairs. "The runner printed nothing this
# harness could read" is a lead; a blank space is not.
mutation_baseline_detail() {
    awk -v limit="$MUTATION_BASELINE_DETAIL_LINES" '
        { n++; if (n <= limit) print }
        END {
            if (n == 0) {
                print "(the runner printed nothing this harness could read about those"
                print " failures — its detail parser may have drifted off the format)"
            } else if (n > limit) {
                printf "(+%d more line(s) — raise MUTATION_BASELINE_DETAIL_LINES to see them)\n", n - limit
            }
        }
    '
}
