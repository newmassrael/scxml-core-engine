// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reading of a `<send>`'s inline `<content>` text is the data model's.
//
// Read in the templates on 2026-11-04: the Rust, Go and C++ backends read the
// text of `<send><content>21</content></send>` under `datamodel="ecmascript"`
// by asking whether the WHOLE document needed a script engine. A document that
// carried one for another reason evaluated the text and sent the number 21; a
// document that carried none sent the string "21", under a comment claiming
// there was no data model. The reading is now decided from the text and the
// data model alone (`filters::inline_content_reading`): a literal is the value
// it denotes, finished at build time, and only an expression of the document
// is evaluated, which is then what costs the machine its engine.
//
// Each case is generated for every backend that writes a `<send>`, and the
// generated machine is read for the event's data as the wire text the build
// finished: the same text whether or not the document needs an engine.

use std::path::{Path, PathBuf};
use std::process::Command;

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

/// An ECMAScript document whose state `s` sends `number`, `text` and `object`
/// to itself, each carrying an inline `<content>`, with `data` as its
/// `<datamodel>` body.
fn document(data: &str, number: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s"
       datamodel="ecmascript">
  <datamodel>{data}</datamodel>
  <state id="s">
    <onentry>
      <send event="number"><content>{number}</content></send>
      <send event="text"><content>  two
        words  </content></send>
      <send event="object"><content>{{"k": 7}}</content></send>
    </onentry>
    <transition event="number"/>
    <transition event="text"/>
    <transition event="object" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#
    )
}

/// `sce-codegen generate -l <language>` of `doc`: whether it succeeded, its
/// stdout and stderr (the manifest among them), and the text of every file it
/// wrote.
fn generate(language: Language, doc: &str) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).expect("an output directory");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", language.canonical_name(), "-o"])
        .arg(&out_dir)
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    let mut generated = String::new();
    for entry in std::fs::read_dir(&out_dir).expect("the output directory") {
        let file = entry.expect("an entry").path();
        if let Ok(text) = std::fs::read_to_string(&file) {
            generated.push_str(&text);
        }
    }
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
        generated,
    )
}

/// How `language` spells the event's data when the build finished it as
/// `wire`: the string literal carrying the JSON text, or for Python the value
/// the JSON denotes (`py_json_literal`). C++ lands it in `contentData`, the
/// sink the expression-site census reads as verbatim. Every backend writes a
/// `<send>`'s inline `<content>`, so every one has a spelling.
fn spell(language: Language, wire: &str) -> String {
    let quoted = format!("\"{}\"", wire.replace('\\', "\\\\").replace('"', "\\\""));
    match language {
        Language::Rust => format!("let event_data: &str = {quoted};"),
        Language::Go => format!("eventDataStr := {quoted}"),
        Language::Kotlin => format!("val sendData = {quoted}"),
        Language::Swift => format!("let sendEventData = {quoted}"),
        Language::Cpp => format!("const std::string contentData = {quoted};"),
        Language::C11 => format!("static const char _lit[] = {quoted};"),
        Language::Python => {
            let value = match wire {
                "21" => "21",
                "\"two words\"" => "\"two words\"",
                "{\"k\":7}" => "{\"k\": 7}",
                other => panic!("no Python spelling written for {other}"),
            };
            format!("_data = {value}")
        }
    }
}

/// The wire texts the three sends carry: a number, the space-normalized
/// string, and an object.
const WIRES: [&str; 3] = ["21", "\"two words\"", "{\"k\":7}"];

/// Nothing else in the document needs an engine, and the content costs none:
/// the literals are their values and the text is the string it spells.
#[test]
fn a_literal_content_is_its_value_and_costs_no_engine() {
    let doc = document("", "21");
    for &language in Language::ALL {
        let (ok, out, generated) = generate(language, &doc);
        assert!(ok, "{language:?}: the machine generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language:?}: no content here is an expression, so no engine:\n{out}"
        );
        for wire in WIRES {
            let line = spell(language, wire);
            assert!(
                generated.contains(&line),
                "{language:?}: the event's data is the value the content denotes, \
                 `{line}` was not written"
            );
        }
    }
}

