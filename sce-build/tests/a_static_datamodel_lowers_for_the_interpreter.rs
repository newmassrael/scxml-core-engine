// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"` lowered to the ecmascript document the Interpreter
// runs (docs/SCE_ACCEPTED_SUBSET.md §2.15) — every statechart the shared
// fixture directory holds, through the same entry `sce-codegen lower` uses.
//
// What is held here is the lowering's own contract, with no engine to run it:
// a document is either lowered to something the Interpreter can read, or
// refused with `generate/unsupported-feature` naming the construct — never
// passed through half lowered. Whether the lowered document then BEHAVES as
// the generated backends do is the C++ suite's question
// (`tests/integration/AStaticDatamodelRunsLoweredUnderTheInterpreterTest.cpp`,
// which replays the scenarios the AOT backends replay).
//
// Nothing is listed: the fixtures are found by what they declare, so a new
// one is judged the day it is added.

use std::path::{Path, PathBuf};

use sce_build::forge::static_js::{lower_file, lower_source};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel")
}

/// Every statechart under the fixture directory that declares
/// `datamodel="sce-static"`, sorted so a failure names the same one each run.
fn static_statecharts() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(fixtures())
        .expect("the fixture directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "scxml"))
        .filter(|path| {
            std::fs::read_to_string(path)
                .expect("a readable fixture")
                .contains(r#"datamodel="sce-static""#)
        })
        .collect();
    found.sort();
    found
}

fn scxml_element<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    node.children().find(|n| {
        n.is_element()
            && n.tag_name().name() == name
            && n.tag_name().namespace() == Some("http://www.w3.org/2005/07/scxml")
    })
}

