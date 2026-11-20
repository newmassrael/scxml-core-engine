// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! docs/adr/0109, Open question 9: a `<log>` honours `sce:sensitivity`. A value
//! read from a field its schema marks `secret`, or computed from one, is written
//! as `sce-secret`, the marker an audit entry holds in its place, and one marked
//! `personal` as `sce-personal`, with no digest, since a diagnostic log holds no
//! pseudonymisation key. The marker replaces the expression where the build
//! lowers it, so every generated lane and the Interpreter's lowering write the
//! same text, and a value of an unmarked field is written as it was.
//!
//! Under any data model but `sce-static` no value has a type a mark can follow,
//! so a statechart that imports a schema marking a field is refused at the
//! import, `validation/attribute-rule-violated` on its `src`, on every target.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};
use tempfile::tempdir;

/// A sign-in attempt whose account names a person and whose password is never
/// written.
const SIGN_IN_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_sign_in"
       sce:event-name="sign_in.tried">
  <datamodel>
    <data id="account" sce:type="string[64]" sce:direction="in" sce:sensitivity="personal"/>
    <data id="password" sce:type="string[32]" sce:direction="in" sce:sensitivity="secret"/>
    <data id="result" sce:type="string[16]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// [`SIGN_IN_SCHEMA`] with no mark.
fn unmarked_schema() -> String {
    SIGN_IN_SCHEMA
        .replace(r#" sce:sensitivity="personal""#, "")
        .replace(r#" sce:sensitivity="secret""#, "")
}

/// A machine that logs each field of the payload of `sign_in.tried`, each field
/// of a record variable of the schema, and the length of a secret. Each `<log>`
/// stands on a row of its own, its label naming what it logs.
const LOGGING: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="sensitive_log">
  <sce:import kind="event-schema" src="schema_sign_in.scxml" as="SignIn"/>
  <datamodel>
    <data id="attempt" sce:type="record:SignIn">
      <sce:set name="account" expr="'ada'"/>
      <sce:set name="password" expr="'hunter2'"/>
      <sce:set name="result" expr="'ok'"/>
    </data>
  </datamodel>
  <state id="idle">
    <transition event="sign_in.tried" type="internal">
      <log label="payload_secret" expr="_event.data.password"/>
      <log label="payload_personal" expr="_event.data.account"/>
      <log label="payload_plain" expr="_event.data.result"/>
    </transition>
    <transition event="go" type="internal">
      <log label="field_secret" expr="attempt.password"/>
      <log label="field_personal" expr="attempt.account"/>
      <log label="length_secret" expr="len(attempt.password)"/>
      <log label="field_plain" expr="attempt.result"/>
    </transition>
  </state>
</scxml>
"#;

/// Each label of [`LOGGING`] whose value is marked, with the marker written in
/// its place.
const MARKED: [(&str, &str); 5] = [
    ("payload_secret", "sce-secret"),
    ("payload_personal", "sce-personal"),
    ("field_secret", "sce-secret"),
    ("field_personal", "sce-personal"),
    ("length_secret", "sce-secret"),
];

/// Each label of [`LOGGING`] whose value is no marked field's.
const PLAIN: [&str; 2] = ["payload_plain", "field_plain"];

/// `statechart` beside `schema`, written in a directory of their own.
fn written(statechart: &str, schema: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().expect("a temporary directory");
    std::fs::write(dir.path().join("schema_sign_in.scxml"), schema).expect("the schema");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, statechart).expect("the statechart");
    (dir, path)
}

/// The rows of `code` that hold `label`.
fn rows<'c>(code: &'c str, label: &str) -> Vec<&'c str> {
    code.lines().filter(|row| row.contains(label)).collect()
}

/// `code`, the text one lane wrote for [`LOGGING`], logs each marked value as
/// its marker on a row of its label, and no unmarked one as a marker.
fn logs_the_markers(lane: &str, code: &str) {
    for (label, marker) in MARKED {
        let held = rows(code, label);
        assert!(
            held.iter().any(|row| row.contains(marker)),
            "{lane}: `{label}` is logged as `{marker}`; its rows:\n{}",
            held.join("\n")
        );
    }
    for label in PLAIN {
        let held = rows(code, label);
        assert!(
            !held.is_empty()
                && held
                    .iter()
                    .all(|row| !row.contains("sce-secret") && !row.contains("sce-personal")),
            "{lane}: `{label}` is logged as it is; its rows:\n{}",
            held.join("\n")
        );
    }
}

/// Every file `language` generates of the machine at `path`, one after the other.
fn generated(path: &Path, language: Language) -> String {
    compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("{language:?} generates the probe: {e:?}"))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n")
}

