# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Read a test runner's own output and say which tests turned red.
#
# `scripts/mutate` prints a count with every verdict — `CAUGHT (1/287 red)`
# — and a count does not say which assertion earned it. Attribution was
# therefore obtained by re-running the whole harness under
# `GTEST_FILTER=<suite>` until only one suite was left, which is a workaround
# for a fact the runner had already printed and the harness discarded.
#
# Sourced rather than inlined so the two parsers can be exercised against
# captured output without a mutation round: they are the part most likely to
# drift, since both read a format owned by somebody else.
#
# Both read stdin and write one name per line.

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

# libtest's verdict lines out of a cargo test binary.
#
# The per-test line only, not the `failures:` block that repeats the names
# further down — one is a verdict and the other is an index of them, and
# reading both would double every count a caller derived from this.
mutation_failures_from_cargo() {
    sed -n 's/^test \(.*\) \.\.\. FAILED$/\1/p'
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
