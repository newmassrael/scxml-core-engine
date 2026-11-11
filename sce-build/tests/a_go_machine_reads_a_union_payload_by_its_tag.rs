// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Go `sce-static` machine holds a union (docs/adr/0092, sections 4, 6.1
//! and 7, the Go row, and its item 4), held to
//! `sce-build/tests/fixtures/static_datamodel/static_union.scxml`:
//!
//! * The union is the machine's own value struct: an unexported tag and one
//!   unexported member per case that holds a record, built only by the
//!   constructor of its case, its tag read by `Kind()`.
//! * The lift reads each union's tag first (`sce.ReadUnion`), then the record
//!   of the case the tag names; a `<sce:lift>` reads a text into a union the
//!   same way (`sce.LiftValue`).
//! * Each read is spelled by the cases the tests leave: the tag by `Kind()`,
//!   a field of the one case left through that case's proven read, and a field
//!   two cases share through its accessor.
//! * A field of a case is written in place, through the pointer the unexported
//!   accessor beside the proven read answers.
//! * The machine's saved form writes a union as its tag beside its case and
//!   reads it back by its tag first; a case record that holds a list writes
//!   and reads it by its type.
//! * The raise seam checks a union by the record of the case it is in.
//!
//! What the generated machine does at run time is held by the scenario
//! `static_union.json`, replayed over the committed machine by
//! `backends/go/tests/integration/static_datamodel/static_scenarios_test.go`.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel")
}

/// The Go machine of the statechart at `path`.
fn go_machine(path: &Path) -> Result<String, String> {
    let language = Language::Go;
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .map_err(|e| format!("{e:?}"))?;
    let names: Vec<String> = output.files.iter().map(|(name, _)| name.clone()).collect();
    output
        .files
        .into_iter()
        .find(|(name, _)| name.ends_with("_sm.go"))
        .map(|(_, code)| code)
        .ok_or_else(|| format!("no machine among {names:?}"))
}

