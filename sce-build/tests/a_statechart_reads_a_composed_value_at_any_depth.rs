// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A `sce-static` statechart reads a composed payload as it reads a variable of
//! the same type, at any depth (docs/adr/0086, section 6): every place is typed
//! by walking the one value type, each record's fields in the scope of the
//! document that declares it.
//!
//! # What is held
//!
//! * Where the document is read (the parse, which judges every expression):
//!   `len(x)` of a list at any depth, an index into a list of any element, a
//!   field of a record element, a `<foreach>` over a list the payload, a
//!   variable or an enclosing loop's item holds, and a list taken whole into a
//!   list variable whose type the value's fits. Each place's type is the walk's:
//!   a value of one type does not stand where another is expected, at depth 1,
//!   2 and 3.
//! * A list that may not fit the variable it is taken into is
//!   `scxml/static-datamodel-rule`, naming the first place it may not and the
//!   narrowing, an `<sce:append>` of each element in a `<foreach>`.
//! * A composed value as an operand, or two compared, is
//!   `expression/unsupported-construct`; tested against `null` it is read.
//! * What no backend lowers is refused by name where the machine is generated:
//!   the gate every route asks first refuses the fixture `static_list_payload`
//!   and a variable no slot holds with `generate/unsupported-feature`, naming
//!   the field and the position.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::forge::error::{ForgeError, Located};
use sce_build::model::SCXMLModel;
use tempfile::tempdir;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel")
}

fn code_of(refusal: &Located<ForgeError>) -> String {
    serde_json::to_value(refusal.error.to_diagnostics()[0].code)
        .expect("a code serialises")
        .as_str()
        .expect("a code is a string")
        .to_string()
}

/// The schemas a reader of `batch.sent` imports, copied beside it.
const SIBLINGS: [&str; 3] = [
    "enum_signal_level.scxml",
    "event_batch_item.scxml",
    "event_batch.scxml",
];

/// `document`, written as `name` beside the batch schemas, read as a build
/// reads it: the parse judges every expression of a `sce-static` document. The
/// analysis a build runs before it asks the gate is run on what parses.
fn read(name: &str, document: &str) -> Result<SCXMLModel, Located<ForgeError>> {
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    let path = dir.path().join(name);
    std::fs::write(&path, document).expect("write the document");
    let path = path.to_str().expect("a utf-8 path");
    let mut model = sce_build::parser::SCXMLParser::new().parse_file(path)?;
    sce_build::analyzer::analyze(&mut model, path);
    Ok(model)
}

/// A reader of `batch.sent` that imports its schema as `Batch` and the item's
/// as `Item`, declares `data` beside a `uint32` `seen`, a `uint8` `cell` and a
/// `bool` `flag`, and runs `body` on the event.
fn reader(data: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="batch_reader">
  <sce:import kind="event-schema" src="event_batch.scxml" as="Batch"/>
  <sce:import kind="event-schema" src="event_batch_item.scxml" as="Item"/>
  <datamodel>
    <data id="seen" sce:type="uint32" expr="0"/>
    <data id="cell" sce:type="uint8" expr="0"/>
    <data id="flag" sce:type="bool" expr="false"/>
    {data}
  </datamodel>
  <state id="idle">
    <transition event="batch.sent" target="idle">
      {body}
    </transition>
  </state>
</scxml>
"#
    )
}

/// `body` read on `batch.sent` with `data` declared: accepted.
fn accepted(data: &str, body: &str) {
    if let Err(refusal) = read("batch_reader.scxml", &reader(data, body)) {
        panic!("accepted where the document is read:\n{body}\n{refusal}");
    }
}

/// `body` read on `batch.sent` with `data` declared: refused with `code`,
/// saying `says`.
fn refused(data: &str, body: &str, code: &str, says: &str) {
    let Err(refusal) = read("batch_reader.scxml", &reader(data, body)) else {
        panic!("refused where the document is read:\n{body}");
    };
    assert_eq!(code_of(&refusal), code, "{body}\n{refusal}");
    let text = refusal.to_string();
    assert!(text.contains(says), "{body}: expected `{says}` in\n{text}");
}

