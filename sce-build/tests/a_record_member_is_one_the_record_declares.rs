// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A member read from a record is one the record declares.
//
// A record is a name whose members an expression may read: a stateful
// import's alias, an algorithm's item over a bounded collection, `_event`.
// Measured 2026-09-21 with the generator at ae8053923c: a procedure that
// imports a codec as `frame` and tests `frame.msgIdd === 1` generated with
// exit 0, and the Rust it emitted read `self.frame.msgIdd`, a field the
// codec's struct does not have. The one member check in the tree walked
// algorithms, where an import's alias is not a value at all.
//
// ⚠ THE NEGATIVE CONTROLS CARRY THE WEIGHT. A check that refused every
// member would pass the refusals here, so a declared field and a declared
// METHOD must both still generate — `frame.encode()` reads a member that
// is registered as a function, not a field, and is the likeliest thing a
// field-only check would refuse.

use sce_build::forge::error::{ExprError, ForgeError};
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .join("tests/forge/resources")
}

fn compile(name: &str, scxml: &str, options: &ForgeCompileOptions) -> Result<(), ForgeError> {
    sce_build::compile_forge_with_imports(
        scxml,
        DocumentLabel::symmetric(name),
        Language::Rust,
        &resource_dir(),
        options,
    )
    .map(|_| ())
    .map_err(|located| located.error)
}

/// The record, member and declared set `scxml` is refused with.
fn unknown_member(
    name: &str,
    scxml: &str,
    options: &ForgeCompileOptions,
) -> (String, String, Vec<String>) {
    match compile(name, scxml, options) {
        Ok(()) => panic!("{name}: generated, but must be refused"),
        Err(ForgeError::Expression(ExprError::UnknownMember {
            record,
            member,
            declared,
        })) => (record, member, declared),
        Err(other) => panic!("{name}: expected UnknownMember, got {other:?}"),
    }
}

/// A procedure importing `codec_simple_frame` (fields `msgId`, `length`,
/// `payload`) as `frame`, guarding its one transition with `cond`.
fn procedure(cond: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" initial="send" version="1.0">
  <sce:import src="codec_simple_frame.scxml" kind="codec" as="frame"/>
  <datamodel>
    <data id="msgId" sce:type="uint8" sce:direction="in"/>
  </datamodel>
  <state id="send">
    <onentry>
      <send sce:service="transport" sce:payload="frame.encode()"/>
    </onentry>
    <transition cond="{cond}" target="done"/>
  </state>
  <final id="done"/>
</scxml>"#
    )
}

#[test]
fn a_declared_field_and_a_declared_method_of_an_import_generate() {
    compile(
        "member_ok",
        &procedure("frame.msgId === 1"),
        &ForgeCompileOptions::default(),
    )
    .expect("a declared field beside a declared method generates");
}

#[test]
fn a_misspelled_field_of_an_import_is_refused_with_every_member() {
    let (record, member, declared) = unknown_member(
        "member_typo",
        &procedure("frame.msgIdd === 1"),
        &ForgeCompileOptions::default(),
    );
    assert_eq!(record, "frame");
    assert_eq!(member, "msgIdd");
    // The codec's fields and its `encode` method, sorted — the whole set
    // the alias may be asked for.
    assert_eq!(declared, ["encode", "length", "msgId", "payload"]);
}

/// An algorithm iterating `local_sub_table` (elements `subscription_entry`,
/// field `callback_id`) and comparing `entry.<member>` with its parameter.
fn collection_scan(member: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" version="1.0">
  <sce:import kind="bounded-collection" src="local_sub_table.scxml" as="subs"/>
  <sce:signature>
    <sce:param name="target" type="uint32"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:foreach item="entry" in="subs">
      <sce:if cond="entry.{member} === target">
        <sce:return expr="0"/>
      </sce:if>
    </sce:foreach>
    <sce:return expr="0xFFFF"/>
  </sce:body>
</scxml>"#
    )
}

/// A single document compiled on its own reads its collection's element
/// where the collection names it, as a document set that holds all three
/// does, so the item's members are judged on either route: the element's
/// schema declares `callback_id` and nothing else.
#[test]
fn a_misspelled_field_of_a_collection_item_is_refused_on_a_single_document() {
    compile(
        "item_ok",
        &collection_scan("callback_id"),
        &ForgeCompileOptions::default(),
    )
    .expect("a declared element field generates");
    let (record, member, declared) = unknown_member(
        "item_typo",
        &collection_scan("callback_idd"),
        &ForgeCompileOptions::default(),
    );
    assert_eq!(record, "entry");
    assert_eq!(member, "callback_idd");
    assert_eq!(declared, ["callback_id"]);
}

/// A collection whose element document is not beside it carries no element:
/// its item is an open record, whose members nothing judges, on every route
/// alike. This pins that branch rather than letting it pass by accident.
#[test]
fn a_collection_whose_element_is_not_beside_it_leaves_its_item_open() {
    let staged = tempfile::tempdir().expect("tempdir");
    let collection =
        std::fs::read_to_string(resource_dir().join("local_sub_table.scxml")).expect("fixture");
    std::fs::write(staged.path().join("local_sub_table.scxml"), collection).expect("stage");
    assert!(!staged.path().join("subscription_entry.scxml").exists());
    sce_build::compile_forge_with_imports(
        &collection_scan("callback_idd"),
        DocumentLabel::symmetric("item_open"),
        Language::Rust,
        staged.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("an item whose element document is absent is an open record");
}
