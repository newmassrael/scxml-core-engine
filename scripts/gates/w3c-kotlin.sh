#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The Kotlin/JVM AOT conformance arm: 202 W3C cases plus the suite's own
# fixtures, run on every script engine SCE offers for the ECMAScript
# datamodel on the JVM.
#
# This lane was CI-only until now, and the reason was mechanical rather than
# principled: `generateScxml` invoked the generator without SOURCE_DATE_EPOCH,
# so simply running the suite rewrote the `generated-at` header of all 449
# committed Kotlin files. A developer could not run it without dirtying the
# tree, so nobody did, and the Kotlin backend's only check lived in a lane
# that — until the same round that added this gate — could not report a
# failure at all. The pin now lives in `backends/kotlin/tests/build.gradle.kts`
# where every caller of the task gets it; exporting it here as well means the
# gate does not depend on that file staying correct to keep the tree clean.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# Same reasoning as `w3c-go`: not skip-capable. The lane obtains its JDK
# through `actions/setup-java` rather than a package install, and this gate is
# selected only when the Kotlin backend changed — a skip there would be
# silence about the exact change that asked for the check.
command -v java >/dev/null 2>&1 \
    || sce_gate_fail "java is not on PATH, and this gate was selected because the Kotlin backend changed. Install a JDK 17+ (apt install openjdk-17-jdk) — skipping here would report green on an unverified backend."

# ── The JDK Gradle will ACTUALLY run on ───────────────────────────
#
# The check above answers whether a JVM exists. It does not answer which one
# Gradle uses, and those are different questions: Gradle honours `JAVA_HOME`
# over the `java` on PATH, and nothing keeps the two in step.
#
# Measured 2026-08-24 on a build machine whose `/etc/environment` carried
# `JAVA_HOME=…java-8-openjdk…` for an unrelated toolchain: `java --version`
# said 17, `update-alternatives` said 17, and Gradle compiled the build
# scripts on 8 — failing on a `ByteArrayOutputStream.toString(Charset)`
# overload that Java 10 added. Nothing in that error named a JDK, and the gate
# had no way to say "you are on the wrong one".
#
# CI never meets this because `actions/setup-java` exports `JAVA_HOME`. So the
# floor is read from the version CI pins, and this makes the same guarantee
# here rather than inheriting whatever the machine happens to export.
#
# An adequate `JAVA_HOME` is left alone — overriding a deliberate choice is
# not this gate's business. `SCE_JAVA_HOME` names one explicitly for a layout
# the search below does not know.
JDK_PIN_FILE="$SCE_REPO_ROOT/.github/workflows/w3c-tests.yml"
JDK_FLOOR="$(sed -n "s/^[[:space:]]*java-version:[[:space:]]*['\"]\{0,1\}\([0-9]\{1,\}\).*/\1/p" \
    "$JDK_PIN_FILE" | head -1)"
[ -n "$JDK_FLOOR" ] \
    || sce_gate_fail "no \`java-version:\` pin found in $JDK_PIN_FILE — this gate derives its JDK floor from the version CI installs, and a missing pin would let it run on any JVM."

# The major version a JDK home reports. `$1` empty means the `java` on PATH.
# Java 8 spells itself `1.8.0_x` and everything since spells `<major>.x`, so
# the leading `1.` is dropped before the first component is read.
sce_java_major() {
    local home="${1:-}" out version
    out="$("${home:+$home/bin/}java" -version 2>&1 | head -1)" || return 1
    version="${out#*\"}"
    version="${version%%\"*}"
    case "$version" in
    1.*) version="${version#1.}" ;;
    esac
    printf '%s' "${version%%.*}"
}

