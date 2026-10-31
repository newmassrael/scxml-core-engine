#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/swift/tests/Sources/W3cMachines/ and the test file beside it
# (backends/swift/tests/Tests/W3cMachinesTests/W3cTests.swift) from the W3C documents of
# `tests/w3c/conformance/fixtures.json` that the Swift backend lowers.
#
# The Swift twin of Go's `generate-w3c -l go`: the suite is not committed, since a template
# edit would then rewrite a file per document, so it is written here and the lane
# (scripts/test_swift_backend.sh) regenerates it before it runs it, which also makes the lane
# judge the generator in the working tree and not an artifact somebody produced earlier.
#
# Only the `simple` harness for now (run the machine to completion and assert it settles in its
# `pass` state, or its `final` state when it has no `pass`): `scheduled` needs a clock the test
# advances and `http` the echo endpoint.
#
# Derived, not listed. Every `simple` fixture is asked of the generator, and one it lowers joins
# the suite by itself, so a construct Swift gains the lowering of is covered the day it lands. A
# refusal by name is exit status 7 (generate/unsupported-feature) and is REPORTED, one line per
# reason with its count, since what stands between Swift and the rest of the corpus is what the
# next slice reads. Any other failure is a defect and stops the run.
#
# Usage (from repo root):
#   scripts/regen_swift_w3c.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
MANIFEST="tests/w3c/conformance/fixtures.json"
GENERATED_DIR="backends/swift/tests/Sources/W3cMachines"
TEST_FILE="backends/swift/tests/Tests/W3cMachinesTests/W3cTests.swift"

# The ids of the fixtures of the `simple` harness, asked of the generator as every other
# backend's build asks, and not read out of the JSON again here.
mapfile -t IDS < <("$CODEGEN" list-fixtures --manifest "$MANIFEST" --catalog w3c --harness simple)
if [ "${#IDS[@]}" -lt 1 ]; then
    echo "error: no simple fixture in $MANIFEST" >&2
    exit 1
fi

# The state a document settles in, by the rule the W3C suites of the other backends apply
# (`detect_pass_state` in sce-codegen): a final state named `pass`, else one named `final` (W3C 415
# is a manual test whose machine halts in its initial final state and has no `pass`). The id is
# printed as the document spells it, which is what `getStateName` answers. A document with neither
# is expected to settle in `pass`, so that it fails visibly and the rule is extended to it.
#
# This is the rule's one copy outside the generator; the day `generate-w3c` serves Swift it comes
# from there.
expected_state_of() {
    local finals found
    finals="$(grep -o '<final[^>]* id="[^"]*"' "$1" | sed 's/.* id="//; s/"$//')"
    found="$(printf '%s\n' "$finals" | grep -ix 'pass' | head -n 1 || true)"
    if [ -z "$found" ]; then
        found="$(printf '%s\n' "$finals" | grep -ix 'final' | head -n 1 || true)"
    fi
    printf '%s\n' "${found:-pass}"
}

# The document of fixture `403a` is resources/403/test403a.scxml: the directory is the number
# and the file keeps the letter.
document_of() {
    local id="$1" number
    number="$(printf '%s' "$id" | tr -cd '0-9')"
    printf 'resources/%s/test%s.scxml\n' "$number" "$id"
}

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

LOWERED=()
REASONS_FILE="$TMP/reasons"
: > "$REASONS_FILE"
for id in "${IDS[@]}"; do
    source="$(document_of "$id")"
    if [ ! -f "$source" ]; then
        echo "error: fixture $id names $source, which does not exist" >&2
        exit 1
    fi
    status=0
    verdict="$("$CODEGEN" generate "$source" -l swift --error-format=json -o "$TMP/$id/" 2>&1)" || status=$?
    case "$status" in
        0)
            LOWERED+=("$id")
            ;;
        7)
            printf '%s\n' "$verdict" | grep -o '"message":"[^"]*"' | head -n 1 | cut -c12-170 >> "$REASONS_FILE"
            ;;
        *)
            echo "error: sce-codegen generate -l swift $source failed with status $status" >&2
            printf '%s\n' "$verdict" >&2
            exit 1
            ;;
    esac
done
if [ "${#LOWERED[@]}" -lt 1 ]; then
    echo "error: the Swift backend lowers no simple W3C fixture" >&2
    exit 1
fi
# A floor, not an equality: a construct Swift gains lowers more fixtures without touching this
# number, but a document the generator stopped lowering must not read as a smaller suite that
# passed. Raise it when a slice lands; measured 2026-10-31 at 62 of 157.
FLOOR=101
if [ "${#LOWERED[@]}" -lt "$FLOOR" ]; then
    echo "error: the Swift backend lowers ${#LOWERED[@]} simple W3C fixture(s), under the floor of $FLOOR: a construct it lowered is refused again" >&2
    exit 1
