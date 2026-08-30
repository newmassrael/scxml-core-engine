#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: ecma262-lowered-kotlin.yml
#
# The gate `docs/SCE_LUA_TRANSLATION_SEAM.md` ended without, on this backend: a
# Kotlin artifact generated with `--script-engine lua`, COMPILED, and RUN.
#
# The C++ half has had `ecma262-lowered-cpp` since 2026-08-29. Kotlin had the
# seam — `ScriptSource.lua`, the lowered arm, a frontend that answers the
# shared table — and no lane that ran an ARTIFACT through it. What existed
# reached the ENGINE with text a test chose: `EcmaScriptSemanticsTest` hands it
# the author's ECMAScript, `LoweredEcma262Test` hands it Lua read out of a
# committed table. Neither compiles a machine.
#
# ⚠ WHY THAT GAP WAS NOT COSMETIC, measured on the day this gate was written:
# the first run of the artifact suite found a defect neither of those two could
# see. The generated Kotlin evaluates a transition's `cond` TWICE — once in
# `processNull<State>()` to choose the target and again in
# `executeTransitionActions` to choose which arm's content to run — so a guard
# with a side effect takes its transition and then records the other arm's
# answer. It is invisible to an engine-level test because no engine is involved
# in it, and invisible to the W3C suite because no fixture there guards on a
# side effect. See `tests/ecmascript/kotlin_lowered_artifact_defects.json`.
#
# WHICH SIDE OF THE SEAM IT MEASURES: build-time lowering, and it is that
# path's CONTRACT rather than a measurement of it.
# `tests/ecmascript/kotlin_lua_divergences.json` says per entry which routes
# into the Lua engine still answer a case differently (`diverges_on`), and this
# gate holds the `build-time-lowering` one in BOTH directions — a case the
# lowered artifact gets wrong without being declared is red, and so is a case
# declared and answered correctly. The second direction is what lets that list
# stay at zero honestly.
#
# THE POPULATION is the shared table (`tests/ecmascript/ecma262_semantics.json`)
# in full, expanded by the same `tools/generate_lowered_ecma262_fixture.py` the
# C++ lane uses. One expander, one population, two backends: a second copy of
# the fixture would be a second population free to fall behind the table.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# The generated artifact is the subject, so the generator is resolved the way
# every other in-tree consumer resolves it rather than from PATH.
CODEGEN="$(sce_gate_codegen)"

# Deliberately NOT `sce_gate_requires_tool`, which offers a SKIP. A gate whose
# whole subject is an artifact it cannot generate has nothing to say when the
# expander is missing, and a lane that skipped here would be claiming a check
# that did not run.
command -v python3 >/dev/null 2>&1 \
    || sce_gate_fail "python3 is not on PATH; tools/generate_lowered_ecma262_fixture.py expands the fixture both artifacts are generated from, so there is nothing to measure without it"

# Same guarantee `w3c-kotlin` makes, from the same helper: Gradle honours
# JAVA_HOME over the `java` on PATH, and a build script compiled on JDK 8 fails
# with a message that names no JDK.
sce_gate_require_jdk "$SCE_REPO_ROOT/.github/workflows/ecma262-lowered-kotlin.yml"

# The emitted machines carry a `generated-at` stamp. Unpinned, every run
# produces new bytes and nothing downstream can be compared.
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-0}"

LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

MODULE="backends/kotlin/lowered-ecma262"
GENERATED="$MODULE/build/lowered-ecma262/generated"

sce_gate_step "building and running the lowered Kotlin artifact and its source-passing control"
status=0
# ⚠ `--rerun`, and it is load-bearing rather than cautious. This gate's verdict
# is read from the run's OUTPUT — the census line below, and the probe controls
# lifted out of it — and Gradle answers an unchanged `test` task UP-TO-DATE or
# FROM-CACHE without producing any. Measured 2026-08-30: the second invocation
# of this gate reported "the suite printed no census line" over an artifact
# that was perfectly fine, because nothing had run.
#
# The deeper reason is the one `w3c-kotlin` carries: a cached green is a
# verdict about inputs, not a run of this turn. For a lane whose whole subject
# is "the artifact executes", that distinction is the lane. `--rerun` scopes to
# the named task, so the generation and compilation above still cache normally.
./gradlew --console=plain :sce-kotlin-lowered-ecma262:test --rerun \
    >"$LOG/gradle.log" 2>&1 || status=$?
cat "$LOG/gradle.log"

if (( status != 0 )); then
    grep -iE 'FAILED|BUILD FAILED|^\s+[A-Za-z0-9_.]+ > ' "$LOG/gradle.log" | head -n 20 >&2
    sce_gate_fail "the lowered Kotlin artifact did not answer ECMA-262 through the build-time lowering"