# An explicitly set, adequate `JAVA_HOME` is respected; anything else is
# chosen here. "Anything else" includes the common case of NO `JAVA_HOME` at
# all, where Gradle would take whatever `java` the PATH happens to offer — on
# two of the three machines in this fleet that is 21, and this gate's own
# header calls itself a mirror of `w3c-tests.yml`, which installs 17. A mirror
# that measures a different JVM than the lane it mirrors is not one.
if [ -z "${JAVA_HOME:-}" ] || [ "$(sce_java_major "${JAVA_HOME:-}" || echo 0)" -lt "$JDK_FLOOR" ]; then
    # The pinned major first, then anything at or above the floor: a machine
    # that carries several JDKs should land on the one CI uses, not merely on
    # one that compiles.
    for _candidate in ${SCE_JAVA_HOME:+"$SCE_JAVA_HOME"} \
        "/usr/lib/jvm/java-$JDK_FLOOR-openjdk-"* /usr/lib/jvm/java-*-openjdk-*; do
        # `javac`, not `java`: a JRE runs the tests but cannot compile them,
        # and one of the fleet's "JDK 21" directories turned out to be exactly
        # that — a JRE whose `bin/` holds four files and no compiler.
        [ -x "$_candidate/bin/javac" ] || continue
        [ "$(sce_java_major "$_candidate" || echo 0)" -ge "$JDK_FLOOR" ] || continue
        export JAVA_HOME="$_candidate"
        break
    done
fi

_jdk_now="$(sce_java_major "${JAVA_HOME:-}" || echo 0)"
[ "$_jdk_now" -ge "$JDK_FLOOR" ] \
    || sce_gate_fail "Gradle would run on JDK $_jdk_now, and this suite needs $JDK_FLOOR+ (the version .github/workflows/w3c-tests.yml installs). JAVA_HOME=${JAVA_HOME:-<unset>}. Install a JDK $JDK_FLOOR, or point SCE_JAVA_HOME at one — running on an older JVM fails inside a build script with a message that names no JDK."
sce_gate_step "Gradle will run on JDK $_jdk_now (floor $JDK_FLOOR from ${JDK_PIN_FILE##*/})"

export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-0}"

LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

# What this gate can honestly claim about the timestamp pin is that THIS RUN
# left the tree as it found it. Comparing against HEAD answers a different
# question — whether the working tree is clean — and an author mid-round
# always fails that one: adding an integration stem brings new generated
# files, and the gate reported them as proof the pin was broken. So the
# baseline is the tree as it stands now, hashed, and the comparison after the
# run is against that.
kotlin_tree_hashes() {
    find backends/kotlin/tests/src -type f -print0 2>/dev/null \
        | sort -z | xargs -0 -r sha256sum
}
TREE_BEFORE="$(kotlin_tree_hashes)"

