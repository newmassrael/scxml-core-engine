// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: the payload of an event taken whole as a record
// (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// `_event.data` names the payload of the event a transition is on, which is a
// record of that event's schema. It is accepted where a whole record of a schema
// is taken — an `<assign>` to a record variable, a `<sce:append>` to a list of
// records — when the schema the variable holds declares the same fields of the
// same types, and refused with a rule naming what it takes otherwise, in every
// language that lowers a payload.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn schema(name: &str, event: &str, fields: &[(&str, &str)]) -> String {
    let data: String = fields
        .iter()
        .map(|(id, ty)| format!("    <data id=\"{id}\" sce:type=\"{ty}\" sce:direction=\"in\"/>\n"))
        .collect();
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="{name}" sce:event-name="{event}">
  <datamodel>
{data}  </datamodel>
</scxml>
"##
    )
}

/// The schemas a probe may hold a record of. `pair.set` is the event the
/// transitions are on, and its payload is a `Pair`.
fn siblings() -> Vec<(&'static str, String)> {
    vec![
        (
            "schema_pair.scxml",
            schema(
                "schema_pair",
                "pair.set",
                &[("left", "uint8"), ("right", "uint8")],
            ),
        ),
        // The fields of `Pair`, declared the other way round.
        (
            "schema_swapped.scxml",
            schema(
                "schema_swapped",
                "pair.swapped",
                &[("right", "uint8"), ("left", "uint8")],
            ),
        ),
        // A field `Pair` does not declare.
        (
            "schema_trio.scxml",
            schema(
                "schema_trio",
                "pair.trio",
                &[("left", "uint8"), ("right", "uint8"), ("depth", "uint8")],
            ),
        ),
        // The fields of `Pair`, one of another type.
        (
            "schema_wide.scxml",
            schema(
                "schema_wide",
                "pair.wide",
                &[("left", "uint16"), ("right", "uint8")],
            ),
        ),
    ]
}

/// A statechart importing `Pair`, the schema `pair.set` carries, and `held`'s
/// own schema, holding a record `held` and a list `log` of it, with `body` on
/// `event`.
fn machine(held: &str, src: &str, event: &str, body: &str) -> String {
    let held_import = if held == "Pair" {
        String::new()
    } else {
        format!(r#"<sce:import kind="event-schema" src="{src}" as="{held}"/>"#)
    };
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_pair.scxml" as="Pair"/>
  {held_import}
  <datamodel>
    <data id="held" sce:type="record:{held}" sce:direction="out">
      <sce:set name="left" expr="0"/>
      <sce:set name="right" expr="0"/>
    </data>
    <data id="log" sce:type="list&lt;record:{held}&gt;" sce:capacity="2" sce:direction="out"/>
  </datamodel>
  <state id="s">
    <transition event="{event}" type="internal">
      {body}
    </transition>
  </state>
</scxml>
"##
    )
}

/// A record of `Trio`, which declares `depth` besides the fields `machine`
/// gives `held` a value for: the record is closed over its fields, so it takes
/// one for that too.
fn machine_with_depth(body: &str) -> String {
    machine("Trio", "schema_trio.scxml", "pair.set", body).replace(
        r#"<sce:set name="right" expr="0"/>"#,
        r#"<sce:set name="right" expr="0"/>
      <sce:set name="depth" expr="0"/>"#,
    )
}

fn run(args: &[&str], doc: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for (name, text) in siblings() {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

fn refused(doc: &str, because: &str) {
    let (ok, out) = run(&["check"], doc);
    assert!(!ok, "expected a refusal ({because}), got:\n{out}");
    assert!(
        out.contains(because),
        "the refusal must say {because:?}:\n{out}"
    );
}

const WHOLE: &str = r#"<assign location="held" expr="_event.data"/>
      <sce:append target="log" expr="_event.data"/>"#;

#[test]
fn the_payload_is_taken_whole_by_a_record_of_its_own_schema() {
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
        &["check", "-l", "go"],
        &["check", "-l", "python"],
        &["check", "-l", "cpp"],
        &["check", "-l", "c11"],
    ] {
        let (ok, out) = run(
            args,
            &machine("Pair", "schema_pair.scxml", "pair.set", WHOLE),
        );
        assert!(ok, "{args:?}: a payload taken whole:\n{out}");
    }
}

#[test]
fn the_payload_is_taken_whole_by_a_record_of_a_schema_with_the_same_fields_in_another_order() {
    // The shape is the fields and their types, not the order the schema writes
    // them in.
    let (ok, out) = run(
        &["check", "-l", "rust"],
        &machine("Swapped", "schema_swapped.scxml", "pair.set", WHOLE),
    );
    assert!(ok, "the same fields, declared the other way round:\n{out}");
}

#[test]
fn the_payload_is_no_record_of_a_schema_that_declares_another_field() {
    let because = "the payload of the event is not a record of the schema Trio";
    refused(
        &machine_with_depth(r#"<assign location="held" expr="_event.data"/>"#),
        because,
    );
    refused(
        &machine_with_depth(r#"<sce:append target="log" expr="_event.data"/>"#),
        because,
    );
}

#[test]
fn the_payload_is_no_record_of_a_schema_that_gives_a_field_another_type() {
    refused(
        &machine("Wide", "schema_wide.scxml", "pair.set", WHOLE),
        "the payload of the event is not a record of the schema Wide",
    );
}

#[test]
fn an_event_that_carries_no_schema_has_no_payload_to_take_whole() {
    // `tick` declares no schema, so `_event.data` on it is no record of one: the
    // rule says which names a whole record is taken by.
    refused(
        &machine("Pair", "schema_pair.scxml", "tick", WHOLE),
        "a whole record of the schema Pair is taken by name",
    );
}
