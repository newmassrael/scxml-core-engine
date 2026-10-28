// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `datamodel="sce-static"`: a `<log expr>` takes a value every backend has a
//! text for (docs/SCE_ACCEPTED_SUBSET.md §2.15, "What a `<log>` shows").
//!
//! The document is compiled once per target (docs/adr/0003). The C++ log
//! writes each value through its `operator<<` (docs/adr/0060) and the C11 one
//! through a `printf` conversion, so a byte string or a record logged under
//! the static data model did not compile there, while Rust, Go, Kotlin and
//! Python printed it each their own way. Such a value is refused at parse, for
//! every target, as `scxml/static-log-unloggable-value`, at the expression on
//! the `<log>`'s own row. The values every backend does log are accepted on
//! every target.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

/// The code the refusal is raised as.
const CODE: &str = "scxml/static-log-unloggable-value";

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Every target a `sce-static` document is checked for, the Interpreter's own
/// check first.
const EVERY_TARGET: [&[&str]; 7] = [
    &["check"],
    &["check", "-l", "rust"],
    &["check", "-l", "kotlin"],
    &["check", "-l", "cpp"],
    &["check", "-l", "go"],
    &["check", "-l", "python"],
    &["check", "-l", "c11"],
];

/// The variables every probe declares: a value of each type a `<log>` takes,
/// a byte string, two records and a list of records.
const DATA: &str = r#"<data id="count" sce:type="uint32" expr="0"/>
    <data id="ratio" sce:type="float64" expr="0.5"/>
    <data id="flag" sce:type="bool" expr="false"/>
    <data id="note" sce:type="string" sce:capacity="16" expr="'x'"/>
    <data id="mode" sce:type="enum:ViewMode" expr="ViewMode.month"/>
    <data id="frame" sce:type="bytes" sce:capacity="8" expr="'ab'"/>
    <data id="draft" sce:type="record:Day">
      <sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="1"/>
      <sce:set name="dayOfMonth" expr="1"/>
    </data>
    <data id="framed" sce:type="record:Framed">
      <sce:set name="sensor" expr="1"/>
      <sce:set name="frame" expr="'ab'"/>
    </data>
    <data id="days" sce:type="list&lt;record:Day&gt;" sce:capacity="3"/>
    <data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:capacity="16" expr="null"/>
    <data id="status" sce:type="optional&lt;enum:ViewMode&gt;" expr="null"/>"#;

/// The optional values no `<log>` takes, declared only by the probes that
/// log them: an optional record is refused by every generated backend's lowering
/// until its own step of docs/adr/0055, so a probe that must be accepted by all of
/// them does not hold one.
const OPTIONAL_UNLOGGED: &str = r#"
    <data id="maybe" sce:type="optional&lt;bytes&gt;" sce:capacity="8" expr="null"/>
    <data id="pick" sce:type="optional&lt;record:Day&gt;" expr="null"/>"#;

/// A statechart declaring [`DATA`] and `extra`, whose `go` transition runs
/// `body` and whose `day.picked` transition, on the event `schema_day.scxml`
/// declares, runs `payload` — written beside the documents it imports, with
/// its own text.
fn probe(extra: &str, body: &str, payload: &str) -> (tempfile::TempDir, PathBuf, String) {
    let dir = tempdir().expect("tempdir");
    for file in [
        "schema_day.scxml",
        "schema_framed.scxml",
        "enum_view_mode.scxml",
    ] {
        std::fs::copy(
            repo_root()
                .join("sce-build/tests/fixtures/static_datamodel")
                .join(file),
            dir.path().join(file),
        )
        .unwrap_or_else(|e| panic!("the fixture {file}: {e}"));
    }
    let text = format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <sce:import kind="event-schema" src="schema_framed.scxml" as="Framed"/>
  <sce:import kind="enum" src="enum_view_mode.scxml" as="ViewMode"/>
  <datamodel>
    {DATA}{extra}
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      {body}
    </transition>
    <transition event="day.picked" type="internal">
      {payload}
    </transition>
  </state>
</scxml>
"##
    );
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, &text).expect("write probe");
    (dir, path, text)
}

/// `sce-codegen <args…> <path>` with JSON diagnostics, as `(succeeded,
/// stderr, stdout)`.
fn run(args: &[&str], path: &Path) -> (bool, String, String) {
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// The diagnostic records `stderr` carries, one JSON object per line.
fn records(stderr: &str) -> Vec<serde_json::Value> {
    stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(serde_json::Value::is_object)
        .collect()
}

/// The 1-based row of the one `<log>` in `text`, and the 1-based column its
/// `expr` value starts at.
fn log_value_position(text: &str) -> (u64, u64) {
    let (index, line) = text
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("<log"))
        .expect("the probe holds a <log>");
    let attribute = line.find("expr=\"").expect("the <log> has an expr");
    let row = u64::try_from(index + 1).expect("a row fits");
    let col = u64::try_from(attribute + "expr=\"".len() + 1).expect("a column fits");
    (row, col)
}

