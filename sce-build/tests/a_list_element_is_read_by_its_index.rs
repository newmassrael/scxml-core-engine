// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: an element of a list variable, read by its index
// (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// A list is not a value an expression reads — it is filled by `<sce:append>`,
// emptied by `<sce:clear>`, measured by `len(…)` and walked by `<foreach>` —
// but one of its elements is a number like any other, and a screen that keeps
// a cursor into a list reads `days[cursor]`. The read is checked as an integer
// operation is: an index below zero or not below the length is a failure, the
// block ends and `error.execution` says so, and no backend reads past the end.
// What it is not is a way to write an element, or to read one that is not a
// number.

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

/// A statechart with a `list<uint8>` `picked`, a `list<int64>` `big`, a
/// `uint32` `total`, a `uint8` `small`, a `bool` `flag` and the given `body`
/// in one transition.
fn machine(body: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <datamodel>
    <data id="picked" sce:type="list&lt;uint8&gt;" sce:capacity="4"/>
    <data id="big" sce:type="list&lt;int64&gt;" sce:capacity="4"/>
    <data id="total" sce:type="uint32" expr="0"/>
    <data id="small" sce:type="uint8" expr="0"/>
    <data id="flag" sce:type="bool" expr="false"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      {body}
    </transition>
  </state>
</scxml>
"##
    )
}

/// `sce-codegen <args…> probe.scxml` over `doc`, as `(succeeded, output)`.
fn run(args: &[&str], doc: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
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

fn refused(body: &str, because: &str) {
    let (ok, out) = run(&["check"], &machine(body));
    assert!(!ok, "expected a refusal ({because}), got:\n{out}");
    assert!(
        out.contains(because),
        "the refusal must say {because:?}:\n{out}"
    );
}

#[test]
fn an_element_is_read_by_an_index_in_every_place_a_number_stands() {
    let body = r#"
      <assign location="small" expr="picked[0]"/>
      <assign location="small" expr="picked[len(picked) - 1]"/>
      <assign location="total" expr="picked[small] + picked[1] * 2"/>
      <assign location="flag" expr="picked[0] === picked[1]"/>
      <assign location="small" expr="big[0]"/>
      <assign location="small" expr="picked[picked[0]]"/>
      <assign location="total" expr="len(picked) - 1"/>
      <assign location="small" expr="len(picked)"/>
      <assign location="flag" expr="small &lt; len(picked)"/>
      <if cond="big[0] &gt; 0">
        <assign location="total" expr="picked[total]"/>
      </if>
      <foreach array="picked" item="v" index="i">
        <assign location="total" expr="total + picked[i] + v"/>
      </foreach>"#;
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
        &["check", "-l", "cpp"],
        &["check", "-l", "go"],
        &["check", "-l", "python"],
        &["check", "-l", "c11"],
    ] {
        let (ok, out) = run(args, &machine(body));
        assert!(ok, "{args:?}: an element read by its index:\n{out}");
    }
}

