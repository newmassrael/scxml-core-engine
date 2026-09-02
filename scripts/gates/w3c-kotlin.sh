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

# Which JVM Gradle will actually run on, from the shared helper.
#
# Two gates drive Gradle now — this one and `ecma262-lowered-kotlin` — and the
# ~50 lines this call replaced were about to become two copies of one answer to
# "which java is this". They live in `lib.sh` with the measurement that
# produced them: a build machine whose `/etc/environment` pointed `JAVA_HOME`
# at a JDK 8 while `java --version` said 17.
#
# The floor is READ from the workflow this gate mirrors, so a mirror cannot
# measure a different JVM than the lane it mirrors.
sce_gate_require_jdk "$SCE_REPO_ROOT/.github/workflows/w3c-tests.yml"

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
# ⚠ Since 2026-08-31 the names are no longer all there is, because a name in a
# comment could not say whether either class RAN. The only coverage assertion
# this gate carried was a total over a floor of 200 on a suite of 373, so 173
# cases — these two among them — could have stopped running inside a green
# run. The class population is now DERIVED from the suite's sources and
# compared against every row's JUnit report in both directions, by
# `scripts/gates/kotlin_coverage.py`, which carries the reasoning for both
# halves. These two names are covered by that set now, not by this paragraph —
# and `sce-build/tests/kotlin_coverage_verdict.rs` asks the derivation for them
# by name, so the coverage is a case rather than this sentence.
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

# ── What this backend still cannot lower ───────────────────────────
#
# `EcmaScriptToLuaTransformer` WAS the fallback behind every lowering entry
# point in `LuaScriptEngine`: text `sce-build`'s frontend refused was rewritten
# rather than refused. The seam document said so in prose since the frontend
# landed and NOTHING COUNTED IT, so this block was born as a ratchet — a
# ceiling over the fallback's uses, driven down toward `kotlin-retire-rewriter`.
#
# ⚠ THE CEILING RETIRED WITH ITS SUBJECT, on 2026-08-30, and replacing it was
# not optional. With the fallback deleted, `rewriter=0` is a STRUCTURAL zero:
# nothing in this tree can raise it and nothing can lower it, so holding it
# under a ceiling would be a gate that cannot fail — the exact shape this
# repository keeps paying for.
#
# ⚠⚠ What those four call sites became is a REFUSAL (§scxml-5.9.1), which is a
# real event with a text behind it. It cannot be a ceiling either, because some
# refusals are the specification working: a suite asking for `foo` with nothing
# named `foo` gets exactly the `error.execution` the clause requires. So the
# census is held by CLASSIFICATION rather than by count — every refused text
# matches a declared entry in the file below, in both directions — and an
# undeclared refusal is red. That is the ratchet now: the file goes DOWN as the
# frontend learns shapes and callers re-tag, and the entries that remain are
# the ones carrying a clause that says they should.
REFUSALS_JSON="tests/ecmascript/kotlin_frontend_refusals.json"

# ⚠ And the two populations a declaration's PRODUCERS are resolved against.
#
# Every entry in the file above carries a sentence of the shape "THIS ENTRY
# DOES NOT LEAVE while test307 is registered". Nothing re-read those sentences
# until 2026-08-30, which makes them exactly the kind this repository has
# already watched rot twice — a per-call figure quoted from a deleted probe,
# and a lane size lifted out of a neighbouring measurement. So each entry names
# its producers and the reader below RESOLVES every name here: a run of digits
# against the conformance registry, anything else against the Kotlin test
# sources. A name that is neither is red rather than skipped, which is what
# makes retiring test307 take that entry with it.
FIXTURES_JSON="tests/w3c/conformance/fixtures.json"
KOTLIN_TEST_SOURCES="backends/kotlin/tests/src/test/kotlin"

# ⚠⚠⚠ AND A FLOOR UNDER THE FRONTEND, which is not decoration. Measured
# 2026-08-30, twice: a probe that wrote to `System.err` reported ZERO
# fallbacks over a run that in fact took the fallback 100 times, because
# Gradle swallows the test JVM's stderr. A census that never arrives and a
# census of a run with nothing to report BOTH read zero, so the low number
# alone cannot tell "we are done" from "nobody measured". The frontend's own
# successes are what separates them: on this suite it answers tens of thousands
# of times, so anything near zero means the census did not happen.
FRONTEND_FLOOR=1000
CENSUS="$LOG/lowering-census.tsv"
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

