// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An optional value is written, tested, read with a default, read where it is
//! proven present and compared (docs/adr/0055, section 2 and step 3): one case
//! per row of the section's table, each judged fit and generated for Rust, whose
//! step of docs/adr/0055 lowers every row, and one case per refusal, each
//! refused where the document is checked, by its code.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-21: the statechart's judge typed an optional
//! variable as a value it did not know, so `due + 1`, `due > 3` and an
//! assignment of `due` to an `int64` were judged fit, and the lexer refused
//! `??` by name.

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

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The enum the probes import (`Status`), copied beside each.
const ENUM_FILE: &str = "enum_item_status.scxml";

/// The file name a probe imports the schema [`SCHEMA_DUE`] from.
const SCHEMA_DUE_FILE: &str = "schema_item_due.scxml";

/// An event schema whose fields may be absent — a day, a bounded text and a
/// value of an enum — beside a field every delivery carries.
const SCHEMA_DUE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_item_due" sce:event-name="item.due">
  <sce:import src="enum_item_status.scxml" kind="enum" as="Status"/>
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="due" sce:type="optional&lt;int64&gt;" sce:direction="in"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:max-size="16" sce:direction="in"/>
    <data id="status" sce:type="optional&lt;enum:Status&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// The variables every probe declares: an optional day, text, flag and value of
/// `Status`, and a plain number and value of `Status` beside them.
const DATA: &str = r#"<data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:capacity="16" expr="null"/>
    <data id="flag" sce:type="optional&lt;bool&gt;" expr="null"/>
    <data id="status" sce:type="optional&lt;enum:Status&gt;" expr="null"/>
    <data id="count" sce:type="int64" expr="0"/>
    <data id="plain" sce:type="enum:Status" expr="Status.open"/>"#;

/// A `sce-static` statechart that imports `Status` and [`SCHEMA_DUE`] as `Due`,
/// declares [`DATA`] and `states` as written, starting in `idle`.
fn machine(states: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="optional_probe">
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:import kind="event-schema" src="{SCHEMA_DUE_FILE}" as="Due"/>
  <datamodel>
    {DATA}
  </datamodel>
  {states}
</scxml>
"#
    )
}

