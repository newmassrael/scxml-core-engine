// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The standard host features (docs/SCE_HOST_FEATURES.md) are one set: a table
//! of invoke types ([`sce_build::forge::standard_host::TYPES`]), the records the
//! standard library holds for them, and what a build does with a document that
//! runs one.
//!
//! The first case ties the table to the library in both directions, because a
//! type with no record is a promise nothing keeps and a record no type names is
//! one nothing reads. The rest drive the CLI, since the aliases of a typed
//! invoke resolve through the imports the way a build reads them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::standard_host::{document, lookup, TYPES};
use sce_build::generator::Language;
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

/// `sce-codegen <args…> <doc>` with JSON diagnostics: whether it succeeded, and
/// everything it wrote.
fn run(args: &[&str], doc: &str, siblings: &[(&str, &str)]) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for (name, text) in siblings {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
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

/// Every backend, by the identifier `-l` takes.
fn languages() -> Vec<&'static str> {
    Language::ALL
        .iter()
        .map(|language| language.canonical_name())
        .collect()
}

/// A `sce-static` statechart that asks `sce-host-kv-get` for `key`, a string
/// the machine holds with the bound `capacity`, importing the records `request`
/// and `result` name as the standard library's.
fn kv_get(capacity: &str, request: &str, result: &str, param: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="{request}" as="KvGetRequest"/>
  <sce:import kind="event-schema" src="{result}" as="KvGetResult"/>
  <datamodel>
    <data id="key" sce:type="string" sce:capacity="{capacity}" expr="''"/>
    <data id="wide" sce:type="int64" expr="0"/>
    <data id="found" sce:type="bool" expr="false" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="go" target="asking"/>
  </state>
  <state id="asking">
    <invoke type="sce-host-kv-get" id="get" sce:request="KvGetRequest" sce:result="KvGetResult">
      <param name="key" expr="{param}"/>
    </invoke>
    <transition event="done.invoke.get" target="idle">
      <assign location="found" expr="_event.data.found"/>
    </transition>
  </state>
</scxml>
"#
    )
}

const REQUEST: &str = "sce:std/host/kv_get_request.scxml";
const RESULT: &str = "sce:std/host/kv_get_result.scxml";