fi

mkdir -p "$GENERATED_DIR" "$(dirname "$TEST_FILE")"
# Every file below is generated and nothing else lives here: clearing by what each file says it
# is takes the first and would leave a hand-written file alone.
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear --recursive "$GENERATED_DIR"

{
    printf '// SCE-GENERATED -- DO NOT EDIT\n'
    printf '// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial\n'
    printf '// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael\n'
    printf '//\n'
    printf '// Written by scripts/regen_swift_w3c.sh: one test per W3C document of the `simple` harness that\n'
    printf '// the Swift backend lowers. The machine runs to completion and must settle in its `pass` state,\n'
    printf '// or in its `final` state for a manual test that has no `pass` (W3C 415).\n\n'
    printf 'import SceRuntime\nimport Testing\nimport W3cMachines\n\n'
    printf '// The run every other backend gives a `simple` fixture: start the machine and let it tick, for\n'
    printf '// at most three seconds, until it reaches a final state. A machine that starts a child session\n'
    printf '// or sends to itself is only finished by the ticks.\n'
    printf 'private func settled<Policy: StatePolicy>(_ policy: Policy) -> (engine: Engine<Policy>, completed: Bool) {\n'
    printf '    let engine = Engine(policy: policy)\n'
    printf '    engine.initialize()\n'
    printf '    let completed = engine.runUntilCompletion(timeout: .seconds(3), pollInterval: .milliseconds(10))\n'
    printf '    return (engine, completed)\n}\n'
} > "$TEST_FILE"

# A fixture the backend lowers and that cannot settle yet, with the reason it cannot. It is run
# under `withKnownIssue`, so it is held both ways: it still has to fail, and the day it passes the
# suite says so and the entry goes. An entry whose fixture is no longer lowered is an error, since
# nothing would run it.
declare -A KNOWN_GAPS=(
    [561]="a receiver builds no DOM from an XML payload: SceLua has no DOM binding (§scxml-B-2-8-1, the first rung)"
)
for gap in "${!KNOWN_GAPS[@]}"; do
    case " ${LOWERED[*]} " in
        *" $gap "*) ;;
        *)
            echo "error: the recorded gap $gap names a fixture the Swift backend no longer lowers" >&2
            exit 1
            ;;
    esac
done

for id in "${LOWERED[@]}"; do
    # A document that starts a child session is written as its own machine and one more for each
    # child it starts (`test220__sce_synth_invoke__invoke_0_sm.swift`), all of which the target
    # compiles; the one under test is the machine of the document.
    machine="$TMP/$id/test${id}_sm.swift"
    if [ ! -f "$machine" ]; then
        echo "error: the generator wrote no $machine for fixture $id" >&2
        exit 1
    fi
    cp "$TMP/$id"/*_sm.swift "$GENERATED_DIR/"
    # The policy class is named from the document, so it is read from the file it was written
    # to rather than spelled again here.
    policy="$(grep -o 'public final class [A-Za-z0-9_]*Policy' "$machine" | head -n 1 | awk '{print $4}')"
    if [ -z "$policy" ]; then
        echo "error: no policy class in $machine" >&2
        exit 1
    fi
    expected="$(expected_state_of "$(document_of "$id")")"
    {
        printf '\n@Test("W3C %s settles in its %s state")\n' "$id" "$expected"
        printf 'func w3c_%s() {\n' "$id"
        printf '    let policy = %s()\n' "$policy"
        if [ -n "${KNOWN_GAPS[$id]:-}" ]; then
            printf '    withKnownIssue("%s") {\n' "${KNOWN_GAPS[$id]}"
            printf '        let run = settled(policy)\n'
            printf '        #expect(run.completed, "W3C %s did not run to a final state")\n' "$id"
            printf '        #expect(policy.getStateName(run.engine.getCurrentState()) == "%s", "W3C %s settled in another state")\n' "$expected" "$id"
            printf '    }\n'
        else
            printf '    let run = settled(policy)\n'
            printf '    #expect(run.completed, "W3C %s did not run to a final state")\n' "$id"
            printf '    #expect(policy.getStateName(run.engine.getCurrentState()) == "%s", "W3C %s settled in another state")\n' "$expected" "$id"
        fi
        printf '}\n'
    } >> "$TEST_FILE"
done

echo "Swift W3C (simple harness): ${#LOWERED[@]} of ${#IDS[@]} fixtures lowered into $GENERATED_DIR/"
refused=$(( ${#IDS[@]} - ${#LOWERED[@]} ))
echo "Not lowered yet ($refused), refused by name:"
sort "$REASONS_FILE" | uniq -c | sort -rn | sed 's/^/  /'
