#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: nl-ir-closure.yml
#
# The NL->IR closure ledger, measured against the tree.
#
# `docs/SCE_NL_IR_CLOSURE.md` names what remains on SCE's side of the
# natural-language-specification -> IR path. This gate is what stops that
# file becoming prose: every row is measured here, and the two halves fail
# when they disagree.
#
# TWO QUESTIONS, AND THEY ARE NOT THE SAME ONE
#
#   exit status   is the ledger an honest description of the tree?
#                 rc=0 when every row's recorded Status matches what is
#                 measured below -- in BOTH directions. A row recorded
#                 `closed` that the tree contradicts is a false green; a row
#                 recorded `open` that the tree has in fact closed is a stale
#                 denominator. Both are refused.
#
#   closure: line is there anything left to do?
#                 `closure: REACHED` only when every row measures closed.
#
# So this gate is GREEN today with rows still open — the sweep below prints
# how many, and a count written here would be stale the round after it was
# written. That is deliberate: a
# gate that went red for unfinished work could not be run in the push path at
# all, and the thing worth guarding is not that the work is unfinished -- it
# is that the record of what is unfinished stays true.
#
# A consumer that wants the ending predicate reads BOTH: rc=0 AND a line
# matching `^closure: REACHED`. Neither alone is the answer. A ledger can be
# malformed while nothing is left to do, and work can be left while the
# ledger is perfectly well-formed.
#
# ⚠ The row set lives in two places on purpose -- here and in the ledger --
# and `ledger_and_gate_agree_on_the_row_set` fails when one grows a row the
# other does not have. One site would let a row be dropped from the
# denominator by deleting it, which is the failure this whole file exists to
# refuse.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

LEDGER="docs/SCE_NL_IR_CLOSURE.md"
[[ -f "$LEDGER" ]] || sce_gate_fail "the closure ledger is missing: $LEDGER"

# ─────────────────────────────────────────────────────────────────────
# Row predicates. Each answers ONE question: has this row closed?
#
# `closed` (0) or `open` (1). A predicate never prints the verdict -- the
# sweep below does -- so that a row cannot report itself.
#
# Where a row's condition is a crisp fact about the tree it is measured
# directly. Where the row is new machinery, the predicate names the fact the
# repair must make true rather than a filename it must use: a repair is free
# to choose its own names, and a predicate keyed on a name this file guessed
# would pass for a file that does nothing.
#
# ⚠ EVERY SWEEP ASKS THE SOURCE, NOT THE BUILD OUTPUT. `sce_grep` below
# refuses binary files and prunes the directories a build writes into. This
# is not defensive style: the first run of this gate reported G2 CLOSED
# because `web/visualizer/wasm/sce_build_bg.wasm` — the compiled parser —
# carries the literal `sce:req` in its string table. A predicate that reads
# a build artifact answers a question about the compiler, not about the
# visualizer, and it answers it green. Other sessions build into this tree
# while a gate runs, so which artifacts exist is not even stable.
# ─────────────────────────────────────────────────────────────────────

# Recursive grep, source only: `-I` drops binary files, and the prunes drop
# the trees a build writes. Takes the same arguments as `grep -r`.
sce_grep() {
    grep -rI \
        --exclude-dir=wasm --exclude-dir=target --exclude-dir=build \
        --exclude-dir=node_modules --exclude-dir=dist --exclude-dir=.git \
        "$@"
}

# C1 — identifier-bearing attributes checked against the W3C grammar.
#
# The check has to run at PARSE, on the attributes themselves. Its presence
# is measured as a refusal reachable from the parser, not as a helper that
# exists somewhere: a validator nothing calls is what §1 of the accepted
# subset already records as the defect.
row_C1() {
    sce_grep -qE 'fn [a-z_]*(ncname|event_token|id_grammar|identifier_grammar)' \
        sce-build/src/ 2>/dev/null || return 1
    sce_grep -qE 'validation/(invalid-id|id-grammar|event-name-grammar|malformed-identifier)' \
        sce-build/src/ 2>/dev/null || return 1
    return 0
}

# C2 — a value written into a string literal is encoded for it.
#
# Measured at the door, the way comment encoding already is. The tell that
# the repair happened per-site instead is `action.label` still reaching a Go
# string literal with no escaper while `action.expr` beside it has one.
row_C2() {
    sce_grep -qE 'string_literal|literal_text' sce-build/src/generator.rs 2>/dev/null || return 1
    ! grep -qE '\{\{ *action\.label *(\| *default\([^)]*\) *)?\}\}' \
        tools/codegen/templates/go/actions/log.go.jinja2 2>/dev/null
}

# C3 — the decoder has a production caller.
#
# Tests and the macro's own explanatory comment do not count: the defect is
# that a reader parsing markers back reads the encoded form, and only a
# reader closes it.
row_C3() {
    sce_grep -l 'comment_text::decode' sce-build/src/ tools/ 2>/dev/null \
        | grep -vE '(/tests?/|_test\.|\.jinja2$)' \
        | grep -q .
}

