#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: forge-conformance.yml
#
# Kotlin arm of the forge conformance suite; the workflow's job is
# `conformance-kotlin`, and it reaches this script through `scripts/gate`
# exactly as the other five arms do.
#
# ⚠ WHY THIS EXISTS. `forge-conformance.yml` has SIX arms — rust, python, go,
# kotlin, cpp, c — and `scripts/gates/` mirrored FIVE of them. Measured
# 2026-09-18: this file did not exist, and no other gate ran the Kotlin forge
# harness either. `w3c-kotlin` runs `:sce-kotlin-tests:test`, which is the W3C
# suite in a different Gradle project, so its green said nothing about this one.
# A change that broke only the Kotlin forge lowering therefore passed every
# local lane and went red in CI after the push — and in this repository a
# passing local lane is part of what authorises that push, so the gap sat
# exactly where it did the most damage.
#
# This is the shape `forge-c`'s registry note records, with the halves swapped:
# there the code existed and NOTHING ran it; here CI ran it and the local mirror
# did not. Both were invisible for the same reason — the sibling arms made the
# set look complete.
#
# ⚠⚠ JUDGE THE WHOLE SUITE, NOT ONE REPORT. Gradle writes one
# `TEST-<class>.xml` per test class, and this harness does not put everything in
# one: `NumericalConformanceTest` holds the per-fixture cases while each
# `<sce:test-vector>` sidecar lands in its OWN class with its own report.
# Reading one file and comparing its count to another language's module is how a
# green suite looks like it is missing a fixture — measured 2026-09-18,
# `TEST-NumericalConformanceTest.xml` said 129 where the Python harness
# collected 130, and the difference was a sidecar class passing in the file next
# to it. The workflow's own summary step sums the directory for this reason, and
# so does the reporting below.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# ⚠ The JDK Gradle runs on is chosen here, from the version CI installs, and not
# inherited from the machine. Gradle honours `JAVA_HOME` over the `java` on PATH,
# and a build machine's default JDK is whatever its package manager last
# installed: Gradle 8.11.1 with this repository's Kotlin plugin cannot configure
# on JDK 25, and dies in seconds with a message that names no JDK (`* What went
# wrong: 25.0.4`, measured 2026-09-30 on both build machines). `w3c-kotlin` and
# `ecma262-lowered-kotlin` already ask for the guarantee; this arm was the third
# Gradle driver and the one that did not, which is why it took a hand-set
# `JAVA_HOME` on the remote command to run at all.
sce_gate_require_jdk "$SCE_REPO_ROOT/.github/workflows/forge-conformance.yml"

# The Gradle build resolves the generator from target/debug via
# gradle/sce-codegen.gradle.kts, which is what `deps: ["codegen-build"]` in the
# registry guarantees is there.
#
# The queue kind's stress runs (SCE Protocol-Synthesis RFC §synth-5-P) write
# their histories into the directory SCE_QUEUE_HISTORY_DIR names, under
# `target/` and emptied first: a file left from an earlier run would be judged
# in place of one this run did not write. build.gradle.kts makes the test task
# run whenever the variable is set, so the directory cannot stay empty behind
# an UP-TO-DATE.
source "$SCE_REPO_ROOT/scripts/lib/sce_codegen.sh"
QUEUE_HISTORIES="$SCE_REPO_ROOT/target/queue-histories/kotlin"
rm -rf "$QUEUE_HISTORIES"
mkdir -p "$QUEUE_HISTORIES"
SCE_QUEUE_HISTORY_DIR="$QUEUE_HISTORIES" ./gradlew --console=plain :sce-forge-runtime-kotlin:jvmTest \
    || sce_gate_fail "Kotlin forge conformance"

# 4 Lamport capacities recorded 3 times, 6 SCQ shapes recorded 25 times and 5
# segmented shapes recorded 25 times. A run that recorded fewer would pass the
# judgement below, so the count is held. Linearizability is judged by the command
# every backend's histories are judged by, not by a checker of the arm's own.
shopt -s nullglob
queue_histories=("$QUEUE_HISTORIES"/*.json)
shopt -u nullglob
(( ${#queue_histories[@]} >= 287 )) \
    || sce_gate_fail "Kotlin queue histories: ${#queue_histories[@]} written, expected at least 287"
"$(sce_codegen_require "$SCE_REPO_ROOT")" check-queue-history "${queue_histories[@]}" \
    || sce_gate_fail "Kotlin queue histories are not linearizable"

# ⚠ A gate that builds an artifact and does not look at the result is how this
# workflow "came to be mirrored in name only" (forge-go.sh's words). Gradle
# exits non-zero on a failing test, so the check above is the verdict; this
# reports WHAT ran, so a suite that silently stops collecting shows up as a
# number that drops rather than as a green.
REPORTS="backends/kotlin/forge-runtime/build/test-results/jvmTest"
if [ ! -d "$REPORTS" ]; then
    sce_gate_fail "Kotlin forge conformance produced no test-results directory"
fi

total=$(grep -rohE 'tests="[0-9]+"' "$REPORTS" | grep -oE '[0-9]+' | paste -sd+ | bc)
failed=$(grep -rohE 'failures="[0-9]+"' "$REPORTS" | grep -oE '[0-9]+' | paste -sd+ | bc)
errored=$(grep -rohE 'errors="[0-9]+"' "$REPORTS" | grep -oE '[0-9]+' | paste -sd+ | bc)
suites=$(find "$REPORTS" -name 'TEST-*.xml' | wc -l)
echo "Kotlin forge conformance: ${total:-0} test(s) in ${suites} suite(s), ${failed:-0} failed, ${errored:-0} errored"

# An empty sweep reads as a pass everywhere it is not asserted against.
if [ "${total:-0}" -eq 0 ]; then
    sce_gate_fail "Kotlin forge conformance ran zero tests"
fi