# Every script engine SCE offers for the ECMAScript datamodel on the JVM, not
# just the default.
#
# `EcmaScriptSemanticsTest` already measures all three engines against the
# shared ECMA-262 table (`tests/ecmascript/ecma262_semantics.json`) on every
# run, so expression semantics are not what is missing here — that was
# measured, and it corrects a debt entry that said QuickJS had no lane at all.
#
# The table's SIZE is deliberately not restated here. This comment said
# "58-case" until 2026-08-29, by which time the table held 98: a count in prose
# is the one thing nothing re-answers, which is the reason the divergence lists
# beside that table exist at all, and the same repair the Lua engine's own KDoc
# took. What only the default engine ever saw is
# the other half: the 226 generated machines, and everything an engine does
# for them that an expression table never asks for — session lifecycle,
# `setCurrentEvent`, `executeForeach`, the `In()` state-query callback. A
# defect there is invisible to both the table and a Rhino-only suite.
#
# ⚠ Lua WAS deliberately absent, on the reasoning that running it here would
# assert SCE offers it for the ECMAScript datamodel — the opposite of what the
# suite's own case then established. That premise was retired on 2026-08-30
# together with the case it named: `sce-build`'s ECMAScript frontend is linked
# into this backend's Lua engine, `tests/ecmascript/kotlin_lua_divergences.json`
# holds nothing, and the case is now
# `theLuaEngineDivergesExactlyWhereItIsDeclaredTo`. Running the selection here
# asserts what is measured rather than what was hoped.
#
# ⚠⚠ Running Lua here does NOT assert that Lua is an ECMAScript engine. The
# claim each row makes is per-pair and bounded: the generated machines,
# emitted for the LANGUAGE named, run correctly on the ENGINE named.
#
# ⚠ What this comment ALSO said until 2026-08-29 — "It passes this suite
# (measured: 230 cases)" — was false. Run that day with
# `./gradlew :sce-kotlin-tests:test -Psce.script.engine=lua`, TWO cases failed:
# `SendParamPayloadTest` (W3C SCXML 6.2, a repeated `<param>` name loses one of
# its values) and `XmlDataIsADomTreeTest` (W3C SCXML B.2, a `<data>` element's
# XML does not arrive as a document).
#
# ⚠⚠ BOTH CLOSED 2026-08-30, and not by anybody fixing them. `sce-build`'s
# ECMAScript frontend is now linked into this backend's Lua engine, and the
# same suite on the same selection is 371 of 371. The A/B is in
# `docs/SCE_LUA_TRANSLATION_SEAM.md`'s eleventh round: neutering the frontend
# path brings back exactly those two, beside the 44 expression divergences —
# which is what makes the attribution the frontend rather than a coincidence.
#
# No total is written here on purpose. The one that was ("230 cases") was stale
# by 131, and the replacement drafted the same morning ("361 tests") was stale
# by six before the day ended — a suite gains cases, which is what a healthy
# one does. The FAILING CASES are named instead: those two names are what a
# reader can act on, and a name does not rot the way a denominator does.
#
# ── What the rows are, and why they are PAIRS ─────────────────────
#
# `engine:language`. The engine is what evaluates; the language is what the
# generated machines were emitted for. They were one field while every engine
# read one language, and the MECHANICAL blocker this gate carried as a debt
# until 2026-08-30 was exactly that conflation: the committed Kotlin tree is
# emitted for ONE language, so an engine reading the other one had nothing to
# be handed. `generate-w3c --script-engine` and `-Psce.generated.overlay` are
# the two halves that lift it — the suite now generates per language and the
# committed tree is only used by the engines whose language it is.
#
# ⚠⚠⚠ This is also what keeps the gate honest THROUGH the day the backend's
# default flips. `Language::Kotlin.default_script_engine_target()` moving to
# Lua re-emits the committed tree as lowered Lua, and rows pinned to "the
# committed tree" would then hand Rhino and QuickJS a language they refuse —
# 226 failures each, reported as a conformance regression rather than as the
# artifact swap it is. Rows naming their language keep meaning the same thing
# across that flip, and it is `COMMITTED_LANGUAGE` below, derived from the
# tree, that follows the flip instead of a constant that would not.
#
# The POPULATION is not this array's to choose. Every language an engine
# accepts is a route into it that something must run, and
# `GateEnginePairsTest` fails when a row is missing — it reads this array and
# asks each engine, through `acceptsLanguage`, which languages it takes. An
# engine added to `W3CTestBase.KNOWN_ENGINES`, or an existing one that gains
# an adapter, therefore turns this array red rather than being silently
# unmeasured.
KOTLIN_ENGINE_PAIRS=(rhino:ecmascript quickjs:ecmascript lua:ecmascript lua:lua)
REPORTS="backends/kotlin/tests/build/test-results/test"

# ── Which language the committed machines are emitted for ─────────
#
# Asked of the GENERATOR, never declared here. This backend emits for two
# languages and the committed tree holds one of them; writing down which would
# be a constant that is correct until the day it matters most — the day the
# default flips — and silently wrong after it.
#
# ⚠ The first attempt asked the TREE instead, by counting files carrying
# `ScriptSource.lua(` against `ScriptSource.ecmascript(`. Measured 2026-08-30,
# that reads 159 against 159 on the SAME tree, and would have failed as
# "mixed": a generated machine emits BOTH arms of the run-time helper that
# re-wraps a `ScriptSource` it was handed (`evaluateSendContent` switches on
# `source.language`), so both spellings appear in every machine that has one.
# What varies with the selection is the CALL SITE that carries the author's
# expression, and telling those apart by grep is a parse of Kotlin. The
# manifest already answers the question exactly.
#
# The step from "the generator's default" to "what the committed tree holds"
# is not assumed either: `generateScxml` regenerates that tree in place on
# every row whose language is this one, and the tree-hash comparison at the
# bottom of this gate fails if the result differs from what is committed. So
# the committed tree IS the default-target output, checked on this run, not
# claimed.
# The committed machines. Named once: the overlay replaces exactly this
# directory, and the "did the selection reach the templates" check below
# compares against exactly it.
COMMITTED_MACHINES="backends/kotlin/tests/src/main/kotlin/com/sce/generated"
[ -d "$COMMITTED_MACHINES" ] \
    || sce_gate_fail "$COMMITTED_MACHINES is missing — there are no committed Kotlin machines for the rows below to run or to compare a generated tree against."