/// Pinned by the marker the static lowering's `<log>` arm lowers in place of the
/// expression (`crate::forge::sensitivity::logged_marker`).
#[test]
fn every_generated_lane_logs_a_marked_value_as_its_marker() {
    let (_dir, path) = written(LOGGING, SIGN_IN_SCHEMA);
    for language in [
        Language::Rust,
        Language::Cpp,
        Language::Kotlin,
        Language::Go,
        Language::Python,
        Language::C11,
        Language::Swift,
    ] {
        logs_the_markers(&format!("{language:?}"), &generated(&path, language));
    }
}

/// The Interpreter's lowering writes the marker's literal where the document
/// wrote the expression, so the lowered `<log>` reads no marked field.
#[test]
fn the_interpreter_logs_a_marked_value_as_its_marker() {
    let (_dir, path) = written(LOGGING, SIGN_IN_SCHEMA);
    let lowered =
        sce_build::forge::static_js::lower_file(path.to_str().expect("a utf-8 path"), Vec::new())
            .unwrap_or_else(|e| panic!("the Interpreter lowers the probe: {e:?}"));
    logs_the_markers("Interpreter", &lowered);
    for (label, _) in MARKED {
        for row in rows(&lowered, label) {
            assert!(
                !row.contains("password") && !row.contains("account"),
                "Interpreter: `{label}` reads no marked field: {row}"
            );
        }
    }
}

/// The same machine with no mark on its schema logs every field as it is.
#[test]
fn an_unmarked_schema_logs_every_field_as_it_is() {
    let (_dir, path) = written(LOGGING, &unmarked_schema());
    let code = generated(&path, Language::Rust);
    assert!(
        !code.contains("sce-secret") && !code.contains("sce-personal"),
        "Rust: no marker where no field is marked:\n{code}"
    );
}

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Every target a document is checked for, the Interpreter's own check first.
const EVERY_TARGET: [&[&str]; 8] = [
    &["check"],
    &["check", "-l", "rust"],
    &["check", "-l", "kotlin"],
    &["check", "-l", "cpp"],
    &["check", "-l", "go"],
    &["check", "-l", "python"],
    &["check", "-l", "c11"],
    &["check", "-l", "swift"],
];

/// `sce-codegen <args...> <path>` with JSON diagnostics, as `(succeeded, stderr)`.
fn run(args: &[&str], path: &Path) -> (bool, String) {
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
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        ),
    )
}

/// An `ecmascript` statechart that imports [`SIGN_IN_SCHEMA`] and logs its
/// payload: the import stands on row 4.
const UNTYPED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="ecmascript" name="untyped_log">
  <sce:import kind="event-schema" src="schema_sign_in.scxml" as="SignIn"/>
  <state id="idle">
    <transition event="sign_in.tried" type="internal">
      <log label="payload" expr="_event.data.password"/>
    </transition>
  </state>
</scxml>
"#;

/// Pinned by `crate::forge::sensitivity::refuse_untyped`, run where the imports
/// are resolved.
#[test]
fn an_ecmascript_statechart_that_imports_a_marked_schema_is_refused_on_every_target() {
    let (_dir, path) = written(UNTYPED, SIGN_IN_SCHEMA);
    for args in EVERY_TARGET {
        let (ok, out) = run(args, &path);
        assert!(
            !ok,
            "{args:?}: a marked schema was imported untyped:\n{out}"
        );
        let record = out
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find(|record| record["code"] == "validation/attribute-rule-violated")
            .unwrap_or_else(|| panic!("{args:?}: refused at the import:\n{out}"));
        assert_eq!(
            record["location"]["line"], 4,
            "{args:?}: the refusal is on the import's row:\n{out}"
        );
        assert_eq!(
            record["actual"], "schema_sign_in.scxml",
            "{args:?}: `actual` is the import's src:\n{out}"
        );
        let message = record["message"].as_str().unwrap_or_default();
        assert!(
            message.contains("`/account` personal") && message.contains("datamodel=\"sce-static\""),
            "{args:?}: the message names the first marked field and the data model that \
             follows it:\n{out}"
        );
    }
}

/// The same statechart over a schema that marks nothing is accepted.
#[test]
fn an_ecmascript_statechart_that_imports_an_unmarked_schema_is_accepted() {
    let (_dir, path) = written(UNTYPED, &unmarked_schema());
    let (ok, out) = run(&["check"], &path);
    assert!(
        ok && !out.contains("sce:sensitivity"),
        "an unmarked schema is imported untyped:\n{out}"
    );
}