/// An assignment of `expr` to `location`.
fn assign(location: &str, expr: &str) -> String {
    format!(r#"<assign location="{location}" expr="{expr}"/>"#)
}

#[test]
fn a_length_is_read_of_a_list_at_any_depth() {
    // Depth 1, 2 and 3 of the payload: a field, an element, a field of an
    // element; each a list `len` measures as a `uint32`.
    for list in [
        "_event.data.items",
        "_event.data.grid[0]",
        "_event.data.items[0].tags",
    ] {
        accepted("", &assign("seen", &format!("len({list})")));
    }
}

#[test]
fn a_place_is_typed_by_its_type_at_every_depth() {
    // What each place holds stands where its own type does ...
    accepted("", &assign("cell", "_event.data.grid[1][0]"));
    accepted("", &assign("seen", "_event.data.items[0].id"));
    // ... and nowhere else: a text, a number and a number, at depth 1, 3 and 3,
    // where a `bool` is expected. A place the walk left untyped would stand
    // anywhere.
    for (depth, expr) in [
        (1, "_event.data.note"),
        (3, "_event.data.grid[1][0]"),
        (3, "_event.data.items[0].id"),
    ] {
        let body = assign("flag", expr);
        let Err(refusal) = read("batch_reader.scxml", &reader("", &body)) else {
            panic!("depth {depth}: `{expr}` is no bool");
        };
        assert_eq!(code_of(&refusal), "expression/type-mismatch", "{refusal}");
    }
}

#[test]
fn a_field_an_element_does_not_declare_is_no_place() {
    refused(
        "",
        &assign("seen", "_event.data.items[0].nope"),
        "expression/unsupported-construct",
        "a field the element's schema does not declare",
    );
}

#[test]
fn a_loop_walks_a_list_the_payload_or_an_enclosing_item_holds() {
    let kept = r#"<data id="kept" sce:type="list&lt;record:Item&gt;[3]"/>"#;
    // The payload's list, a list of an element of it, a list an enclosing
    // loop's item holds, and a list variable's item's list.
    accepted(
        kept,
        r#"<foreach array="_event.data.items" item="each" index="i">
             <assign location="seen" expr="seen + each.id + i"/>
             <foreach array="each.tags" item="word">
               <assign location="seen" expr="seen + len(_event.data.items[i].tags)"/>
             </foreach>
           </foreach>
           <foreach array="_event.data.items[0].tags" item="word"/>
           <foreach array="_event.data.grid" item="row">
             <foreach array="row" item="n">
               <assign location="cell" expr="n"/>
             </foreach>
           </foreach>
           <foreach array="kept" item="k">
             <foreach array="k.tags" item="word"/>
           </foreach>"#,
    );
    // A loop's item is the element's type: a number of a list of lists' list
    // is no bool.
    refused(
        "",
        r#"<foreach array="_event.data.grid" item="row">
             <foreach array="row" item="n">
               <assign location="flag" expr="n"/>
             </foreach>
           </foreach>"#,
        "expression/type-mismatch",
        "uint8 where",
    );
    // A text is no list to walk.
    refused(
        "",
        r#"<foreach array="_event.data.note" item="c"/>"#,
        "scxml/static-datamodel-rule",
        "walks a list variable, or a list the event's payload",
    );
}

#[test]
fn a_list_is_taken_whole_into_a_variable_whose_type_it_fits() {
    accepted(
        r#"<data id="kept" sce:type="list&lt;record:Item&gt;[3]"/>"#,
        &assign("kept", "_event.data.items"),
    );
    accepted(
        r#"<data id="tags" sce:type="list&lt;string[8]&gt;[4]"/>"#,
        &assign("tags", "_event.data.items[0].tags"),
    );
}

#[test]
fn a_list_that_may_not_fit_is_refused_naming_the_narrowing() {
    // Its bound: three items do not fit two.
    refused(
        r#"<data id="few" sce:type="list&lt;record:Item&gt;[2]"/>"#,
        &assign("few", "_event.data.items"),
        "scxml/static-datamodel-rule",
        "'_event.data.items' is a list<record:Item>[batch_items=3], which may hold more than \
         the list<record:Item>[2] expected there: `few` takes it only element by element, an \
         <sce:append> of each in a <foreach> over `_event.data.items`",
    );
    // Its element, at the first place it differs.
    refused(
        r#"<data id="ids" sce:type="list&lt;uint32&gt;[3]"/>"#,
        &assign("ids", "_event.data.items"),
        "scxml/static-datamodel-rule",
        "'_event.data.items/*' is a record:Item where a uint32 is expected",
    );
    // An inner bound: a tag of 8 bytes does not fit 4.
    refused(
        r#"<data id="short" sce:type="list&lt;string[4]&gt;[4]"/>"#,
        &assign("short", "_event.data.items[0].tags"),
        "scxml/static-datamodel-rule",
        "'_event.data.items[0].tags/*' is a string[8], which may hold more than the string[4]",
    );
}

#[test]
fn a_composed_value_is_no_operand() {
    refused(
        "",
        &assign("flag", "_event.data.grid[0] === _event.data.grid[1]"),
        "expression/unsupported-construct",
        "`_event.data.grid[0]`, a value of `list<uint8>[2]`, as an operand",
    );
    refused(
        "",
        &assign("seen", "len(_event.data.items) + _event.data.items"),
        "expression/unsupported-construct",
        "`_event.data.items`, a value of `list<record:Item>[batch_items=3]`, as an operand",
    );
    // A list that may be absent is tested against `null`, and read through.
    accepted(
        "",
        r#"<if cond="_event.data.spare !== null">
             <assign location="seen" expr="len(_event.data.spare.value)"/>
           </if>"#,
    );
}

#[test]
fn the_list_payload_fixture_is_read_and_refused_by_name_where_it_is_generated() {
    let text =
        std::fs::read_to_string(fixtures().join("static_list_payload.scxml")).expect("the fixture");
    let model = read("static_list_payload.scxml", &text)
        .unwrap_or_else(|refusal| panic!("every expression of the fixture is accepted: {refusal}"));
    let refusal = sce_build::analyzer::can_generate_static(&model, "static_list_payload.scxml")
        .expect_err("no backend lowers a composed read yet");
    assert_eq!(code_of(&refusal), "generate/unsupported-feature");
    // `kept` holds records of `Item`, which holds a list: the first use the gate
    // meets, before the reads of the payload.
    let text = refusal.to_string();
    assert!(
        text.contains("a statechart's variable") && text.contains("the field `tags`"),
        "{text}"
    );
}

#[test]
fn a_variable_no_backend_holds_is_read_and_refused_by_name_where_it_is_generated() {
    let document = reader(
        r#"<data id="rows" sce:type="list&lt;list&lt;uint8&gt;[2]&gt;[2]"/>"#,
        "",
    )
    .replace(
        r#"<transition event="batch.sent" target="idle">"#,
        r#"<transition event="go" target="idle"><assign location="seen" expr="len(rows[0])"/>"#,
    );
    let model = read("batch_reader.scxml", &document)
        .unwrap_or_else(|refusal| panic!("a list of lists is read at depth 2: {refusal}"));
    let refusal = sce_build::analyzer::can_generate_static(&model, "batch_reader.scxml")
        .expect_err("no variable lowers a list of lists yet");
    assert_eq!(code_of(&refusal), "generate/unsupported-feature");
    assert!(
        refusal
            .to_string()
            .contains("`list<uint8>[2]` in a statechart's variable is a type of the language"),
        "{refusal}"
    );
}
