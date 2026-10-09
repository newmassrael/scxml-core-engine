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
    for args in EVERY_TARGET {
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

/// A one-transition statechart over a list of records, beside the files it imports:
/// `files` are the schemas (and enums) copied from the fixtures next to the probe,
/// `imports` and `data` are the document's own lines for them, and `body` is the
/// transition's content.
fn list_machine(
    files: &[&str],
    imports: &str,
    data: &str,
    body: &str,
) -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    for file in files {
        std::fs::copy(
            repo_root()
                .join("sce-build/tests/fixtures/static_datamodel")
                .join(file),
            dir.path().join(file),
        )
        .unwrap_or_else(|e| panic!("the fixture {file}: {e}"));
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(
        &path,
        format!(
            r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  {imports}
  <datamodel>
    {data}
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

/// `sce-codegen <args…>` over a probe `list_machine` wrote, as `(succeeded, output)`.
fn check_machine(args: &[&str], machine: &(tempfile::TempDir, PathBuf)) -> (bool, String) {
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(&machine.1)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// Every target a `sce-static` document is checked for, the Interpreter's own check first.
const EVERY_TARGET: [&[&str]; 7] = [
    &["check"],
    &["check", "-l", "rust"],
    &["check", "-l", "kotlin"],
    &["check", "-l", "cpp"],
    &["check", "-l", "go"],
    &["check", "-l", "python"],
    &["check", "-l", "c11"],
];

/// A statechart with a `list<record:Day>` `days`, a `record:Day` `draft`, a
/// `uint32` `total`, a `uint8` `small` and the given `body` in one transition,
/// beside the schema it imports.
fn record_machine(body: &str) -> (tempfile::TempDir, PathBuf) {
    list_machine(
        &["schema_day.scxml"],
        r#"<sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>"#,
        r#"<data id="days" sce:type="list&lt;record:Day&gt;" sce:capacity="3"/>
    <data id="draft" sce:type="record:Day">
      <sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="1"/>
      <sce:set name="dayOfMonth" expr="1"/>
    </data>
    <data id="total" sce:type="uint32" expr="0"/>
    <data id="small" sce:type="uint8" expr="0"/>"#,
        body,
    )
}

fn check_records(args: &[&str], body: &str) -> (bool, String) {
    check_machine(args, &record_machine(body))
}

#[test]
fn a_field_of_an_element_is_read_by_the_elements_index() {
    let body = r#"
      <assign location="small" expr="days[0].dayOfMonth"/>
      <assign location="total" expr="days[len(days) - 1].year + days[small].month * 2"/>
      <if cond="days[0].dayOfMonth &lt; days[1].dayOfMonth">
        <assign location="small" expr="1"/>
      </if>"#;
    for args in EVERY_TARGET {
        let (ok, out) = check_records(args, body);
        assert!(ok, "{args:?}: a field of an indexed element:\n{out}");
    }
}

/// A statechart with a `list<record:Labelled>` `labels` — a record with a string
/// field of at most eight bytes — a `string` `note` of sixteen, a `bool` `same` and
/// the given `body` in one transition, beside the schema it imports.
fn labelled_machine(body: &str) -> (tempfile::TempDir, PathBuf) {
    list_machine(
        &["schema_labelled.scxml"],
        r#"<sce:import kind="event-schema" src="schema_labelled.scxml" as="Labelled"/>"#,
        r#"<data id="labels" sce:type="list&lt;record:Labelled&gt;" sce:capacity="3"/>
    <data id="note" sce:type="string" sce:capacity="16" expr="'hello'"/>
    <data id="same" sce:type="bool" expr="false"/>
    <data id="small" sce:type="uint8" expr="0"/>"#,
        body,
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
    for args in EVERY_TARGET {
        let (ok, out) = check_machine(args, &labelled_machine(body));
        assert!(ok, "{args:?}: a string field of an indexed element:\n{out}");
    }
}

/// A statechart with a `list<record:Framed>` `frames` — a record with a byte string
/// field of at most eight bytes — a `bytes` `spare` of sixteen, a `uint32` `size`, a
/// `bool` `same`, a `uint8` `small` and the given `body` in one transition.
fn framed_machine(body: &str) -> (tempfile::TempDir, PathBuf) {
    list_machine(
        &["schema_framed.scxml"],
        r#"<sce:import kind="event-schema" src="schema_framed.scxml" as="Framed"/>"#,
        r#"<data id="frames" sce:type="list&lt;record:Framed&gt;" sce:capacity="3"/>
    <data id="spare" sce:type="bytes" sce:capacity="16" expr="'hello'"/>
    <data id="size" sce:type="uint32" expr="0"/>
    <data id="same" sce:type="bool" expr="false"/>
    <data id="small" sce:type="uint8" expr="0"/>"#,
        body,
    )
}

#[test]
fn a_byte_string_field_of_an_element_is_read_by_the_elements_index() {
    let body = r#"
      <assign location="spare" expr="frames[0].frame"/>
      <assign location="size" expr="len(frames[len(frames) - 1].frame)"/>
      <assign location="same" expr="frames[0].frame === frames[1].frame"/>
      <if cond="frames[small].frame === 'ab'">
        <assign location="small" expr="frames[0].sensor"/>
      </if>"#;
    for args in EVERY_TARGET {
        let (ok, out) = check_machine(args, &framed_machine(body));
        assert!(
            ok,
            "{args:?}: a byte string field of an indexed element:\n{out}"
        );
    }
}

/// A statechart with a `list<record:View>` `seen` — a record with an enum field —
/// a `record:View` `shown`, a `uint32` `leads`, a `uint8` `small` and the given
/// `body` in one transition.
fn viewed_machine(body: &str) -> (tempfile::TempDir, PathBuf) {
    list_machine(
        &["schema_view.scxml", "enum_view_mode.scxml"],
        r#"<sce:import kind="event-schema" src="schema_view.scxml" as="View"/>
  <sce:import kind="enum" src="enum_view_mode.scxml" as="ViewMode"/>"#,
        r#"<data id="seen" sce:type="list&lt;record:View&gt;" sce:capacity="3"/>
    <data id="shown" sce:type="record:View">
      <sce:set name="layout" expr="ViewMode.month"/>
      <sce:set name="zoom" expr="1"/>
    </data>
    <data id="leads" sce:type="uint32" expr="0"/>
    <data id="small" sce:type="uint8" expr="0"/>"#,
        body,
    )
}

#[test]
fn an_enum_field_of_an_element_is_read_by_the_elements_index() {
    let body = r#"
      <assign location="shown.layout" expr="seen[0].layout"/>
      <assign location="shown.layout" expr="seen[len(seen) - 1].layout"/>
      <if cond="seen[small].layout === ViewMode.week">
        <assign location="leads" expr="leads + 1"/>
      </if>
      <if cond="seen[0].layout !== seen[1].layout">
        <assign location="small" expr="seen[0].zoom"/>
      </if>"#;
    for args in EVERY_TARGET {
        let (ok, out) = check_machine(args, &viewed_machine(body));
        assert!(ok, "{args:?}: an enum field of an indexed element:\n{out}");
    }
}

#[test]
fn an_enum_value_taken_from_an_element_is_used_as_an_enum_value_is() {
    // The same rule as for any enum value: stored in a variable of its own enum, logged,
    // or compared with a value of that enum; no arithmetic, no index, no number.
    for (body, because) in [
        (
            r#"<assign location="small" expr="seen[0].layout"/>"#,
            "seen[0].layout",
        ),
        (
            r#"<if cond="seen[0].layout === 1"><assign location="small" expr="1"/></if>"#,
            "a comparison of an enum value with anything but a value of the same enum",
        ),
        (
            r#"<assign location="shown.layout" expr="seen[seen[0].layout].layout"/>"#,
            "an enum value as an index",
        ),
    ] {
        let (ok, out) = check_machine(&["check"], &viewed_machine(body));
        assert!(!ok, "expected a refusal ({because}), got:\n{out}");
        assert!(
            out.contains(because),
            "the refusal must say {because:?}:\n{out}"
        );
    }
}

/// An element is no value an expression holds, so it is taken whole only where a record of
/// its schema is written: the record is copied from it field by field, on every target.
#[test]
fn an_element_is_copied_whole_into_a_record_of_its_schema() {
    let body = r#"
      <assign location="draft" expr="days[0]"/>
      <assign location="draft" expr="days[len(days) - 1]"/>
      <assign location="draft" expr="days[small]"/>"#;
    for args in EVERY_TARGET {
        let (ok, out) = check_records(args, body);
        assert!(ok, "{args:?}: an element copied whole:\n{out}");
    }
}

/// The copy reads the index once for each field of the record it writes, so an index that
/// names the record would change under it; and a spelling that is not an element at an
/// index is a name or nothing.
#[test]
fn an_element_is_copied_only_where_the_copy_means_what_it_says() {
    for (body, because) in [
        (
            r#"<assign location="draft" expr="days[draft.dayOfMonth]"/>"#,
            "the index of the element reads `draft`",
        ),
        (
            r#"<assign location="draft" expr="days[0].year"/>"#,
            "a whole record of the schema Day is taken by name",
        ),
        (
            r#"<assign location="draft" expr="small[0]"/>"#,
            "a whole record of the schema Day is taken by name",
        ),
        (
            r#"<assign location="draft" expr="days[0] + days[1]"/>"#,
            "a whole record of the schema Day is taken by name",
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

#[test]
fn an_element_of_another_schema_is_no_source_for_a_record() {
    let machine = list_machine(
        &["schema_day.scxml", "schema_labelled.scxml"],
        r#"<sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <sce:import kind="event-schema" src="schema_labelled.scxml" as="Labelled"/>"#,
        r#"<data id="days" sce:type="list&lt;record:Day&gt;" sce:capacity="3"/>
    <data id="tag" sce:type="record:Labelled">
      <sce:set name="sensor" expr="1"/>
      <sce:set name="label" expr="'a'"/>
    </data>"#,
        r#"<assign location="tag" expr="days[0]"/>"#,
    );
    let (ok, out) = check_machine(&["check"], &machine);
    assert!(!ok, "expected a refusal, got:\n{out}");
    assert!(
        out.contains("an element of the list `days` is a record of the schema Day"),
        "the refusal must name the two schemas:\n{out}"
    );
}

#[test]
fn an_element_of_a_list_of_records_is_still_no_value() {
    for (body, because) in [
        (
            r#"<sce:append target="days" expr="days[0]"/>"#,
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