fi

# ── The artifacts are what the run claims they were ────────────────
#
# Asked of the EMITTED SOURCE, after the run, and this is the assertion that
# keeps the suite above from being two copies of one measurement. Both
# artifacts answer the same 98 cases and — with both divergence arrays empty —
# both answer them the same way, so the suite alone cannot tell a real pair
# from a subject compared against itself. What separates them is how their
# expressions reach the engine, and that is visible in the machines.
#
# A `generate --script-engine lua` that accepted the flag and emitted the
# default anyway would produce a control-shaped subject, and every case in it
# would still pass. The count is the only thing that would notice.
lowered_sm="$GENERATED/ecma262_lowered/ecma262_loweredSm.kt"
source_sm="$GENERATED/ecma262_source/ecma262_sourceSm.kt"
for artifact in "$lowered_sm" "$source_sm"; do
    [ -f "$artifact" ] \
        || sce_gate_fail "$artifact was not emitted — the run above compiled machines this check cannot find, so nothing here is about the artifacts that ran"
done

# ⚠ NOT "the control carries zero". Measured 2026-08-30, that check failed on
# a correct pair: a generated machine emits BOTH arms of the run-time helper
# that re-wraps a `ScriptSource` it was handed (`evaluateSendContent` switches
# on `source.language`), so ONE occurrence of each spelling appears in every
# machine whatever it was generated for. `w3c-kotlin` recorded the same trap
# from the other side, where counting one tree read 159 against 159.
#
# So the relation asserted is the MIRROR, which two selections of one document
# satisfy exactly and nothing else does: what the subject spells `lua` the
# control spells `ecmascript`, one call site for one call site, and the
# helper's other arm is the same single occurrence in both.
count_pairs() { grep -c "ScriptSource.$2(" "$1" || true; }
lowered_lua="$(count_pairs "$lowered_sm" lua)";     lowered_lua="${lowered_lua:-0}"
lowered_ecma="$(count_pairs "$lowered_sm" ecmascript)"; lowered_ecma="${lowered_ecma:-0}"
source_lua="$(count_pairs "$source_sm" lua)";       source_lua="${source_lua:-0}"
source_ecma="$(count_pairs "$source_sm" ecmascript)"; source_ecma="${source_ecma:-0}"

# The floor is what makes the mirror non-vacuous. A selection that never
# reached the templates emits two IDENTICAL machines, and identical machines
# are trivially mirrors of each other — with both counts down at the helper's
# single arm. A document expanded from the shared table carries a call site per
# case, so the real number is in the hundreds and 50 is a floor no accident
# clears.
if (( lowered_lua < 50 )); then
    sce_gate_fail "the lowered artifact carries only $lowered_lua ScriptSource.lua(...) call site(s) — the shared table expands to a call site per case, so this is a machine generated without --script-engine lua and the control below would be compared against itself"
fi
if (( lowered_lua != source_ecma )); then
    sce_gate_fail "the subject carries $lowered_lua ScriptSource.lua(...) call site(s) against the control's $source_ecma ScriptSource.ecmascript(...) — two selections of ONE document must carry one call site for one call site, so these are not the same document"
fi
if (( lowered_ecma != source_lua )); then
    sce_gate_fail "the helper's other arm appears $lowered_ecma time(s) in the subject and $source_lua time(s) in the control. That arm is emitted by the run-time helper and does not vary with the selection, so a difference here means one of these machines carries author expressions in the language the other one is for"
fi
if (( lowered_ecma >= lowered_lua )); then
    sce_gate_fail "the subject carries $lowered_ecma ScriptSource.ecmascript(...) against $lowered_lua ScriptSource.lua(...) — a lowered machine's own language must dominate it, and this one is not lowered"
fi
sce_gate_step "the two artifacts mirror: $lowered_lua lowered call site(s) against $source_ecma source-passing ones, with the helper's other arm once in each"

# ── The control is named, not defaulted ────────────────────────────
#
# `tests/ecmascript/…` and the module's own build file both spell the control's
# language explicitly, and that is load-bearing rather than tidy: the day
# `Language::Kotlin.default_script_engine_target()` flips, an omitted argument
# would silently make the control a second copy of the subject. The manifests
# the generation wrote are what say which language each artifact actually got,
# so they are read rather than the build file re-read.
for stem in ecma262_lowered:lua ecma262_source:ecmascript; do
    name="${stem%%:*}"
    want="${stem##*:}"
    manifest="$GENERATED/$name/manifest.json"
    [ -f "$manifest" ] \
        || sce_gate_fail "$manifest is missing — the generation no longer records what it emitted, so which language each artifact got would be a claim about the build file rather than about the artifact"
    got="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("script_engine_language",""))' "$manifest")"
    [ "$got" = "$want" ] \
        || sce_gate_fail "the $name artifact's manifest reports script_engine_language='$got', not '$want' — the selection this lane is named for did not reach the generator"
