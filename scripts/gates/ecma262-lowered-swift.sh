#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: swift-backend.yml
#
# The Swift twin of `ecma262-lowered-kotlin`: a Swift artifact whose every expression reaches the
# Lua session as the Lua the build-time frontend lowered, COMPILED, and RUN.
#
# What existed before reached the engine with text a test chose: `EcmaScriptSemanticsTests` hands
# the engine the lowered Lua of a committed table. Neither compiles a machine, and a defect in the
# code generator (a guard evaluated twice, an expression written to the wrong place) is invisible
# to a test that calls an engine entry point and to the W3C suite alike, because no fixture there
# guards on a side effect. Kotlin's lane found exactly that on the day it was built.
#
# THE POPULATION is the shared table (`tests/ecmascript/ecma262_semantics.json`) in full, expanded
# by the same `tools/generate_lowered_ecma262_fixture.py` the C++ and Kotlin lanes use, into one
# machine that asks every case. The expected answers are read from the table and are not in it.
#
# WHAT THERE IS NOT: a source-passing control. Swift has one route into its Lua session, the
# build-time lowering, so what Kotlin reads off two artifacts (and checks are mirror images, so as
# not to compare a subject with itself) is read off one here, and the claim that the artifact is a
# lowered one is made of its own source instead: it must carry a call site per case.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# The toolchain is the one `bx.toml` pins, which is also what the runner installs
# (`scripts/install_swift_toolchain.sh`).
sce_gate_require_swift

# Not a skip: a gate whose whole subject is an artifact it cannot generate has nothing to say when
# the expander is missing, and a lane that skipped here would claim a check that did not run.
command -v python3 >/dev/null 2>&1 \
    || sce_gate_fail "python3 is not on PATH; tools/generate_lowered_ecma262_fixture.py expands the document the artifact is generated from, so there is nothing to measure without it"

LOG="$(mktemp)"
sce_gate_on_exit "rm -f '$LOG'"

# The status of the script, not of the `tee` it is piped to.
set +e
scripts/test_swift_backend.sh ecma262-lowered 2>&1 | tee "$LOG"
status="${PIPESTATUS[0]}"
set -e
[ "$status" -eq 0 ] \
    || sce_gate_fail "scripts/test_swift_backend.sh ecma262-lowered exited $status (the artifact failed to generate, to build or to answer ECMA-262; the output above names it)"

# ── The artifact is a lowered one ──────────────────────────────────
#
# Asked of the EMITTED SOURCE, after the run. The suite alone cannot tell a lowered artifact from a
# machine that asked nothing, and a case it never reached is a finding of its own, but a machine
# generated from an empty or truncated document would still answer every case it carries. A
# document expanded from the shared table writes one `<assign>` per value case, so the real number
# is in the dozens and 50 is a floor no accident clears.
artifact="backends/swift/tests/Sources/Ecma262LoweredMachines/ecma262_lowered_sm.swift"
[ -f "$artifact" ] \
    || sce_gate_fail "$artifact was not emitted: the run above compiled a machine this check cannot find, so nothing here is about the artifact that ran"
sites="$(grep -c 'assignData(' "$artifact" || true)"
sites="${sites:-0}"
if (( sites < 50 )); then
    sce_gate_fail "the artifact carries only $sites lowered <assign> call site(s): the shared table expands to one per value case, so this machine was not generated from it"
fi
sce_gate_step "the artifact carries $sites lowered call site(s)"

# ── The suite's own census, so a GREEN run states what it measured ─
#
# A number that only exists on a red run is a number nobody can cite from a green one, and the run
# can be answered from a cache without producing any output (the Kotlin lane met that with Gradle),
# so the line is demanded and not only looked for.
census="$(sed -n 's/.*\(LoweredEcma262Swift census: .*\)/\1/p' "$LOG" | head -n 1)"
[ -n "$census" ] \
    || sce_gate_fail "the suite printed no census line, so this run cannot say how large a population it asked"
sce_gate_step "$census"

# ── The exclusion list has a ceiling HERE too ──────────────────────
#
# `tests/ecmascript/swift_lowered_artifact_defects.json` is an exclusion list: a case named there is
# not required to answer ECMA-262, which is the one thing in this lane that can make it green while
# the artifact is wrong, so it is held from TWO sources. The suite has `maxDefects` in the same
# file as the assertion that reads it, and raising the constant agrees with you. This ceiling is in
# another language and reads the count off the census the suite PRINTED: the argument for moving
# it has to be made twice.
DEFECT_CEILING=3
excused="$(sed -n 's/.*[[:space:]]codegen-defects=\([0-9]\{1,\}\).*/\1/p' <<<"$census")"
[ -n "$excused" ] \
    || sce_gate_fail "the census names no codegen-defects count, so this run cannot say how many cases the exclusion list excused: an exclusion list nobody counts is a way to make this lane green"
if (( excused > DEFECT_CEILING )); then
    sce_gate_fail "the exclusion list excused $excused case(s), over the ceiling of $DEFECT_CEILING. Every entry in tests/ecmascript/swift_lowered_artifact_defects.json is a code-generation defect someone is expected to REMOVE; a list that grows instead is this lane going green on an artifact that answers the language wrong"
fi
asked="$(sed -n 's/.*[[:space:]]cases=\([0-9]\{1,\}\).*/\1/p' <<<"$census")"
sce_gate_step "the exclusion list excused $excused of ${asked:-?} case(s), within the ceiling of $DEFECT_CEILING"

# ── The probe controls, held HERE and not only by the suite ────────
#
# Every condition verdict rests on a probe: a verdict is refused when the probe says the engine
# could not evaluate the expression. A probe stuck on one answer decides the whole measurement:
# stuck on "refused" makes every condition case a divergence, stuck on "evaluated" makes a genuine
# refusal read as the answer `false`. The suite asserts it distinguishes; what the suite cannot
# assert is that it still ASKS.
refused="$(sed -n 's/.*[[:space:]]lowered-control-refused=\([^[:space:]]*\).*/\1/p' <<<"$census")"
evaluable="$(sed -n 's/.*[[:space:]]lowered-control-evaluable=\([^[:space:]]*\).*/\1/p' <<<"$census")"
[ -n "$refused" ] && [ -n "$evaluable" ] \
    || sce_gate_fail "the census names no probe control: the suite stopped reporting whether the probe every condition verdict rests on can tell a refusal from an answer"
[ "$refused" = "<unevaluated>" ] \
    || sce_gate_fail "the refusal control read '$refused', not the unevaluated sentinel: the probe is not reporting refusals, so a guard the engine would not parse reads as the answer false"
[ "$evaluable" != "<unevaluated>" ] \
    || sce_gate_fail "the evaluable control read the unevaluated sentinel over a literal: the probe is stuck on refusal, so every condition case is a divergence by construction"
sce_gate_step "the refusal probe reported both outcomes, on this run"

sce_gate_step "build-time lowering answers ECMA-262 through a compiled Swift artifact"