# ── The shipped host defaults must accept the default artifact ─────
#
# ⚠ The rows above all NAME their engine, so none of them can see this. What
# they cannot cover is the run nobody configures: a host that constructs the
# engine this tree ships as its default and is handed a machine generated with
# no `--script-engine`. That pairing is the mis-supply `ScriptSource` exists to
# prevent, it fails at RUN TIME in the host's own process, and until this block
# existed the only thing asserting it was a sentence in a KDoc.
#
# The check is derived on both sides. `COMMITTED_LANGUAGE` is the manifest's
# answer, above. Which languages an engine accepts is not restated here either:
# `KOTLIN_ENGINE_PAIRS` IS that relation — `GateEnginePairsTest` derives the
# population from `ScxmlScriptEngine.acceptsLanguage` and fails on a missing
# row — so "this engine accepts that language" is exactly "that pair is in the
# array".
#
# Each site is read with a narrow parse that must match EXACTLY ONE line, the
# same discipline `GateEnginePairsTest` uses reading this file from the other
# direction. A site that stops being greppable is a refusal, not a skip: a
# silent zero here would be a lane reporting that it checked nothing.
kotlin_default_engine_of() {
    local label="$1" file="$2" pattern="$3" hits
    [ -f "$file" ] \
        || sce_gate_fail "$file is missing, so the $label default engine cannot be read. This gate pairs every shipped default against the language the committed machines carry; a site it cannot read is unchecked, not absent."
    hits="$(grep -cE "$pattern" "$file" || true)"
    [ "$hits" = "1" ] \
        || sce_gate_fail "expected exactly one line matching \`$pattern\` in $file, found $hits. The $label default engine is what a host gets when it configures nothing, and a parse that matches none — or two — is a check that reports on the wrong line."
    grep -oE "$pattern" "$file" | head -1
}

# `W3CTestBase.DEFAULT_ENGINE` — what a `./gradlew test` with no
# `-Psce.script.engine` hands the committed machines.
_suite_default="$(
    kotlin_default_engine_of "conformance suite" \
        "backends/kotlin/tests/src/test/kotlin/com/sce/w3c/W3CTestBase.kt" \
        'const val DEFAULT_ENGINE: String = "[a-z0-9]+"' \
    | sed -E 's/.*"([a-z0-9]+)"/\1/'
)"

# The Spring starter's `@ConditionalOnMissingBean` engine — what an application
# that adds the starter and declares no bean of its own gets.
_starter_default="$(
    kotlin_default_engine_of "Spring starter" \
        "backends/kotlin/spring-boot-starter/src/main/kotlin/com/sce/spring/SceAutoConfiguration.kt" \
        'open fun scxmlScriptEngine\(\): ScxmlScriptEngine = [A-Za-z]+ScriptEngine\(\)' \
    | sed -E 's/.*= ([A-Za-z]+)ScriptEngine\(\)/\1/' \
    | tr '[:upper:]' '[:lower:]'
)"
# `QuickJSScriptEngine` lowercases to `quickjs`, which is the wire spelling
# already; `Rhino` and `Lua` do too. A class whose name does not reduce to a
# known engine is caught by the membership test below rather than mapped here.

for _default in "conformance suite:$_suite_default" "Spring starter:$_starter_default"; do
    _site="${_default%%:*}"
    _engine="${_default##*:}"
    [ -n "$_engine" ] \
        || sce_gate_fail "the $_site default engine parsed to an empty name, so the pairing below would be asserted about nothing."
    _accepted=0
    for pair in "${KOTLIN_ENGINE_PAIRS[@]}"; do
        if [ "$pair" = "$_engine:$COMMITTED_LANGUAGE" ]; then
            _accepted=1
        fi
    done
    (( _accepted == 1 )) \
        || sce_gate_fail "the $_site ships \`$_engine\` as its default engine and the generator's default artifact is emitted for \`$COMMITTED_LANGUAGE\`, but \`$_engine:$COMMITTED_LANGUAGE\` is not a row of KOTLIN_ENGINE_PAIRS — which is the relation \`acceptsLanguage\` answers, held by GateEnginePairsTest. A host that configures nothing would be handed a machine its engine refuses, at run time, in its own process. Move the default engine to one that takes $COMMITTED_LANGUAGE, or move \`Language::Kotlin.default_script_engine_target()\` back."
    sce_gate_step "the $_site default engine ($_engine) accepts the language the default artifact carries ($COMMITTED_LANGUAGE)"