done
sce_gate_step "both manifests report the language their artifact was asked for"

# ── The suite's own census, so a GREEN run states what it measured ─
#
# A number that only exists on a red run is a number nobody can cite from a
# green one. `docs/SCE_LUA_TRANSLATION_SEAM.md` re-derives from this line
# rather than from a paragraph someone typed.
census="$(sed -n 's/.*\(LoweredEcma262Kotlin census: .*\)/\1/p' "$LOG/gradle.log" | head -1)"
[ -n "$census" ] \
    || sce_gate_fail "the suite printed no census line, so this run cannot say how large a population it asked"
sce_gate_step "$census"

# ── The exclusion list has a ceiling HERE too ──────────────────────
#
# `tests/ecmascript/kotlin_lowered_artifact_defects.json` is an exclusion list:
# a case named there is not required to answer ECMA-262. That is the one thing
# in this lane that can make it green while the artifact is wrong, so it is
# held from TWO sources rather than one.
#
# The suite has `MAX_DEFECTS`, and that constant lives in the same file as the
# assertion that reads it — raise the constant and the assertion agrees with
# you. This ceiling is in a different language, reads the count off the census
# the suite PRINTED, and is mutated by
# `sce-build/tests/mutations/ecma262_lowered_kotlin.cases`. Neither implies the
# other: the suite's ceiling counts entries in the FILE, this one counts
# entries that actually excused a case in the population.
#
# The tighter of the two wins, which is the safe direction for a ceiling. If
# they ever disagree, the number to move is the one someone can argue for —
# and the argument has to be made twice, in two languages, which is the point.
DEFECT_CEILING=3
excused="$(sed -n 's/.*[[:space:]]codegen-defects=\([0-9]\{1,\}\).*/\1/p' <<<"$census")"
[ -n "$excused" ] \
    || sce_gate_fail "the census names no codegen-defects count, so this run cannot say how many cases the exclusion list excused — an exclusion list nobody counts is a way to make this lane green"
if (( excused > DEFECT_CEILING )); then
    sce_gate_fail "the exclusion list excused $excused case(s), over the ceiling of $DEFECT_CEILING. Every entry in tests/ecmascript/kotlin_lowered_artifact_defects.json is a code-generation defect someone is expected to REMOVE; a list that grows instead is this lane going green on an artifact that answers the language wrong"
fi
asked="$(sed -n 's/.*[[:space:]]cases=\([0-9]\{1,\}\).*/\1/p' <<<"$census")"
sce_gate_step "the exclusion list excused $excused of ${asked:-?} case(s), within the ceiling of $DEFECT_CEILING"

# The probe controls, held HERE and not only by the suite's own assertions.
#
# Every condition verdict rests on a probe: a verdict is refused when the probe
# says the engine could not evaluate the expression. A probe stuck on one
# answer therefore decides the whole measurement — stuck on "refused" makes
# every condition case a divergence, stuck on "evaluated" makes a genuine
# §scxml-5.9.1 refusal read as the answer `false`. The suite asserts it
# distinguishes; what the suite cannot assert is that it still ASKS.
for artifact in lowered source; do
    refused="$(sed -n "s/.*[[:space:]]${artifact}-control-refused=\([^[:space:]]*\).*/\1/p" <<<"$census")"
    evaluable="$(sed -n "s/.*[[:space:]]${artifact}-control-evaluable=\([^[:space:]]*\).*/\1/p" <<<"$census")"
    [ -n "$refused" ] && [ -n "$evaluable" ] \
        || sce_gate_fail "the census names no probe control for the $artifact artifact — the suite stopped reporting whether the probe every condition verdict rests on can tell a refusal from an answer"
    [ "$refused" = "<unevaluated>" ] \
        || sce_gate_fail "the $artifact artifact's refusal control read '$refused', not the unevaluated sentinel — the probe is not reporting §scxml-5.9.1 refusals, so a guard the engine would not parse reads as the answer false"
    [ "$evaluable" != "<unevaluated>" ] \
        || sce_gate_fail "the $artifact artifact's evaluable control read the unevaluated sentinel over a literal — the probe is stuck on refusal, so every condition case is a divergence by construction"
done
sce_gate_step "the refusal probe reported both outcomes on both artifacts, on this run"

sce_gate_step "build-time lowering answers ECMA-262 through a compiled Kotlin artifact"