/// The probe of `extra`, `body` and `payload` is refused on every target as
/// [`CODE`], at the `expr` of its one `<log>`, naming `expr` and its type `ty`.
fn refused_at_the_log(extra: &str, body: &str, payload: &str, expr: &str, ty: &str) {
    let (_dir, path, text) = probe(extra, body, payload);
    let (row, col) = log_value_position(&text);
    for args in EVERY_TARGET {
        let (ok, stderr, stdout) = run(args, &path);
        assert!(
            !ok,
            "{args:?}: `{expr}`, of type {ty}, was accepted in a <log>:\n{stderr}{stdout}"
        );
        let record = records(&stderr)
            .into_iter()
            .find(|record| record["code"] == CODE)
            .unwrap_or_else(|| panic!("{args:?}: `{expr}` is refused as {CODE}:\n{stderr}"));
        assert_eq!(
            record["location"]["line"], row,
            "{args:?}: the refusal is on the <log>'s row:\n{stderr}"
        );
        assert_eq!(
            record["location"]["col"], col,
            "{args:?}: the refusal is at the expression's first character:\n{stderr}"
        );
        assert_eq!(
            record["actual"], expr,
            "{args:?}: `actual` is the expression as written:\n{stderr}"
        );
        assert_eq!(
            record["stage"], "validation",
            "{args:?}: the refusal is the document's:\n{stderr}"
        );
        let message = record["message"].as_str().unwrap_or_default();
        assert!(
            message.contains(&format!("<log expr=\"{expr}\">"))
                && message.contains(&format!("of type `{ty}`")),
            "{args:?}: the message names the element, the expression and its type:\n{stderr}"
        );
    }
}

#[test]
fn a_byte_string_variable_is_refused_in_a_log() {
    refused_at_the_log("", r#"<log expr="frame"/>"#, "", "frame", "bytes");
}

#[test]
fn a_byte_string_field_of_a_record_is_refused_in_a_log() {
    refused_at_the_log(
        "",
        r#"<log expr="framed.frame"/>"#,
        "",
        "framed.frame",
        "bytes",
    );
}

#[test]
fn a_record_variable_is_refused_in_a_log() {
    refused_at_the_log(
        "",
        r#"<log label="draft" expr="draft"/>"#,
        "",
        "draft",
        "record:Day",
    );
}

#[test]
fn a_record_item_of_a_foreach_is_refused_in_a_log() {
    refused_at_the_log(
        "",
        r#"<foreach array="days" item="d">
        <log expr="d"/>
      </foreach>"#,
        "",
        "d",
        "record:Day",
    );
}

#[test]
fn a_payload_taken_whole_is_refused_in_a_log() {
    refused_at_the_log(
        "",
        "",
        r#"<log expr="_event.data"/>"#,
        "_event.data",
        "record:Day",
    );
}

#[test]
fn an_optional_byte_string_is_refused_in_a_log() {
    refused_at_the_log(
        OPTIONAL_UNLOGGED,
        r#"<log expr="maybe"/>"#,
        "",
        "maybe",
        "optional<bytes>",
    );
}

#[test]
fn an_optional_record_is_refused_in_a_log() {
    refused_at_the_log(
        OPTIONAL_UNLOGGED,
        r#"<log expr="pick"/>"#,
        "",
        "pick",
        "optional<record:Day>",
    );
}

/// A list variable is no value anywhere an expression reads one, so it is
/// refused before its type is asked, by the rule every read of a list is held
/// to, and not as a value a `<log>` cannot show.
#[test]
fn a_list_variable_is_refused_in_a_log_as_any_read_of_a_list_is() {
    let (_dir, path, _text) = probe("", r#"<log expr="days"/>"#, "");
    for args in EVERY_TARGET {
        let (ok, stderr, stdout) = run(args, &path);
        assert!(!ok, "{args:?}: a list was logged:\n{stderr}{stdout}");
        let codes: Vec<String> = records(&stderr)
            .iter()
            .filter_map(|record| record["code"].as_str().map(str::to_string))
            .collect();
        assert!(
            codes
                .iter()
                .any(|code| code == "expression/unsupported-construct")
                && !codes.iter().any(|code| code == CODE),
            "{args:?}: a list is refused as a read of a list:\n{stderr}"
        );
    }
}

#[test]
fn a_value_every_backend_writes_is_logged_on_every_target() {
    let (_dir, path, _text) = probe(
        "",
        r#"<log expr="count"/>
      <log label="ratio" expr="ratio"/>
      <log expr="flag"/>
      <log expr="note"/>
      <log label="mode" expr="mode"/>
      <log expr="draft.year"/>
      <log expr="framed.sensor"/>
      <log label="size" expr="len(frame)"/>
      <log expr="count + 1"/>
      <log expr="'text'"/>"#,
        r#"<log expr="_event.data.month"/>"#,
    );
    for args in EVERY_TARGET {
        let (ok, stderr, stdout) = run(args, &path);
        assert!(
            ok,
            "{args:?}: a bool, a string, an integer, a real and an enum value are \
             logged:\n{stderr}{stdout}"
        );
    }
}

#[test]
fn an_optional_value_of_a_logged_type_is_logged_on_every_target() {
    let (_dir, path, _text) = probe(
        "",
        r#"<log label="due" expr="due"/>
      <log expr="title"/>
      <log label="status" expr="status"/>"#,
        "",
    );
    for args in EVERY_TARGET {
        let (ok, stderr, stdout) = run(args, &path);
        assert!(
            ok,
            "{args:?}: an optional number, text and enum value are logged:\n{stderr}{stdout}"
        );
    }
}

/// Under `ecmascript` the same `<log>` of a byte string is the script engine's
/// to show at run time, and the static data model's rule does not reach it.
#[test]
fn the_rule_is_the_static_data_models_alone() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("probe.scxml");
    std::fs::write(
        &path,
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s"
       datamodel="ecmascript">
  <datamodel>
    <data id="frame" expr="[1, 2]"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <log expr="frame"/>
    </transition>
  </state>
</scxml>
"#,
    )
    .expect("write probe");
    let (ok, stderr, stdout) = run(&["check"], &path);
    assert!(
        ok && !stderr.contains(CODE),
        "an ecmascript <log> of any value is accepted:\n{stderr}{stdout}"
    );
}
