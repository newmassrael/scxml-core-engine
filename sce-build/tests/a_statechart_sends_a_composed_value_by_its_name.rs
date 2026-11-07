// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A statechart sends a composed value by the name of the place that holds it
//! (docs/adr/0086, section 5.2): a `<param>` that gives a composed field of the
//! schema the document imports for the event names a variable, a field of a
//! record variable, or a part of the payload of the event the transition is
//! on, and is accepted when the place's type fits the field's (section 5.1).
//! The Rust backend writes it by walking its type (section 5.3).
//!
//! # What is held
//!
//! * `check -l rust` takes `static_composed_send`, which forwards four lists
//!   and an optional list of the payload and sends a list variable.
//! * The Rust machine writes each value into the runtime's `ScriptValue` by
//!   one case per constructor: a list as an `Array` of its elements, a record
//!   as an `Object` of its fields, an absent value as `Null`, a 64-bit integer
//!   as the text of its digits, a byte string as its Latin-1 text and an enum
//!   value as its declared name, as a scalar param crosses.
//! * A value whose type may not fit the field's is refused where it is
//!   written, naming the field, the schema that reads it and the place.
//! * A value that names no place is refused.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::{compile_scxml_lang_typed, find_template_dir_for};
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The documents a sender of `batch.sent` imports, copied beside it.
const SIBLINGS: [&str; 3] = [
    "enum_signal_level.scxml",
    "event_batch_item.scxml",
    "event_batch.scxml",
];

/// The Rust machine of `static_composed_send`, generated where the fixture is.
fn sender_machine() -> String {
    let path = fixtures().join("static_composed_send.scxml");
    let language = sce_build::generator::Language::Rust;
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("Rust sends a composed value: {e:?}"));
    output
        .files
        .into_iter()
        .find(|(name, _)| name == "static_composed_send_sm.rs")
        .map(|(_, code)| code)
        .expect("the machine's Rust is generated")
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

/// `sce-codegen <args> <file>` over `document` written beside the batch
/// schemas as `batch_sender.scxml`, and whether it succeeded, with what it
/// said.
fn run(args: &[&str], document: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    let path = dir.path().join("batch_sender.scxml");
    std::fs::write(&path, document).expect("write the document");
    run_on(args, &path)
}

/// `sce-codegen <args> <path>`, and whether it succeeded, with what it said.
fn run_on(args: &[&str], path: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// A statechart that holds the variables `data` and, as it starts, sends
/// `batch.sent` to itself with `params`, after a param for each composed field
/// of the schema's first revision, which every delivery carries
/// (docs/adr/0086, section 10.1), each from a variable of its own type.
fn sender(data: &str, params: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="batch_sender">
  <sce:import kind="event-schema" src="event_batch.scxml" as="Batch"/>
  <sce:import kind="event-schema" src="event_batch_item.scxml" as="Item"/>
  <sce:import kind="enum" src="enum_signal_level.scxml" as="SignalLevel"/>
  <datamodel>
    <data id="f_items" sce:type="list&lt;record:Item&gt;[batch_items=3]"/>
    <data id="f_grid" sce:type="list&lt;list&lt;uint8&gt;[2]&gt;[2]"/>
    <data id="f_levels" sce:type="list&lt;enum:SignalLevel&gt;[2]"/>
    <data id="f_maybe" sce:type="list&lt;optional&lt;uint16&gt;&gt;[2]"/>
    <data id="f_spare" sce:type="optional&lt;list&lt;uint8&gt;[2]&gt;" expr="null"/>
    {data}
  </datamodel>
  <state id="idle">
    <onentry>
      <send event="batch.sent" target="#_internal">
        <param name="items" expr="f_items"/>
        <param name="grid" expr="f_grid"/>
        <param name="levels" expr="f_levels"/>
        <param name="maybe" expr="f_maybe"/>
        <param name="spare" expr="f_spare"/>
        {params}
      </send>
    </onentry>
  </state>
</scxml>
"##
    )
}

/// Pinned by `ComposedLowering::of`'s `sends` and the gate's `<send>` arm
/// (`sce-build/src/forge/event_schema_check.rs`) and by
/// `Judge::composed_param` (`sce-build/src/forge/static_datamodel.rs`).
#[test]
fn rust_takes_a_send_of_composed_values_that_fit() {
    let (ok, out) = run_on(
        &["check", "-l", "rust"],
        &fixtures().join("static_composed_send.scxml"),
    );
    assert!(ok, "Rust sends a composed value that fits:\n{out}");
}

