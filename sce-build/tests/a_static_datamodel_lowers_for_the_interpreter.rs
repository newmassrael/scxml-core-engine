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
    // Floor: a scan that found nothing to lower would pass. The counter, the
    // overflow machine and the payload reader use nothing the lowering lacks.
    assert!(
        lowered.len() >= 3,
        "lowered {lowered:?}, refused {:?}",
        refused.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
    // What is refused says which construct, so an author knows what to change.
    for (name, text) in &refused {
        assert!(
            text.contains("has no ecmascript lowering"),
            "{name}: the refusal names the construct: {text}"
        );
    }
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