/// [`machine`] with one state, `idle`, whose content is `body`.
fn statechart(body: &str) -> String {
    machine(&format!(r#"<state id="idle">{body}</state>"#))
}

/// [`statechart`] with one transition on `tick` whose content is `content`.
fn on_tick(content: &str) -> String {
    statechart(&format!(
        r#"<transition event="tick">{content}</transition>"#
    ))
}

/// `sce-codegen <args…> <file>` in a directory holding the enum, the schema and
/// `documents`, `file` being one of `documents`. Returns `(exit_ok, stderr +
/// stdout)`.
fn run(args: &[&str], documents: &[(&str, &str)], file: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::copy(fixtures().join(ENUM_FILE), dir.path().join(ENUM_FILE)).expect("copy the enum");
    std::fs::write(dir.path().join(SCHEMA_DUE_FILE), SCHEMA_DUE).expect("write the schema");
    for (name, text) in documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(file));
    if matches!(args.first(), Some(&"generate")) {
        command.arg("-o").arg(out_dir.path());
    }
    let out = command.output().expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// `text` (with `extra` documents beside it) is judged fit, and generated for
/// Rust, whose step of docs/adr/0055 lowers every row of the table: refused by
/// no rule of an expression or of the data model, and by no lowering.
fn judged_fit(text: &str, extra: &[(&str, &str)]) {
    let mut documents = vec![("probe.scxml", text)];
    documents.extend_from_slice(extra);
    let (ok, out) = run(&["generate", "-l", "rust"], &documents, "probe.scxml");
    assert!(
        !out.contains("\"code\":\"expression/") && !out.contains("scxml/static-datamodel-rule"),
        "the document is judged fit:\n{out}"
    );
    assert!(ok, "Rust lowers the document:\n{out}");
}

/// `text` (with `extra` documents beside it) is refused where it is checked,
/// with `code`, and the refusal holds `needle`.
fn refused(text: &str, extra: &[(&str, &str)], code: &str, needle: &str) {
    let mut documents = vec![("probe.scxml", text)];
    documents.extend_from_slice(extra);
    let (ok, out) = run(&["check", "-l", "rust"], &documents, "probe.scxml");
    assert!(!ok, "expected a refusal with {code}:\n{out}");
    assert!(out.contains(code), "expected the code {code} in:\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in:\n{out}");
}

const TYPE_MISMATCH: &str = "expression/type-mismatch";
const UNSUPPORTED: &str = "expression/unsupported-construct";
const DATAMODEL_RULE: &str = "scxml/static-datamodel-rule";

// ── The rows of the table ────────────────────────────────────────────────

/// absent: `null`, the optional value of the place it lands in.
#[test]
fn null_is_an_absent_value_where_an_optional_is_expected() {
    judged_fit(
        &on_tick(
            r#"<assign location="due" expr="null"/>
               <assign location="title" expr="null"/>
               <assign location="status" expr="null"/>"#,
        ),
        &[],
    );
}

/// a present value: a value of `T` where an `optional<T>` is expected.
#[test]
fn a_value_of_t_is_a_present_value_of_optional_t() {
    judged_fit(
        &on_tick(
            r#"<assign location="due" expr="count"/>
               <assign location="due" expr="5"/>
               <assign location="title" expr="'soon'"/>
               <assign location="flag" expr="true"/>
               <assign location="status" expr="Status.done"/>
               <assign location="status" expr="plain"/>"#,
        ),
        &[],
    );
}

/// is it present: `x !== null`, `x === null`, a `bool`.
#[test]
fn a_test_of_presence_is_a_bool() {
    judged_fit(
        &on_tick(
            r#"<if cond="due !== null"><assign location="count" expr="1"/></if>
               <if cond="null === title"><assign location="count" expr="2"/></if>
               <if cond="status === null"><assign location="count" expr="3"/></if>"#,
        ),
        &[],
    );
}

/// the value or a default: `x ?? d`, of `T` with a `d` of `T`, of
/// `optional<T>` with one that may be absent.
#[test]
fn a_read_with_a_default_is_a_value_of_t() {
    judged_fit(
        &on_tick(
            r#"<assign location="count" expr="due ?? 0"/>
               <assign location="due" expr="due ?? null"/>
               <log expr="title ?? 'none'"/>
               <assign location="plain" expr="status ?? Status.open"/>
               <assign location="status" expr="status ?? null"/>"#,
        ),
        &[],
    );
}

/// the value where proven: `x.value`, of `T`.
#[test]
fn a_proven_read_is_a_value_of_t() {
    judged_fit(
        &on_tick(
            r#"<if cond="due !== null"><assign location="count" expr="due.value + 1"/></if>
               <if cond="status !== null"><assign location="plain" expr="status.value"/></if>
               <if cond="title !== null"><log expr="'due: ' + title.value"/></if>"#,
        ),
        &[],
    );
}

/// equality: total, over two optional values or one and a value of `T`.
#[test]
fn equality_is_total() {
    judged_fit(
        &on_tick(
            r#"<if cond="due === count"><assign location="count" expr="1"/></if>
               <if cond="due !== due"><assign location="count" expr="2"/></if>
               <if cond="title === 'soon'"><assign location="count" expr="3"/></if>
               <if cond="status === Status.done"><assign location="count" expr="4"/></if>
               <if cond="status !== plain"><assign location="count" expr="5"/></if>"#,
        ),
        &[],
    );
}

/// A conditional of `T` and `null` is of `optional<T>`: it lands in an
/// optional place, and in no place of `T`.
#[test]
fn a_conditional_of_a_value_and_null_is_optional() {
    judged_fit(
        &on_tick(
            r#"<assign location="due" expr="count &gt; 0 ? count : null"/>
               <assign location="status" expr="count &gt; 0 ? Status.done : null"/>"#,
        ),
        &[],
    );
    refused(
        &on_tick(r#"<assign location="count" expr="count &gt; 0 ? count : null"/>"#),
        &[],
        TYPE_MISMATCH,
        "may be absent",
    );
}

// ── What an optional value is not for ────────────────────────────────────

#[test]
fn an_optional_value_is_no_value_of_t() {
    refused(
        &on_tick(r#"<assign location="count" expr="due"/>"#),
        &[],
        TYPE_MISMATCH,
        "may be absent",
    );
}

#[test]
fn ordering_an_optional_value_is_refused() {
    refused(
        &on_tick(r#"<if cond="due &gt; 3"><assign location="count" expr="1"/></if>"#),
        &[],
        TYPE_MISMATCH,
        "may be absent",
    );
}

#[test]
fn arithmetic_on_an_optional_value_is_refused() {
    refused(
        &on_tick(r#"<assign location="count" expr="due + 1"/>"#),
        &[],
        TYPE_MISMATCH,
        "may be absent",
    );
}

#[test]
fn len_of_an_optional_text_is_refused() {
    refused(
        &on_tick(r#"<assign location="count" expr="len(title)"/>"#),
        &[],
        TYPE_MISMATCH,
        "may be absent",
    );
}

#[test]
fn a_text_joined_with_an_optional_text_is_refused() {
    refused(
        &on_tick(r#"<log expr="'due: ' + title"/>"#),
        &[],
        TYPE_MISMATCH,
        "may be absent",
    );
}

#[test]
fn a_truth_test_of_an_optional_bool_is_refused() {
    refused(
        &on_tick(r#"<if cond="flag"><assign location="count" expr="1"/></if>"#),
        &[],
        TYPE_MISMATCH,
        "may be absent",
    );
    refused(
        &on_tick(r#"<if cond="!flag"><assign location="count" expr="1"/></if>"#),
        &[],
        TYPE_MISMATCH,
        "may be absent",
    );
    // A test of a flag that may be absent is written against `true`.
    judged_fit(
        &on_tick(r#"<if cond="flag === true"><assign location="count" expr="1"/></if>"#),
        &[],
    );
}

#[test]
fn null_beside_a_value_that_is_never_absent_is_refused() {
    refused(
        &on_tick(r#"<if cond="count === null"><assign location="count" expr="1"/></if>"#),
        &[],
        TYPE_MISMATCH,
        "null",
    );
    // A value of an enum is never absent either.
    refused(
        &on_tick(r#"<if cond="plain === null"><assign location="count" expr="1"/></if>"#),
        &[],
        UNSUPPORTED,
        "never absent",
    );
}

#[test]
fn a_default_for_a_value_that_is_never_absent_is_refused() {
    refused(
        &on_tick(r#"<assign location="count" expr="count ?? 0"/>"#),
        &[],
        TYPE_MISMATCH,
        "never absent",
    );
}

#[test]
fn a_default_of_another_type_is_refused() {
    refused(
        &on_tick(r#"<assign location="count" expr="due ?? 'none'"/>"#),
        &[],
        TYPE_MISMATCH,
        "'none'",
    );
    refused(
        &on_tick(r#"<assign location="plain" expr="status ?? 3"/>"#),
        &[],
        UNSUPPORTED,
        "a default that is not of the enum",
    );
}

#[test]
fn an_optional_value_of_an_enum_is_no_value_of_the_enum() {
    refused(
        &on_tick(r#"<assign location="plain" expr="status"/>"#),
        &[],
        TYPE_MISMATCH,
        "may be absent",
    );
    refused(
        &on_tick(r#"<assign location="status" expr="3"/>"#),
        &[],
        UNSUPPORTED,
        "neither `null` nor of the enum",
    );
    refused(
        &on_tick(r#"<if cond="status === 3"><assign location="count" expr="1"/></if>"#),
        &[],
        UNSUPPORTED,
        "a comparison of an enum value",
    );
}

/// `null` stands only where an optional value is declared: an enum variable
/// that is not optional takes none.
#[test]
fn null_is_no_value_of_an_enum_that_is_not_optional() {
    refused(
        &on_tick(r#"<assign location="plain" expr="null"/>"#),
        &[],
        TYPE_MISMATCH,
        "null",
    );
}

/// An optional value is assigned whole: its `.value` is a read.
#[test]
fn the_value_of_an_optional_value_is_not_assigned() {
    refused(
        &on_tick(r#"<if cond="due !== null"><assign location="due.value" expr="1"/></if>"#),
        &[],
        UNSUPPORTED,
        "assigned whole",
    );
}

// ── Where an optional value crosses ──────────────────────────────────────

/// A `<send>` carries an optional value as JSON, to a field of the schema that
/// reads it which may be absent too, and a value of `T` to such a field.
#[test]
fn a_send_carries_an_optional_value_to_a_field_that_may_be_absent() {
    judged_fit(
        &on_tick(
            r##"<send event="item.due" target="#_internal">
                 <param name="id" expr="1"/>
                 <param name="due" expr="due"/>
                 <param name="title" expr="title"/>
                 <param name="status" expr="status"/>
               </send>
               <send event="item.due" target="#_internal">
                 <param name="id" expr="1"/>
                 <param name="due" expr="count"/>
                 <param name="title" expr="'soon'"/>
                 <param name="status" expr="plain"/>
               </send>"##,
        ),
        &[],
    );
}

#[test]
fn a_send_of_an_optional_value_to_a_field_that_is_never_absent_is_refused() {
    refused(
        &on_tick(
            r##"<send event="item.due" target="#_internal"><param name="id" expr="due"/></send>"##,
        ),
        &[],
        DATAMODEL_RULE,
        "which may be absent",
    );
}

/// A `<donedata>` carries an optional value as JSON.
#[test]
fn a_donedata_carries_an_optional_value() {
    judged_fit(
        &machine(
            r#"<state id="idle"><transition event="go" target="done"/></state>
  <final id="done"><donedata><param name="due" expr="due"/></donedata></final>"#,
        ),
        &[],
    );
}

/// A carrier that is text alone has no spelling for an absent value.
#[test]
fn a_carrier_of_text_refuses_an_optional_value_naming_the_default() {
    refused(
        &on_tick(
            r#"<send event="note" type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor" target="http://example.invalid/hook"><param name="due" expr="due"/></send>"#,
        ),
        &[],
        DATAMODEL_RULE,
        "due ?? <value>",
    );
    refused(
        &statechart(r#"<invoke type="x-sce-host" id="h"><param name="k" expr="due"/></invoke>"#),
        &[],
        DATAMODEL_RULE,
        "due ?? <value>",
    );
}

/// A child's optional variable takes no `<param>` yet, refused by name.
#[test]
fn a_childs_optional_variable_takes_no_param_yet() {
    const CHILD: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="busy" datamodel="sce-static">
  <datamodel><data id="due" sce:type="optional&lt;int64&gt;" expr="null"/></datamodel>
  <state id="busy"><transition event="finish" target="end"/></state>
  <final id="end"/>
</scxml>
"#;
    refused(
        &machine(
            r#"<state id="idle">
    <invoke type="scxml" id="child" src="child.scxml"><param name="due" expr="count"/></invoke>
    <transition event="done.invoke.child" target="done"/>
  </state>
  <final id="done"/>"#,
        ),
        &[("child.scxml", CHILD)],
        DATAMODEL_RULE,
        "hands no value to yet",
    );
}

/// A `<log>` takes an optional value as it is.
#[test]
fn a_log_takes_an_optional_value() {
    judged_fit(&on_tick(r#"<log expr="due"/><log expr="status"/>"#), &[]);
}
