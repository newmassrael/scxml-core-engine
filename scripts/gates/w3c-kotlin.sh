#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The Kotlin/JVM (Rhino) AOT conformance arm: 202 W3C cases plus the suite's
# own fixtures.
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

sce_gate_step "generating and running the Kotlin W3C suite"
status=0
./gradlew --console=plain :sce-kotlin-tests:test >"$LOG/kotlin.log" 2>&1 || status=$?
cat "$LOG/kotlin.log"

if (( status != 0 )); then
    grep -iE 'FAILED|BUILD FAILED|^\s+[A-Za-z0-9_.]+ > ' "$LOG/kotlin.log" | head -n 20 >&2
    sce_gate_fail "Kotlin conformance suite failed"
fi

# Gradle reports BUILD SUCCESSFUL for a `test` task it decided was UP-TO-DATE,
# and for one that compiled and ran nothing. Neither is a conformance result,
# so the verdict is read from the JUnit XML the run produced rather than from
# the build's exit status. The floor is 200 against a current 218.
REPORTS="backends/kotlin/tests/build/test-results/test"
[ -d "$REPORTS" ] \
    || sce_gate_fail "no JUnit results under $REPORTS — the suite reported success without producing a result file"

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
    sce_gate_fail "Kotlin conformance: $failures failure(s), $errors error(s) across $cases case(s)"
fi

if (( cases < 200 )); then
    sce_gate_fail "only $cases Kotlin case(s) ran (expected at least 200) — the suite covered less than its name claims"
fi

# The pin is the reason this gate can exist locally at all, so it is checked
# rather than assumed: a run that leaves the committed tree modified has
# reintroduced the wall-clock stamp, and the next gate would be the one to
# discover it.
dirty="$(git status --porcelain -- backends/kotlin/tests/src | wc -l)"
if (( dirty != 0 )); then
    git status --porcelain -- backends/kotlin/tests/src | head -n 5 >&2
    sce_gate_fail "the suite left $dirty committed Kotlin file(s) modified — SOURCE_DATE_EPOCH is not reaching the generator"
fi

sce_gate_step "$cases Kotlin case(s) passed, committed tree unchanged"