# C4 — the C11 template no longer reads `_*` as a wildcard.
row_C4() {
    ! sce_grep -qF "'_*'" tools/codegen/templates/c/ 2>/dev/null
}

# C5 — a document-order stem exists and is registered.
#
# Existence of the directory is not enough: the integration layout requires
# a stem to carry its registration sites, and an unregistered stem is a
# fixture no channel runs.
row_C5() {
    local stem
    stem=$(find integration_resources -maxdepth 1 -type d -name '*order*' 2>/dev/null | head -1)
    [[ -n "$stem" ]] || return 1
    sce_grep -q "$(basename "$stem")" tests/ backends/ 2>/dev/null
}

# G1 — the acceptance record, its pin, and the variant axis.
#
# The variant is the half that is easy to drop: an acceptance of the base
# build says nothing about a build with an optional feature composed in, so
# a pin without one is an acceptance of an unnamed thing.
row_G1() {
    sce_grep -qE 'acceptance_record|AcceptanceRecord' sce-build/src/ 2>/dev/null || return 1
    sce_grep -qE '\bvariant\b' sce-build/src/acceptance_report.rs 2>/dev/null
}

# G2 — the visualizer renders an annotation family.
row_G2() {
    sce_grep -qE 'sce:req|sce:provenance|sce:unresolved' web/visualizer/ 2>/dev/null
}

# G3 — a review artefact with the requirement column for a non-statechart kind.
row_G3() {
    ls sce-build/src/*_review_table.rs sce-build/src/forge/*_review_table.rs \
        >/dev/null 2>&1 && return 0
    sce_grep -qE 'review artefact for the .* kind|kind_review_table' sce-build/src/ 2>/dev/null
}

# G4 — decomposition, and a delegation checked against its destination.
#
# The manifest's own words are the tell for the first half: it currently
# says decomposition is a thing it "cannot express yet".
row_G4() {
    ! grep -qE 'cannot express (it )?yet' sce-build/src/requirement_manifest.rs 2>/dev/null || return 1
    sce_grep -qE 'parent_id|decomposes_into|child_ids' sce-build/src/requirement_manifest.rs 2>/dev/null
}

# S1 — only the model and the parser name the nested block fields.
#
# The model defines them; the parser builds them. Every other reader is
# supposed to descend through `Action::nested_blocks`, which is the single
# definition of what lies inside an action.
row_S1() {
    local offenders
    offenders=$(sce_grep -lE 'then_actions|else_actions|elseif_branches' \
        sce-build/src/ --include=*.rs 2>/dev/null \
        | grep -vE 'sce-build/src/(model|parser)\.rs$' | wc -l)
    [[ "$offenders" -eq 0 ]]
}

# S2 — a top-level <script> carries its annotations, and the walk reaches them.
#
# Both halves, because either alone leaves the annotation unread: the parser
# builds the action with defaults, and the shared walk never visits the
# global scripts at all.
row_S2() {
    grep -qE 'global_scripts' sce-build/src/requirements_report.rs 2>/dev/null || return 1
    ! awk '/model\.global_scripts\.push\(Action \{/,/\}\);/' sce-build/src/parser.rs 2>/dev/null \
        | grep -q '\.\.Default::default()'
}

# S3 — the dead constant is gone from every site that carried it, and
# registration refuses it.
#
# The AOT tree is swept for a DECLARATION, not for the word. The guard the row
# asks for has to name the member it refuses, so a sweep for the bare word
# reads that guard as the defect and leaves the row unreachable by the repair
# its own condition demands. A member declared or initialised is the word
# followed by `=`, `;`, `{` or `[` with no `::` before it; the guard's
# `TestClass::DESCRIPTION` is not one, and neither is prose that mentions it.
#
# The guard is ASKED, not named. A probe test type is compiled against the
# registrar twice: plain, which must compile — otherwise the second refusal
# says nothing about the member — and carrying the member, which must not. A
# guard found by grepping for `static_assert` would pass for one that refuses
# nothing.
S3_DECLARATION='(^|[^:[:alnum:]_])DESCRIPTION[[:space:]]*[=;{[]'

s3_probe() {
    printf '#include "AotTestRegistry.h"\nnamespace SCE::W3C::AotTests {\nstruct Probe : AotTestBase {\n    static constexpr int TEST_ID = 1;\n    %s\n    bool run() override { return true; }\n    int getTestId() const override { return TEST_ID; }\n};\ninline static AotTestRegistrar<Probe> registrar_Probe;\n}\n' "$1" \
        | "${CXX:-c++}" -std=c++20 -fsyntax-only -x c++ -I tests/w3c/aot_tests - >/dev/null 2>&1
}

row_S3() {
    ! sce_grep -qE "$S3_DECLARATION" tests/w3c/aot_tests/ 2>/dev/null || return 1
    ! sce_grep -qF 'DESCRIPTION' cmake/SCEStaticW3CTest.cmake 2>/dev/null || return 1
    ! grep -qF 'DESCRIPTION' CLAUDE.md 2>/dev/null || return 1
    command -v "${CXX:-c++}" >/dev/null 2>&1 \
        || sce_gate_cannot_run "row S3 is measured by compiling a probe, and no C++ compiler was found (\$CXX or c++)"
    s3_probe '' \
        || sce_gate_fail "row S3's control probe no longer compiles against tests/w3c/aot_tests/AotTestRegistry.h, so a refused member would prove nothing: bring s3_probe in $0 back in line with AotTestBase"
    ! s3_probe 'static constexpr const char *DESCRIPTION = "restated";'
}

# S4 — no header's target claim contradicts the fixture's own specnum.
#
# `specnum` in `resources/<id>/metadata.txt` is the derived source of truth.
# A claim that is FINER than it (6.2.2 against 6.2) is not a contradiction,
# which is why this compares prefixes rather than strings.
row_S4() {
    python3 - <<'PY'
import glob, os, re, sys
spec = {}
for md in glob.glob("resources/*/metadata.txt"):
    m = re.search(r"^\s*specnum:\s*(\S+)", open(md, encoding="utf-8",
                                                errors="replace").read(), re.M)
    if m:
        spec[os.path.basename(os.path.dirname(md))] = m.group(1)
claim = re.compile(r"@brief\s+W3C\s+SCXML\s+([0-9A-H](?:\.[0-9]+)*)\s*:")
bad = 0
for h in glob.glob("tests/w3c/aot_tests/Test*.h"):
    tid = re.match(r"Test(\w+)\.h$", os.path.basename(h))
    if not tid:
        continue
    c = claim.search(open(h, encoding="utf-8", errors="replace").read())
    truth = spec.get(tid.group(1))
    if not c or not truth:
        continue
    a = c.group(1)
    if not (a == truth or a.startswith(truth + ".") or truth.startswith(a + ".")):
        bad += 1
sys.exit(1 if bad else 0)
PY
}

