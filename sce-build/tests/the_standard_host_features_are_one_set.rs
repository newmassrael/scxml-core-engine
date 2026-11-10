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

use sce_build::forge::codegen_matrix::emitting;
use sce_build::forge::model::ForgeKind;
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
    emitting(ForgeKind::Statechart)
        .map(Language::canonical_name)
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

// ── A model's generation and its answer schema (docs/adr/0087, section 4) ──

/// The answer schema the probes below name: a title and a slot.
const ANSWER: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="answer" sce:event-name="probe.answer">
  <datamodel>
    <data id="title" sce:type="string[16]" sce:direction="in"/>
    <data id="slot" sce:type="uint16" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// An open enum, which has no grammar: a value of it may be a number no name
/// spells.
const OPEN_KIND: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="open_kind" sce:underlying-type="uint8"
       sce:strict-variants="false">
  <datamodel>
    <data id="variants">
      <sce:variant name="meeting" value="0"/>
    </data>
  </datamodel>
</scxml>"#;

/// An answer schema holding the open enum.
const OPEN_ANSWER: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="open_answer" sce:event-name="probe.open">
  <sce:import kind="enum" src="open_kind.scxml" as="OpenKind"/>
  <datamodel>
    <data id="kind" sce:type="enum:OpenKind" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// An answer schema whose record holds a list of records with a lower count:
/// a plan of one to three steps (docs/adr/0087, "Landing order" item 8).
const PLAN: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="plan" sce:event-name="probe.plan">
  <sce:import kind="event-schema" src="plan_step.scxml" as="Step"/>
  <datamodel>
    <data id="steps" sce:type="list&lt;record:Step&gt;[1..3]" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// One step of [`PLAN`].
const PLAN_STEP: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="plan_step" sce:event-name="probe.plan.step">
  <datamodel>
    <data id="title" sce:type="string[8]" sce:direction="in"/>
    <data id="at" sce:type="uint8" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// The siblings every model probe is read beside.
const MODEL_SIBLINGS: [(&str, &str); 5] = [
    ("answer.scxml", ANSWER),
    ("open_kind.scxml", OPEN_KIND),
    ("open_answer.scxml", OPEN_ANSWER),
    ("plan.scxml", PLAN),
    ("plan_step.scxml", PLAN_STEP),
];

/// A `sce-static` statechart that asks `invoke_type` for a text, the invoke
/// carrying `attrs` (an `sce:answer`, say) and the extra `<param>`s `extra`.
fn model_generate(invoke_type: &str, attrs: &str, extra: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="sce:std/host/model_generate_request.scxml" as="GenerateRequest"/>
  <sce:import kind="event-schema" src="sce:std/host/model_generate_result.scxml" as="GenerateResult"/>
  <sce:import kind="event-schema" src="answer.scxml" as="Answer"/>
  <datamodel>
    <data id="prompt" sce:type="string[64]" expr="'Name a day'"/>
    <data id="said" sce:type="string[4096]" expr="''" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="go" target="asking"/>
  </state>
  <state id="asking">
    <invoke type="{invoke_type}" id="gen"{attrs} sce:request="GenerateRequest" sce:result="GenerateResult">
      <param name="prompt" expr="prompt"/>
      <param name="max_tokens" expr="64"/>
      <param name="temperature_milli" expr="0"/>{extra}
    </invoke>
    <transition event="done.invoke.gen" target="idle">
      <assign location="said" expr="_event.data.text"/>
    </transition>
  </state>
</scxml>
"#
    )
}

const MODEL_GENERATE: &str = "sce-host-model-generate";

/// The id of `answer.scxml`'s payload schema, which the build writes into the
/// request.
const ANSWER_ID: &str = "urn:sce:schema:answer:1";