/// The same content in a document that needs an engine for another reason is
/// the same value, written the same way: the reading does not follow from
/// what else the document holds.
#[test]
fn the_reading_does_not_follow_from_an_engine_needed_elsewhere() {
    let doc = document(r#"<data id="v" expr="1 + 1"/>"#, "21");
    for &language in Language::ALL {
        let (ok, out, generated) = generate(language, &doc);
        assert!(ok, "{language:?}: the machine generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":true"),
            "{language:?}: the `<data>` initializer needs the engine:\n{out}"
        );
        assert!(
            !out.contains("\"kind\":\"send-content-expr\""),
            "{language:?}: no content here is an expression:\n{out}"
        );
        for wire in WIRES {
            let line = spell(language, wire);
            assert!(
                generated.contains(&line),
                "{language:?}: with an engine in the machine the content is still the \
                 value it denotes, `{line}` was not written"
            );
        }
    }
}

/// How `language` writes the done data it finished at build time, up to the
/// markup's first element, and how it would have handed the markup to its
/// engine. C++ and Kotlin land it in `contentData`, the sink the
/// expression-site census reads as verbatim.
fn donedata_spelling(language: Language) -> (&'static str, &'static str) {
    match language {
        Language::Rust => (
            "done_event_data = String::from(\"<books",
            "evaluate_expression(&sid, \"<books",
        ),
        Language::Go => (
            "doneEventData = \"<books",
            "EvaluateExpression(p.SessionID, `<books",
        ),
        Language::Kotlin => ("val contentData = \"<books", "engineDD.evaluateExpr(sidDD"),
        Language::Swift => ("doneEventData = \"<books", "evaluateJSON(\"<books"),
        Language::Cpp => (
            "const std::string contentData = \"<books",
            "DoneDataHelper::evaluateContent(",
        ),
        Language::Python => (
            "_done_data = \"<books",
            "_done_data = engine._script_engine.evaluate_expression(",
        ),
        Language::C11 => (
            "_sce_donedata = setmetatable({ \\\"<books",
            "_sce_donedata = (<books",
        ),
    }
}

/// A `<donedata>`'s XML `<content>` is read by the same classification: the
/// done event carries the document text for the receiver to build the DOM
/// from, and no backend hands the markup to its engine as an expression,
/// which raised error.execution where the document carried a document.
#[test]
fn a_donedata_xml_content_is_its_text_and_not_an_expression() {
    let doc = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s"
       datamodel="ecmascript">
  <state id="s">
    <transition target="f"/>
  </state>
  <final id="f">
    <donedata><content><books xmlns=""><book title="x"/></books></content></donedata>
  </final>
</scxml>
"#;
    for &language in Language::ALL {
        let (carried, evaluated) = donedata_spelling(language);
        let (ok, out, generated) = generate(language, doc);
        assert!(ok, "{language:?}: the machine generates:\n{out}");
        assert!(
            generated.contains(carried),
            "{language:?}: the done data is the XML as written, `{carried}` was not written"
        );
        assert!(
            !generated.contains(evaluated),
            "{language:?}: the XML is handed to the engine as an expression (`{evaluated}`)"
        );
    }
}

/// Text that is an expression of the document is evaluated when the send
/// runs, and that is what costs the machine its engine: the analysis names
/// the content as the cause.
#[test]
fn an_expression_content_costs_the_engine() {
    let doc = document("", "20 + 1");
    for &language in Language::ALL {
        let (ok, out, generated) = generate(language, &doc);
        assert!(ok, "{language:?}: the machine generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":true")
                && out.contains("\"kind\":\"send-content-expr\""),
            "{language:?}: the expression is the engine's cause:\n{out}"
        );
        assert!(
            !generated.contains(&spell(language, "21")),
            "{language:?}: an expression is not folded into a value at build time"
        );
        assert!(
            generated.contains(&spell(language, "\"two words\"")),
            "{language:?}: the text beside it is still the string it spells"
        );
    }
}
