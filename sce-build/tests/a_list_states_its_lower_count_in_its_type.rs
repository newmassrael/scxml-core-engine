// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A list states its lower count in its type, beside its bound
//! (docs/adr/0086, section 15).
//!
//! # What is held
//!
//! * An event schema's field reads `list<T>[M..N]` at any depth, the lower
//!   count in the model beside the bound, and a named bound pairs with it
//!   (`[M..name=N]`), the name moving the bound alone.
//! * A lower count past the bound is refused, as written and after a build's
//!   `--limit` moved the bound below it.
//! * A lower count where nothing reads it is refused where it stands: in the
//!   bracket of a text or a byte string, in a statechart variable's type, in
//!   an algorithm's slot, and on a list a later revision of its schema adds.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::forge::error::{ForgeError, Located};
use sce_build::forge::model::ForgeDocument;
use sce_build::forge::parser::parse_forge;
use sce_build::forge::value_type::{Bound, ValueType};
use sce_build::DocumentLabel;

fn label() -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: "probe",
        diagnostic_label: "probe.scxml",
    }
}

fn code_of(refusal: &Located<ForgeError>) -> String {
    serde_json::to_value(refusal.error.to_diagnostics()[0].code)
        .expect("a code serialises")
        .as_str()
        .expect("a code is a string")
        .to_string()
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// An event schema with no imports whose field `f` is typed `ty` (as the
/// grammar spells it, `<` escaped here) and carries `extra`, at the revision
/// `root` declares.
fn schema(ty: &str, extra: &str, root: &str) -> String {
    let ty = ty.replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="probe" sce:event-name="probe.sent"{root}>
  <datamodel>
    <data id="n" sce:type="uint32" sce:direction="in"/>
    <data id="f" sce:type="{ty}" {extra} sce:direction="in"/>
  </datamodel>
</scxml>
"#
    )
}

/// The type of the field `f` the parser read, or the refusal's code and
/// sentence.
fn field_type(document: &str) -> Result<ValueType, (String, String)> {
    match parse_forge(document, label()) {
        Ok(Some(ForgeDocument::EventSchema(model))) => Ok(model
            .fields
            .iter()
            .find(|f| f.id == "f")
            .expect("the field f")
            .sce_type
            .clone()),
        Ok(other) => panic!("not an event schema: {other:?}"),
        Err(refusal) => Err((code_of(&refusal), refusal.to_string())),
    }
}

/// `sce-codegen --error-format=json [--limit L]... check <path>`: whether it
/// succeeded, and what it wrote.
fn check(limits: &[&str], path: &Path) -> (bool, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sce-codegen"));
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json");
    for limit in limits {
        command.arg("--limit").arg(limit);
    }
    let out = command
        .arg("check")
        .arg(path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

#[test]
fn an_event_field_holds_a_lower_count_at_any_depth() {
    let held = field_type(&schema("list<uint8>[1..3]", "", "")).expect("a lower count");
    let ValueType::List { max, .. } = &held else {
        panic!("a list: {held:?}");
    };
    assert_eq!(*max, Bound::of(3).at_least(1));
    let nested = field_type(&schema("list<list<string[8]>[2..4]>[1..3]", "", "")).expect("nested");
    assert_eq!(nested.to_string(), "list<list<string[8]>[2..4]>[1..3]");
    let named = field_type(&schema("list<uint8>[1..probe_items=3]", "", "")).expect("named");
    let ValueType::List { max, .. } = &named else {
        panic!("a list: {named:?}");
    };
    assert_eq!(
        (max.min, max.max, max.limit.as_deref()),
        (1, 3, Some("probe_items"))
    );
    // A lower count of zero is the list that states none.
    assert_eq!(
        field_type(&schema("list<uint8>[0..3]", "", "")),
        field_type(&schema("list<uint8>[3]", "", ""))
    );
}

#[test]
fn a_lower_count_past_its_bound_is_refused() {
    for ty in ["list<uint8>[4..3]", "list<list<uint8>[5..4]>[3]"] {
        let (code, said) = field_type(&schema(ty, "", "")).expect_err(ty);
        assert_eq!(code, "validation/attribute-rule-violated", "{ty}");
        assert!(said.contains("is past its bound"), "{ty}: {said}");
    }
}

#[test]
fn a_limit_moved_below_the_lower_count_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, schema("list<uint8>[3..probe_items=8]", "", "")).expect("write");
    let (ok, out) = check(&["probe_items=5"], &path);
    assert!(ok, "a bound moved to 5 keeps the lower count 3:\n{out}");
    let (ok, out) = check(&["probe_items=2"], &path);
    assert!(!ok, "a bound moved to 2 leaves no value:\n{out}");
    assert!(
        out.contains("\"code\":\"validation/attribute-rule-violated\"")
            && out.contains("is past its bound"),
        "{out}"
    );
}