done

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

        # ── The integration stems, which `generate-w3c` does not emit ─────
        #
        # `generate-w3c` covers `com/sce/generated` and stops there. The other
        # 41 committed machine directories are integration stems, emitted by
        # per-stem regen scripts, and they follow the DEFAULT script-engine
        # language — so on a row whose language is not the committed one they
        # are the mis-supply the check below refuses.
        #
        # ⚠ TWO halves, and both are derived rather than listed.
        # `generate-integration` enumerates `integration_resources/`, which is
        # 34 of them; the rest keep their fixture somewhere else
        # (`examples/ai_loop/ai_loop.scxml`, a `--host-processor` flag the
        # fan-out cannot carry) and are reachable only as their own scripts.
        # Naming those would be a list to keep current, so the split is asked
        # of the tree: a script whose stem has no `integration_resources/`
        # directory is one the fan-out did not drive.
        #
        # ⚠⚠ And the producers are found by WHAT THEY WRITE, not by what they
        # are called. Measured 2026-08-30: `regen_native_action.sh` emits the
        # Rust, Go AND Kotlin trees from one script, so a glob over
        # `regen_*_kotlin.sh` misses `com/sce/integration/statechart_native_action`
        # — which would leave exactly one directory in the committed language
        # and hang this suite for the same reason the whole block exists.
        local kt_root="$tree/backends/kotlin/tests/src/main/kotlin"
        "$(sce_gate_codegen)" generate-integration -l kotlin \
            --script-engine "$language" --output-dir "$kt_root" >/dev/null \
            || sce_gate_fail "generating the Kotlin integration stems for --script-engine $language"

        local producer stem
        for producer in scripts/regen_*.sh; do
            grep -q 'backends/kotlin/tests/src/main/kotlin}\?/com/sce/integration' "$producer" \
                || continue
            stem="$(basename "$producer" .sh)"; stem="${stem#regen_}"; stem="${stem%_kotlin}"
            [ -d "integration_resources/$stem" ] && continue
            SCE_SCRIPT_ENGINE="$language" SCE_SCRIPT_ENGINE_FOR=kotlin \
            SCE_KOTLIN_GENERATED_ROOT="$kt_root" \
                bash "$producer" >/dev/null \
                || sce_gate_fail "$producer failed emitting its Kotlin machines for --script-engine $language"
        done

        # ── Every source that CARRIES a language must be IN the overlay ────
        #
        # ⚠ Measured 2026-08-30, from a run that did not fail — it HUNG.
        # `sce.generated.overlay` replaces `com/sce/generated` and nothing
        # else, so the 42 committed integration stems stayed in
        # `$COMMITTED_LANGUAGE` and were compiled beside the overlay and handed
        # to an engine that does not read that language. Rhino answered every
        # `setCurrentEvent` with an `EcmaError` out of `JSON.parse`, the
        # eventless drain never settled, and ONE ForkJoin worker burned 1047
        # CPU-seconds of 1056 elapsed inside it. The case that exists to stop a
        # macrostep that cannot end — `EventlessMacrostepIsBounded` — fails
        # under the same mismatch, so nothing stopped it, and the CI job has no
        # `timeout-minutes` to end it either: it ran for over an hour before a
        # person looked.
        #
        # ⚠⚠ A mis-supply is therefore NOT reliably a red. That is the whole
        # reason this is a precondition rather than something the suite's own
        # result is trusted to show: a suite that never returns reports
        # nothing at all, and a lane waiting on it looks exactly like a lane
        # doing work.
        #
        # The population is DERIVED rather than listed. A file that constructs
        # a `ScriptSource` hands its engine text in a NAMED language and can be
        # mis-supplied; a file that constructs none cannot. `com/sce/http` and
        # `com/sce/interpreter` hold one support file each and construct none,
        # so they leave this set on their own — no exemption names them, and
        # nothing has to be remembered when a fifth directory arrives.
        local committed_root="backends/kotlin/tests/src/main/kotlin"
        local overlay_root="$tree/$committed_root"
        local replaced overlay_files committed_in_replaced uncovered
        local carriers carrier_count stray name

        # WHICH populations the build will replace, asked of the overlay
        # exactly as `backends/kotlin/tests/build.gradle.kts` asks it: it adds
        # each `com/sce/<name>` the overlay carries as a source directory and
        # excludes the committed `com/sce/<name>/**` wholesale.
        replaced="$(cd "$overlay_root/com/sce" 2>/dev/null \
            && find . -mindepth 1 -maxdepth 1 -type d | sed 's|^\./||' | sort || true)"
        [ -n "$replaced" ] \
            || sce_gate_fail "the $language overlay carries no package directory under $overlay_root/com/sce, so the build would replace nothing and compile the committed machines while this row claims to measure $language"

        overlay_files="$(cd "$overlay_root" 2>/dev/null && find . -name '*.kt' \
            | sed 's|^\./||' | sort -u || true)"

        # ── The hand-authored files inside a replaced population ──────────
        #
        # A replaced directory is excluded WHOLESALE, and not everything in one
        # is generated. `com/sce/integration/package-info.kt` says so in its own
        # KDoc — *"the only hand-authored file under `com.sce.integration`"* —
        # and no generator emits it, so no amount of regeneration would put it
        # back. It also cannot vary with the script-engine language, having no
        # machine in it, so carrying the committed copy across is exact rather
        # than approximate.
        #
        # ⚠ The discriminator is this repository's own `// Source:` marker, and
        # it is what keeps this from becoming a hole: a GENERATED file missing
        # from the overlay is a producer that did not run, and copying it here
        # would paper over exactly the defect the check below exists to find.
        # Only files without the marker move.
        local handwritten
        handwritten="$(
            for name in $replaced; do
                find "$committed_root/com/sce/$name" -name '*.kt' \
                    -exec grep -L '^// Source:' {} \; 2>/dev/null
            done | sed "s|^$committed_root/||" | sort -u
        )"
        while IFS= read -r hand; do
            [ -n "$hand" ] || continue
            [ -e "$overlay_root/$hand" ] && continue
            mkdir -p "$overlay_root/$(dirname "$hand")"
            cp "$committed_root/$hand" "$overlay_root/$hand" \
                || sce_gate_fail "could not carry the hand-authored $hand into the $language overlay"
        done <<<"$handwritten"

        # ⚠ Re-read, because the copies above changed it.
        overlay_files="$(cd "$overlay_root" 2>/dev/null && find . -name '*.kt' \
            | sed 's|^\./||' | sort -u || true)"

        # (1) EVERY committed file in a replaced population must be in the
        # overlay — not only the ones that carry a language.
        #
        # ⚠ Measured 2026-08-30, and this check was WRONG the first time. It
        # compared only the sources constructing a `ScriptSource`, passed with
        # "covers all 200", and the build then failed to compile: the exclusion
        # is per-DIRECTORY, so `statechart_bytes` and
        # `statechart_delayed_host_send` — two stems that carry no script at
        # all — were excluded with their neighbours and the overlay had no copy
        # to put back. A predicate narrower than the exclusion it guards is a
        # predicate that passes for the wrong reason.
        committed_in_replaced="$(
            for name in $replaced; do
                find "$committed_root/com/sce/$name" -name '*.kt' 2>/dev/null
            done | sed "s|^$committed_root/||" | sort -u
        )"
        uncovered="$(comm -23 \
            <(printf '%s\n' "$committed_in_replaced" | grep -v '^$') \
            <(printf '%s\n' "$overlay_files" | grep -v '^$'))"
        if [ -n "$uncovered" ]; then
            printf '%s\n' "committed sources the $language overlay excludes without replacing:" >&2
            printf '%s\n' "$uncovered" | head -n 10 >&2
            sce_gate_fail "the $language overlay replaces $(printf '%s\n' "$replaced" | tr '\n' ' ')but does not carry $(printf '%s\n' "$uncovered" | wc -l) committed file(s) inside them. The build excludes those directories wholesale, so each missing file is an unresolved reference at compile time. Generate those stems for $language too"
        fi

        # (2) And every source that CARRIES a language must be inside a
        # replaced population, or it survives the exclusion in the committed
        # language and is handed to this row's engine anyway — which is the
        # mis-supply that HANGS rather than fails.
        carriers="$(grep -rlE 'ScriptSource\.(lua|ecmascript)\(' "$committed_root" \
            --include='*.kt' 2>/dev/null | sed "s|^$committed_root/||" | sort -u || true)"
        carrier_count="$(printf '%s\n' "$carriers" | grep -c . || true)"

        # ⚠⚠ The floor, and it is the half a reader deletes as redundant. If
        # the pattern above ever stops matching, check (2) compares an EMPTY
        # set and passes — reporting full coverage for a tree it could not
        # read. Measured 2026-08-30: 200 committed sources construct a
        # `ScriptSource` (159 under `generated`, 41 under `integration`), so a
        # number near zero means the reading failed rather than that nothing
        # carries a language.
        local CARRIER_FLOOR=100
        if (( carrier_count < CARRIER_FLOOR )); then
            sce_gate_fail "only $carrier_count committed Kotlin source(s) were read as carrying a script-engine language, under the floor of $CARRIER_FLOOR. This tree emits a \`ScriptSource\` in every machine that has a script, so a number this low means the reading failed — and a reading that failed reports every overlay as complete"
        fi

        stray=""
        while IFS= read -r carrier; do
            [ -n "$carrier" ] || continue
            case "$carrier" in
                com/sce/*) name="${carrier#com/sce/}"; name="${name%%/*}" ;;
                *) name="" ;;
            esac
            printf '%s\n' "$replaced" | grep -qx "$name" || stray="$stray$carrier"$'\n'
        done <<<"$carriers"
        if [ -n "$stray" ]; then
            printf '%s\n' "committed sources that carry a language and are outside every replaced population:" >&2
            printf '%s\n' "$stray" | head -n 10 >&2
            sce_gate_fail "$(printf '%s\n' "$stray" | grep -c .) committed source(s) carry a script-engine language and sit outside the directories this $language overlay replaces, so they compile as $COMMITTED_LANGUAGE and are handed to the $language row's engine. That mis-supply does not fail this suite, it HANGS it — a machine whose events the engine cannot read never settles its eventless macrostep"
        fi

        sce_gate_step "the $language overlay replaces $(printf '%s\n' "$replaced" | tr '\n' ' ')and carries every committed file inside them, including all $carrier_count that carry a language"
    fi
    KOTLIN_TREE_FOR_LANGUAGE="$tree"
}

# ── Which classes must have RUN, derived from the suite's own sources ──
#
# ⚠⚠⚠ Until 2026-08-31 the only coverage assertion below was a TOTAL held
# over a floor of 200, on a suite of 373. A hundred and seventy-three cases
# could stop running with this gate green — and two of them are named in this
# file's own header: `SendParamPayloadTest` (W3C SCXML 6.2, a repeated
# `<param>` name) and `XmlDataIsADomTreeTest` (W3C SCXML B.2, a `<data>`
# element's XML arriving as a document), the pair the Lua engine failed on
# 2026-08-29 and passes since `sce-build`'s frontend was linked into it. That
# header calls those names "what a reader can act on", and they are — but a
# name in a comment is not a lane. This file has already paid for that shape
# twice: an array reading `(rhino quickjs)` with a paragraph of prose
# explaining the third engine's absence, and a "230 cases" total stale by 131.
# Measured 2026-08-31 on `-Psce.script.engine=lua`: both classes run and both
# pass — 373 cases, 0 failures, 0 skipped. Nothing here would have said so had
# they not.
#
# So the population is DERIVED from the test sources and compared with each
# row's report in BOTH directions. It is not a list to keep and it has no
# exemption table: a class this reader cannot account for is a difference, and
# a difference is red.
#
# The derivation is a FIXPOINT rather than a grep for `@Test`, and both it and
# the per-row comparison are `scripts/gates/kotlin_coverage.py`, which carries
# the reasoning for each.
#
# ⚠⚠ THEY LEFT THIS FILE on 2026-09-02, and the move is the point rather than
# tidiness. What they decide — an empty arm is not a green row — could until
# then only be measured BY HAND, because their input is a JUnit report
# directory that does not exist until this gate has run Gradle, so no runner in
# the corpus could reach them here. As a program they take a directory an
# ordinary test can build, and `sce-build/tests/kotlin_coverage_verdict.rs`
# builds one: the empty arm is a case now, not a sentence.
#
# ⚠⚠⚠ Derived ONCE for every row, which is checked rather than assumed: the
# overlay replaces `src/main/kotlin/com/sce` only, so the JUnit classes under
# `src/test` are the committed ones on every row, and the tree-hash comparison
# at the bottom of this gate fails if any row rewrote `src`.
KOTLIN_COVERAGE="$SCE_REPO_ROOT/scripts/gates/kotlin_coverage.py"
KOTLIN_RUNNABLE="$LOG/runnable-classes.txt"

# The floor under the DERIVATION — not under any report — lives with the
# derivation, in `kotlin_coverage.py`. It refuses rather than printing a short
# list, so a reader that parsed nothing cannot hand the rows below an empty set
# for them all to match.
python3 "$KOTLIN_COVERAGE" derive "$KOTLIN_TEST_SOURCES" >"$KOTLIN_RUNNABLE" \
    || sce_gate_fail "the set of test classes that must run could not be derived from $KOTLIN_TEST_SOURCES — the refusal is above. Every row below compares its report against that set, so without it there is nothing to compare against"

runnable_count="$(grep -c . "$KOTLIN_RUNNABLE" || true)"
sce_gate_step "$runnable_count runnable test class(es) derived from $KOTLIN_TEST_SOURCES; every row below must report exactly them"

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
        "-Psce.lua.loweringCensus=$CENSUS" \
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
    # produced rather than from the build's exit status.
    #
    # ⚠ MEASURED on 2026-09-02, not reasoned. This row was made to run nothing
    # — an `--init-script` narrowing the test filter to a class that does not
    # exist — and Gradle printed `BUILD SUCCESSFUL in 15s` over an executed
    # `:sce-kotlin-tests:test`, leaving `$REPORTS` present and holding no
    # `TEST-*.xml` at all. What refused that arm is the comparison against the
    # derived class set, which named all 251 — in that state every total is 0,
    # and a total of 0 says that nothing ran without saying which claims went
    # unmade.
    #
    # ⚠⚠ `$REPORTS` is shared by all four rows and is NOT emptied here, which
    # would make a class that stopped running in row 2 invisible behind row 1's
    # leftover file. That it does not is measured rather than assumed: a probe
    # file placed in that directory on 2026-08-31 was gone after the next
    # invocation, so Gradle's Test task clears its own result directory before
    # it writes. Were that to change, the leftovers of a row that ran MORE
    # classes would surface as an unaccounted class on the row that ran fewer.
    cases="$(python3 "$KOTLIN_COVERAGE" verdict \
        --reports "$REPORTS" \
        --runnable "$KOTLIN_RUNNABLE" \
        --label "$engine over $language machines")" \
        || sce_gate_fail "Kotlin conformance on $engine over $language machines: the row's coverage verdict refused — its diagnosis is above"

    sce_gate_step "$cases Kotlin case(s) passed on $engine over $language machines, across exactly the $runnable_count class(es) its sources declare"
done

# ── The lowering census, read from the file the suite wrote ────────
#
# Held in BOTH directions, for the reason the block above states: a missing
# census and a run with nothing to report are the same small number, so the
# frontend's successes are asserted as well as the refusals.
[ -f "$CENSUS" ] \
    || sce_gate_fail "no lowering census at $CENSUS — the suite ran without recording which half answered each expression, so this run cannot say what the Kotlin backend refused. The property is forwarded by backends/kotlin/tests/build.gradle.kts; a Test task that stopped forwarding it would look exactly like a backend that refused nothing"

frontend_hits="$(grep -c '^frontend' "$CENSUS" || true)"; frontend_hits="${frontend_hits:-0}"
refused_hits="$(grep -c '^refused' "$CENSUS" || true)"; refused_hits="${refused_hits:-0}"

if (( frontend_hits < FRONTEND_FLOOR )); then
    sce_gate_fail "the census records only $frontend_hits frontend lowering(s), under the floor of $FRONTEND_FLOOR. This suite lowers tens of thousands of expressions, so a number this low means the census did not happen rather than that the frontend was not used — and a census that did not happen reports zero refusals too"
fi

# Every refusal is CLASSIFIED, in both directions.
#
# ⚠ A ceiling cannot outlive the retirement it was measuring. This block held
# `rewriter_hits` under `REWRITER_CEILING` while the fallback stood; the
# fallback is deleted, so that count is structurally zero — no change to this
# tree can move it — and a predicate nothing can move is not a measurement.
#
# ⚠⚠ What the fallback's four call sites became is a REFUSAL, and refusals
# cannot be held under a ceiling either: §scxml-5.9.1 makes some of them the
# specification working as designed (a suite asking for `foo` with nothing
# named `foo` is asking for exactly that), and "how many" cannot tell those
# from a gap in the frontend. WHICH ONES can. So each refused text is matched
# against a declared entry, and an undeclared refusal is RED rather than
# counted — which is the rule an exemption list has to obey to be one.
[ -f "$REFUSALS_JSON" ] \
    || sce_gate_fail "the declared refusals are missing at $REFUSALS_JSON. Every refusal the census records is matched against that file in both directions, so without it this gate measures nothing"

# ⚠ Every declaration carries its JUSTIFICATION, not just its text.
#
# Measured 2026-08-30: the first form of this block read `entry['text']` and
# nothing else, so `{"text": "…"}` with no kind, no clause and no reason was a
# valid declaration — an exemption list that silences the very check it exists
# to arm. Proved by running that reader against exactly such an entry: it
# printed the text and exited 0.
#
# The reader below has NO default arm. A kind it does not know is not "other",
# it is unclassified, and unclassified is the state this file exists to make
# red.
#
# It also reduces each kind to a DISPOSITION, which is what lets the ratchet
# below be honest. `specification` and `control` refusals have no path to zero
# — the clause requires them — so a ceiling over the whole list would be a
# predicate whose zero is unreachable by construction, which is the shape the
# retired `REWRITER_CEILING` had on the day its subject was deleted.
#
# ⚠⚠ And every declaration RESOLVES its producers, which is the half that
# stops a `why` from being a story. Each entry ends by promising it stays "while
# test307 is registered"; nothing re-read that promise until the reader below
# did, and this repository has already paid twice for a quotable sentence that
# nobody re-measured. Digits resolve against the conformance registry and
# anything else against the Kotlin test sources — a name that is neither is
# unclassified, and unclassified is red rather than skipped.
declared_rows="$(python3 - "$REFUSALS_JSON" "$FIXTURES_JSON" "$KOTLIN_TEST_SOURCES" <<'PY'
import json
import pathlib
import sys

DISPOSITION = {
    "specification": "stays",
    "control": "stays",
    "frontend-gap": "leaves",
    "caller-mistag": "leaves",
}

doc = json.load(open(sys.argv[1], encoding="utf-8"))
entries = doc["refusals"]

# The two populations a producer name is resolved against. Neither is a
# spelling: the registry is what `generate-w3c` and the CMake registration both
# read, and the class set is the files that actually exist, so an entry cannot
# outlive the case it names.
fixtures = {
    fixture["id"]
    for fixture in json.load(open(sys.argv[2], encoding="utf-8"))["fixtures"]
}
classes = {path.stem for path in pathlib.Path(sys.argv[3]).rglob("*.kt")}

faults = []
for index, entry in enumerate(entries):
    text = entry.get("text")
    if not isinstance(text, str) or not text:
        faults.append(f"entry {index} carries no `text`")
        continue
    kind = entry.get("kind")
    if kind not in DISPOSITION:
        faults.append(
            f"{text!r} carries kind {kind!r}, which is not one of "
            + ", ".join(sorted(DISPOSITION))
        )
    if not entry.get("clause"):
        faults.append(f"{text!r} cites no `clause`")
    if not entry.get("why"):
        faults.append(f"{text!r} gives no `why`")

    producers = entry.get("produced_by")
    if not isinstance(producers, list) or not producers:
        faults.append(
            f"{text!r} names no `produced_by`. An entry that says nothing "
            f"reaches it cannot be checked against the tree at all"
        )
        continue
    for name in producers:
        if not isinstance(name, str) or not name:
            faults.append(f"{text!r} names an empty producer")
        elif name.isdigit():
            if name not in fixtures:
                faults.append(
                    f"{text!r} names W3C fixture {name!r}, which "
                    f"{sys.argv[2]} does not register"
                )
        elif name not in classes:
            faults.append(
                f"{text!r} names producer {name!r}, which is neither a "
                f"registered W3C fixture id nor a test class under "
                f"{sys.argv[3]}"
            )

if faults:
    for fault in faults:
        print(fault, file=sys.stderr)
    sys.exit(1)

for entry in entries:
    print(f"{DISPOSITION[entry['kind']]}\t{entry['text']}")
PY
)" || sce_gate_fail "$REFUSALS_JSON does not declare its refusals in full. Every entry carries a kind from the closed set, the clause it rests on, why it is a refusal rather than a gap, and the producers that reach it — each of which must resolve to a fixture $FIXTURES_JSON registers or a test class under $KOTLIN_TEST_SOURCES. An entry missing any of them is a text somebody added to quiet this lane rather than a refusal anybody classified"

declared_refusals="$(printf '%s\n' "$declared_rows" | cut -f2-)"
observed_refusals="$(grep '^refused' "$CENSUS" | cut -f3- | sort -u || true)"

undeclared="$(comm -23 \
    <(printf '%s\n' "$observed_refusals" | grep -v '^$' | sort -u) \
    <(printf '%s\n' "$declared_refusals" | grep -v '^$' | sort -u))"
if [ -n "$undeclared" ]; then
    printf '%s\n' "the texts this run refused without a declaration:" >&2
    printf '%s\n' "$undeclared" >&2
    sce_gate_fail "the Kotlin Lua engine refused $(printf '%s\n' "$undeclared" | wc -l) text(s) that $REFUSALS_JSON does not declare. Each is either an ECMAScript shape the frontend should learn, Lua a caller tagged as ECMAScript and should re-tag, or a refusal §scxml-5.9.1 wants — and only the last belongs in that file, carrying the clause that makes it one. Unclassified is RED: a refusal nobody classified is a gap nobody looked at"
fi

unseen="$(comm -13 \
    <(printf '%s\n' "$observed_refusals" | grep -v '^$' | sort -u) \
    <(printf '%s\n' "$declared_refusals" | grep -v '^$' | sort -u))"
if [ -n "$unseen" ]; then
    printf '%s\n' "declared refusals this run never produced:" >&2
    printf '%s\n' "$unseen" >&2
    sce_gate_fail "$REFUSALS_JSON declares $(printf '%s\n' "$unseen" | wc -l) refusal(s) this run did not produce. Remove them — a list that keeps an entry nothing reaches cannot be trusted in the other direction either, and each one it keeps is a case a reader believes is still exercised"
fi

# ⚠⚠ THE RATCHET, and it is over the half that can actually reach zero.
#
# A `leaves` entry is a shape the frontend should learn or a caller that should
# re-tag: both have a repair, and the file itself says adding one is a claim to
# argue in `docs/SCE_LUA_TRANSLATION_SEAM.md` rather than a way to make this
# lane green. So the ceiling is ZERO and it ratchets, exactly as the count it
# replaced did.
#
# ⚠⚠⚠ Why not the whole list. `specification` and `control` refusals cannot
# leave — §scxml-5.9.1 requires the first and a deliberate mis-tag measures the
# second — so a ceiling over every entry would mix in roles whose zero is
# unreachable by construction, and a predicate that cannot reach its own target
# measures nobody's progress. Splitting by disposition is what makes the zero
# both true today and losable tomorrow.
REFUSAL_GAP_CEILING=0
leaving="$(printf '%s\n' "$declared_rows" | grep '^leaves' | cut -f2- || true)"
leaving_count="$(printf '%s\n' "$leaving" | grep -c . || true)"; leaving_count="${leaving_count:-0}"
if (( leaving_count > REFUSAL_GAP_CEILING )); then
    printf '%s\n' "declared refusals that are gaps rather than the specification working:" >&2
    printf '%s\n' "$leaving" >&2
    sce_gate_fail "$REFUSALS_JSON declares $leaving_count refusal(s) of a kind that HAS a repair — a shape the frontend should learn, or a caller handing Lua through the ECMAScript door — over the ceiling of $REFUSAL_GAP_CEILING. Recording one honestly is not a way to make this lane green: fix the frontend or re-tag the caller, and argue the entry in docs/SCE_LUA_TRANSLATION_SEAM.md if it must stay"
fi

sce_gate_step "KotlinLowering census: frontend=$frontend_hits refused=$refused_hits, every refusal declared in $REFUSALS_JSON, $leaving_count of them a gap (ceiling $REFUSAL_GAP_CEILING)"

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
