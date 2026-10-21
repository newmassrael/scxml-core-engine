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

/// What a run of `sce-codegen check -l rust` of a probe gave back.
struct Run {
    ok: bool,
    /// Its stderr, then its stdout.
    out: String,
    /// The path the probe was named by on the command line.
    named: String,
}

/// `sce-codegen check -l rust` of `probe` in a directory that holds the documents it
/// imports, its refusals written in `format` (`json` or `human`).
fn run(probe: &str, format: &str) -> Run {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("probe_status.scxml"), STATUS).expect("write the enum");
    std::fs::write(dir.path().join("probe_moved.scxml"), MOVED).expect("write the schema");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, probe).expect("write the probe");
    let out = Command::new(codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg(format!("--error-format={format}"))
        .args(["check", "-l", "rust"])
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    Run {
        ok: out.status.success(),
        out: String::from_utf8_lossy(&out.stderr).into_owned()
            + &String::from_utf8_lossy(&out.stdout),
        named: path.display().to_string(),
    }
}

/// `sce-codegen check -l rust` of `probe`, its refusals written as records: the exit
/// status and what it wrote.
fn check(probe: &str) -> (bool, String) {
    let Run { ok, out, .. } = run(probe, "json");
    (ok, out)
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

/// A statechart that holds a written record of the list (`record_write`, whose text
/// is 96 bytes, `merge_text`) and a string of `capacity` bytes, and on `moved` sends
/// the list `record_list.edit` (whose text is 32, `shared_typed_text`) with `text` as
/// its text.
fn text_parent(capacity: u32, text: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="holding" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="probe_moved.scxml" as="Moved"/>
  <sce:import kind="enum" src="probe_status.scxml" as="Status"/>
  <sce:import kind="event-schema" src="sce:std/shared/record_write.scxml" as="Write"/>
  <datamodel>
    <data id="note" sce:type="string" sce:capacity="{capacity}" expr="'x'"/>
    <data id="written" sce:type="record:Write">
      <sce:set name="id" expr="0"/>
      <sce:set name="field" expr="0"/>
      <sce:set name="value" expr="0"/>
      <sce:set name="text" expr="''"/>
      <sce:set name="wallTime" expr="0"/>
      <sce:set name="counter" expr="0"/>
      <sce:set name="nodeId" expr="0"/>
      <sce:set name="parts" expr="1"/>
    </data>
  </datamodel>
  <state id="holding">
    <invoke type="scxml" id="list" src="sce:std/shared/record_list.scxml">
      <param name="announce" expr="true"/>
    </invoke>
    <transition event="moved" type="internal">
      <send target="#_list" event="record_list.edit">
        <param name="id" expr="_event.data.id"/>
        <param name="field" expr="1"/>
        <param name="value" expr="0"/>
        <param name="text" expr="{text}"/>
        <param name="now" expr="0"/>
      </send>
    </transition>
  </state>
</scxml>
"##
    )
}

/// The defect an application found (docs/adr/0050): the text of a record of 96 bytes
/// was sent as the list's 32-byte edit text, and `check` answered ok; the bound was
/// held only where the list read the delivery, which refused every one past 32.
#[test]
fn a_record_text_past_the_receivers_bound_is_refused() {
    refused(
        &text_parent(32, "written.text"),
        &[
            "reads it as its field `text`",
            "may hold 96 UTF-8 bytes",
            "holds at most 32",
            "sce:capacity of 32 or less",
        ],
    );
}

/// A variable's `sce:capacity` is the bound it declares, and a literal's is its own
/// length; one within the field's bound is sent as it is.
#[test]
fn a_text_is_held_to_the_bound_its_declaration_states() {
    refused(&text_parent(33, "note"), &["may hold 33 UTF-8 bytes"]);
    let (ok, out) = check(&text_parent(32, "note"));
    assert!(ok, "a 32-byte text into a 32-byte field is sent:\n{out}");
    let long = "'a text of thirty-three bytes ok!!'";
    refused(&text_parent(32, long), &["may hold 33 UTF-8 bytes"]);
    let (ok, out) = check(&text_parent(32, "note + 'x'"));
    assert!(!ok, "a join of 32 and 1 bytes is 33:\n{out}");
}

/// The narrowing the language offers is an assignment: a text taken into a variable
/// of the receiver's bound first (which fails as error.execution when it does not
/// fit) is sent as that variable.
#[test]
fn a_text_narrowed_into_a_variable_of_the_bound_is_sent() {
    let narrowed = text_parent(32, "note").replace(
        r##"<send target="#_list" event="record_list.edit">"##,
        r##"<assign location="note" expr="written.text"/>
      <send target="#_list" event="record_list.edit">"##,
    );
    let (ok, out) = check(&narrowed);
    assert!(
        ok,
        "the record's text narrowed by an assignment is sent:\n{out}"
    );
}

/// The 1-based row and column `needle` starts at in `text`.
fn written_at(text: &str, needle: &str) -> (u64, u64) {
    let offset = text.find(needle).expect("the probe spells the needle");
    let before = &text[..offset];
    let row = before.matches('\n').count() as u64 + 1;
    let col = before[before.rfind('\n').map_or(0, |i| i + 1)..]
        .chars()
        .count() as u64
        + 1;
    (row, col)
}

/// Measured 2026-10-21: an application's document of some thirty sends was refused
/// with the rule alone, and the `<param>` it was about had to be found by hand. The
/// record stands at the `<param>`, and the `human` form leads with the same place.
#[test]
fn the_refusal_names_the_param_it_stands_at() {
    let probe = text_parent(32, "written.text");
    let (row, col) = written_at(&probe, r#"<param name="text""#);

    let json = run(&probe, "json");
    assert!(!json.ok, "expected a refusal:\n{}", json.out);
    let record: serde_json::Value = json
        .out
        .lines()
        .filter(|line| line.starts_with('{'))
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).expect("one JSON record per line")
        })
        .find(|record| record["code"] == "scxml/static-datamodel-rule")
        .unwrap_or_else(|| panic!("expected scxml/static-datamodel-rule:\n{}", json.out));
    // The document is named as the command line named it.
    assert_eq!(
        (
            record["location"]["file"].as_str(),
            record["location"]["line"].as_u64(),
            record["location"]["col"].as_u64(),
        ),
        (Some(json.named.as_str()), Some(row), Some(col)),
        "the record does not stand at the <param>: {record}"
    );

    // The same place, written as a compiler writes it ahead of the rule.
    let human = run(&probe, "human");
    assert!(!human.ok, "expected a refusal:\n{}", human.out);
    let at = format!("{}:{row}:{col}: ", human.named);
    assert!(
        human
            .out
            .lines()
            .any(|line| line.starts_with(&at) && line.contains("<param name=\"text\">")),
        "the human refusal does not lead with `{at}`:\n{}",
        human.out
    );
}
