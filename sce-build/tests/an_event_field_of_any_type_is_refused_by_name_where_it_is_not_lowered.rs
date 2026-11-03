// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An event schema's field holds any type of the one type grammar
//! (docs/adr/0086, section 1), and a position that does not lower a
//! composition yet refuses it by name where it reads the schema (section 7.3).
//!
//! # What is held
//!
//! * The parser reads a list of records that hold lists, a list of lists, a
//!   list of an enum, a list of optional values and an optional list as the
//!   field's type, every bound inside it, and its bound names (`[name=N]`).
//! * The revision rules of a composed field (section 10): a list a later
//!   revision adds takes no default and reads empty; a record a later revision
//!   adds is refused, since no literal spells one; a default on a composition is
//!   refused at the attribute.
//! * A composed field holds no one value, so it declares no quantity and no
//!   retained initial value.
//! * A statechart that imports a composed schema, and an algorithm that names
//!   one as a record, are refused by name with `generate/unsupported-feature`,
//!   naming the field and the type.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::forge::error::{ForgeError, Located};
use sce_build::forge::model::{ForgeDocument, SceType};
use sce_build::forge::value_type::{Bound, ValueType};
use sce_build::DocumentLabel;
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

fn code_of(refusal: &Located<ForgeError>) -> String {
    serde_json::to_value(refusal.error.to_diagnostics()[0].code)
        .expect("a code serialises")
        .as_str()
        .expect("a code is a string")
        .to_string()
}

/// `document` read as the forge parser reads it.
fn parsed(document: &str) -> Result<ForgeDocument, Located<ForgeError>> {
    sce_build::forge::parser::parse_forge_with_imports(document, DocumentLabel::symmetric("probe"))
        .map(|parsed| parsed.expect("a forge document").document)
}

/// An event schema of revision 2 whose one field `f` is typed `ty` (as the
/// document spells it, `<` escaped) and carries `extra` attributes.
fn schema_with(ty: &str, extra: &str) -> String {
    let ty = ty.replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="probe" sce:event-name="probe.sent" sce:revision="2">
  <sce:import kind="event-schema" src="event_batch_item.scxml" as="Item"/>
  <datamodel>
    <data id="first" sce:type="uint8" sce:direction="in"/>
    <data id="f" sce:type="{ty}" {extra} sce:direction="in"/>
  </datamodel>
</scxml>
"#
    )
}

fn list(elem: ValueType, max: Bound) -> ValueType {
    ValueType::List {
        elem: Box::new(elem),
        max,
    }
}

#[test]
fn a_field_holds_a_composition_of_any_depth_every_bound_inside_it() {
    let text = std::fs::read_to_string(fixtures().join("event_batch.scxml")).expect("the fixture");
    let Ok(ForgeDocument::EventSchema(schema)) = parsed(&text) else {
        panic!("event_batch is an event schema")
    };
    let declared = |id: &str| {
        schema
            .fields
            .iter()
            .find(|f| f.id == id)
            .unwrap_or_else(|| panic!("the field `{id}`"))
    };
    assert_eq!(
        declared("items").sce_type,
        list(
            ValueType::Record {
                alias: "Item".into()
            },
            Bound::named("batch_items", 3)
        )
    );
    assert_eq!(
        declared("grid").sce_type,
        list(
            list(ValueType::Scalar(SceType::Uint8), Bound::of(2)),
            Bound::of(2)
        )
    );
    assert_eq!(
        declared("maybe").sce_type,
        list(
            ValueType::Optional {
                elem: Box::new(ValueType::Scalar(SceType::Uint16))
            },
            Bound::of(2)
        )
    );
    assert_eq!(
        declared("spare").sce_type,
        ValueType::Optional {
            elem: Box::new(list(ValueType::Scalar(SceType::Uint8), Bound::of(2)))
        }
    );
    // A list the second revision added takes no default: it reads empty.
    let late = declared("late");
    assert_eq!(late.since, Some(2));
    assert_eq!(late.default, None);
    // No bound stands beside the type: every one is inside it.
    assert!(schema.fields.iter().all(|f| f.max_size.is_none()));
    // The leaf view names the first composed field.
    let composed = schema.entry_schema().expect_err("a composed schema");
    assert_eq!(composed.field, "items");
    assert_eq!(composed.composition.constructor, "list");
}

