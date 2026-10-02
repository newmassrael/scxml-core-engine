// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: a variable declared `list<record:<alias>>`
// (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// A record is built by its `<sce:set>`s and updated a field at a time, so a
// list takes one whole, by the name of a record of its schema: a record
// variable, or the record item of a `<foreach>` over a list of the same. What
// a loop binds is read and not written, and a record item's fields are the
// schema's. Each refusal here is one that would otherwise reach generated code
// that does not compile.

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

const DAY: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_day" sce:event-name="day.picked">
  <datamodel>
    <data id="year" sce:type="uint16" sce:direction="in"/>
    <data id="month" sce:type="uint8" sce:direction="in"/>
    <data id="dayOfMonth" sce:type="uint8" sce:direction="in"/>
  </datamodel>
</scxml>
"##;

/// A statechart importing `schema_day.scxml` as `Day`, with a record `draft`,
/// two lists of `Day`, a `uint32` `total` and the given transition `body`.
fn machine(body: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <datamodel>
    <data id="draft" sce:type="record:Day">
      <sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="1"/>
      <sce:set name="dayOfMonth" expr="1"/>
    </data>
    <data id="days" sce:type="list&lt;record:Day&gt;" sce:capacity="3"/>
    <data id="copies" sce:type="list&lt;record:Day&gt;" sce:capacity="3"/>
    <data id="total" sce:type="uint32" expr="0"/>
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

fn run(args: &[&str], doc: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("schema_day.scxml"), DAY).expect("write schema");
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
fn a_list_of_records_is_filled_walked_and_copied_by_name() {
    let body = r#"
      <sce:append target="days" expr="draft"/>
      <foreach array="days" item="d" index="i">
        <assign location="total" expr="total + d.dayOfMonth + i"/>
        <sce:append target="copies" expr="d"/>
        <foreach array="copies" item="c">
          <assign location="total" expr="total + c.month"/>
        </foreach>
      </foreach>
      <sce:clear target="copies"/>"#;
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
    ] {
        let (ok, out) = run(args, &machine(body));
        assert!(ok, "{args:?}: a list of records:\n{out}");
    }
}

#[test]
fn a_list_takes_a_record_written_as_its_name() {
    // A field is a number, not a record; a name nothing declares is no record.
    for expr in ["draft.year", "total", "nothing", "draft.year + 1"] {
        refused(
            &format!(r#"<sce:append target="days" expr="{expr}"/>"#),
            "takes a record of that schema",
        );
    }
    // A loop's record item is a name only inside its loop.
    refused(
        r#"<foreach array="days" item="d"/><sce:append target="copies" expr="d"/>"#,
        "takes a record of that schema",
    );
}

#[test]
fn a_record_item_reads_the_schemas_fields_and_nothing_else() {
    refused(
        r#"<foreach array="days" item="d"><assign location="total" expr="total + d.nope"/></foreach>"#,
        "nope",
    );
}

#[test]
fn what_a_loop_binds_is_read_and_not_written() {
    for location in ["d.year", "d", "i"] {
        refused(
            &format!(
                r#"<foreach array="days" item="d" index="i"><assign location="{location}" expr="1"/></foreach>"#
            ),
            "which a <foreach> binds",
        );
    }
}
