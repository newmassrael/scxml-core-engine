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
# reader closes it. So the call is looked for in code: not on a comment line,
# whose prose can name the decoder while nothing calls it, and not inside a
# `#[cfg(test)]` MODULE, where this tree keeps a file's unit tests.
#
# ⚠ The module, not the attribute. The first version of this predicate
# stopped reading at a file's first `#[cfg(test)]`, and `forge/sourcemap.rs`
# puts one on a `use` thirty lines before its reader — so the tree read open
# with the caller present. The row's controls caught it.
row_C3() {
    local file
    while IFS= read -r file; do
        [[ "$file" =~ (/tests?/|_test\.|\.jinja2$) ]] && continue
        awk '
            function test_module(s) {
                return s ~ /^[[:space:]]*(pub[^[:space:]]*[[:space:]]+)?mod[[:space:]]/
            }
            pending && test_module($0) { exit }
            /^[[:space:]]*#\[cfg\(test\)\]/ {
                rest = $0
                sub(/^[[:space:]]*#\[cfg\(test\)\]/, "", rest)
                if (test_module(rest)) exit
                pending = rest ~ /^[[:space:]]*$/
                next
            }
            !/^[[:space:]]*(#\[.*\])?[[:space:]]*$/ { pending = 0 }
            /^[[:space:]]*(\/\/|\/\*|\*)/ { next }
            /comment_text::decode/ { found = 1; exit }
            END { exit !found }' "$file" && return 0
    done < <(sce_grep -l 'comment_text::decode' sce-build/src/ tools/ 2>/dev/null)
    return 1
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

# G2 — the visualizer renders the annotation family, drawing the SAME
# dependency closure the acceptance report derives.
#
# ASKED, not named — and this row is where naming was most dangerous. The
# predicate was one grep for `sce:req|sce:provenance|sce:unresolved` under
# `web/visualizer/`, which a COMMENT mentioning the attribute closes. It
# had already produced one false green from a build artifact (the compiled
# parser's string table), which is why `sce_grep` prunes `wasm/` at all.
# Row G3 then showed the other face of a name predicate — the artefact
# built, shipping and measured `open`, because the file was spelled
# differently than the glob guessed.
#
# ⛔ The row's real content is the half a grep cannot see. A requirement's
# evidence is NOT the elements carrying its id: measured over this gate's
# own probe, REQ-A's fragment reaches the arming `<send>` and the
# `<cancel>` naming it, and NEITHER carries `sce:req`. A browser that
# picked elements by attribute would draw a picture disagreeing with the
# table the machine measures, and nothing would catch it, because a
# rendered diagram is not bytes a test can diff. That is why the closure
# is produced by the acceptance report's own function and shipped as data.
#
# So this runs both ends:
#
#   the producer   `sce-codegen annotation-overlay` emits the overlay:
#                  every node with what it claims (empty where nothing
#                  does), and per requirement the closure its evidence
#                  rests on — INCLUDING nodes carrying no id.
#
#   the consumer   the browser module is loaded under node and asked to
#                  stamp a graph. It must mark from what it was GIVEN: a
#                  claimed edge claimed, an unclaimed state unclaimed. A
#                  module that stamped everything alike fails both ways.
#
# ⚠ The control is the load-bearing half, as in G3 and G4: the producer
# must ACCEPT the probe first, and a failure there stops the gate rather
# than reporting the row — every check below is vacuous without it.
G2_PROBE='<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="null" name="g2_probe">
  <state id="idle">
    <onentry>
      <send event="tick" delay="3s" id="armTimer"/>
    </onentry>
    <transition event="tick" target="done" sce:req="REQ-A"/>
  </state>
  <state id="done">
    <onentry>
      <cancel sendid="armTimer"/>
    </onentry>
  </state>
</scxml>'

# Ask the browser module what it does with an overlay it is handed.
#
# Written here rather than kept under `web/` so the fixture a check is
# judged against cannot be edited into agreement with a module that
# stopped applying it — the discipline row G4's manifests already follow.
g2_consumer_probe() {
    cat <<'PROBE'
const fs = require('fs');
const { AnnotationOverlay } = require(process.argv[3]);
const overlay = new AnnotationOverlay(JSON.parse(fs.readFileSync(process.argv[2], 'utf8')));

function fail(why) { console.error(why); process.exit(1); }

// The unclaimed element is the reviewer's finding, so it must survive
// into the overlay rather than being filtered out as uninteresting.
if (overlay.unclaimedPaths().length === 0) {
    fail('the overlay carries no unclaimed element');
}

// THE DISCRIMINATOR. A node in REQ-A's fragment that carries no REQ-A.
// A consumer deriving by attribute cannot produce one.
const supporting = overlay
    .fragmentPaths('REQ-A')
    .filter((p) => overlay.isSupportingNode(p, 'REQ-A'));
if (supporting.length === 0) {
    fail('REQ-A fragment reaches no node that carries no id: this is the id set, not the closure');
}

// The consumer stamps from what it was given, in BOTH directions.
const states = [{ id: 'idle' }, { id: 'done' }];
const transitions = [{ source: 'idle' }];
overlay.applyTo(states, transitions);
if (transitions[0].annotationClass !== 'sce-claimed') {
    fail('a transition the overlay says claims REQ-A was not marked claimed');
}
if (states[1].annotationClass !== 'sce-unclaimed') {
    fail('a state the overlay says claims nothing was not marked unclaimed');
}
// Control on the control: a module marking everything claimed would pass
// the line above only by accident, so the other polarity is asserted too.
if (states[0].annotationClass !== 'sce-unclaimed') {
    fail('every element was marked claimed, so the marking says nothing');
}
PROBE
}

# Run the visualizer's OWN node and link builders over a structure that
# carries an overlay, and ask what the renderer would be handed.
#
# `vm` rather than `require`, because these files are browser scripts that
# declare globals instead of exporting: loading them any other way would
# mean keeping a second copy here, and a copy is what stops being the
# thing under test. The two globals they reach for are stubbed, and only
# those two — a stub standing in for the builder itself would make this
# probe agree with anything.
g2_render_probe() {
    cat <<'PROBE'
const fs = require('fs');
const vm = require('vm');
const path = require('path');
const overlayPath = process.argv[2];
const root = process.argv[3];

function fail(why) { console.error(why); process.exit(1); }

const { AnnotationOverlay } = require(path.join(root, 'web/visualizer/annotation-overlay.js'));
const overlay = new AnnotationOverlay(JSON.parse(fs.readFileSync(overlayPath, 'utf8')));

const sandbox = {
    console,
    logger: { debug() {}, info() {}, warn() {}, error() {} },
    SCXMLVisualizer: {
        isCompoundOrParallel: () => false,
        findCollapsedAncestor: () => null,
    },
};
vm.createContext(sandbox);
for (const file of ['visualizer/node-builder.js', 'visualizer/link-builder.js']) {
    vm.runInContext(fs.readFileSync(path.join(root, 'web/visualizer', file), 'utf8'), sandbox, {
        filename: file,
    });
}

// The structure the page hands the visualizer, for the probe document.
const states = [
    { id: 'idle', type: 'atomic', children: [] },
    { id: 'done', type: 'atomic', children: [] },
];
const transitions = [
    { id: 't0', source: 'idle', target: 'done', event: 'tick' },
];

const visualizer = {
    states,
    transitions,
    initialState: '',
    nodes: [],
    debugMode: false,
    annotationOverlay: overlay,
};

// Read back through the context: a top-level `class` is a lexical
// binding in the context's global scope, not a property of the sandbox
// object, so `sandbox.NodeBuilder` would be undefined and the probe
// would fail for a reason that has nothing to do with the tree.
const NodeBuilder = vm.runInContext('NodeBuilder', sandbox);
const LinkBuilder = vm.runInContext('LinkBuilder', sandbox);

const nodes = new NodeBuilder(visualizer).buildNodes();
const links = new LinkBuilder(visualizer).buildLinks();

const done = nodes.find((n) => n.id === 'done');
if (!done) { fail('the node builder produced no node for a state in the structure'); }
if (done.annotationClass !== 'sce-unclaimed') {
    fail(`a state nothing claims reached the renderer as ${JSON.stringify(done.annotationClass)}; `
        + 'the diagram cannot distinguish an unclaimed element');
}

const edge = links.find((l) => l.linkType === 'transition');
if (!edge) { fail('the link builder produced no transition link'); }
if (edge.annotationClass !== 'sce-claimed') {
    fail(`the transition claiming REQ-A reached the renderer as ${JSON.stringify(edge.annotationClass)}`);
}
if (!(edge.requirements || []).includes('REQ-A')) {
    fail(`the claimed ids did not reach the renderer: ${JSON.stringify(edge.requirements)}`);
}

// The renderer turns those fields into what the browser paints. Asked
// through the same function the renderer calls, so a class the renderer
// assembles differently is caught here rather than assumed.
const { annotationClassFor, requirementIdsFor } = require(
    path.join(root, 'web/visualizer/annotation-overlay.js')
);
if (!annotationClassFor(edge).includes('sce-claimed')) {
    fail('the renderer class function drops the claimed mark');
}
if (!annotationClassFor(done).includes('sce-unclaimed')) {
    fail('the renderer class function drops the unclaimed mark');
}
if (requirementIdsFor(edge) !== 'REQ-A') {
    fail(`the renderer id attribute is ${JSON.stringify(requirementIdsFor(edge))}`);
}
// Control: an element with no annotation at all must get no attribute,
// or every element would carry one and the mark would say nothing.
if (requirementIdsFor({}) !== null) {
    fail('an unannotated element was given a requirement attribute');
}

// The marks must be VISIBLE, and the stylesheet is where that is decided.
// Checked against the module's own constants rather than against literals
// typed here, so a rename on either side is caught instead of silently
// leaving the diagram unstyled — which is exactly how this feature first
// shipped: every rule sat behind a scope class nothing ever added.
const {
    SCE_ANNOTATION_CLAIMED,
    SCE_ANNOTATION_UNCLAIMED,
    SCE_ANNOTATIONS_ON,
} = require(path.join(root, 'web/visualizer/annotation-overlay.js'));
const css = fs.readFileSync(path.join(root, 'web/visualizer/visualizer.css'), 'utf8');
// Per SELECTOR, not per file. "The class appears somewhere" is satisfied
// by any one unrelated rule — measured: renaming the scope on the
// unclaimed rules left twelve other mentions and a substring check still
// passed. What has to be true is that each MARK is styled UNDER the
// scope the page adds, so both names must meet in one selector.
const selectors = css.split('}').map((block) => block.split('{')[0]);
for (const mark of [SCE_ANNOTATION_CLAIMED, SCE_ANNOTATION_UNCLAIMED]) {
    const scoped = selectors.some(
        (sel) => sel.includes(`.${SCE_ANNOTATIONS_ON}`) && sel.includes(`.${mark}`)
    );
    if (!scoped) {
        fail(`no selector styles .${mark} under .${SCE_ANNOTATIONS_ON}, so the mark is invisible`);
    }
}
PROBE
}

row_G2() {
    local bin dir overlay_json
    bin="$(sce_gate_codegen)" || sce_gate_cannot_run \
        "row G2 is measured by running the overlay producer, and sce-codegen could not be provided"

    # Absent entirely is this row REOPENING, not a broken gate.
    "$bin" annotation-overlay --help >/dev/null 2>&1 || return 1

    command -v node >/dev/null 2>&1 \
        || sce_gate_cannot_run "row G2 loads the browser module under node, and node was not found"

    dir="$(mktemp -d)"
    sce_gate_on_exit "rm -rf '$dir'"
    printf '%s\n' "$G2_PROBE" >"$dir/probe.scxml"
    overlay_json="$dir/overlay.json"

    # The control, and the only outcome here that stops the gate.
    "$bin" annotation-overlay "$dir/probe.scxml" >"$overlay_json" 2>/dev/null || sce_gate_fail \
        "row G2's control was refused: the overlay producer rejected a document this gate wrote.
  Either the probe in $0 owes an update, or the producer now refuses what it should accept. Until
  that is settled the checks below say nothing, so no verdict is given for the row."

    # The browser module must exist and be loadable as a module.
    [[ -f web/visualizer/annotation-overlay.js ]] || return 1

    g2_consumer_probe \
        | node - "$overlay_json" "$PWD/web/visualizer/annotation-overlay.js" >/dev/null 2>&1 \
        || return 1

    # And the DIAGRAM must carry it. This is the half a grep cannot do,
    # and the half that was wrong: the first version of this row grepped
    # `renderer.js` for `annotationClass` — which passed while the field
    # was read by the renderer and written by nobody, so the module was
    # dead code and the row read closed. A name check in a predicate
    # whose own header says ASKED, NOT NAMED.
    #
    # So the real builders are RUN, under node, over a structure carrying
    # an overlay, and the objects they hand the renderer are asked what
    # they carry. A builder that drops the annotation fails here; so does
    # a builder that invents one.
    g2_render_probe \
        | node - "$overlay_json" "$PWD" >/dev/null 2>&1 \
        || return 1
}

# G3 — a review artefact with the requirement column for a non-statechart kind.
#
# ASKED, not named — row G4's discipline, and this row is why it is worth
# stating twice. The predicate here was a filename glob
# (`*_review_table.rs`) with a prose grep behind it, and measured 2026-09-15
# it was wrong in BOTH directions at once: an empty file with that name
# would have closed the row, and the working artefact that actually landed
# did not match the glob, so the row read `open` with the thing it asks for
# built, tested and shipping. A name predicate does not merely pass things
# it should not; it also fails the thing it was written to find.
#
# So the artefact is RUN, over documents this gate writes:
#
#   the column      a mapping row claiming a requirement prints it in
#                   `source`, and the rows claiming nothing print
#                   `(none)` — the block that makes the table a trace
#                   table rather than a list of annotations.
#
#   the refusal     a kind SCE declares no annotation site for is
#                   REFUSED, not handed an empty table. Zero rows has two
#                   causes and they must not collapse: rendering "nobody
#                   can annotate this kind" as "reviewed, nothing to
#                   report" is the vacuous green this row would otherwise
#                   be closed by.
#
# ⚠ The control is the load-bearing half, for G4's reason: an artefact
# that refused every document would satisfy the refusal check above, so
# the lookup must be ACCEPTED first and a failure there stops the gate
# instead of reporting the row.
G3_LOOKUP='<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="g3_probe_lookup">
  <datamodel>
    <data id="raw" sce:type="uint8" sce:direction="in"/>
    <data id="out" sce:type="string" sce:direction="out"/>
    <data id="mapping" sce:default="NEUTRAL">
      <sce:entry key="0" value="PARK"/>
      <sce:entry key="1" value="DRIVE" sce:req="REQ-G3"/>
    </data>
  </datamodel>
</scxml>'

# A kind the grammar gives no place to put a claim.
G3_CONDITION='<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="condition" name="g3_probe_condition">
  <datamodel>
    <data id="rpm" sce:type="uint32" sce:direction="in"/>
    <data id="result" sce:type="bool" sce:direction="out" expr="rpm &gt;= 10"/>
  </datamodel>
</scxml>'

row_G3() {
    local bin dir table
    bin="$(sce_gate_codegen)" || sce_gate_cannot_run \
        "row G3 is measured by running the review artefact, and sce-codegen could not be provided"

    # Absent entirely is this row REOPENING, not a broken gate.
    "$bin" review-table --help >/dev/null 2>&1 || return 1

    dir="$(mktemp -d)"
    sce_gate_on_exit "rm -rf '$dir'"
    printf '%s\n' "$G3_LOOKUP" >"$dir/lookup.scxml"
    printf '%s\n' "$G3_CONDITION" >"$dir/condition.scxml"

    # The control, and the only outcome here that stops the gate.
    table="$("$bin" review-table "$dir/lookup.scxml" 2>/dev/null)" || sce_gate_fail \
        "row G3's control was refused: a lookup this gate wrote produced no review table.
  Either the forge grammar moved and the probe in $0 owes an update, or the artefact now
  refuses what it should accept. Until that is settled the refusal below says nothing, so
  no verdict is given for the row."

    # The requirement column carries the claim.
    grep -qF '"source":"REQ-G3"' <<<"$table" || return 1
    # And the rows claiming nothing collect, rather than being dropped —
    # an artefact that printed only annotated rows would pass the line
    # above while losing the reading the table exists for.
    grep -qF '"source":"(none)"' <<<"$table" || return 1

    # A kind with no annotation site is refused AND prints no rows.
    "$bin" review-table "$dir/condition.scxml" >"$dir/cond.out" 2>/dev/null && return 1
    [[ ! -s "$dir/cond.out" ]] || return 1
}

# G4 — decomposition, and a delegation checked against its destination.
#
# ASKED, not named — the discipline row S3 already follows, and the reason
# is not style. This row was measured for one round by two greps over
# `requirement_manifest.rs`: the absence of the phrase "cannot express yet"
# and the presence of the word `decomposes_into`. Neither reads the arrival
# check at all, and the row's `Closed when` has TWO clauses. Measured
# 2026-09-15: with `requirement_set.rs` and its test deleted from a
# worktree — a tree that does not compile — the old predicate still
# answered CLOSED. A predicate that cannot see the half its row is
# sharpest about is a false green waiting for someone to delete the half.
#
# So the check is RUN, over manifests this gate writes, and both clauses
# are put to it:
#
#   decomposition   a parent naming a child that exists is accepted; one
#                   naming a child that does not is refused. The control
#                   alone proves the field is EXPRESSED — the manifest
#                   loader is `deny_unknown_fields`, so a `decomposes_into`
#                   the type does not carry is a manifest that will not
#                   load — and the mutant proves it is JUDGED.
#
#   delegation      a delegation whose destination carries the id is
#                   accepted; one whose destination does not is refused.
#
# ⚠ THE CONTROL IS THE LOAD-BEARING HALF. A check that refused everything
# would pass a mutants-only predicate, and so would a binary whose
# subcommand had merely been renamed. So an arriving delegation must be
# ACCEPTED first, and the failure of that control stops the gate rather
# than reporting the row — a refusal proves nothing about the tree when
# the acceptance beside it did not happen.
#
# ⚠ The binary is resolved through the shared locator, which REBUILDS one
# built from other sources. Asking a stale binary would answer about a
# tree that no longer exists, which is the precise failure this row was
# written to stop being possible.
G4_EXTRACTION='"extraction":{"ids":"native","trace":"none","modality_convention":"english-modal-verbs","method":"ai-pass-1"}'

# A manifest holding $2 as its requirement list, spelled as the loader
# takes it. Written here rather than kept under `tests/` so that the
# fixture a mutant is judged against cannot be edited into agreement with
# a check that stopped refusing.
g4_manifest() {
    printf '{"doc_id":"%s","rev":"D1",%s,"requirements":[%s]}\n' "$1" "$G4_EXTRACTION" "$2"
}

# Does every cross-document claim in this set land? Silent either way; the
# verdict is the exit status.
g4_closure() {
    "$1" requirement-closure --manifest "$2" --manifest "$3" >/dev/null 2>&1
}

row_G4() {
    local bin dir
    bin="$(sce_gate_codegen)" || sce_gate_cannot_run \
        "row G4 is measured by running the cross-document check, and sce-codegen could not be provided"

    # Absent entirely is this row REOPENING — the check left the tree — and
    # not a gate that has broken, so it is answered `open` and the sweep
    # carries on to the other rows.
    "$bin" requirement-closure --help >/dev/null 2>&1 || return 1

    dir="$(mktemp -d)"
    sce_gate_on_exit "rm -rf '$dir'"

    g4_manifest LOWER '{"id":"L-1"}' >"$dir/lower.json"
    g4_manifest UPPER \
        '{"id":"U-1","disposition":{"kind":"delegated","to_doc":"LOWER","to_id":"L-1"}}' \
        >"$dir/arrives.json"
    g4_manifest UPPER \
        '{"id":"U-1","disposition":{"kind":"delegated","to_doc":"LOWER","to_id":"L-9"}}' \
        >"$dir/lost.json"
    # Partly HERE and partly ELSEWHERE, which is the shape the row names: one
    # child in the parent's own document, one in the other. The mutant moves
    # only the second, so what it isolates is the cross-document half.
    g4_manifest UPPER \
        '{"id":"U-2","decomposes_into":[{"doc":"UPPER","id":"U-3"},{"doc":"LOWER","id":"L-1"}]},{"id":"U-3"}' \
        >"$dir/split.json"
    g4_manifest UPPER \
        '{"id":"U-2","decomposes_into":[{"doc":"UPPER","id":"U-3"},{"doc":"LOWER","id":"L-9"}]},{"id":"U-3"}' \
        >"$dir/split_lost.json"

    # The control, and the only outcome here that stops the gate: a
    # delegation that arrives carries no decomposition and must be
    # accepted. If it is not, this gate is measuring its own fixtures
    # against a loader that has moved, and every refusal below would be
    # reported as a closed row for a reason that has nothing to do with G4.
    g4_closure "$bin" "$dir/arrives.json" "$dir/lower.json" || sce_gate_fail \
        "row G4's control was refused: an arriving delegation over two manifests this gate
  wrote is no longer accepted by \`requirement-closure\`. Either the manifest format moved
  and the fixtures in $0 owe an update, or the check now refuses what it should pass. Until
  that is settled a refusal below says nothing, so no verdict is given for the row."

    # Decomposition — expressed (the control loads at all) and judged.
    g4_closure "$bin" "$dir/split.json" "$dir/lower.json" || return 1
    ! g4_closure "$bin" "$dir/split_lost.json" "$dir/lower.json" || return 1

    # Delegation — checked against the document it names.
    ! g4_closure "$bin" "$dir/lost.json" "$dir/lower.json" || return 1
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
#
# The claim is whatever the header's @brief PARAGRAPH states, not only a
# section written first on its line: `(W3C SCXML 6.2 AOT)` closing a brief
# and `W3C SCXML 3.6/3.4:` are claims too, and a detector anchored on
# `@brief W3C SCXML <one token>:` read both as no claim at all. Every section
# the brief states is judged. The body below the brief is not read — its
# citations name the other sections a test touches, legitimately. A lettered
# label must carry a dotted part, or `W3C C++` would read as appendix C.
#
# A sweep that matched no header would find no contradiction and say closed,
# so every REGISTERED fixture must be matched to its header and a specnum
# (a variant id such as 403a reads its stem's metadata), or the row is not
# measured at all.
row_S4() {
    local rc=0
    python3 - <<'PY' || rc=$?
import glob, json, os, re, sys
spec = {}
for md in glob.glob("resources/*/metadata.txt"):
    m = re.search(r"^\s*specnum:\s*(\S+)", open(md, encoding="utf-8",
                                                errors="replace").read(), re.M)
    if m:
        spec[os.path.basename(os.path.dirname(md))] = m.group(1)
token = r"(?:[0-9]+(?:\.[0-9]+)*|[A-H](?:\.[0-9]+)+)"
statement = re.compile(r"\bW3C(?:\s+SCXML)?\s+(" + token + r"(?:\s*/\s*" + token + r")*)\b")
def brief(lines):
    for i, line in enumerate(lines):
        if "@brief" not in line:
            continue
        paragraph = [line.split("@brief", 1)[1]]
        for following in lines[i + 1:]:
            text = re.sub(r"^\s*(?:\*|//+!?)\s?", "", following)
            if (not re.match(r"^\s*(?:\*|//)", following)
                    or following.strip().startswith("*/")
                    or not text.strip() or text.lstrip().startswith("@")):
                break
            paragraph.append(text)
        return " ".join(paragraph)
    return ""
ids = [f["id"] for f in json.load(open("tests/w3c/conformance/fixtures.json"))["fixtures"]]
unmatched, bad = [], 0
for tid in ids:
    header = f"tests/w3c/aot_tests/Test{tid}.h"
    truth = spec.get(tid) or spec.get(re.sub(r"[a-z]+$", "", tid))
    if not os.path.isfile(header) or not truth:
        unmatched.append(tid)
        continue
    text = brief(open(header, encoding="utf-8", errors="replace").read().split("\n"))
    for stated in statement.finditer(text):
        for a in re.split(r"\s*/\s*", stated.group(1)):
            if not (a == truth or a.startswith(truth + ".") or truth.startswith(a + ".")):
                bad += 1
if not ids or unmatched:
    print(f"S4 unmatched fixture(s): {unmatched or 'the registry lists none'}", file=sys.stderr)
    sys.exit(3)
sys.exit(1 if bad else 0)
PY
    if (( rc == 3 )); then
        sce_gate_cannot_run "row S4 could not match every registered W3C fixture to its header and a specnum, so finding no contradiction would prove nothing"
    fi
    return "$rc"
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
