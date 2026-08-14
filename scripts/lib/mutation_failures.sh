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