PROBE_DOC="resources/150/test150.scxml"
[ -f "$PROBE_DOC" ] \
    || sce_gate_fail "$PROBE_DOC is missing. It is the document this gate generates to ask the manifest which script-engine language this backend emits for by default; without an answer, no row below can tell whether it needs its own tree."
_probe_dir="$LOG/language-probe"
mkdir -p "$_probe_dir"
COMMITTED_LANGUAGE="$(
    "$(sce_gate_codegen)" generate "$PROBE_DOC" -o "$_probe_dir" -l kotlin 2>/dev/null \
        | python3 -c 'import json,sys; print(json.load(sys.stdin).get("script_engine_language",""))'
)"
[ -n "$COMMITTED_LANGUAGE" ] \
    || sce_gate_fail "the manifest for $PROBE_DOC carries no \`script_engine_language\`, so the language the committed machines are emitted for is unknown. That field is present only when the document needs a script engine — if this document stopped needing one, point PROBE_DOC at one that does rather than defaulting, because every row below reads this to decide whether it must generate its own tree."
sce_gate_step "the generator emits $COMMITTED_LANGUAGE by default, so that is what the committed machines hold"

# At least one row must run against the committed tree, and not for tidiness:
# the in-place regeneration those rows perform is what the tree-hash check at
# the bottom compares, and it is that comparison — not this script — that ties
# `COMMITTED_LANGUAGE` to the files in git. Every row overlaying its own tree
# would leave the committed one compiled by nothing and regenerated by
# nothing, with the gate still green.
_committed_rows=0
for pair in "${KOTLIN_ENGINE_PAIRS[@]}"; do
    # `if`, not `[ … ] && …`. Under `set -e` the second form makes the loop's
    # status that of its LAST iteration, so a final row whose language is not
    # the committed one would end this script — silently, with no gate message
    # — exactly when the array is arranged as it is today.
    if [ "${pair##*:}" = "$COMMITTED_LANGUAGE" ]; then
        _committed_rows=$((_committed_rows + 1))
    fi
done
(( _committed_rows > 0 )) \
    || sce_gate_fail "no row in KOTLIN_ENGINE_PAIRS names the committed language ($COMMITTED_LANGUAGE), so nothing regenerates the committed tree in place and the tree-hash check below would compare it against itself untouched. Add the pair for whichever engine reads $COMMITTED_LANGUAGE."