/// The Go machine of the fixture.
fn machine() -> String {
    go_machine(&fixtures().join("static_union.scxml"))
        .unwrap_or_else(|e| panic!("Go holds a union: {e}"))
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

const UNION: &str = "StaticUnionUnionShapeUnion";

/// Pinned by `GoComposed::union_type` and `GoComposed::union_def`
/// (`sce-build/src/forge/composed_lowering/go.rs`) and by the payload channel
/// (`go_composed_payload`, `sce-build/src/forge/generator.rs`).
#[test]
fn the_payload_holds_the_union_in_the_machines_own_value_struct() {
    let code = machine();
    holds(
        &code,
        &[
            &format!("type {UNION} struct {{\n\ttag StaticUnionKindEnum\n"),
            "\tcaseCircle StaticUnionRadiusRecord\n",
            "\tcasePath StaticUnionPathRecord\n",
            &format!("func (u {UNION}) Kind() StaticUnionKindEnum {{\n\treturn u.tag\n}}"),
            &format!(
                "func {UNION}Dot() {UNION} {{\n\treturn {UNION}{{tag: StaticUnionKindEnumDot}}\n}}"
            ),
            &format!(
                "func {UNION}Circle(r StaticUnionRadiusRecord) {UNION} {{\n\treturn {UNION}{{tag: \
                 StaticUnionKindEnumCircle, caseCircle: r}}\n}}"
            ),
            &format!("func (u {UNION}) ProvenRect() StaticUnionRectRecord {{"),
            &format!("func (u *{UNION}) provenRectPlace() *StaticUnionRectRecord {{"),
            &format!("func (u {UNION}) ProvenFieldRadius() uint16 {{"),
            // The lift reads the tag first, then the case it names.
            "sce.ReadUnion(sceValue, \"kind\", func(sceValue sce.PayloadValue) \
             (StaticUnionKindEnum, error) {",
        ],
    );
    // An empty case holds no member.
    assert!(!code.contains("caseDot"), "{code}");
}

/// Pinned by `go_union_member` and the read the expression lowering spells by
/// the cases the facts leave (`lower_union_reads`, `sce-build/src/forge/expr.rs`),
/// and by `GoComposed::union_member_place` and the `<assign>` that writes
/// through it (`union_field_place`, `sce-build/src/forge/static_lowering.rs`).
#[test]
fn each_read_and_write_is_spelled_by_the_cases_the_tests_leave() {
    let code = machine();
    holds(
        &code,
        &[
            // The tag by its method.
            "p.pendingShapesSentPayload.one.Kind() == StaticUnionKindEnumDot",
            // A field of the one case left, through that case's proven read.
            "p.pendingShapesSentPayload.one.ProvenRect().width",
            // A field two cases share, through its accessor.
            "p.pendingShapesSentPayload.one.ProvenFieldRadius()",
            // A field of the built variable's proven case, written in place.
            "p.vBuilt.provenRectPlace().width = ",
            "p.vBuilt.ProvenRect().width",
        ],
    );
}

/// Pinned by `union_variable_field` (`sce-build/src/forge/static_lowering.rs`)
/// and `GoComposed::union_value`: a variable of the union starts at the case its
/// `<sce:set>`s build, and one that may be absent starts absent.
#[test]
fn a_union_variable_starts_built_or_absent() {
    let code = machine();
    holds(
        &code,
        &[
            &format!("vBuilt: {UNION}Rect(StaticUnionRectRecord{{width: "),
            &format!("vKept: optional.None[{UNION}](),"),
            &format!("vLifted: optional.None[{UNION}](),"),
        ],
    );
}

/// Pinned by `GoComposed::saved_union_ext`, `GoComposed::saved_record_ext` and
/// the saved form the Go template writes a variable by (`saved_write`,
/// `saved_read`, `tools/codegen/templates/go/state_machine.go.jinja2`).
#[test]
fn a_saved_union_is_its_tag_beside_its_case_and_is_read_back_by_its_tag() {
    let code = machine();
    holds(
        &code,
        &[
            &format!("func (u {UNION}) ToSaved() any {{\n\tswitch u.tag {{"),
            "\t\treturn sce.SavedWriteUnion(\"kind\", \"circle\", u.caseCircle.ToSaved())\n",
            "\t\treturn sce.SavedWriteUnion(\"kind\", \"dot\", nil)\n",
            &format!("func {UNION}FromSaved(value any, what string) ({UNION}, error) {{"),
            "sce.SavedReadUnionTag(value, what, \"kind\")",
            "\t\trecord, err := StaticUnionRadiusRecordFromSaved(value, what)\n",
            // A case record that holds a list writes and reads it by its type, held to
            // its bound.
            "func (r StaticUnionPathRecord) ToSaved() any {",
            "\tsaved.Set(\"points\", (func(sceSaved0 []uint8) any { return \
             sce.SavedWriteList(sceSaved0, sce.SavedWriteUint8) })(r.points))\n",
            "func StaticUnionPathRecordFromSaved(value any, what string) (StaticUnionPathRecord, \
             error) {",
            "sce.SavedReadListWithin(sceSaved0, sceWhat0, 0, 3, sce.SavedReadUint8)",
            // Each variable writes and reads its union by the union's own form.
            "variables.Set(\"built\", p.vBuilt.ToSaved())",
            &format!(
                "variables.Set(\"kept\", optional.Wire(p.vKept, func(v {UNION}) any {{ return \
                 v.ToSaved() }}))"
            ),
            &format!("{UNION}FromSaved(raw"),
            &format!(
                "sce.SavedReadOptional(raw2, \"kept\", {UNION}FromSaved, optional.None[{UNION}], \
                 optional.Some[{UNION}])"
            ),
        ],
    );
}

/// Pinned by the Go lift (`GoTarget::lift`, `sce-build/src/forge/static_lowering.rs`):
/// a text is read into the union whole by the reader a delivery's field of it is
/// read by.
#[test]
fn a_text_is_lifted_into_the_union_by_its_reader() {
    let code = machine();
    holds(
        &code,
        &[
            &format!(
                "sce.LiftValue(p.pendingShapeSaidPayload.text, \"Shape\", 2, 256, func(sceValue \
                 sce.PayloadValue) ({UNION}, error) {{ return sce.ReadUnion(sceValue, \"kind\","
            ),
            &format!("p.vLifted = optional.Some[{UNION}](sceLifted)"),
        ],
    );
}

/// Pinned by `go_bound_check`'s union arm
/// (`sce-build/src/forge/composed_lowering/go.rs`): the raise seam of
/// `shapes.sent` checks each union by the record of the case it is in, at the
/// union's own path, so a label's text past its bound is refused before
/// anything is sent.
#[test]
fn the_raise_seam_checks_a_union_by_its_case() {
    let code = machine();
    holds(
        &code,
        &[
            "func() error { switch one.Kind() { case StaticUnionKindEnumLabel: return ",
            "sce.CheckText(\"one\" + \"/text\", one.ProvenLabel().text, 8)",
            "case StaticUnionKindEnumPath: return sce.CheckList(\"one\" + \"/points\", \
             one.ProvenPath().points, 3,",
        ],
    );
}