/// Pinned by `RustComposed`'s writers under `RustCarrier::ScriptValue`
/// (`sce-build/src/forge/composed_lowering.rs`) and by `composed_wire_value`
/// (`sce-build/src/forge/static_lowering.rs`): each part of the payload is
/// written from the place it is read at, by one case per constructor.
#[test]
fn each_value_is_written_by_walking_its_type() {
    let code = sender_machine();
    let s = "::sce_rust_runtime::ScriptValue";
    let p = "::sce_rust_runtime::event_payload";
    holds(
        &code,
        &[
            &format!("{s}::Array(ev.items.iter().map(|h| {s}::Object(vec![(\"id\".to_string(), {s}::Int(i64::from(h.id))), (\"amount\".to_string(), {s}::String(h.amount.to_string())), (\"label\".to_string(), {s}::String(h.label.to_string())), (\"level\".to_string(), {s}::String(h.level.sce_name().to_string())), "),
            &format!("(\"due\".to_string(), h.due.as_ref().map_or({s}::Null, |h| {s}::String(h.to_string()))), "),
            &format!("(\"blob\".to_string(), {s}::String({p}::bytes_as_payload_text(&h.blob))), "),
            &format!("(\"tags\".to_string(), {s}::Array(h.tags.iter().map(|h| {s}::String(h.to_string())).collect()))].into_iter().collect())).collect())"),
            &format!("{s}::Array(ev.grid.iter().map(|h| {s}::Array(h.iter().map(|h| {s}::Int(i64::from(*h))).collect())).collect())"),
            &format!("{s}::Array(ev.levels.iter().map(|h| {s}::String(h.sce_name().to_string())).collect())"),
            &format!("{s}::Array(ev.maybe.iter().map(|h| h.as_ref().map_or({s}::Null, |h| {s}::Int(i64::from(*h)))).collect())"),
            &format!("ev.spare.as_ref().map_or({s}::Null, |h| {s}::Array(h.iter().map(|h| {s}::Int(i64::from(*h))).collect()))"),
            &format!(".late.iter().map(|h| {s}::Int(i64::from(*h))).collect())"),
        ],
    );
}

/// Pinned by the fit `Judge::composed_param` asks of the place's type and the
/// field's (`sce-build/src/forge/static_datamodel.rs`).
#[test]
fn a_value_that_may_not_fit_is_refused_naming_the_field_and_the_place() {
    let document = sender(
        r#"<data id="wide" sce:type="list&lt;uint8&gt;[3]"/>"#,
        r#"<param name="late" expr="wide"/>"#,
    );
    let (ok, out) = run(&["check", "-l", "rust"], &document);
    assert!(!ok, "a list of 3 may not fit a field of 2:\n{out}");
    assert!(out.contains("scxml/static-datamodel-rule"), "{out}");
    assert!(
        out.contains("the schema `event_batch` this document declares `batch.sent` by reads it as its field `late`"),
        "{out}"
    );
    assert!(out.contains("'wide' is a "), "{out}");
}

/// Pinned by the place `Judge::composed_param` asks the value for
/// (`sce-build/src/forge/static_datamodel.rs`).
#[test]
fn a_value_that_names_no_place_is_refused() {
    let document = sender(
        r#"<data id="wide" sce:type="list&lt;uint8&gt;[3]"/>"#,
        r#"<param name="late" expr="len(wide)"/>"#,
    );
    let (ok, out) = run(&["check", "-l", "rust"], &document);
    assert!(!ok, "a length is no list:\n{out}");
    assert!(out.contains("scxml/static-datamodel-rule"), "{out}");
    assert!(out.contains("names no such place"), "{out}");
}

/// Pinned by `ComposedLowering::of` (`sce-build/src/forge/event_schema_check.rs`):
/// a backend that does not send a composed value yet refuses the document by
/// name, never generating it. A whole send gives every composed field of the
/// first revision from a variable of its type, and such a backend refuses the
/// first of those variables before it reaches the `<send>`; the gate's `<send>`
/// arm is held by `a_send_or_raise_of_the_event_of_a_composed_schema_is_refused_by_name`.
#[test]
fn a_backend_that_sends_none_refuses_the_send_by_name() {
    let document = sender(
        r#"<data id="kept" sce:type="list&lt;uint8&gt;[2]"/>"#,
        r#"<param name="late" expr="kept"/>"#,
    );
    for language in ["kotlin", "cpp", "go", "python", "c11"] {
        let (ok, out) = run(&["check", "-l", language], &document);
        assert!(!ok, "{language} sends no composed value yet:\n{out}");
        assert!(
            out.contains("generate/unsupported-feature"),
            "{language}: {out}"
        );
        assert!(out.contains("lowering of its list"), "{language}: {out}");
    }
    let (ok, out) = run(&["check", "-l", "rust"], &document);
    assert!(ok, "Rust sends a list variable that fits:\n{out}");
}
