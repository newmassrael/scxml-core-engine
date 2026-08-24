# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Read a refused build's own output and say what refused it.
#
# `scripts/mutate` reports a mutation the compiler rejected as INCONCLUSIVE
# and, until this file existed, said no more than "mutated tree does not
# compile". The author's next question is always the same one — rejected for
# what — and the answer had to be obtained by re-applying the case by hand and
# re-running the build, because the harness sent the build that already knew
# to `/dev/null`.
#
# Measured 2026-08-24 on the three INCONCLUSIVE cases of
# `undecodable_payload_is_reported.cases`: all three were rejected for a reason
# the compiler had printed and this harness had discarded, and each of the
# three was a DIFFERENT reason. Recovering them cost a hand-written script that
# reproduced work the round had already done.
#
# Sourced rather than inlined for the reason `mutation_failures.sh` is: this
# reads a format somebody else owns, so it is the part most likely to drift,
# and it can be exercised against captured output without a mutation round.
#
# Reads stdin and writes the excerpt, at most one diagnostic's worth.

# How many lines of the refusal to quote under a verdict.
#
# A verdict block is two lines today, and a full rustc diagnostic with its
# `help:` and source excerpt runs past ten. Six is the span that carries the
# error line plus the note that usually names the fix, without turning a round
# over eight cases into a wall of compiler output.
MUTATION_BUILD_REFUSAL_LINES="${MUTATION_BUILD_REFUSAL_LINES:-6}"

# The lines of a refused build that name the refusal.
#
# `tail` is the wrong end of the log. A cargo build that fails ends with
# "could not compile `x` (lib) due to 1 previous error", which repeats what the
# verdict already said and names no file, no line and no rule. The line that
# answers the question is the FIRST one the compiler wrote, so the excerpt
# starts there.
#
# Two shapes, because two compilers: rustc writes `error: ` and `error[E0433]: `
# at the start of a line, and a C or C++ compiler writes `path:line:col: error:`
# in the middle of one. Matching both in one pass is what keeps the cargo and
# ctest runners on a single parser rather than two that drift apart.
#
# A log with no diagnostic at all falls back to its tail, because a build can
# also fail for a reason that is not a diagnostic — a missing toolchain, a full
# disk, a linker that ran out of memory — and those print at the end. Falling
# back is deliberate: an excerpt that is empty whenever the parser does not
# recognise the failure would be indistinguishable from the silence this file
# exists to end.
mutation_build_refusal() {
    awk -v limit="$MUTATION_BUILD_REFUSAL_LINES" '
        { line[NR] = $0 }
        !seen && ($0 ~ /^error(\[|:)/ || $0 ~ /: error(\[|:)/) { seen = NR }
        END {
            if (seen) { first = seen; last = seen + limit - 1 }
            else      { first = NR - limit + 1; last = NR }
            if (first < 1) first = 1
            if (last > NR) last = NR
            for (i = first; i <= last; i++) print line[i]
        }
    '
}