#[test]
fn a_text_and_a_byte_string_take_no_lower_count() {
    for ty in ["string[1..8]", "bytes[0..4]", "list<string[1..8]>[3]"] {
        let (code, said) = field_type(&schema(ty, "", "")).expect_err(ty);
        assert_eq!(code, "validation/attribute-rule-violated", "{ty}");
        assert!(said.contains("takes no lower count"), "{ty}: {said}");
    }
}

#[test]
fn a_list_a_later_revision_adds_states_no_lower_count() {
    let added = schema(
        "list<uint8>[1..3]",
        r#"sce:since="2""#,
        r#" sce:revision="2""#,
    );
    let (code, said) = field_type(&added).expect_err("a lower count on an added list");
    assert_eq!(code, "validation/attribute-rule-violated");
    assert!(said.contains("a later revision adds"), "{said}");
    // One that states none reads empty from an earlier delivery, and an
    // optional one reads absent.
    for ty in ["list<uint8>[3]", "optional<list<uint8>[1..3]>"] {
        let added = schema(ty, r#"sce:since="2""#, r#" sce:revision="2""#);
        assert!(field_type(&added).is_ok(), "{ty}: {:?}", field_type(&added));
    }
}

/// A `sce-static` statechart whose one variable is typed `ty`.
fn statechart(ty: &str) -> String {
    let ty = ty.replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="s" datamodel="sce-static" name="probe">
  <datamodel>
    <data id="kept" sce:type="{ty}"/>
  </datamodel>
  <state id="s"/>
</scxml>
"#
    )
}

#[test]
fn a_variable_states_no_lower_count() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, statechart("list<uint8>[3]")).expect("write");
    let (ok, out) = check(&[], &path);
    assert!(ok, "a list variable that states no lower count:\n{out}");
    for ty in ["list<uint8>[1..3]", "list<list<uint8>[1..2]>[3]"] {
        std::fs::write(&path, statechart(ty)).expect("write");
        let (ok, out) = check(&[], &path);
        assert!(!ok, "{ty} in a variable:\n{out}");
        assert!(
            out.contains("\"code\":\"validation/attribute-rule-violated\"")
                && out.contains("starts empty and is filled an element at a time"),
            "{ty}:\n{out}"
        );
    }
}

/// An algorithm returning a list built in a local, the return typed `ret`
/// and the local `local`.
fn algorithm(ret: &str, local: &str) -> String {
    let (ret, local) = (
        ret.replace('<', "&lt;").replace('>', "&gt;"),
        local.replace('<', "&lt;").replace('>', "&gt;"),
    );
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe" version="1.0">
  <sce:signature>
    <sce:param name="n" type="int64"/>
    <sce:return type="{ret}"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="{local}"/>
    <sce:append target="out" expr="n"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#
    )
}

#[test]
fn an_algorithm_slot_states_no_lower_count() {
    assert!(parse_forge(&algorithm("list<int64>[4]", "list<int64>[4]"), label()).is_ok());
    for (ret, local) in [
        ("list<int64>[1..4]", "list<int64>[4]"),
        ("list<int64>[4]", "list<int64>[1..4]"),
    ] {
        let refusal = parse_forge(&algorithm(ret, local), label()).expect_err(ret);
        assert_eq!(code_of(&refusal), "validation/attribute-rule-violated");
        assert!(
            refusal
                .to_string()
                .contains("starts empty and is filled an element at a time"),
            "{refusal}"
        );
    }
}