#[test]
fn every_fixture_is_lowered_or_refused_by_name() {
    let mut lowered = Vec::new();
    let mut refused = Vec::new();
    for path in static_statecharts() {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        match lower_file(path.to_str().unwrap(), Vec::new()) {
            Ok(document) => {
                let parsed = roxmltree::Document::parse(&document)
                    .unwrap_or_else(|e| panic!("{name}: the lowered document is not XML: {e}"));
                let root = parsed.root_element();
                assert_eq!(
                    root.attribute("datamodel"),
                    Some("ecmascript"),
                    "{name}: the Interpreter runs an ecmascript document"
                );
                // The library is installed before anything that reads it, and
                // only when something does.
                let calls_the_library = document.contains("SceStatic.");
                let first_data = scxml_element(root, "datamodel")
                    .and_then(|datamodel| scxml_element(datamodel, "data"));
                if calls_the_library {
                    assert_eq!(
                        first_data.and_then(|d| d.attribute("id")),
                        Some("SceStaticInstalled"),
                        "{name}: the library is the document's first <data>"
                    );
                }
                // A lowered document is an ecmascript one, and lowering that
                // again changes nothing.
                assert_eq!(
                    lower_source(&document, &name).expect("lowers again"),
                    document,
                    "{name}: lowering is idempotent"
                );
                lowered.push(name);
            }
            Err(refusal) => {
                let text = format!("{refusal:?}");
                assert!(
                    text.contains("UnsupportedFeature"),
                    "{name}: a document it cannot lower is refused as an unsupported feature, \
                     not by another error: {text}"
                );
                refused.push((name, text));
            }
        }
    }
    // Floor: a scan that found nothing to lower would pass. What is lowered
    // today — scalars, checked integers, the typed payload, lists, records,
    // calls of scalar algorithms and a `<send>`'s `<param>`s — is at least these
    // twenty-five machines, and the floor rises as the lowering grows.
    assert!(
        lowered.len() >= 25,
        "lowered {lowered:?}, refused {:?}",
        refused.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
    // The pairs of a `<send>` are expressions the walk lowers, in the attribute
    // each is written in, and the Interpreter's own `<send>` reads them once when
    // it runs: so the two fixtures that send themselves their `<param>`s lower,
    // an enum value among them as the name its enum declares.
    for name in ["static_send_params", "static_wire_enum"] {
        assert!(
            lowered.iter().any(|lowered_name| lowered_name == name),
            "{name} is lowered for the Interpreter: lowered {lowered:?}"
        );
    }
    // A host operation is performed by the host installed on the machine
    // (`INativeActionHost`), so `static_host_call` lowers: its arguments are
    // expressions, lowered as any are.
    assert!(
        lowered.iter().any(|name| name == "static_host_call"),
        "static_host_call is lowered for the Interpreter: lowered {lowered:?}"
    );
    // What is refused says which construct, so an author knows what to change.
    for (name, text) in &refused {
        assert!(
            text.contains("has no ecmascript lowering"),
            "{name}: the refusal names the construct: {text}"
        );
    }
}

fn lowered_fixture(machine: &str) -> String {
    let path = fixtures().join(format!("{machine}.scxml"));
    lower_file(path.to_str().unwrap(), Vec::new())
        .unwrap_or_else(|e| panic!("{machine} does not lower: {e:?}"))
}

/// A record is declared by one `<data>` built whole from its fields, read as
/// the author wrote it, and written a field at a time as an `<assign>` of the
/// whole record — through the library, which returns the record changed in that
/// field and leaves the one it was given alone.
#[test]
fn a_record_is_built_whole_and_written_a_field_at_a_time() {
    let out = lowered_fixture("static_record_fields");
    let document = roxmltree::Document::parse(&out).expect("well-formed");
    // The attributes as a reader decodes them, which is what the script engine
    // is handed: `'` is written `&apos;` in the text of the document.
    let attribute = |name: &str| -> Vec<(String, String)> {
        document
            .descendants()
            .filter(|n| n.is_element())
            .filter_map(|n| {
                n.attribute(name)
                    .map(|v| (n.tag_name().name().to_string(), v.to_string()))
            })
            .collect()
    };
    let expr = attribute("expr");
    let has = |element: &str, value: &str| expr.contains(&(element.to_string(), value.to_string()));
    assert!(
        has("data", "({'year': 2026, 'month': 9, 'dayOfMonth': 24})"),
        "the record is built whole from its fields: {expr:?}"
    );
    assert!(
        has(
            "assign",
            "SceStatic.set(shown, 'dayOfMonth', SceStatic.U8.add(shown.dayOfMonth, 1))"
        ),
        "a field written from the record's own value: {expr:?}"
    );
    assert!(
        has(
            "assign",
            "SceStatic.set(shown, 'year', SceStatic.field(_event.data, 'year', 'uint16'))"
        ),
        "a field replaced from the typed payload: {expr:?}"
    );
    assert!(
        attribute("cond").contains(&(
            "transition".to_string(),
            "shown.dayOfMonth < 28".to_string()
        )),
        "a read of a field, in a guard that cannot fail, is the author's"
    );
    // The `<sce:set>`s are gone as elements — the header comment that names
    // them is the author's, and stays.
    assert!(
        !document
            .descendants()
            .any(|n| n.is_element() && n.tag_name().name() == "set"),
        "{out}"
    );
}

/// An algorithm the document imports travels in it: installed by the library's
/// `<data>` under `SceStatic.algorithms`, and called by that name from the
/// guard that calls it — the function a generated machine would have linked.
#[test]
fn an_imported_algorithm_travels_in_the_document_and_is_called_by_its_name() {
    let out = lowered_fixture("static_record");
    let document = roxmltree::Document::parse(&out).expect("well-formed");
    let installer = document
        .descendants()
        .find(|n| n.attribute("id") == Some("SceStaticInstalled"))
        .and_then(|n| n.attribute("expr"))
        .expect("the library is installed");
    assert!(
        installer.contains(
            "SceStatic.algorithms.days_in_month = function (year, month) { var days = 31;"
        ),
        "the algorithm is installed beside the library: {installer}"
    );
    let guard = document
        .descendants()
        .filter(|n| n.attribute("event") == Some("next"))
        .find_map(|n| n.attribute("cond"))
        .expect("the guard that calls it");
    assert_eq!(
        guard, "shown.dayOfMonth < SceStatic.algorithms.days_in_month(shown.year, shown.month)",
        "the call reads the record's fields, and fails as an expression of the document does"
    );

    // A may-fail algorithm keeps its preconditions, and the document that
    // calls it says nothing of how a failure is passed on: it is a throw.
    let sync = lowered_fixture("sync_client");
    assert!(
        sync.contains("SceStatic.require("),
        "a precondition of sync_failure is checked: {sync}"
    );
}

/// A document under another data model is not this lowering's to change.
#[test]
fn a_document_under_another_data_model_is_returned_as_written() {
    let ecmascript = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="s">
  <datamodel><data id="n" expr="1 + 2"/></datamodel>
  <state id="s"/>
</scxml>"#;
    assert_eq!(lower_source(ecmascript, "plain").unwrap(), ecmascript);
}
