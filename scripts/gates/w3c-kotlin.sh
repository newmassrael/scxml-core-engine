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

# Both script engines SCE offers for the ECMAScript datamodel on the JVM, not
# just the default.
#
# `EcmaScriptSemanticsTest` already measures all three engines against the
# shared 58-case ECMA-262 table on every run, so expression semantics are not
# what is missing here — that was measured, and it corrects a debt entry that
# said QuickJS had no lane at all. What only the default engine ever saw is
# the other half: the 226 generated machines, and everything an engine does
# for them that an expression table never asks for — session lifecycle,
# `setCurrentEvent`, `executeForeach`, the `In()` state-query callback. A
# defect there is invisible to both the table and a Rhino-only suite.
#
# Lua is deliberately absent. It passes this suite (measured: 230 cases), and
# running it here anyway would assert that SCE offers it for the ECMAScript
# datamodel, which is the opposite of what
# `luaIsNotAnEcmaScriptEngineAndSaysSo` establishes.
KOTLIN_ENGINES=(rhino quickjs)
REPORTS="backends/kotlin/tests/build/test-results/test"

for engine in "${KOTLIN_ENGINES[@]}"; do
    sce_gate_step "generating and running the Kotlin W3C suite on $engine"
    status=0
    ./gradlew --console=plain :sce-kotlin-tests:test "-Psce.script.engine=$engine" \
        >"$LOG/kotlin-$engine.log" 2>&1 || status=$?
    cat "$LOG/kotlin-$engine.log"

    if (( status != 0 )); then
        grep -iE 'FAILED|BUILD FAILED|^\s+[A-Za-z0-9_.]+ > ' "$LOG/kotlin-$engine.log" | head -n 20 >&2
        sce_gate_fail "Kotlin conformance suite failed on $engine"
    fi

    # Gradle reports BUILD SUCCESSFUL for a `test` task it decided was
    # UP-TO-DATE, and for one that compiled and ran nothing. Neither is a
    # conformance result, so the verdict is read from the JUnit XML the run
    # produced rather than from the build's exit status. The floor is 200
    # against a current 226.
    [ -d "$REPORTS" ] \
        || sce_gate_fail "no JUnit results under $REPORTS — the $engine suite reported success without producing a result file"

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
        sce_gate_fail "Kotlin conformance on $engine: $failures failure(s), $errors error(s) across $cases case(s)"
    fi

    if (( cases < 200 )); then
        sce_gate_fail "only $cases Kotlin case(s) ran on $engine (expected at least 200) — the suite covered less than its name claims"
    fi

    sce_gate_step "$cases Kotlin case(s) passed on $engine"
done

# The pin is the reason this gate can exist locally at all, so it is checked
# rather than assumed: a run that rewrites the tree it was handed has
# reintroduced the wall-clock stamp, and the next gate would be the one to
# discover it.
if [[ "$(kotlin_tree_hashes)" != "$TREE_BEFORE" ]]; then
    diff <(printf '%s\n' "$TREE_BEFORE") <(kotlin_tree_hashes) | head -n 10 >&2
    sce_gate_fail "the suite rewrote the Kotlin tree it was handed — SOURCE_DATE_EPOCH is not reaching the generator"
fi

sce_gate_step "committed tree unchanged after ${#KOTLIN_ENGINES[@]} engine(s)"

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