#[test]
fn a_list_a_later_revision_adds_takes_no_default() {
    let refusal = parsed(&schema_with(
        "list<uint8>[2]",
        r#"sce:since="2" sce:default="0""#,
    ))
    .expect_err("a default on a list");
    assert_eq!(code_of(&refusal), "validation/attribute-rule-violated");
    let said = refusal.to_string();
    assert!(said.contains("sce:default"), "{said}");
    assert!(said.contains("reads empty"), "{said}");
}

#[test]
fn a_record_a_later_revision_adds_is_refused_at_its_revision() {
    let refusal = parsed(&schema_with("record:Item", r#"sce:since="2""#))
        .expect_err("a record has no literal default");
    assert_eq!(code_of(&refusal), "validation/attribute-rule-violated");
    assert!(refusal.to_string().contains("sce:since"), "{refusal}");
    // Declared optional, it reads absent.
    assert!(parsed(&schema_with("optional<record:Item>", r#"sce:since="2""#)).is_ok());
}

#[test]
fn a_default_on_a_composition_is_refused_at_the_attribute() {
    let refusal = parsed(&schema_with(
        "optional<list<uint8>[2]>",
        r#"sce:since="2" sce:default="1""#,
    ))
    .expect_err("no literal spells a list");
    assert_eq!(code_of(&refusal), "validation/attribute-rule-violated");
    assert!(
        refusal.to_string().contains("no literal spells"),
        "{refusal}"
    );
}

#[test]
fn a_composed_field_holds_no_quantity_and_no_initial_value() {
    for extra in [r#"sce:quantity="m""#, r#"sce:retain="nvm" sce:initial="0""#] {
        let refusal = parsed(&schema_with("list<uint8>[2]", extra)).expect_err(extra);
        assert_eq!(
            code_of(&refusal),
            "validation/attribute-rule-violated",
            "{extra}"
        );
        assert!(refusal.to_string().contains("holds one value"), "{refusal}");
    }
}

/// `sce-codegen <args> <file>` over `documents` written beside the batch
/// fixtures, and whether it succeeded, with what it said.
fn run(args: &[&str], documents: &[(&str, &str)], file: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for name in [
        "enum_signal_level.scxml",
        "event_batch_item.scxml",
        "event_batch.scxml",
    ] {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    for (name, text) in documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(file))
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

const READER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="batch_reader">
  <sce:import kind="event-schema" src="event_batch.scxml" as="Batch"/>
  <datamodel>
    <data id="seen" sce:type="uint32" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="batch.sent" target="idle"/>
  </state>
</scxml>
"#;

#[test]
fn a_statechart_that_imports_a_composed_schema_is_refused_by_name() {
    let (ok, out) = run(
        &["generate", "-l", "rust"],
        &[("batch_reader.scxml", READER)],
        "batch_reader.scxml",
    );
    assert!(!ok, "a statechart reads no list yet:\n{out}");
    assert!(out.contains("generate/unsupported-feature"), "{out}");
    assert!(out.contains("the field `items`"), "{out}");
    assert!(out.contains("a statechart"), "{out}");
}

const ITEM_ID: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="item_id" version="1.0">
  <sce:import kind="event-schema" src="event_batch_item.scxml" as="Item"/>
  <sce:signature>
    <sce:param name="item" type="record:Item"/>
    <sce:return type="uint32"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="item.id"/>
  </sce:body>
</scxml>
"#;

#[test]
fn an_algorithm_that_names_a_composed_record_is_refused_by_name() {
    let (ok, out) = run(
        &["generate", "-l", "rust"],
        &[("item_id.scxml", ITEM_ID)],
        "item_id.scxml",
    );
    assert!(!ok, "an algorithm holds no record with a list yet:\n{out}");
    assert!(out.contains("generate/unsupported-feature"), "{out}");
    assert!(out.contains("the field `tags`"), "{out}");
    assert!(out.contains("an algorithm"), "{out}");
}