/// The file names under `stdlib/host`, without the extension.
fn library_documents() -> BTreeSet<String> {
    std::fs::read_dir(repo_root().join("stdlib/host"))
        .expect("stdlib/host")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "scxml"))
        .map(|path| {
            path.file_stem()
                .expect("a file stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn the_table_and_the_library_name_each_other() {
    let named: BTreeSet<String> = TYPES
        .iter()
        .flat_map(|t| t.request.into_iter().chain(Some(t.result)))
        .map(str::to_string)
        .collect();
    let held = library_documents();
    assert!(
        held.len() >= 10,
        "stdlib/host held {} documents: a test that read none would pass",
        held.len()
    );
    assert_eq!(
        named, held,
        "a record of the library no type names, or a type naming a record the library lacks"
    );
    let embedded: BTreeSet<String> = sce_build::forge::stdlib::documents()
        .map(|(name, _)| name)
        .collect();
    for name in &named {
        assert!(
            embedded.contains(&document(name)),
            "{name} is not embedded in the generator"
        );
    }
    let types: BTreeSet<&str> = TYPES.iter().map(|t| t.invoke_type).collect();
    assert_eq!(types.len(), TYPES.len(), "two operations share one type");
    for t in TYPES {
        assert!(
            t.invoke_type.starts_with("sce-host-")
                && !sce_build::host_processor_analyzer::is_reserved_type(t.invoke_type),
            "{} is not a type a host may serve",
            t.invoke_type
        );
        assert_eq!(lookup(t.invoke_type), Some(t));
    }
}

#[test]
fn a_standard_type_is_served_without_a_declaration() {
    let doc = kv_get("256", REQUEST, RESULT, "key");
    for language in languages() {
        let (ok, text) = run(&["check", "-l", language], &doc, &[]);
        assert!(ok, "{language} refused a standard host invoke: {text}");
        assert!(
            text.contains(r#""host_invoker_types":["sce-host-kv-get"]"#),
            "{language}: the type was not served: {text}"
        );
    }
}

#[test]
fn a_standard_type_is_held_to_the_records_of_the_library() {
    let own = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       name="kv_get_request" sce:kind="event-schema" sce:event-name="host.kv.get.request">
  <datamodel>
    <data id="key" sce:type="string" sce:max-size="256" sce:direction="in"/>
  </datamodel>
</scxml>"#;
    let doc = kv_get("256", "kv_get_request.scxml", RESULT, "key");
    let (ok, text) = run(&["check"], &doc, &[("kv_get_request.scxml", own)]);
    assert!(
        !ok,
        "a record of the same name and shape was accepted: {text}"
    );
    assert!(
        text.contains("validation/typed-invoke-schema") && text.contains("sce:std/host/"),
        "{text}"
    );
}

#[test]
fn a_standard_type_names_its_result() {
    let doc = kv_get("256", REQUEST, RESULT, "key").replace(r#" sce:result="KvGetResult""#, "");
    let (ok, text) = run(&["check"], &doc, &[]);
    assert!(
        !ok,
        "a standard invoke with no result record was accepted: {text}"
    );
    assert!(text.contains("validation/typed-invoke-schema"), "{text}");
}

#[test]
fn an_operation_with_no_request_takes_no_argument() {
    let doc = |param: &str| {
        format!(
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="asking" datamodel="ecmascript" name="probe">
  <sce:import kind="event-schema" src="sce:std/host/clock_now_result.scxml" as="Now"/>
  <state id="asking">
    <invoke type="sce-host-clock-now" id="now" sce:result="Now">{param}</invoke>
  </state>
</scxml>
"#
        )
    };
    let (ok, text) = run(&["check"], &doc(""), &[]);
    assert!(ok, "a clock read with no argument was refused: {text}");
    let (ok, text) = run(&["check"], &doc(r#"<param name="zone" expr="'x'"/>"#), &[]);
    assert!(!ok, "an argument to an operation that takes none: {text}");
    assert!(text.contains("validation/typed-invoke-request"), "{text}");
    // The engine's own deadline is not an argument.
    let (ok, text) = run(
        &["check"],
        &doc(r#"<param name="_sce_deadline_ms" expr="500"/>"#),
        &[],
    );
    assert!(ok, "a deadline was counted as an argument: {text}");
}

#[test]
fn a_value_the_machine_computes_is_held_to_the_field_it_supplies() {
    // A string held by a variable whose bound is the field's own is shown to
    // fit, on every backend.
    let doc = kv_get("256", REQUEST, RESULT, "key");
    for language in languages() {
        let (ok, text) = run(&["check", "-l", language], &doc, &[]);
        assert!(ok, "{language}: {text}");
    }
    // One bounded wider than the field might not fit.
    let (ok, text) = run(&["check"], &kv_get("512", REQUEST, RESULT, "key"), &[]);
    assert!(
        !ok,
        "a string of 512 bytes was taken for a field of 256: {text}"
    );
    assert!(text.contains("scxml/static-datamodel-rule"), "{text}");
    // A value of the wrong type is no value of the field.
    let (ok, text) = run(&["check"], &kv_get("256", REQUEST, RESULT, "wide"), &[]);
    assert!(!ok, "an integer was taken for a string field: {text}");
    assert!(text.contains("scxml/static-datamodel-rule"), "{text}");
    // A text no declaration bounds is a run-time fact.
    let (ok, text) = run(
        &["check"],
        &kv_get("256", REQUEST, RESULT, "key + key"),
        &[],
    );
    assert!(
        !ok,
        "a text of no stated bound was taken for a field: {text}"
    );
    assert!(text.contains("scxml/static-datamodel-rule"), "{text}");
}