# The generated tree for a language the committed one is not, produced once
# and reused by every row that names it. The path lands in
# `KOTLIN_TREE_FOR_LANGUAGE` rather than on stdout, because every refusal
# below calls `sce_gate_fail` and `sce_gate_fail` is an `exit`: called from
# inside a `$( … )` it would end the SUBSHELL, and the gate would carry on
# past a tree it had just refused to accept.
KOTLIN_TREE_FOR_LANGUAGE=""
kotlin_tree_for_language() {
    local language="$1" tree="$LOG/tree-$1"
    if [ ! -d "$tree" ]; then
        "$(sce_gate_codegen)" generate-w3c -l kotlin --script-engine "$language" \
            --output-dir "$tree" >/dev/null \
            || sce_gate_fail "generating the Kotlin W3C machines for --script-engine $language"

        # The overlay replaces the MACHINES and keeps the committed JUnit
        # classes, which is only sound while those classes do not vary with
        # the language. They do not today — the engine language reaches a test
        # class through the machine it constructs and nowhere else — but that
        # is a property of the templates rather than a law, so it is compared
        # rather than assumed. A test class that starts varying makes this
        # fail here instead of running the wrong pairing quietly.
        local emitted committed name
        for emitted in "$tree/backends/kotlin/tests/src/test/kotlin/com/sce/w3c/"Test*.kt; do
            [ -e "$emitted" ] || sce_gate_fail "the $language generation emitted no JUnit classes under $tree"
            name="$(basename "$emitted")"
            committed="backends/kotlin/tests/src/test/kotlin/com/sce/w3c/$name"
            cmp -s "$emitted" "$committed" \
                || sce_gate_fail "the generated JUnit class $name differs between the $COMMITTED_LANGUAGE and $language artifacts, so overlaying only the machines would run the committed test class against machines it was not generated with. Widen the overlay to the test sources, or keep the classes language-independent."
        done

        # ⚠ The row is only worth its minutes while the selection REACHED the
        # templates. A `generate-w3c` that accepted `--script-engine` and
        # generated its default anyway would produce machines byte-identical to
        # the committed ones — and this gate would then hand the Lua engine
        # ECMAScript under the name `lua:lua`, which the engine accepts through
        # its adapter and passes. A green row measuring the route it was
        # written to stop measuring is worse than no row.
        #
        # Whole-tree rather than per-file: which machines differ depends on
        # which documents carry expressions, and that is the registry's
        # business, not this check's. What must be true is that SOMETHING did.
        local machines="$tree/backends/kotlin/tests/src/main/kotlin/com/sce/generated"
        [ -d "$machines" ] \
            || sce_gate_fail "the $language generation wrote no machines under $machines"
        if diff -rq "$COMMITTED_MACHINES" "$machines" >/dev/null 2>&1; then
            sce_gate_fail "\`generate-w3c --script-engine $language\` produced machines identical to the committed $COMMITTED_LANGUAGE ones, so the selection did not reach the templates. Running this pair would hand the $language engine $COMMITTED_LANGUAGE and report it as $language coverage."
        fi
    fi
    KOTLIN_TREE_FOR_LANGUAGE="$tree"
}

for pair in "${KOTLIN_ENGINE_PAIRS[@]}"; do
    engine="${pair%%:*}"
    language="${pair##*:}"
    overlay=()
    if [ "$language" = "$COMMITTED_LANGUAGE" ]; then
        sce_gate_step "running the Kotlin W3C suite on $engine over the committed $language machines"
    else
        sce_gate_step "generating $language machines and running the Kotlin W3C suite on $engine over them"
        kotlin_tree_for_language "$language"
        overlay=(-Psce.generated.overlay="$KOTLIN_TREE_FOR_LANGUAGE/backends/kotlin/tests")
    fi

    status=0
    ./gradlew --console=plain :sce-kotlin-tests:test "-Psce.script.engine=$engine" \
        "${overlay[@]}" \
        >"$LOG/kotlin-$engine-$language.log" 2>&1 || status=$?
    cat "$LOG/kotlin-$engine-$language.log"

    if (( status != 0 )); then
        grep -iE 'FAILED|BUILD FAILED|^\s+[A-Za-z0-9_.]+ > ' "$LOG/kotlin-$engine-$language.log" | head -n 20 >&2
        sce_gate_fail "Kotlin conformance suite failed on $engine over $language machines"
    fi

    # Gradle reports BUILD SUCCESSFUL for a `test` task it decided was
    # UP-TO-DATE, and for one that compiled and ran nothing. Neither is a
    # conformance result, so the verdict is read from the JUnit XML the run
    # produced rather than from the build's exit status. The floor is 200
    # against a current 226.
    [ -d "$REPORTS" ] \
        || sce_gate_fail "no JUnit results under $REPORTS — the $engine/$language run reported success without producing a result file"

    read -r cases failures errors < <(
        python3 - "$REPORTS" <<'PY'
import glob, re, sys, xml.etree.ElementTree as ET
t = f = e = 0
for p in glob.glob(sys.argv[1] + "/*.xml"):
    r = ET.parse(p).getroot()
    t += int(r.get("tests", 0)); f += int(r.get("failures", 0)); e += int(r.get("errors", 0))
print(t, f, e)
PY
    )

    if (( failures != 0 || errors != 0 )); then
        sce_gate_fail "Kotlin conformance on $engine over $language machines: $failures failure(s), $errors error(s) across $cases case(s)"
    fi

    if (( cases < 200 )); then
        sce_gate_fail "only $cases Kotlin case(s) ran on $engine over $language machines (expected at least 200) — the suite covered less than its name claims"
    fi

    sce_gate_step "$cases Kotlin case(s) passed on $engine over $language machines"
