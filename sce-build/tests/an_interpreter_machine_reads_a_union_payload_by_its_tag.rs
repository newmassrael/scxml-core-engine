// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Interpreter's lowering holds a union (docs/adr/0092, sections 4, 6.1 and
//! 7, the Interpreter row), held to
//! `sce-build/tests/fixtures/static_datamodel/static_union.scxml`:
//!
//! * A union is the wire's own object: its tag's key holding the declared name
//!   of the case's variant beside the case's fields.
//! * The door of an event whose payload holds one reads it by the descriptor of
//!   its type, its tag first and then the record of the case the tag names; a
//!   `<sce:lift>` reads a text into one by the same descriptor.
//! * A union variable is built where it is declared, the case's record with the
//!   tag's key set; a field of its case is written by an `<assign>` of the whole
//!   variable with that field changed; one is taken whole as it is.
//!
//! What the lowered machine does when the Interpreter runs it is held by
//! `tests/integration/AStaticDatamodelRunsLoweredUnderTheInterpreterTest.cpp`
//! (`AUnionIsReadBuiltWrittenAndLiftedAsTheWireObject`).

use std::path::{Path, PathBuf};

use sce_build::forge::static_js::lower_file;

fn manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The fixture at `relative` (under `sce-build/tests/fixtures`), lowered for
/// the Interpreter.
fn lowered(relative: &str) -> String {
    let path = manifest().join("tests/fixtures").join(relative);
    lower_file(path.to_str().expect("a utf-8 path"), Vec::new())
        .unwrap_or_else(|e| panic!("{relative} is not lowered for the Interpreter: {e:?}"))
}

/// The value of the attribute `name` of each SCXML element `element` of
/// `document` whose attribute `key` is `value`, in document order, as a reader
/// decodes it.
fn attributes(
    document: &roxmltree::Document<'_>,
    element: &str,
    (key, value): (&str, &str),
    name: &str,
) -> Vec<String> {
    document
        .descendants()
        .filter(|n| {
            n.is_element()
                && n.tag_name().name() == element
                && n.tag_name().namespace() == Some("http://www.w3.org/2005/07/scxml")
                && n.attribute(key) == Some(value)
        })
        .filter_map(|n| n.attribute(name).map(str::to_string))
        .collect()
}

/// The expression the library and the doors are installed by.
fn installer(document: &roxmltree::Document<'_>) -> String {
    attributes(document, "data", ("id", "SceStaticInstalled"), "expr")
        .pop()
        .expect("the library is installed")
}

/// Whether `text` holds `needle`, with `text` in the failure.
fn holds(text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "the lowering holds `{needle}`:\n{text}"
    );
}

/// The descriptor of `union:Shape` (`union_shape.scxml`), as the library reads
/// a value of it: each case by the declared name of its variant, in the tag
/// enum's order. Pinned by `JsComposed::read_union`
/// (`sce-build/src/forge/composed_lowering/ecmascript.rs`).
const SHAPE: &str = "{union: {tag: 'kind', cases: {'dot': null, 'circle': {record: {'radius': \
                     'uint16'}}, 'disc': {record: {'radius': 'uint16'}}, 'rect': {record: \
                     {'width': 'uint16', 'height': 'uint16'}}, 'label': {record: ";

/// Pinned by `ComposedLowering::ECMASCRIPT.unions` (`event_schema_check.rs`)
/// and `JsComposed::read_union`: the door of `shapes.sent` reads the whole
/// payload, each union of it by its descriptor, at the top, in a list and
/// where it may be absent.
#[test]
fn the_door_reads_each_union_of_the_payload_by_its_descriptor() {
    let out = lowered("static_datamodel/static_union.scxml");
    let document = roxmltree::Document::parse(&out).expect("well-formed");
    let installer = installer(&document);
    holds(
        &installer,
        &format!("SceStatic.door(\"shapes.sent\", {{record: {{'one': {SHAPE}"),
    );
    holds(&installer, &format!("'many': {{list: {SHAPE}"));
    holds(&installer, &format!("'maybe': {{optional: {SHAPE}"));
}

/// Pinned by `JsComposed::union_value` and the `UnionVar` start of
/// `union_variable_field` (`static_lowering.rs`): a built union starts at the
/// record of its case with the tag's key set, an optional one at `null`.
#[test]
fn a_union_variable_starts_as_the_object_of_its_case_or_absent() {
    let out = lowered("static_datamodel/static_union.scxml");
    let document = roxmltree::Document::parse(&out).expect("well-formed");
    assert_eq!(
        attributes(&document, "data", ("id", "built"), "expr"),
        ["SceStatic.set(({'width': 1, 'height': 2}), 'kind', 'rect')"]
    );
    assert_eq!(
        attributes(&document, "data", ("id", "kept"), "expr"),
        ["null"]
    );
}

/// Pinned by `JsComposed::union_member_write` and the note of the holder in
/// the `<assign>` branch of a union's field (`static_lowering.rs`): the field
/// of the case a test proves is written by an `<assign>` of the whole variable,
/// written again with that field changed; and a union taken whole is the
/// payload's object as it is.
#[test]
fn a_field_of_a_case_is_written_by_writing_the_variable_whole() {
    let out = lowered("static_datamodel/static_union.scxml");
    let document = roxmltree::Document::parse(&out).expect("well-formed");
    let built = attributes(&document, "assign", ("location", "built"), "expr");
    assert_eq!(
        built.len(),
        1,
        "one <assign> writes `built` whole: {built:?}"
    );
    holds(&built[0], "SceStatic.set(built, 'width', ");
    holds(&built[0], "built.width");
    assert!(
        attributes(&document, "assign", ("location", "built.width"), "expr").is_empty(),
        "no <assign> writes a field of the union in place:\n{out}"
    );
    let kept = attributes(&document, "assign", ("location", "kept"), "expr");
    assert_eq!(kept.len(), 1, "one <assign> takes `kept` whole: {kept:?}");
    assert!(
        kept[0].starts_with("SceStatic.taken('shapes.sent', _event.data)")
            && kept[0].ends_with(".one"),
        "`kept` takes the payload's union as it is: {}",
        kept[0]
    );
}

/// Pinned by the `Lifted::Whole` arm of `JsTarget::lift` (`static_js.rs`): a
/// text is lifted into the union by the descriptor a delivery of it is read by.
#[test]
fn a_text_is_lifted_into_the_union_by_its_descriptor() {
    let out = lowered("static_datamodel/static_union.scxml");
    let document = roxmltree::Document::parse(&out).expect("well-formed");
    let script = document
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "script")
        .filter_map(|n| n.text())
        .find(|text| text.contains("SceStatic.lift("))
        .unwrap_or_else(|| panic!("a <script> lifts the text:\n{out}"));
    holds(script, &format!(", {SHAPE}"));
    holds(script, "lifted = sceLift.value;");
}
