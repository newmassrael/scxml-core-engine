# SCE NL→IR closure ledger

What remains on SCE's side of the natural-language-specification → IR path,
as a closed list with a measurable condition per row.

This file is the **denominator**. It is written before, and independently of,
whatever closes the rows — a coverage number whose denominator was written by
the same pass that fills it is always 100%, because what was left out is
missing from both sides. Anything that closes a row must therefore not edit
this file's `Closed when` column to match what it built.

`scripts/gates/nl-ir-closure.sh` measures every row against the tree. It
answers two different questions, and they must not be collapsed:

| | question | how it answers |
|---|---|---|
| exit status | is this ledger an honest description of the tree? | `rc=0` when every row's `Status` matches what the tree measures |
| `closure:` line | is there anything left to do? | `closure: REACHED` only when every row measures closed |

So the gate is green today with rows still open — the sweep prints how many,
and a count written here would be stale the round after it was true — and goes
red the moment a
row's recorded status stops matching the tree — in **either** direction. A row
marked `closed` that the tree contradicts is a false green; a row marked `open`
that the tree has in fact closed is a stale denominator, and both are defects
this gate is here to refuse.

⚠ Adding or removing a row means editing this file **and**
`scripts/gates/nl-ir-closure.sh`. Neither half is the register on its own, and
the gate fails when they disagree — the two-site registration this repository
already uses for tree-wide gates.

## Out of scope, stated rather than hidden

Promoting one schema that both SCE and the upstream section store read — and
re-seating the requirement manifest on it — is **not** a row here. It needs a
requirement unit below the section, which is an addition to the upstream
store's schema and not a change SCE can make. Measured: 47 of 192 stored
sections state more than one modality and 21 state a negative beside a
positive, so a `modality` field on the existing section cannot carry it. A row
SCE cannot close would make this ledger's `closure:` line unreachable by
construction, which is the one thing an ending predicate must never be.

## Rows

Status is `open` or `closed`.

| id | Status | What remains | Closed when |
|---|---|---|---|
| C1 | closed | Identifier-bearing attributes are not checked against the grammar W3C gives them. `<state id="s0*/X">` is accepted and becomes a code identifier, so the emitted source does not compile. | A parse-time check refuses a hostile `id`, `event` or `target`. Grammar is the NCName family — a token starts with a letter or `_`, continues with alphanumerics, `_` or `-`, and `.` separates tokens — which refuses 0 of 2480 event descriptors and 1 of 5281 ids in this tree (a pre-expansion template placeholder). The literal "alphanumeric" reading is refused: it rejects W3C's own conformance documents 364 and 576, whose event is `In-s11p112`. |
| C2 | closed | A value written into a string literal is not encoded for it. Templates apply an escaper at some sites and not others; a `<log label>` holding a line break is written unescaped into a C++ and a Go string literal, so the emitted source does not compile. | Encoding is a property of the one door every template passes through, as comment encoding already is — not of a per-site filter a template author can forget. |
| C3 | closed | Readers do not decode what the comment encoder wrote. `SCE-MAP:` markers and Go `//line` directives are comments, so a reader parsing them back reads the encoded form. | `comment_text::decode` has at least one production caller. |
| C4 | closed | Only the C11 template still reads `_*` as a wildcard; `event_descriptor` reads it as the literal token it is spelled as. C11 therefore takes a `_*` transition on every event where the other six take it on none. The template's own prose states the divergence as justification, and cites §5.9.3 — the clause for Legal Data Values, not Event Descriptors. | `'_*'` does not appear in `tools/codegen/templates/c/`. |
| C5 | closed | Two defects removed with Kotlin's hoisted wildcard block are untested: the `else` branch carried no `cond`, and it hand-wrote its result instead of calling the renderer. They owe a fixture on the document-order axis, which is not the descriptor-spelling stem's axis. | A document-order stem exists under `integration_resources/` and carries its registration sites. |
| G1 | closed | ③ of the three trace columns — the regression oracle. There is no acceptance record and no scenario pin, so nothing says what was approved or whether behaviour has moved since. With it belongs the variant axis: coverage is per variant, and an acceptance pins a (manifest revision, variant, document sha) triple. | An acceptance record can be taken and re-checked, and it carries a variant. |
| G2 | open | The visualizer renders no annotation family, so the diagram a reviewer reads cannot show which elements a requirement claims and which are unclaimed. | The visualizer renders the requirement annotations, drawing the same dependency closure the acceptance report derives rather than a second walk of its own. |
| G3 | open | The trace table covers the statechart family only. The other kind families have no review artefact carrying the requirement column. | A review artefact with the requirement column exists for at least one non-statechart kind family. |
| G4 | closed | A requirement met partly here and partly elsewhere cannot be expressed — the manifest has no parent/child — and nothing checks that a `delegated` requirement actually arrives in the document it names. | The manifest expresses decomposition, and a delegation is checked against the document it delegates to. |
| S1 | closed | `Action::nested_blocks` is the single definition of what lies inside an action, but fifteen files outside the model still name the nested block fields directly. The annotation readers were moved onto the shared definition; the analyzers were not. | Only the model and the parser name those fields — the model because it defines them, the parser because it builds them. |
| S2 | closed | A top-level `<script>` is built without reading annotations, so an `sce:req` on it is dropped at parse; and the shared walk never visits the global scripts, so a stamped annotation would still be unread. Both halves are open. | A top-level `<script>` carries its annotations, and the shared walk reaches them. |
| S3 | closed | `DESCRIPTION` is a dead constant: 209 AOT headers declare it, a CMake template generates it, this repository's own instructions prescribe it, and nothing reads it. | No AOT header declares it, no template generates it, no instruction prescribes it, and a guard stops it growing back. |
| S4 | closed | An AOT header's `@brief W3C SCXML <section>:` states which section the test targets, and 71 of the 157 that make the claim contradict the fixture's own `specnum`. | No header's target claim contradicts `specnum`. ⚠ A guard must reject the target claim only: 299 body lines across 104 headers cite other sections legitimately, and a guard refusing every section mention would delete them. |
| S5 | closed | `tests/CMakeLists.txt` cites `sce-build/tests/w3c_registry_cmake_parity.rs`, which does not exist. The file is `w3c_registry_drives_cmake.rs`. | Every test file `tests/CMakeLists.txt` cites exists. |

## Rows this ledger has already closed

Kept so they are not re-derived as open. Both were measured closed on
2026-09-14, after a first pass reported them missing by searching for names
they do not use — a reminder that absence found by guessing a name is not a
measurement.

| id | What it was | Where it landed |
|---|---|---|
| D | The first real consumer, so that the manifest and the table are measured against a specification nobody here controls rather than against fixtures their own author wrote. | `sce-build/tests/iso13400_requirement_closure.rs` — a sealed coordinate manifest for ISO 13400-2:2019 §12.6 and the statechart an authoring pass produced from §12.6.1. The standard's text is deliberately absent; the coordinates carry a recorded re-derivation procedure. |
| L | Refuse a specification's identity in an executable position, permit it in comments, and carry an allowlist for the protocols SCE implements. | `sce-build/tests/a_standard_named_in_code_is_one_sce_implements.rs`, registered as a tree-wide gate. The discriminator is the position rather than the word, so a citation survives and a branch does not. |
