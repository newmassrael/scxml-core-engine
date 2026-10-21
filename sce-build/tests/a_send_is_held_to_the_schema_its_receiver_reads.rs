// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A `<param>` a `sce-static` statechart sends is held to the field that reads it,
//! where the build knows the reader (docs/SCE_ACCEPTED_SUBSET.md §2.15, "Child
//! sessions"): the schema the document itself declares the event by, and the schema
//! a child it starts declares the event by.
//!
//! # What was wrong
//!
//! Measured 2026-10-21: a parent that invokes the standard shared list
//! (`sce:std/shared/record_list.scxml`) and sends it `record_list.edit` with
//! `<param name="value" expr="_event.data.status"/>`, a value of an enum, passed
//! `check`. The parent sent the variant's declared name, a string, and the child,
//! whose `value` is an `int64`, refused every delivery when it read the field. Only a
//! literal was ever held to a schema, and only to the sender's own.

use std::path::{Path, PathBuf};
use std::process::Command;

fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The enum the probe holds a status in.
const STATUS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="enum" name="probe_status" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="open" value="2"/>
      <sce:variant name="done" value="7"/>
    </data>
  </datamodel>
</scxml>
"#;

/// The event that moves an item: an id and a status.
const MOVED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="probe_moved" sce:event-name="moved">
  <sce:import src="probe_status.scxml" kind="enum" as="Status"/>
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="status" sce:type="enum:Status" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A statechart that starts the standard shared list as `list` and, on `moved`,
/// sends `record_list.edit` to `target` with `value` as the field `value`. `imports`
/// are written beside the probe's own.
fn parent(imports: &str, target: &str, value: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="holding" datamodel="sce-static" name="probe">
  <sce:import kind="enum" src="probe_status.scxml" as="Status"/>
  <sce:import kind="event-schema" src="probe_moved.scxml" as="Moved"/>
  {imports}
  <datamodel>
    <data id="label" sce:type="string" sce:capacity="8" expr="'x'"/>
  </datamodel>
  <state id="holding">
    <invoke type="scxml" id="list" src="sce:std/shared/record_list.scxml">
      <param name="announce" expr="true"/>
    </invoke>
    <transition event="moved" type="internal">
      <send target="{target}" event="record_list.edit">
        <param name="id" expr="_event.data.id"/>
        <param name="field" expr="1"/>
        <param name="value" expr="{value}"/>
        <param name="text" expr="''"/>
        <param name="now" expr="0"/>
      </send>
    </transition>
  </state>
</scxml>
"#
    )
}

/// `sce-codegen check -l rust` of `probe` in a directory that holds the documents it
/// imports: the exit status and what it wrote.
fn check(probe: &str) -> (bool, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("probe_status.scxml"), STATUS).expect("write the enum");
    std::fs::write(dir.path().join("probe_moved.scxml"), MOVED).expect("write the schema");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, probe).expect("write the probe");
    let out = Command::new(codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["check", "-l", "rust"])
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// `probe` is refused as `scxml/static-datamodel-rule`, naming every one of `reasons`.
fn refused(probe: &str, reasons: &[&str]) {
    let (ok, out) = check(probe);
    assert!(!ok, "expected a refusal:\n{out}");
    assert!(
        out.contains("\"code\":\"scxml/static-datamodel-rule\""),
        "expected scxml/static-datamodel-rule:\n{out}"
    );
    for reason in reasons {
        assert!(out.contains(reason), "expected `{reason}` in:\n{out}");
    }
}

#[test]
fn a_value_of_an_enum_is_not_sent_where_the_child_reads_a_number() {
    refused(
        &parent("", "#_list", "_event.data.status"),
        &[
            "the child",
            "reads it as its field `value`",
            "the value is of the enum `Status`",
            "the field is int64",
        ],
    );
}

#[test]
fn a_string_is_not_sent_where_the_child_reads_a_number() {
    refused(
        &parent("", "#_list", "label"),
        &[
            "reads it as its field `value`",
            "the value is string",
            "the field is int64",
        ],
    );
}

/// The schema the document itself declares the event by holds a value it sends, to
/// any target, as it holds a literal.
#[test]
fn a_value_is_held_to_the_schema_the_sender_declares_the_event_by() {
    refused(
        &parent(
            r#"<sce:import kind="event-schema" src="sce:std/shared/record_edit.scxml" as="Edit"/>"#,
            "#_internal",
            "_event.data.status",
        ),
        &[
            "this document declares `record_list.edit` by",
            "the value is of the enum `Status`",
        ],
    );
}

/// What the field holds is accepted: a number of another width, whose width is held
/// when the delivery is read.
#[test]
fn a_number_is_sent_where_the_child_reads_a_number() {
    let (ok, out) = check(&parent("", "#_list", "_event.data.id"));
    assert!(ok, "a uint32 sent to an int64 field is accepted:\n{out}");
}