/// `sce-codegen generate -l <language>` of `doc` beside the model siblings:
/// whether it succeeded, what it said, and the text of every file of the
/// machine it wrote (named after the machine, `probe`), so that a schema unit
/// written beside it, which declares its own id and JSON Schema, is not read as
/// the machine's.
fn generate(language: &str, doc: &str) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    for (name, text) in MODEL_SIBLINGS {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", language]);
    if language == "go" {
        command.args(["--go-module-prefix", "example.test/generated"]);
    }
    let out = command
        .arg(&path)
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    let mut written = String::new();
    let mut machine_files = 0;
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let path = entry.expect("an entry").path();
        let of_machine = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().to_lowercase().contains("probe"));
        if path.is_file() && of_machine {
            machine_files += 1;
            written.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            written.push('\n');
        }
    }
    assert!(
        !out.status.success() || machine_files > 0,
        "{language} wrote no file named after the machine: a test that read none would pass"
    );
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
        written,
    )
}

#[test]
fn a_model_generation_is_served_on_every_backend_with_its_answer_schema() {
    let doc = model_generate(MODEL_GENERATE, r#" sce:answer="Answer""#, "");
    for language in languages() {
        let (ok, text) = run(&["check", "-l", language], &doc, &MODEL_SIBLINGS);
        assert!(ok, "{language} refused a model's generation: {text}");
        assert!(
            text.contains(&format!(r#""host_invoker_types":["{MODEL_GENERATE}"]"#)),
            "{language}: the type was not served: {text}"
        );
    }
}

/// The build writes the answer schema's id into the request, and declares the
/// machine's table of answer schemas, the id beside both of its grammars. C and
/// C++ hold each grammar as characters, so only the id is read as text there.
#[test]
fn the_build_writes_the_answer_schema_and_the_table_of_its_grammars() {
    let doc = model_generate(MODEL_GENERATE, r#" sce:answer="Answer""#, "");
    for language in languages() {
        let (ok, text, written) = generate(language, &doc);
        assert!(ok, "{language} did not generate: {text}");
        assert!(
            written.contains(ANSWER_ID),
            "{language}: neither the request nor the table names {ANSWER_ID}:\n{written}"
        );
        if !matches!(language, "c11" | "cpp") {
            assert!(
                written.contains("root ::="),
                "{language}: the table holds no GBNF grammar:\n{written}"
            );
            assert!(
                written.contains("json-schema.org/draft/2020-12/schema"),
                "{language}: the table holds no JSON Schema:\n{written}"
            );
        }
    }
}

/// An answer schema whose record holds a list of records is named as any other
/// is (docs/adr/0087, "Landing order" item 8): the build writes its id into the
/// request, and the machine's table holds the composed record's JSON Schema,
/// its list's lower and upper counts and the step's record among its `$defs`,
/// beside its GBNF grammar, both folded from the type as a record of entries'
/// are. C and C++ hold each grammar as characters, so only the id is read as
/// text there.
#[test]
fn an_answer_schema_whose_record_holds_a_list_is_in_the_table_with_its_grammars() {
    let doc = model_generate(MODEL_GENERATE, r#" sce:answer="Plan""#, "").replace(
        r#"<sce:import kind="event-schema" src="answer.scxml" as="Answer"/>"#,
        r#"<sce:import kind="event-schema" src="plan.scxml" as="Plan"/>
  <sce:import kind="event-schema" src="plan_step.scxml" as="Step"/>"#,
    );
    assert!(doc.contains(r#"as="Plan""#), "the probe imports the plan");
    for language in languages() {
        let (ok, text, written) = generate(language, &doc);
        assert!(ok, "{language} did not generate: {text}");
        assert!(
            written.contains("urn:sce:schema:plan:1"),
            "{language}: neither the request nor the table names the plan:\n{written}"
        );
        if !matches!(language, "c11" | "cpp") {
            // `defs/plan_step` is the step's `$ref`, read past the `$` a
            // language escapes in a text.
            for part in ["minItems", "maxItems", "defs/plan_step", "root ::="] {
                assert!(
                    written.contains(part),
                    "{language}: the table holds no `{part}`:\n{written}"
                );
            }
        }
    }
}

/// An invoke that names no answer schema asks for a free text: its request's
/// `answer_schema` is empty and the machine declares no table.
#[test]
fn a_generation_that_names_no_answer_schema_declares_no_table() {
    let doc = model_generate(MODEL_GENERATE, "", "");
    let (ok, text, written) = generate("rust", &doc);
    assert!(ok, "a free generation was refused: {text}");
    assert!(
        !written.contains("ANSWER_SCHEMAS") && !written.contains("urn:sce:schema:"),
        "a machine that names no answer schema declared one:\n{written}"
    );
}

/// A literal written into an unsigned field of a typed request takes the field's
/// type (the probes write `max_tokens` as `64` into a `uint32`), and one the
/// field's range does not hold is refused where it is written, never narrowed.
#[test]
fn a_literal_takes_the_type_of_the_request_field_it_supplies() {
    let accepted = model_generate(MODEL_GENERATE, "", "");
    let (ok, text) = run(&["check"], &accepted, &MODEL_SIBLINGS);
    assert!(ok, "a literal an unsigned field holds was refused: {text}");
    for (field, past) in [("max_tokens", "-1"), ("temperature_milli", "70000")] {
        let doc = accepted.replace(
            &format!(
                "<param name=\"{field}\" expr=\"{}\"/>",
                if field == "max_tokens" { "64" } else { "0" }
            ),
            &format!("<param name=\"{field}\" expr=\"{past}\"/>"),
        );
        assert_ne!(
            doc, accepted,
            "the probe writes {field} as the test expects"
        );
        let (ok, text) = run(&["check"], &doc, &MODEL_SIBLINGS);
        assert!(!ok, "{past} was taken for {field}: {text}");
        assert!(
            text.contains("expression/literal-out-of-range"),
            "{field} = {past}: {text}"
        );
    }
}

/// The document does not write the field the build writes.
#[test]
fn a_document_does_not_write_the_answer_schema_of_a_request() {
    let doc = model_generate(
        MODEL_GENERATE,
        r#" sce:answer="Answer""#,
        "\n      <param name=\"answer_schema\" expr=\"'urn:sce:schema:other:1'\"/>",
    );
    let (ok, text) = run(&["check"], &doc, &MODEL_SIBLINGS);
    assert!(!ok, "a document wrote the answer schema itself: {text}");
    assert!(
        text.contains("validation/typed-invoke-request") && text.contains("sce:answer"),
        "{text}"
    );
}

#[test]
fn an_answer_schema_is_refused_on_another_type() {
    let doc = kv_get("256", REQUEST, RESULT, "key").replace(
        r#"<invoke type="sce-host-kv-get" id="get""#,
        r#"<invoke type="sce-host-kv-get" id="get" sce:answer="KvGetResult""#,
    );
    let (ok, text) = run(&["check"], &doc, &[]);
    assert!(!ok, "an answer schema on a key read was accepted: {text}");
    assert!(
        text.contains("validation/model-answer-schema") && text.contains(MODEL_GENERATE),
        "{text}"
    );
}

#[test]
fn an_answer_schema_the_document_does_not_import_is_refused() {
    let doc = model_generate(MODEL_GENERATE, r#" sce:answer="Answr""#, "");
    let (ok, text) = run(&["check"], &doc, &MODEL_SIBLINGS);
    assert!(!ok, "an alias no import names was accepted: {text}");
    assert!(
        text.contains("validation/model-answer-schema") && text.contains("Answr"),
        "{text}"
    );
}

#[test]
fn an_answer_schema_with_no_grammar_is_refused() {
    let doc = model_generate(MODEL_GENERATE, r#" sce:answer="OpenAnswer""#, "").replace(
        r#"<sce:import kind="event-schema" src="answer.scxml" as="Answer"/>"#,
        r#"<sce:import kind="event-schema" src="open_answer.scxml" as="OpenAnswer"/>"#,
    );
    let (ok, text) = run(&["check"], &doc, &MODEL_SIBLINGS);
    assert!(
        !ok,
        "an answer schema holding an open enum was accepted: {text}"
    );
    assert!(
        text.contains("validation/model-answer-schema") && text.contains("grammar"),
        "{text}"
    );
}