#[test]
fn an_element_is_typed_as_the_list_holds_it() {
    // A `uint8` element is a number and no truth value. (A wide element into a
    // narrow place is the checked narrowing every integer store has, not a
    // refusal.)
    refused(r#"<assign location="flag" expr="picked[0]"/>"#, "picked[0]");
    refused(
        r#"<if cond="big[0]"><assign location="small" expr="1"/></if>"#,
        "big[0]",
    );
}

#[test]
fn an_index_is_a_whole_number() {
    refused(
        r#"<assign location="small" expr="picked[true]"/>"#,
        "an index is a whole number",
    );
    refused(
        r#"<assign location="small" expr="picked['a']"/>"#,
        "an index is a whole number",
    );
}

#[test]
fn only_a_list_is_indexed_and_an_element_is_not_a_list() {
    refused(
        r#"<assign location="small" expr="total[0]"/>"#,
        "only a list, a byte string or a constant table is indexed",
    );
    refused(
        r#"<assign location="small" expr="picked[0][0]"/>"#,
        "only a list, a byte string or a constant table is indexed",
    );
}

#[test]
fn a_list_is_still_no_value_and_an_element_is_still_no_place() {
    refused(
        r#"<assign location="small" expr="picked"/>"#,
        "reading the list `picked` as a value",
    );
    refused(r#"<assign location="picked[0]" expr="1"/>"#, "picked[0]");
}

/// A statechart with a `list<record:Day>` `days`, a `record:Day` `draft`, a
/// `uint32` `total`, a `uint8` `small` and the given `body` in one transition,
/// beside the schema it imports.
fn record_machine(body: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    std::fs::copy(
        repo_root().join("sce-build/tests/fixtures/static_datamodel/schema_day.scxml"),
        dir.path().join("schema_day.scxml"),
    )
    .expect("the Day schema");
    let path = dir.path().join("probe.scxml");
    std::fs::write(
        &path,
        format!(
            r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <datamodel>
    <data id="days" sce:type="list&lt;record:Day&gt;" sce:capacity="3"/>
    <data id="draft" sce:type="record:Day">
      <sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="1"/>
      <sce:set name="dayOfMonth" expr="1"/>
    </data>
    <data id="total" sce:type="uint32" expr="0"/>
    <data id="small" sce:type="uint8" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      {body}
    </transition>
  </state>
</scxml>
"##
        ),
    )
    .expect("write probe");
    (dir, path)
}

fn check_records(args: &[&str], body: &str) -> (bool, String) {
    let (_dir, path) = record_machine(body);
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

#[test]
fn a_field_of_an_element_is_read_by_the_elements_index() {
    let body = r#"
      <assign location="small" expr="days[0].dayOfMonth"/>
      <assign location="total" expr="days[len(days) - 1].year + days[small].month * 2"/>
      <if cond="days[0].dayOfMonth &lt; days[1].dayOfMonth">
        <assign location="small" expr="1"/>
      </if>"#;
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
        &["check", "-l", "cpp"],
        &["check", "-l", "go"],
        &["check", "-l", "python"],
        &["check", "-l", "c11"],
    ] {
        let (ok, out) = check_records(args, body);
        assert!(ok, "{args:?}: a field of an indexed element:\n{out}");
    }
}

/// A statechart with a `list<record:Labelled>` `labels` — a record with a string
/// field of at most eight bytes — a `string` `note` of sixteen, a `bool` `same` and
/// the given `body` in one transition, beside the schema it imports.
fn labelled_machine(body: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    std::fs::copy(
        repo_root().join("sce-build/tests/fixtures/static_datamodel/schema_labelled.scxml"),
        dir.path().join("schema_labelled.scxml"),
    )
    .expect("the Labelled schema");
    let path = dir.path().join("probe.scxml");
    std::fs::write(
        &path,
        format!(
            r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_labelled.scxml" as="Labelled"/>
  <datamodel>
    <data id="labels" sce:type="list&lt;record:Labelled&gt;" sce:capacity="3"/>
    <data id="note" sce:type="string" sce:capacity="16" expr="'hello'"/>
    <data id="same" sce:type="bool" expr="false"/>
    <data id="small" sce:type="uint8" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      {body}
    </transition>
  </state>
</scxml>
"##
        ),
    )
    .expect("write probe");
    (dir, path)
}

fn check_labelled(args: &[&str], body: &str) -> (bool, String) {
    let (_dir, path) = labelled_machine(body);
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

#[test]
fn a_string_field_of_an_element_is_read_by_the_elements_index() {
    let body = r#"
      <assign location="note" expr="labels[0].label"/>
      <assign location="note" expr="labels[len(labels) - 1].label"/>
      <assign location="same" expr="labels[0].label === labels[1].label"/>
      <if cond="labels[small].label === 'a'">
        <assign location="small" expr="labels[0].sensor"/>
      </if>"#;
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
        &["check", "-l", "cpp"],
        &["check", "-l", "go"],
        &["check", "-l", "python"],
        &["check", "-l", "c11"],
    ] {
        let (ok, out) = check_labelled(args, body);
        assert!(ok, "{args:?}: a string field of an indexed element:\n{out}");
    }
}

#[test]
fn an_element_of_a_list_of_records_is_still_no_value() {
    for (body, because) in [
        (
            r#"<assign location="draft" expr="days[0]"/>"#,
            "a whole record of the schema Day is taken by name",
        ),
        (
            r#"<assign location="small" expr="days[0].nothere"/>"#,
            "days[0].nothere",
        ),
        (
            r#"<assign location="days[0].year" expr="1"/>"#,
            "days[0].year",
        ),
    ] {
        let (ok, out) = check_records(&["check"], body);
        assert!(!ok, "expected a refusal ({because}), got:\n{out}");
        assert!(
            out.contains(because),
            "the refusal must say {because:?}:\n{out}"
        );
    }
}
