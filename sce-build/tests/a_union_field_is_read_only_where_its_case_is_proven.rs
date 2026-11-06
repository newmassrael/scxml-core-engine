// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A `sce-static` statechart reads a union by testing its tag (docs/adr/0092,
//! section 6.1): the tag is a value of its enum wherever the union is, and a
//! field of a case is read only where a test of the tag leaves cases that each
//! declare it with one type. A read the build has not proven is
//! `expression/union-read-unproven`, placed at the read, naming the cases that
//! declare the field and the test that proves one; a field no case declares is
//! `expression/unknown-member`. One analysis (`forge::narrowing`) decides what
//! a test proves, beside the facts of an optional value (docs/adr/0055), so each
//! construct that narrows is held here by a read it admits and a read it does
//! not.
//!
//! Every document is judged where it is read (the parse judges every
//! expression of a `sce-static` statechart), so no backend's lowering is asked.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located};
use sce_build::model::SCXMLModel;
use tempfile::tempdir;

fn union_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// The records the cases hold, copied from the union fixtures: `radius`, and
/// `width` and `height`.
const RECORDS: [&str; 2] = ["shape_radius.scxml", "shape_rect.scxml"];

/// The tag's enum: a dot, a circle, a disc and a rect.
const KIND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="probe_kind" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="dot" value="0"/>
      <sce:variant name="circle" value="1"/>
      <sce:variant name="disc" value="2"/>
      <sce:variant name="rect" value="3"/>
    </data>
  </datamodel>
</scxml>
"#;

/// A shape: a dot of nothing, a circle and a disc that both hold the record of
/// a radius, and a rect of a width and a height.
const SHAPE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="union" name="probe_shape">
  <sce:import kind="enum" src="probe_kind.scxml" as="ShapeKind"/>
  <sce:import kind="event-schema" src="shape_radius.scxml" as="Radius"/>
  <sce:import kind="event-schema" src="shape_rect.scxml" as="Rect"/>
  <datamodel>
    <data id="kind" sce:type="enum:ShapeKind">
      <sce:case variant="dot"/>
      <sce:case variant="circle" record="Radius"/>
      <sce:case variant="disc" record="Radius"/>
      <sce:case variant="rect" record="Rect"/>
    </data>
  </datamodel>
</scxml>
"#;

/// The event that carries a shape, a list of them, and one that may be absent.
const EVENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="probe_event" sce:event-name="probe.sent">
  <sce:import kind="union" src="probe_shape.scxml" as="Shape"/>
  <datamodel>
    <data id="one" sce:type="union:Shape" sce:direction="in"/>
    <data id="many" sce:type="list&lt;union:Shape&gt;[1..3]" sce:direction="in"/>
    <data id="maybe" sce:type="optional&lt;union:Shape&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A reader of `probe.sent` that imports the tag's enum as `Kind` (another
