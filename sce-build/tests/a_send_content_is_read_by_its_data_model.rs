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
/// expression-site census reads as verbatim; C11, in a machine with no engine,
/// copies it into the buffer the machine holds.
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
            "static const char sce_lit_[] = \"<books",
            "_sce_donedata = (<books",
        ),
    }
}

/// A `<donedata>`'s XML `<content>` is read by the same classification: the
/// done event carries the document text for the receiver to build the DOM
/// from, and no backend hands the markup to its engine as an expression,
/// which raised error.execution where the document carried a document. Nothing
/// is evaluated, so the machine carries no engine.
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
            out.contains("\"needs_script_engine\":false"),
            "{language:?}: the XML is no expression, so no engine:\n{out}"
        );
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

/// An ECMAScript parent that invokes an inline child whose one `<final>`
/// carries `content` as its `<donedata>` `<content>`, and reads the done
/// event's data in a guard, which costs the parent an engine of its own.
fn invoking_document(content: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="wait"
       datamodel="ecmascript">
  <state id="wait">
    <invoke type="scxml" id="reporter">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
               datamodel="ecmascript" initial="reported">
          <final id="reported">
            <donedata><content>{content}</content></donedata>
          </final>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.reporter" cond="_event.data === 21" target="pass"/>
    <transition event="done.invoke.reporter" target="fail"/>
  </state>
  <final id="pass"/>
  <final id="fail"/>
</scxml>
"#
    )
}

/// How `language` writes a done data finished at build time as the JSON text
/// `21`. C++ and Kotlin land it in `contentData`, the sink the expression-site
/// census reads as verbatim; C11, in a machine with no engine, copies it into
/// the buffer the machine holds.
fn fixed_donedata_spelling(language: Language) -> &'static str {
    match language {
        Language::Rust => "done_event_data = String::from(\"21\");",
        Language::Go => "doneEventData = \"21\"",
        Language::Kotlin => "val contentData = \"21\"",
        Language::Swift => "doneEventData = \"21\"",
        Language::Cpp => "const std::string contentData = \"21\";",
        Language::Python => "_done_data = \"21\"",
        Language::C11 => "static const char sce_lit_[] = \"21\";",
    }
}

/// How a backend whose child is a value its parent holds lifts the child's
/// done data: the parent asks every child through one accessor, and the
/// child's accessor answers from its own buffer when it has no engine and from
/// its engine's state when it has one.
struct DoneDataLift {
    /// The parent's call, the same for every child.
    asked: &'static str,
    /// The child's accessor reading its buffer.
    from_buffer: &'static str,
    /// The child's accessor reading its engine's state.
    from_engine: &'static str,
}

/// The lift `language` writes, for C11; every other backend's child stashes
/// its done data on its runtime whether or not it has an engine, and the
/// parent's runtime lifts it from there, with nothing per child to read.
fn donedata_lift(language: Language) -> Option<DoneDataLift> {
    match language {
        Language::C11 => Some(DoneDataLift {
            asked: "_done_data_wire(&sm->child_",
            from_buffer: "memcpy(out, sm->done_data, _done_n);",
            from_engine: "lua_getglobal(sm->L, \"_sce_donedata_literal\");",
        }),
        Language::Rust
        | Language::Go
        | Language::Kotlin
        | Language::Swift
        | Language::Cpp
        | Language::Python => None,
    }
}

/// A child whose only use of a script would be its `<donedata>`'s inline
/// `<content>`, when the text is a literal, needs no engine: the text is
/// finished at build time by the reading every backend's `<final>` template
/// asks, and the analysis asks the same one. So the parent's engine is its
/// own guard's alone, and the child's done data reaches it all the same.
#[test]
fn a_child_whose_done_data_is_fixed_costs_no_engine() {
    let doc = invoking_document("21");
    for &language in Language::ALL {
        let (ok, out, generated) = generate(language, &doc);
        assert!(ok, "{language:?}: the machines generate:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":true")
                && out.contains("\"kind\":\"transition-guard\""),
            "{language:?}: the parent's guard needs the engine:\n{out}"
        );
        assert!(
            !out.contains("\"kind\":\"child-invoke-needs-script-engine\""),
            "{language:?}: the child's done data is a literal, so the child needs no \
             engine:\n{out}"
        );
        let line = fixed_donedata_spelling(language);
        assert!(
            generated.contains(line),
            "{language:?}: the child's done data is the value the content denotes, \
             `{line}` was not written"
        );
        if let Some(lift) = donedata_lift(language) {
            assert!(
                generated.contains(lift.asked) && generated.contains(lift.from_buffer),
                "{language:?}: the parent asks the child for its done data, and a child \
                 with no engine answers from its buffer"
            );
            assert!(
                !generated.contains(lift.from_engine),
                "{language:?}: a child with no engine has no engine state to answer from"
            );
        }
    }
}