done

# The pin is the reason this gate can exist locally at all, so it is checked
# rather than assumed: a run that rewrites the tree it was handed has
# reintroduced the wall-clock stamp, and the next gate would be the one to
# discover it.
if [[ "$(kotlin_tree_hashes)" != "$TREE_BEFORE" ]]; then
    diff <(printf '%s\n' "$TREE_BEFORE") <(kotlin_tree_hashes) | head -n 10 >&2
    sce_gate_fail "the suite rewrote the Kotlin tree it was handed — SOURCE_DATE_EPOCH is not reaching the generator"
fi

sce_gate_step "committed tree unchanged after ${#KOTLIN_ENGINE_PAIRS[@]} engine/language pair(s)"

# The same generator, asked for a project of the caller's own.
#
# `--output-dir` used to emit Kotlin fixed to this repository's package names —
# `com.sce.generated`, `com.sce.w3c` — with no build files and none of the
# hand-authored classes the generated tests extend. The SCE Kotlin projects are
# not published anywhere a consumer could resolve them from, so the emitted
# settings reach them as a composite build; whether that actually resolves is
# not something a structural check can answer, which is why this runs Gradle.
#
# One fixture, not 202: the claim is about packaging, and the composite build,
# the package rewrite and the three shipped classes are all exercised by one.
sce_gate_step "building an emitted Kotlin suite under a package root of its own"
SUITE="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$SUITE'"

"$(sce_gate_codegen)" generate-w3c -l kotlin \
    --output-dir "$SUITE" \
    --suite-package com.scegate.conformance \
    -t 144 >/dev/null \
    || sce_gate_fail "emitting a standalone Kotlin suite"

suite_status=0
./gradlew -p "$SUITE/backends/kotlin/tests" test --console=plain \
    >"$LOG/gradle-suite.log" 2>&1 || suite_status=$?
if (( suite_status != 0 )); then
    tail -n 40 "$LOG/gradle-suite.log" >&2
    sce_gate_fail "an emitted Kotlin suite must build and pass from its own tree — that is what --output-dir claims"
fi

# Same reason as the floor above: BUILD SUCCESSFUL covers a test task that
# compiled and ran nothing, and here that is one missing line rather than 202.
SUITE_REPORTS="$SUITE/backends/kotlin/tests/build/test-results/test"
[ -d "$SUITE_REPORTS" ] \
    || sce_gate_fail "the emitted Kotlin suite reported success without producing a result file"

read -r suite_cases suite_failures suite_errors < <(
    python3 - "$SUITE_REPORTS" <<'PY'
import glob, sys, xml.etree.ElementTree as ET
t = f = e = 0
for p in glob.glob(sys.argv[1] + "/*.xml"):
    r = ET.parse(p).getroot()
    t += int(r.get("tests", 0)); f += int(r.get("failures", 0)); e += int(r.get("errors", 0))
print(t, f, e)
PY
)
if (( suite_cases < 1 || suite_failures != 0 || suite_errors != 0 )); then
    sce_gate_fail "the emitted Kotlin suite ran $suite_cases case(s) with $suite_failures failure(s) and $suite_errors error(s)"
fi

sce_gate_step "the emitted Kotlin suite built and passed $suite_cases case(s)"
