// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A host-run `<invoke>` typed by `sce:request` (docs/SCE_ACCEPTED_SUBSET.md
//! §2.12) promised its host the whole record, so under `datamodel="sce-static"`
//! each `<param>` is held to the field it supplies where it is written: a
//! computed value by its type, which this model knows before the machine runs.
//!
//! Each case writes a statechart whose one invoke asks with a record of four
//! fields, `text` (16 bytes), `n` (int32), `flag` and `ratio` (float32), and
//! changes one argument. The accepted case comes first: without it, every
//! refusal below could be the document failing for a reason of its own.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::emitting;
use sce_build::forge::model::ForgeKind;
use tempfile::tempdir;

const REQUEST: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="ask_request" sce:kind="event-schema" sce:event-name="ask.request">
  <datamodel>
    <data id="text" sce:type="string" sce:max-size="16" sce:direction="in"/>
    <data id="n" sce:type="int32" sce:direction="in"/>
    <data id="flag" sce:type="bool" sce:direction="in"/>
    <data id="ratio" sce:type="float32" sce:direction="in"/>
  </datamodel>
</scxml>"#;

const RESULT: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="ask_result" sce:kind="event-schema" sce:event-name="ask.result">
  <datamodel>
    <data id="ok" sce:type="bool" sce:direction="in"/>
  </datamodel>
</scxml>"#;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// `sce-codegen check <args> <doc>` with the two schemas beside the document:
/// whether it succeeded, and everything it wrote.
fn check(args: &[&str], doc: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("ask_request.scxml"), REQUEST).expect("write schema");
    std::fs::write(dir.path().join("ask_result.scxml"), RESULT).expect("write schema");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .arg("check")
        .args(args)
        .args(["--host-invoker", "x-app-host"])
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// A statechart holding `capacity`-byte text `label`, a `uint32`, an `int32`, a
/// `bool` and two reals, whose invoke supplies each field of the record by the
/// expression given.
fn doc(capacity: &str, text: &str, n: &str, flag: &str, ratio: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="a" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="ask_request.scxml" as="AskRequest"/>
  <sce:import kind="event-schema" src="ask_result.scxml" as="AskResult"/>
  <datamodel>
    <data id="label" sce:type="string" sce:capacity="{capacity}" expr="''"/>
    <data id="count_u" sce:type="uint32" expr="0"/>
    <data id="count_s" sce:type="int32" expr="0"/>
    <data id="byte_v" sce:type="uint8" expr="0"/>
    <data id="truth" sce:type="bool" expr="false"/>
    <data id="real_s" sce:type="float32" expr="0.0"/>
    <data id="real_d" sce:type="float64" expr="0.0"/>
    <data id="finished" sce:type="bool" expr="false" sce:direction="out"/>
  </datamodel>
  <state id="a">
    <transition event="go" target="b"/>
  </state>
  <state id="b">
    <invoke type="x-app-host" id="ask" sce:request="AskRequest" sce:result="AskResult">
      <param name="text" expr="{text}"/>
      <param name="n" expr="{n}"/>
      <param name="flag" expr="{flag}"/>
      <param name="ratio" expr="{ratio}"/>
    </invoke>
    <transition event="done.invoke.ask" target="a">
      <assign location="finished" expr="_event.data.ok"/>
    </transition>
  </state>
</scxml>
"#
    )
}

#[test]
fn values_the_fields_hold_are_accepted_on_every_backend() {
    // `byte_v` widens into an int32, `label` is bound within the field's 16
    // bytes, and a float32 variable is the field's own type.
    let accepted = doc("16", "label", "byte_v", "truth", "real_s");
    for language in emitting(ForgeKind::Statechart).map(|language| language.canonical_name()) {
        let (ok, text) = check(&["-l", language], &accepted);
        assert!(ok, "{language} refused a request every field holds: {text}");
    }
    let (ok, text) = check(
        &[],
        &doc("8", "label", "count_s", "truth &amp;&amp; !truth", "real_s"),
    );
    assert!(
        ok,
        "a smaller text, an int32 and a bool expression were refused: {text}"
    );
}

#[test]
fn a_value_a_field_cannot_hold_is_refused_where_it_is_written() {
    let refused = [
        // 17 bytes may be held where the field holds 16.
        doc("17", "label", "count_s", "truth", "real_s"),
        // A uint32 does not fit an int32 whole.
        doc("16", "label", "count_u", "truth", "real_s"),
        // A number is no text, and a text no number.
        doc("16", "count_s", "count_s", "truth", "real_s"),
        doc("16", "label", "label", "truth", "real_s"),
        // An integer is no truth value.
        doc("16", "label", "count_s", "count_s", "real_s"),
        // A double does not fit a float32 whole.
        doc("16", "label", "count_s", "truth", "real_d"),
        // A text no declaration bounds is a run-time fact.
        doc("16", "label + label", "count_s", "truth", "real_s"),
    ];
    for document in &refused {
        let (ok, text) = check(&[], document);
        assert!(
            !ok,
            "a value its field cannot hold was accepted: {document}"
        );
        assert!(
            text.contains("scxml/static-datamodel-rule"),
            "the refusal is not on its row: {text}"
        );
    }
}
