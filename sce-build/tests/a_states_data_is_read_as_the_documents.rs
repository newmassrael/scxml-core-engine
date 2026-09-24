// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A state's `<data>` is read as the document's is.
//!
//! W3C SCXML 5.3 gives `<data>` one meaning wherever a `<datamodel>` holds
//! it — the document's, or a state's — and the generator read the two with
//! two readers. A state's took the element's text alone, so an in-line XML
//! value, which every backend initialises as a DOM at document scope, was
//! initialised as null at state scope. And it never looked at `sce:kind`:
//! a kind declared in a state's datamodel generated nothing and said nothing
//! (measured 2026-09-24).
//!
//! Every backend initialises a state's variables through the same call as
//! the document's — a macro both scopes share, or the runtime helper both
//! reach — so the generated code is where the value has to arrive. The
//! value's text carries no quote, so it reads the same in every language's
//! literal and the check needs no per-backend spelling.

use std::path::Path;
use std::process::Command;

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" name="scoped_data" initial="idle">
  <datamodel>
    <data id="doc"><shelf xmlns=""><shelfmark>document-scoped</shelfmark></shelf></data>
  </datamodel>
  <state id="idle">
    <datamodel>
      <data id="inner"><shelf xmlns=""><shelfmark>state-scoped</shelfmark></shelf></data>
      <data id="rpmStatus" sce:kind="lookup">
        <datamodel>
          <data id="raw" sce:type="int32" sce:direction="in"/>
          <data id="status" sce:type="string" sce:direction="out"/>
          <data id="mapping" sce:default="OFF">
            <sce:entry key="0" value="OFF"/>
            <sce:entry key="1" value="RUNNING"/>
          </data>
        </datamodel>
      </data>
    </datamodel>
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// The state-scoped value as its text appears in any generated literal.
const STATE_VALUE: &str = "<shelfmark>state-scoped</shelfmark>";

fn model() -> SCXMLModel {
    SCXMLParser::new()
        .parse_string(DOCUMENT, "scoped_data")
        .unwrap_or_else(|err| panic!("the document parses: {err}"))
}

#[test]
fn a_states_xml_value_is_read_as_the_documents_is() {
    let model = model();
    let doc = model
        .variables
        .iter()
        .find(|v| v.id == "doc")
        .expect("the document's <data id=\"doc\">");
    let state = model.states.get("idle").expect("state idle");
    let inner = state
        .datamodel
        .iter()
        .find(|v| v.id == "inner")
        .expect("the state's <data id=\"inner\">");
    assert_eq!(
        inner.content,
        doc.content.replace("document-scoped", "state-scoped"),
        "the same element reads the same in either scope"
    );
    assert!(
        inner.content.contains(STATE_VALUE),
        "the state's value is its XML: {:?}",
        inner.content
    );
}

#[test]
fn a_kind_declared_in_a_state_is_a_kind() {
    let model = model();
    assert!(
        model.inline_kinds.iter().any(|kind| kind.id == "rpmStatus"),
        "the state's <data sce:kind> is a kind declared in place"
    );
    let state = model.states.get("idle").expect("state idle");
    assert!(
        state.datamodel.iter().all(|v| v.id != "rpmStatus"),
        "a kind declared in place is not also a variable"
    );
}

#[test]
fn a_kind_a_state_cannot_hold_is_refused_there_as_well() {
    let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
        xmlns:sce="http://sce.dev/ext" initial="idle">
  <state id="idle">
    <datamodel><data id="value" sce:kind="timer"/></datamodel>
  </state>
</scxml>"#;
    let err = SCXMLParser::new()
        .parse_string(document, "timer_in_a_state")
        .expect_err("a timer cannot be declared in place");
    let diagnostics = err.to_diagnostics();
    assert!(
        matches!(
            diagnostics.first().map(|d| &d.code),
            Some(DiagnosticCode::ValidationKindNotInlineEligible)
        ),
        "refused as the document's datamodel refuses it: {diagnostics:?}"
    );
    assert_eq!(
        err.location.line,
        Some(4),
        "placed on the state's <data>, where the author wrote it"
    );
}

/// Every backend's generated code carries the state's value into the
/// initialisation it shares with the document's, and reports the kind the
/// state declares as a sibling artifact.
#[test]
fn every_backend_initialises_the_states_value_and_emits_its_kind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("scoped_data.scxml");
    std::fs::write(&input, DOCUMENT).expect("write the document");

    for language in sce_build::generator::Language::ALL {
        let out_dir = dir.path().join(language.canonical_name());
        std::fs::create_dir_all(&out_dir).expect("output dir");
        // The text is read as the generator writes it: an external formatter
        // is a property of the host, and the assertions below are about the
        // generator.
        let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
            .arg("generate")
            .arg(&input)
            .arg("-o")
            .arg(&out_dir)
            .arg("-l")
            .arg(language.canonical_name())
            .arg("--no-format")
            .current_dir(repo_root())
            .output()
            .expect("sce-codegen runs");
        assert!(
            run.status.success(),
            "generate ({language:?}) failed: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        let artifacts = artifacts_of(&String::from_utf8_lossy(&run.stdout));

        let generated: String = artifacts
            .iter()
            .map(|path| std::fs::read_to_string(path).unwrap_or_default())
            .collect();
        assert!(
            generated.contains(STATE_VALUE),
            "generate ({language:?}) does not initialise the state's <data id=\"inner\"> \
             with its value; artifacts: {artifacts:?}"
        );

        let stem = sce_build::filters::to_snake_case("scoped_data_rpmStatus".to_string());
        assert!(
            artifacts.iter().any(|path| {
                let file = Path::new(path)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();
                sce_build::filters::to_snake_case(file).starts_with(&stem)
            }),
            "generate ({language:?}) reported no artifact for the kind the state declares; \
             it reported: {artifacts:?}"
        );
    }
}

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build sits under the repository root")
        .to_path_buf()
}

/// The artifact paths one `generate` run reports in its manifest line.
fn artifacts_of(stdout: &str) -> Vec<String> {
    let line = stdout
        .lines()
        .find(|l| l.starts_with('{'))
        .expect("generate prints one JSON manifest line");
    let manifest: serde_json::Value =
        serde_json::from_str(line).expect("the manifest line is JSON");
    manifest["artifacts"]
        .as_array()
        .expect("the manifest carries an artifacts array")
        .iter()
        .map(|a| {
            a["path"]
                .as_str()
                .expect("every artifact carries a path")
                .to_string()
        })
        .collect()
}