/// The same child whose `<donedata>`'s inline `<content>` is an expression of
/// the document evaluates it as the final is entered, so it keeps its engine,
/// and the parent counts that engine as a cause of its own.
#[test]
fn a_child_whose_done_data_is_an_expression_keeps_its_engine() {
    let doc = invoking_document("20 + 1");
    for &language in Language::ALL {
        let (ok, out, generated) = generate(language, &doc);
        assert!(ok, "{language:?}: the machines generate:\n{out}");
        assert!(
            out.contains("\"kind\":\"child-invoke-needs-script-engine\""),
            "{language:?}: the child evaluates its done data, so it carries an \
             engine:\n{out}"
        );
        let line = fixed_donedata_spelling(language);
        assert!(
            !generated.contains(line),
            "{language:?}: an expression is not folded into a value at build time, \
             `{line}` was written"
        );
        if let Some(lift) = donedata_lift(language) {
            assert!(
                generated.contains(lift.asked) && generated.contains(lift.from_engine),
                "{language:?}: the parent asks the child the same way, and a child with \
                 an engine answers from its engine's state"
            );
            assert!(
                !generated.contains(lift.from_buffer),
                "{language:?}: a child with an engine keeps its done data there, not in \
                 a buffer"
            );
        }
    }
}

/// A parent and the inline child it invokes, both under `datamodel`, the
/// child ending at once in a `<final>` whose `<donedata>` holds `content`.
fn same_model_invoking_document(datamodel: &str, content: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="wait" datamodel="{datamodel}">
  <state id="wait">
    <invoke type="scxml" id="reporter">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
               datamodel="{datamodel}" initial="reported">
          <final id="reported">
            <donedata><content>{content}</content></donedata>
          </final>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.reporter" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#
    )
}

/// How `language`'s child hands the done data of the top-level final it ended
/// at to the runtime its parent lifts it from.
fn donedata_stash(language: Language) -> &'static str {
    match language {
        Language::Rust => "engine.stash_donedata_at_final(done_event_data);",
        Language::Go => "engine.StashDonedataAtFinal(doneEventData)",
        Language::Kotlin => "stashDonedataAtFinal(doneEventData)",
        Language::Swift => "engine.stashDonedataAtFinal(doneEventData)",
        Language::Cpp => "engine.stashDonedataAtFinal(std::move(eventData), std::move(typedData));",
        Language::Python => "engine.done_data = _done_data",
        // A C11 child is a value its parent holds: the parent asks it
        // (`donedata_lift`), and nothing is stashed.
        Language::C11 => "_done_data_wire(",
    }
}

/// A parent under `datamodel` that invokes a child under the same data model,
/// whose done data is the inline `content`, generated for every backend: no
/// machine needs a script engine, and the child's done data still reaches the
/// parent. The text the build finished, `wire` as a backend's string literal
/// spells it, is what the child hands on; on C11, whose parent lifted only a
/// child that carried an engine, the parent asks the child through the one
/// accessor every child answers, which reads the child's buffer.
fn assert_an_engine_less_childs_done_data_reaches_its_parent(
    datamodel: &str,
    content: &str,
    wire: &str,
) {
    let doc = same_model_invoking_document(datamodel, content);
    for &language in Language::ALL {
        let (ok, out, generated) = generate(language, &doc);
        assert!(
            ok,
            "{language:?} {datamodel}: the machines generate:\n{out}"
        );
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language:?} {datamodel}: neither machine needs an engine:\n{out}"
        );
        assert!(
            generated.contains(wire),
            "{language:?} {datamodel}: the child's done data is the string the content \
             spells, finished at build time (`{wire}`)"
        );
        let stash = donedata_stash(language);
        assert!(
            generated.contains(stash),
            "{language:?} {datamodel}: the child hands its done data on (`{stash}`)"
        );
        if let Some(lift) = donedata_lift(language) {
            assert!(
                generated.contains(lift.asked) && generated.contains(lift.from_buffer),
                "{language:?} {datamodel}: the parent asks the child for its done data, \
                 and the child answers from its buffer"
            );
        }
    }
}

/// A `sce-static` child's inline done data is the text its lowering finished
/// (`DoneData::native_content`), held in the machine's own buffer and in no
/// engine; it reaches the parent as an engine child's does.
#[test]
fn a_static_childs_done_data_reaches_its_parent() {
    assert_an_engine_less_childs_done_data_reaches_its_parent(
        "sce-static",
        "static   words",
        "\\\"static words\\\"",
    );
}

/// A `null` child's inline done data is read by the one reading inline content
/// has, which finds no expression under a data model that has none: the
/// space-normalized string, finished at build time, which reaches the parent
/// as an engine child's does.
#[test]
fn a_null_childs_done_data_reaches_its_parent() {
    assert_an_engine_less_childs_done_data_reaches_its_parent(
        "null",
        "null   words",
        "\\\"null words\\\"",
    );
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