/// alias than the union's own), declares a `uint32` `seen` and a `Kind` `tag`,
/// and runs `body` on the event under the guard `cond` (none when empty).
fn reader(cond: &str, body: &str) -> String {
    let guard = if cond.is_empty() {
        String::new()
    } else {
        format!(r#" cond="{cond}""#)
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="union_reader">
  <sce:import kind="enum" src="probe_kind.scxml" as="Kind"/>
  <sce:import kind="event-schema" src="probe_event.scxml" as="Probe"/>
  <datamodel>
    <data id="seen" sce:type="uint32" expr="0"/>
    <data id="tag" sce:type="enum:Kind" expr="Kind.dot"/>
  </datamodel>
  <state id="idle">
    <transition event="probe.sent"{guard} target="idle">
      {body}
    </transition>
  </state>
</scxml>
"#
    )
}

/// `document` read as a build reads it, beside the enum, the union, its records
/// and the event.
fn read(document: &str) -> Result<SCXMLModel, Located<ForgeError>> {
    let dir = tempdir().expect("tempdir");
    for record in RECORDS {
        std::fs::copy(union_fixtures().join(record), dir.path().join(record))
            .expect("copy a record");
    }
    for (name, text) in [
        ("probe_kind.scxml", KIND),
        ("probe_shape.scxml", SHAPE),
        ("probe_event.scxml", EVENT),
    ] {
        std::fs::write(dir.path().join(name), text).expect("write a sibling");
    }
    let path = dir.path().join("union_reader.scxml");
    std::fs::write(&path, document).expect("write the reader");
    let path = path.to_str().expect("a utf-8 path");
    let mut model = sce_build::parser::SCXMLParser::new().parse_file(path)?;
    sce_build::analyzer::analyze(&mut model, path);
    Ok(model)
}

/// `body` under `cond` is accepted where the document is read.
fn accepted(cond: &str, body: &str) {
    if let Err(refusal) = read(&reader(cond, body)) {
        panic!("accepted where the document is read:\n{body}\n{refusal}");
    }
}

/// `body` under `cond` is refused, the first record matching `code` and the
/// refusal saying each of `says`.
fn refused(cond: &str, body: &str, code: fn(DiagnosticCode) -> bool, says: &[&str]) {
    let Err(refusal) = read(&reader(cond, body)) else {
        panic!("refused where the document is read:\n{body}");
    };
    let found = refusal.error.to_diagnostics()[0].code;
    assert!(code(found), "{found:?} for\n{body}\n{refusal}");
    let text = refusal.to_string();
    for part in says {
        assert!(text.contains(part), "{body}: expected `{part}` in\n{text}");
    }
}

fn unproven(code: DiagnosticCode) -> bool {
    matches!(code, DiagnosticCode::ExpressionUnionReadUnproven)
}

fn unknown_member(code: DiagnosticCode) -> bool {
    matches!(code, DiagnosticCode::ExpressionUnknownMember)
}

fn assign(location: &str, expr: &str) -> String {
    format!(r#"<assign location="{location}" expr="{expr}"/>"#)
}

#[test]
fn the_tag_is_a_value_of_its_enum_wherever_the_union_is() {
    accepted("", &assign("tag", "_event.data.one.kind"));
    accepted("", &assign("tag", "_event.data.many[0].kind"));
    accepted(
        "_event.data.one.kind === Kind.dot",
        &assign("tag", "_event.data.one.kind"),
    );
    accepted(
        "",
        r#"<if cond="Kind.rect !== _event.data.many[1].kind"><assign location="seen" expr="1"/></if>"#,
    );
}

#[test]
fn a_field_of_a_case_is_refused_where_no_test_proves_its_case() {
    refused(
        "",
        &assign("seen", "_event.data.one.radius"),
        unproven,
        &[
            "_event.data.one.radius",
            "the case(s) circle, disc declare",
            "_event.data.one.kind === Kind.<case>",
        ],
    );
}

#[test]
fn a_field_no_case_declares_is_no_member() {
    refused(
        "",
        &assign("seen", "_event.data.one.depth"),
        unknown_member,
        &["depth", "height", "kind", "radius", "width"],
    );
}

#[test]
fn an_if_proves_its_case_in_its_block() {
    accepted(
        "",
        r#"<if cond="_event.data.one.kind === Kind.circle">
             <assign location="seen" expr="_event.data.one.radius"/>
           </if>"#,
    );
    refused(
        "",
        r#"<if cond="_event.data.one.kind !== Kind.circle">
             <assign location="seen" expr="_event.data.one.radius"/>
           </if>"#,
        unproven,
        &["_event.data.one.radius"],
    );
}

#[test]
fn an_else_is_read_in_the_cases_no_test_of_the_chain_named() {
    // After a dot and a rect, a circle and a disc are left, and both declare
    // `radius` with one type.
    accepted(
        "",
        r#"<if cond="_event.data.one.kind === Kind.dot">
             <assign location="seen" expr="0"/>
           <elseif cond="_event.data.one.kind === Kind.rect"/>
             <assign location="seen" expr="_event.data.one.width"/>
           <else/>
             <assign location="seen" expr="_event.data.one.radius"/>
           </if>"#,
    );
    // After a dot alone a rect is left too, which declares no `radius`.
    refused(
        "",
        r#"<if cond="_event.data.one.kind === Kind.dot">
             <assign location="seen" expr="0"/>
           <else/>
             <assign location="seen" expr="_event.data.one.radius"/>
           </if>"#,
        unproven,
        &["_event.data.one.radius"],
    );
}

#[test]
fn a_field_common_to_the_cases_a_disjunction_leaves_is_read() {
    accepted(
        "",
        r#"<if cond="_event.data.one.kind === Kind.circle || _event.data.one.kind === Kind.disc">
             <assign location="seen" expr="_event.data.one.radius"/>
           </if>"#,
    );
    refused(
        "",
        r#"<if cond="_event.data.one.kind === Kind.circle || _event.data.one.kind === Kind.rect">
             <assign location="seen" expr="_event.data.one.radius"/>
           </if>"#,
        unproven,
        &["_event.data.one.radius"],
    );
}

#[test]
fn the_right_operand_of_and_is_read_where_the_left_is_true() {
    accepted(
        "",
        r#"<if cond="_event.data.one.kind === Kind.rect &amp;&amp; _event.data.one.width &gt; 3">
             <assign location="seen" expr="1"/>
           </if>"#,
    );
    refused(
        "",
        r#"<if cond="_event.data.one.kind !== Kind.rect &amp;&amp; _event.data.one.width &gt; 3">
             <assign location="seen" expr="1"/>
           </if>"#,
        unproven,
        &["_event.data.one.width"],
    );
}

#[test]
fn the_right_operand_of_or_is_read_where_the_left_is_false() {
    accepted(
        "",
        r#"<if cond="_event.data.one.kind !== Kind.rect || _event.data.one.width &gt; 3">
             <assign location="seen" expr="1"/>
           </if>"#,
    );
    accepted(
        "",
        r#"<if cond="!(_event.data.one.kind === Kind.rect) || _event.data.one.width &gt; 3">
             <assign location="seen" expr="1"/>
           </if>"#,
    );
    refused(
        "",
        r#"<if cond="_event.data.one.kind === Kind.rect || _event.data.one.width &gt; 3">
             <assign location="seen" expr="1"/>
           </if>"#,
        unproven,
        &["_event.data.one.width"],
    );
}

#[test]
fn the_branches_of_a_conditional_are_read_where_its_condition_is_true_and_false() {
    accepted(
        "",
        &assign(
            "seen",
            "_event.data.one.kind === Kind.rect ? _event.data.one.width : 0",
        ),
    );
    accepted(
        "",
        &assign(
            "seen",
            "_event.data.one.kind !== Kind.rect ? 0 : _event.data.one.height",
        ),
    );
    refused(
        "",
        &assign(
            "seen",
            "_event.data.one.kind === Kind.rect ? 0 : _event.data.one.width",
        ),
        unproven,
        &["_event.data.one.width"],
    );
}

#[test]
fn a_guard_proves_a_payload_case_in_its_transitions_content() {
    accepted(
        "_event.data.one.kind === Kind.disc",
        &assign("seen", "_event.data.one.radius"),
    );
    refused(
        "_event.data.one.kind === Kind.dot",
        &assign("seen", "_event.data.one.radius"),
        unproven,
        &["_event.data.one.radius"],
    );
}

#[test]
fn an_element_at_an_index_is_no_subject_and_a_loop_item_is_one() {
    // An index may change between the test and the read.
    refused(
        "",
        r#"<if cond="_event.data.many[0].kind === Kind.circle">
             <assign location="seen" expr="_event.data.many[0].radius"/>
           </if>"#,
        unproven,
        &["_event.data.many[0].radius", "<foreach>"],
    );
    // The item of a <foreach> is a subject a test names.
    accepted(
        "",
        r#"<foreach array="_event.data.many" item="shape">
             <if cond="shape.kind === Kind.rect">
               <assign location="seen" expr="shape.height"/>
             </if>
           </foreach>"#,
    );
    refused(
        "",
        r#"<foreach array="_event.data.many" item="shape">
             <assign location="seen" expr="shape.height"/>
           </foreach>"#,
        unproven,
        &["shape.height", "shape.kind === Kind.<case>"],
    );
}

#[test]
fn a_union_that_may_be_absent_is_tested_present_then_by_its_tag() {
    accepted(
        "",
        r#"<if cond="_event.data.maybe !== null &amp;&amp; _event.data.maybe.value.kind === Kind.rect">
             <assign location="seen" expr="_event.data.maybe.value.height"/>
           </if>"#,
    );
    refused(
        "",
        r#"<if cond="_event.data.maybe !== null">
             <assign location="seen" expr="_event.data.maybe.value.height"/>
           </if>"#,
        unproven,
        &["_event.data.maybe.value.height"],
    );
}