# S5 — every test file the CMake lists cite exists.
row_S5() {
    local missing=0 f
    while IFS= read -r f; do
        [[ -f "$f" ]] || missing=1
    done < <(grep -ohE 'sce-build/tests/[A-Za-z0-9_]+\.rs' tests/CMakeLists.txt 2>/dev/null | sort -u)
    [[ "$missing" -eq 0 ]]
}

ROWS=(C1 C2 C3 C4 C5 G1 G2 G3 G4 S1 S2 S3 S4 S5)

# ─────────────────────────────────────────────────────────────────────
# The two sites must name the same rows.
# ─────────────────────────────────────────────────────────────────────
ledger_rows() {
    grep -oE '^\| (C|G|S)[0-9]+ \| (open|closed) \|' "$LEDGER" | awk '{print $2}'
}

mapfile -t LEDGER_ROWS < <(ledger_rows | sort)
mapfile -t GATE_ROWS < <(printf '%s\n' "${ROWS[@]}" | sort)

if [[ "${LEDGER_ROWS[*]}" != "${GATE_ROWS[*]}" ]]; then
    sce_gate_fail "$(printf 'the ledger and this gate name different rows.\n  ledger: %s\n  gate:   %s\nA row must be added to, or removed from, BOTH.' \
        "${LEDGER_ROWS[*]}" "${GATE_ROWS[*]}")"
fi

# ─────────────────────────────────────────────────────────────────────
# Measure.
# ─────────────────────────────────────────────────────────────────────
recorded_status() {
    grep -oE "^\| $1 \| (open|closed) \|" "$LEDGER" | awk '{print $4}'
}

disagreements=()
open_rows=()
closed_rows=()

for row in "${ROWS[@]}"; do
    recorded=$(recorded_status "$row")
    if [[ -z "$recorded" ]]; then
        sce_gate_fail "row $row has no recorded status in $LEDGER"
    fi

    if "row_$row"; then
        measured="closed"
        closed_rows+=("$row")
    else
        measured="open"
        open_rows+=("$row")
    fi

    if [[ "$recorded" != "$measured" ]]; then
        disagreements+=("  $row: the ledger records '$recorded', the tree measures '$measured'")
    fi
done

printf 'nl-ir-closure: %d row(s) — %d closed, %d open\n' \
    "${#ROWS[@]}" "${#closed_rows[@]}" "${#open_rows[@]}" >&2
if (( ${#open_rows[@]} )); then
    printf '  open: %s\n' "${open_rows[*]}" >&2
fi
if (( ${#closed_rows[@]} )); then
    printf '  closed: %s\n' "${closed_rows[*]}" >&2
fi

# The line a consumer greps for. Printed on stdout, prefixed, so that it
# survives another line being added above or below it.
if (( ${#open_rows[@]} == 0 )); then
    echo "closure: REACHED"
else
    echo "closure: NOT REACHED"
fi

if (( ${#disagreements[@]} )); then
    sce_gate_fail "$(printf 'the ledger no longer describes the tree:\n%s\n\nUpdate %s so its Status column says what the tree actually measures. A row recorded closed that the tree contradicts is a false green; a row recorded open that the tree has closed is a stale denominator.' \
        "$(printf '%s\n' "${disagreements[@]}")" "$LEDGER")"
fi
