// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"` — SCE's platform-defined, statically typed data
// model (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// A variable under it is a native field of the generated machine, so every
// rule here is one a field needs: a declared type, an initial value that
// is a typed expression, an id generated code can spell. Each refusal is
// asserted on its wire code AND on the row it is placed on, because a
// refusal on the wrong line sends the author to the wrong element.
//
// The last cases pin the other half of the model's first step: until a
// backend's templates lower it, generation refuses it for that backend
// rather than handing forge-language expressions to a script engine.

use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Run `sce-codegen <args…> <doc>` with JSON diagnostics and return
/// `(exit_ok, stdout + stderr)`. `check` with no `--language` fails only
/// on the document axis; `check -l X` fails on X's backend axis too.
fn run(args: &[&str], doc: &str) -> (bool, String) {
    run_beside(args, doc, &[])
}

/// [`run`], with `siblings` — `(file name, text)` — written beside the
/// document, where its `<sce:import src>`s resolve.
fn run_beside(args: &[&str], doc: &str, siblings: &[(&str, &str)]) -> (bool, String) {
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

/// A statechart with the given `datamodel` value and `<datamodel>` body.
/// The `<datamodel>` opens on line 4, so its first child is on line 5.
fn doc(datamodel: &str, data: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="{datamodel}">
  <datamodel>
    {data}
  </datamodel>
  <state id="s"><transition event="go" target="done"/></state>
  <final id="done"/>
</scxml>
"##
    )
}

/// Assert a refusal carries `code` and is placed on `line`.
fn assert_refused_at(out: &str, code: &str, line: u32) {
    let record = out
        .lines()
        .find(|l| l.contains(&format!("\"code\":\"{code}\"")))
        .unwrap_or_else(|| panic!("expected {code}, got:\n{out}"));
    // A refusal placed on a row alone (an import) carries no column.
    assert!(
        record.contains(&format!("\"line\":{line},"))
            || record.contains(&format!("\"line\":{line}}}")),
        "{code} must be placed on line {line}:\n{record}"
    );
}

#[test]
fn a_typed_variable_is_accepted_under_the_static_data_model() {
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="count" sce:type="uint32" expr="0"/>"#,
        ),
    );
    assert!(ok, "a typed <data> is what sce-static asks for:\n{out}");
}

#[test]
fn an_untyped_variable_is_refused_on_its_own_line() {
    let (ok, out) = run(
        &["check"],
        &doc("sce-static", r#"<data id="count" expr="0"/>"#),
    );
    assert!(!ok, "a <data> with no sce:type must be refused:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn a_variable_read_from_src_is_refused() {
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="count" sce:type="uint32" src="count.json"/>"#,
        ),
    );
    assert!(!ok, "src has no type to give a field:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
    assert!(
        out.contains("count.json"),
        "`actual` is the src as written:\n{out}"
    );
}

#[test]
fn a_variable_with_in_line_content_is_refused() {
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="count" sce:type="uint32">7</data>"#,
        ),
    );
    assert!(!ok, "in-line content has no type to give a field:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn script_text_is_refused_under_the_static_data_model() {
    let (ok, out) = run(
        &["check"],
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s"
       datamodel="sce-static">
  <state id="s">
    <onentry><script>x = 1</script></onentry>
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"##,
    );
    assert!(!ok, "script text needs a scripting language:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn a_declared_type_under_ecmascript_is_the_authoring_io_declaration() {
    // Under `ecmascript`, `sce:type` with `sce:direction` is the typed
    // input/output declaration the authoring tool drives a statechart
    // through. The first cut of this model refused it as unread, and the
    // authoring suite's run-driven cases went red in CI: the attribute was
    // read, only not by the generator.
    let (ok, out) = run(
        &["check"],
        &doc(
            "ecmascript",
            r#"<data id="count" sce:type="int32" sce:direction="out" expr="0"/>"#,
        ),
    );
    assert!(ok, "the authoring I/O declaration is admitted:\n{out}");
}

#[test]
fn a_misspelled_type_is_refused_on_its_line() {
    // Two layers refuse this, and which one answers depends on whether the
    // build loads the XSD: the schema types `sce:type` as
    // `sceTypeOrEnumRef` (`xml/schema-validation`), and without it the
    // parser's `read_type_attr` refuses the same value
    // (`validation/invalid-attribute`). The case pins the outcome both
    // share — refused, on the attribute's line, naming what was written.
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="count" sce:type="uint33" expr="0"/>"#,
        ),
    );
    assert!(!ok, "uint33 is no type:\n{out}");
    let record = out
        .lines()
        .find(|l| {
            l.contains("\"code\":\"xml/schema-validation\"")
                || l.contains("\"code\":\"validation/invalid-attribute\"")
        })
        .unwrap_or_else(|| panic!("expected a type refusal, got:\n{out}"));
    assert!(
        record.contains("\"line\":5") && record.contains("uint33"),
        "the refusal names `uint33` on line 5:\n{record}"
    );
}

#[test]
fn a_variable_id_is_held_to_the_code_identifier_grammar() {
    // `raw-value` is a valid xs:ID and so a valid statechart <data id>;
    // under sce-static it names a field, and `-` is an operator in every
    // target language.
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="raw-value" sce:type="uint32" expr="0"/>"#,
        ),
    );
    assert!(!ok, "a field cannot be spelled `raw-value`:\n{out}");
    assert!(
        out.contains("validation/malformed-code-identifier"),
        "expected the code-identifier refusal, got:\n{out}"
    );
}

#[test]
fn the_same_id_is_admitted_under_ecmascript() {
    // The grammar is widened only where the id reaches generated code.
    let (ok, out) = run(
        &["check"],
        &doc("ecmascript", r#"<data id="raw-value" expr="0"/>"#),
    );
    assert!(ok, "an ecmascript <data id> stays an xs:ID:\n{out}");
}

#[test]
fn cpp_names_each_construct_it_does_not_lower_yet() {
    // C++ lowers scalar, enum, record and list variables, guards, `<assign>`,
    // `<if>`, `<foreach>`, `<log>`, `In()`, host actions, an event's typed
    // payload, a call of an imported algorithm, a final's `<donedata>` and an
    // `<invoke type="scxml">`. What is past that is refused by name where the
    // document is read, not left as an undefined name in the generated code.
    let fixtures = repo_root().join("sce-build/tests/fixtures/static_datamodel");
    let bytes_variable = doc(
        "sce-static",
        r#"<data id="frame" sce:type="bytes" expr="''"/>"#,
    );
    let host_invoke = machine(
        r#"<state id="s">
    <invoke type="x-sce-host" id="h"><param name="k" expr="count"/></invoke>
    <transition event="done.invoke.h" target="done"/>
  </state>"#,
    );
    let siblings: Vec<(String, String)> = std::fs::read_dir(&fixtures)
        .expect("the fixture directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "scxml"))
        .map(|path| {
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&path).expect("a fixture"),
            )
        })
        .collect();
    let siblings: Vec<(&str, &str)> = siblings
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let cases = [
        ("a bytes variable", bytes_variable, "of a bytes type"),
        ("a host-run <invoke>", host_invoke, "a host-run <invoke>"),
    ];
    // C++, Go and Python start a scxml child and refuse the rest by name.
    for (lang, name) in [("cpp", "C++"), ("go", "Go"), ("python", "Python")] {
        for (what, document, names) in &cases {
            let (ok, out) = run_beside(
                &["check", "-l", lang, "--go-module-prefix", "x/y"],
                document,
                &siblings,
            );
            assert!(
                !ok,
                "{lang}, {what}: {name} has no lowering for it yet:\n{out}"
            );
            assert!(
                out.contains("generate/unsupported-feature")
                    && out.contains(&format!("no {name} lowering yet")),
                "{lang}, {what}: expected the unsupported-feature refusal naming {name}:\n{out}"
            );
            assert!(
                out.contains(names),
                "{lang}, {what}: it names the construct:\n{out}"
            );
        }
    }
}

#[test]
fn c11_names_each_construct_it_does_not_lower_yet() {
    // C lowers integer, bool, enum and bounded string variables, records of
    // numbers, bools and enums, lists of integers, bools and such records with
    // `<sce:append>`, `<sce:clear>` and `<foreach>`, guards, `<assign>`,
    // `<if>`, `<log>`, `<raise>`, `In()`, `<cancel>`, an event's typed payload of
    // numbers, bools and strings, a call of an imported algorithm, a host action
    // whose arguments are typed expressions of them, the `<param>`s of a final's
    // `<donedata>` and of a `<send>` to the machine's own processor, and an
    // `<invoke type="scxml">` handing numbers and bools. What is past that — a
    // real, a list of reals, a record with a string field, a bytes variable, a
    // `<send>` with a `<content>` or to another processor, a host-run
    // `<invoke>`, a final's `<donedata>` with a `<content>`, a payload field that
    // is bytes or an enum — is refused by
    // name where the document is read, not left as an undefined name in the
    // generated code.
    let fixtures = repo_root().join("sce-build/tests/fixtures/static_datamodel");
    let variable = |data: &str| doc("sce-static", data);
    let cases = [
        (
            "a real variable",
            variable(r#"<data id="ratio" sce:type="float64" expr="0.5"/>"#),
            r#"<data id="ratio" sce:type="float64">"#,
        ),
        (
            "a bytes variable",
            variable(r#"<data id="frame" sce:type="bytes" expr="''"/>"#),
            r#"<data id="frame" sce:type="bytes">"#,
        ),
        (
            "a list of reals",
            variable(r#"<data id="picked" sce:type="list&lt;float64&gt;" sce:capacity="3"/>"#),
            r#"<data id="picked" sce:type="list">"#,
        ),
        (
            "a record with a string field",
            r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_echo.scxml" as="Echo"/>
  <datamodel>
    <data id="heard" sce:type="record:Echo">
      <sce:set name="total" expr="0"/>
      <sce:set name="ok" expr="false"/>
      <sce:set name="tag" expr="''"/>
    </data>
  </datamodel>
  <state id="s"/>
</scxml>
"##
            .to_string(),
            "record:Echo with the field `tag` of type string",
        ),
        (
            "a <send> with a <content>",
            machine(
                r#"<state id="s"><onentry><send event="x"><content>hello</content></send></onentry></state>"#,
            ),
            "a <send> with a <content>",
        ),
        (
            "a host-run <invoke>",
            machine(
                r#"<state id="s">
    <invoke type="x-sce-host" id="h"><param name="k" expr="count"/></invoke>
    <transition event="done.invoke.h" target="done"/>
  </state>"#,
            ),
            "a host-run <invoke>",
        ),
        (
            "a <donedata> with a <content>",
            machine(
                r#"<state id="s"><transition event="go" target="fin"/></state>
  <final id="fin"><donedata><content>hello</content></donedata></final>"#,
            ),
            "a <donedata> with a <content>",
        ),
        (
            "a typed payload with an enum field",
            r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_view.scxml" as="View"/>
  <datamodel><data id="count" sce:type="uint32" expr="0"/></datamodel>
  <state id="s"><transition event="view.shown" cond="_event.data.zoom &gt; 1" target="done"/></state>
  <final id="done"/>
</scxml>
"##
            .to_string(),
            "an event whose payload carries `layout` of type enum:ViewMode",
        ),
    ];
    let siblings: Vec<(String, String)> = std::fs::read_dir(&fixtures)
        .expect("the fixture directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "scxml"))
        .map(|path| {
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&path).expect("a fixture"),
            )
        })
        .collect();
    let siblings: Vec<(&str, &str)> = siblings
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    for (what, document, names) in &cases {
        let (ok, out) = run_beside(&["check", "-l", "c11"], document, &siblings);
        assert!(!ok, "{what}: C has no lowering for it yet:\n{out}");
        assert!(
            out.contains("generate/unsupported-feature") && out.contains("no C11 lowering yet"),
            "{what}: expected the unsupported-feature refusal naming C11:\n{out}"
        );
        // The refusal quotes the construct inside a JSON string.
        let quoted = names.replace('"', "\\\"");
        assert!(
            out.contains(&quoted) || out.contains(names),
            "{what}: it names the construct `{names}`:\n{out}"
        );
    }
}

#[test]
fn every_backend_lowers_the_model_with_no_script_engine() {
    // Each holds the variables as fields and lowers every expression
    // natively, so the machine it generates carries no engine — the manifest
    // says so, and each integration suite (StaticDatamodelTest.kt,
    // backends/rust/tests/tests/static_datamodel.rs,
    // tests/integration/AStaticDatamodelRunsGeneratedCppTest.cpp,
    // backends/go/tests/integration/static_datamodel,
    // backends/python/tests/integration/static_datamodel,
    // backends/c/tests/integration/test_static_scalars.c) drives the machines.
    //
    // A backend that did not lower it would have to refuse instead — a
    // document evaluated by a script engine in a language it never declared is
    // what `datamodel` exists to prevent — and one added to the language list
    // with neither fails here, not for a user.
    for lang in ["kotlin", "rust", "cpp", "go", "python", "c11"] {
        let (ok, out) = run(
            &["check", "-l", lang],
            &machine(
                r#"<state id="s">
    <transition event="tick" cond="count &lt; 10 &amp;&amp; In('s')" type="internal">
      <assign location="count" expr="count + 1"/>
    </transition>
  </state>"#,
            ),
        );
        assert!(ok, "--lang {lang} lowers sce-static:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "--lang {lang}: a sce-static machine needs no script engine:\n{out}"
        );
    }
}

// ── Every expression judged against the typed scope ─────────────────────

/// A `sce-static` machine with two variables — `count: uint32` on line 5,
/// `ready: bool` on line 6 — and `states`, which open on line 8.
fn machine(states: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
    <data id="ready" sce:type="bool" expr="false"/>
  </datamodel>
  {states}
  <final id="done"/>
</scxml>
"##
    )
}

#[test]
fn a_machine_whose_every_expression_is_typed_is_accepted() {
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<state id="s">
    <onentry><log label="n" expr="count"/></onentry>
    <transition event="tick" cond="count &lt; 10 &amp;&amp; In('s')" target="s">
      <assign location="count" expr="count + 1"/>
      <if cond="count === 5"><assign location="ready" expr="true"/></if>
    </transition>
    <transition event="go" cond="ready" target="done"/>
  </state>"#,
        ),
    );
    assert!(ok, "every expression here is typed:\n{out}");
}

#[test]
fn a_variable_initialised_with_a_value_of_another_kind_is_refused() {
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="ready" sce:type="bool" expr="1"/>"#,
        ),
    );
    assert!(
        !ok,
        "an integer does not stand where a bool is declared:\n{out}"
    );
    assert_refused_at(&out, "expression/type-mismatch", 5);
}

#[test]
fn a_variable_with_no_initial_value_is_refused() {
    let (ok, out) = run(
        &["check"],
        &doc("sce-static", r#"<data id="count" sce:type="uint32"/>"#),
    );
    assert!(
        !ok,
        "a field's initial value is written, not implied:\n{out}"
    );
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn a_condition_that_is_not_a_bool_is_refused_on_its_line() {
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<state id="s"><transition event="go" cond="count + 1" target="done"/></state>"#,
        ),
    );
    assert!(!ok, "a condition is a bool:\n{out}");
    assert_refused_at(&out, "expression/type-mismatch", 8);
}

#[test]
fn a_name_nothing_declares_is_refused() {
    // The scope is closed: there is no script engine behind it to supply
    // a name the document does not declare.
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<state id="s"><transition event="go" cond="cuont &gt; 1" target="done"/></state>"#,
        ),
    );
    assert!(!ok, "`cuont` is declared nowhere:\n{out}");
    assert!(
        out.contains("cuont"),
        "the refusal names what was written:\n{out}"
    );
}

#[test]
fn an_assignment_of_another_kind_is_refused() {
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<state id="s"><onentry><assign location="count" expr="true"/></onentry></state>"#,
        ),
    );
    assert!(!ok, "a bool does not stand in a uint32:\n{out}");
    assert_refused_at(&out, "expression/type-mismatch", 8);
}

#[test]
fn an_expression_the_model_has_no_typed_form_for_is_refused() {
    // `eventexpr` is evaluated as script text by every backend's templates,
    // so admitting it would run part of the document in a language it never
    // declared.
    let (ok, out) = run(
        &["check"],
        &machine(r#"<state id="s"><onentry><send eventexpr="'go'"/></onentry></state>"#),
    );
    assert!(!ok, "eventexpr has no typed form here:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 8);
}

// ── A host action's arguments are typed expressions ─────────────────────

#[test]
fn a_host_action_takes_typed_datamodel_arguments_with_no_event_in_scope() {
    // Under any other data model an argument is a bare `_event.data.<field>`
    // and an eventless action takes none. Here an argument is any typed
    // expression over the scope, so a variable is admitted in `<onentry>`.
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &machine(
            r#"<state id="s">
    <onentry>
      <sce:action name="show">
        <sce:arg name="n" expr="count"/>
        <sce:arg name="done" expr="count &gt;= 3 || ready"/>
      </sce:action>
    </onentry>
  </state>"#,
        ),
    );
    assert!(ok, "typed arguments are admitted and lowered:\n{out}");
}

#[test]
fn a_host_action_argument_reading_a_payload_with_none_in_scope_is_refused() {
    // With no triggering event there is no payload to type `_event.data`.
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<state id="s">
    <onentry>
      <sce:action name="show"><sce:arg expr="_event.data.n"/></sce:action>
    </onentry>
  </state>"#,
        ),
    );
    assert!(!ok, "no payload is in scope in <onentry>:\n{out}");
    assert!(
        out.contains("validation/native-action-argument"),
        "expected the native-action argument refusal:\n{out}"
    );
}

// ── A record variable is built whole and updated a field at a time ──────

/// The event-schema a `record:Day` variable is held in.
const SCHEMA_DAY: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_day" sce:event-name="day.picked">
  <datamodel>
    <data id="year" sce:type="uint16" sce:direction="in"/>
    <data id="month" sce:type="uint8" sce:direction="in"/>
    <data id="dayOfMonth" sce:type="uint8" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// Every field of `Day`, each given once — lines 7 to 9 of [`record`].
const EVERY_FIELD: &str = r#"<sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="9"/>
      <sce:set name="dayOfMonth" expr="24"/>"#;

/// A `sce-static` machine holding one `record:Day` variable, `shown`, whose
/// `<data>` opens on line 6 and whose `sets` start on line 7, and `states`.
fn record(sets: &str, states: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <datamodel>
    <data id="shown" sce:type="record:Day">
      {sets}
    </data>
  </datamodel>
  {states}
</scxml>
"##
    )
}

fn run_record(args: &[&str], doc: &str) -> (bool, String) {
    run_beside(args, doc, &[("schema_day.scxml", SCHEMA_DAY)])
}

/// A list of records (SCE_FORGE.md §4.12) is a machine variable too: it starts
/// empty, takes a bound, and its elements are declared as a record
/// variable's are. What it is filled and walked with is
/// `a_list_holds_records.rs`'s.
#[test]
fn a_list_of_records_variable_is_accepted() {
    let (ok, out) = run_record(
        &["check", "-l", "kotlin"],
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <datamodel>
    <data id="days" sce:type="list&lt;record:Day&gt;" sce:capacity="4"/>
  </datamodel>
  <state id="s"/>
</scxml>
"##,
    );
    assert!(ok, "a machine variable is a list of records:\n{out}");
}

#[test]
fn a_record_variable_is_read_and_updated_a_field_at_a_time() {
    // A field is read in a condition and in a host action's argument, and
    // assigned from its own old value and from the payload of the schema's
    // event — every read going through the one scope.
    let (ok, out) = run_record(
        &["check", "-l", "kotlin"],
        &record(
            EVERY_FIELD,
            r#"<state id="s">
    <onentry><sce:action name="show"><sce:arg name="day" expr="shown.dayOfMonth"/></sce:action></onentry>
    <transition event="next" cond="shown.dayOfMonth &lt; 28" type="internal">
      <assign location="shown.dayOfMonth" expr="shown.dayOfMonth + 1"/>
    </transition>
    <transition event="day.picked" type="internal">
      <assign location="shown.year" expr="_event.data.year"/>
    </transition>
  </state>"#,
        ),
    );
    assert!(ok, "a record read and updated field by field:\n{out}");
}

#[test]
fn a_record_missing_a_field_is_refused_at_its_type() {
    let (ok, out) = run_record(
        &["check"],
        &record(
            r#"<sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="9"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "`dayOfMonth` is given by nothing:\n{out}");
    assert_refused_at(&out, "validation/attribute-rule-violated", 6);
    assert!(
        out.contains("dayOfMonth"),
        "the refusal names the field:\n{out}"
    );
}

#[test]
fn a_field_the_schema_does_not_declare_is_refused_at_its_name() {
    let (ok, out) = run_record(
        &["check"],
        &record(
            &format!("{EVERY_FIELD}\n      <sce:set name=\"weekday\" expr=\"1\"/>"),
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "Day has no `weekday`:\n{out}");
    assert_refused_at(&out, "validation/attribute-rule-violated", 10);
}

#[test]
fn a_field_given_twice_is_refused_at_the_second() {
    let (ok, out) = run_record(
        &["check"],
        &record(
            &format!("{EVERY_FIELD}\n      <sce:set name=\"year\" expr=\"2027\"/>"),
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "`year` is given twice:\n{out}");
    assert_refused_at(&out, "validation/attribute-rule-violated", 10);
}

#[test]
fn a_field_value_of_another_kind_is_refused_on_its_line() {
    let (ok, out) = run_record(
        &["check"],
        &record(
            r#"<sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="true"/>
      <sce:set name="dayOfMonth" expr="24"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "a bool does not stand in a uint8 field:\n{out}");
    assert_refused_at(&out, "expression/type-mismatch", 8);
}

#[test]
fn a_record_variable_with_an_expr_is_refused() {
    // There is no record literal: the `<sce:set>`s are the initial value.
    let (ok, out) = run_record(
        &["check"],
        &record(EVERY_FIELD, r#"<state id="s"/>"#).replace(
            r#"sce:type="record:Day">"#,
            r#"sce:type="record:Day" expr="0">"#,
        ),
    );
    assert!(!ok, "a record variable takes no expr:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 6);
}

#[test]
fn a_field_nothing_declares_is_refused_where_it_is_read() {
    let (ok, out) = run_record(
        &["check"],
        &record(
            EVERY_FIELD,
            r#"<state id="s"><transition event="next" cond="shown.weekday &gt; 1" type="internal"/></state>"#,
        ),
    );
    assert!(!ok, "`shown` is closed over Day's fields:\n{out}");
    assert_refused_at(&out, "expression/unknown-member", 12);
}

#[test]
fn a_whole_record_is_assigned_only_from_a_record_by_name() {
    // The event's payload is a class of its own, not a record of the machine:
    // a record is taken whole from another record, by name, and
    // `a_list_holds_records.rs` holds that.
    let (ok, out) = run_record(
        &["check"],
        &record(
            EVERY_FIELD,
            r#"<state id="s"><transition event="day.picked" type="internal"><assign location="shown" expr="_event.data"/></transition></state>"#,
        ),
    );
    assert!(!ok, "a record is taken whole from a record:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 12);
}

// ── A child session is handed what its invoke's <param>s and namelist name ──
//
// Under `sce-static` a variable is a native field, and a field is set by the
// machine's own code, so a value handed to a child session is accepted only
// where the child gives it a field to arrive in: the child is a `sce-static`
// document this build read, declares the name as a top-level `<data>`, and
// declares it as a bool, a string, an integer or a real; the value is held to
// that type as an `<assign>` to it would be. Anything else would be typed here,
// accepted, and never delivered — measured 2026-10-01 on Rust and Kotlin, when
// no generated code handed a parent's `<param>` to a child — so it is refused
// where it is written. Each of Rust and Kotlin delivers the value
// (`a_static_child_is_handed_its_params` in each backend's tests).

/// A `sce-static` parent holding `count: uint32`, whose first state invokes a
/// child session with `params` written inside the `<invoke>`. The `<state>`
/// opens on line 8 and the `<invoke>` on line 9, so a param written on its own
/// line is on line 10.
fn invoking(params: &str) -> String {
    invoking_with(
        "",
        params,
        r#"<data id="start" sce:type="uint32" expr="0"/>"#,
    )
}

/// [`invoking`], with `invoke_attrs` on the `<invoke>` and `child_data` as the
/// child's `<datamodel>`.
fn invoking_with(invoke_attrs: &str, params: &str, child_data: &str) -> String {
    machine(&format!(
        r#"<state id="s">
    <invoke type="scxml" id="child" {invoke_attrs}>
      {params}
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
               version="1.0" initial="busy" datamodel="sce-static">
          <datamodel>{child_data}</datamodel>
          <state id="busy"><transition event="finish" target="end"/></state>
          <final id="end"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.child" target="done"/>
  </state>"#
    ))
}

#[test]
fn a_child_session_with_no_param_is_accepted() {
    // The control: the document the cases below differ from only by a
    // `<param>`.
    for lang in ["rust", "kotlin"] {
        let (ok, out) = run(&["check", "-l", lang], &invoking(""));
        assert!(ok, "{lang}: an invoke with no param is fine:\n{out}");
    }
}

#[test]
fn a_value_the_child_declares_a_variable_for_is_accepted() {
    for (what, param) in [
        ("an expression", r#"<param name="start" expr="count"/>"#),
        ("a literal", r#"<param name="start" expr="7"/>"#),
        ("a location", r#"<param name="start" location="count"/>"#),
        (
            "a computed value",
            r#"<param name="start" expr="count + 1"/>"#,
        ),
    ] {
        for lang in ["rust", "kotlin"] {
            let (ok, out) = run(&["check", "-l", lang], &invoking(param));
            assert!(ok, "{lang}, {what}: the child declares `start`:\n{out}");
        }
    }
}

#[test]
fn a_namelist_name_is_handed_as_the_param_it_abbreviates() {
    // `namelist="count"` is `<param name="count" expr="count"/>`, so the child
    // declares `count` and the value is the parent's.
    let document = invoking_with(
        r#"namelist="count""#,
        "",
        r#"<data id="count" sce:type="uint32" expr="0"/>"#,
    );
    for lang in ["rust", "kotlin"] {
        let (ok, out) = run(&["check", "-l", lang], &document);
        assert!(ok, "{lang}: the child declares `count`:\n{out}");
    }
    let (ok, out) = run(
        &["check"],
        &invoking_with(
            r#"namelist="count""#,
            "",
            r#"<data id="other" sce:type="uint32" expr="0"/>"#,
        ),
    );
    assert!(!ok, "the child declares no `count`:\n{out}");
    assert!(out.contains("count"), "it names what was written:\n{out}");
}

#[test]
fn c11_starts_a_static_child_and_hands_it_a_number() {
    // C holds a child in the parent's own struct and starts it in two steps,
    // handing it what the invoke's `<param>`s name between them
    // (docs/SCE_ACCEPTED_SUBSET.md §2.15, "Child sessions").
    let (ok, out) = run(
        &["check", "-l", "c11"],
        &invoking(r#"<param name="start" expr="count + 1"/>"#),
    );
    assert!(ok, "c11: the child declares `start`, a uint32:\n{out}");
}

#[test]
fn c11_refuses_a_string_handed_to_a_child_and_names_it() {
    // A string in a child is a buffer of the bound its variable declares, which
    // a value arriving from another machine would have to be held to: refused by
    // name until it is, not written past the end of the buffer.
    let document = invoking_with(
        "",
        r#"<param name="title" expr="'x'"/>"#,
        r#"<data id="title" sce:type="string" sce:capacity="8" expr="''"/>"#,
    );
    let (ok, out) = run(&["check", "-l", "c11"], &document);
    assert!(!ok, "c11 has no lowering for it yet:\n{out}");
    assert!(
        out.contains("generate/unsupported-feature") && out.contains("no C11 lowering yet"),
        "expected the unsupported-feature refusal naming C11:\n{out}"
    );
    assert!(
        out.contains(r#"a <param name=\"title\"> handed to a string variable of the child"#),
        "it names the param and the invoke:\n{out}"
    );
}

#[test]
fn c11_refuses_a_child_that_needs_a_host_to_perform_its_actions() {
    // The child is a value its parent holds and starts: it has no host to give
    // its act table to, and only a host that supplies the acts can start it.
    let document = machine(
        r#"<state id="s">
    <invoke type="scxml" id="child">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
               version="1.0" initial="busy" datamodel="sce-static">
          <datamodel><data id="start" sce:type="uint32" expr="0"/></datamodel>
          <state id="busy">
            <onentry>
              <sce:action name="announce"><sce:arg name="seen" expr="start"/></sce:action>
            </onentry>
            <transition event="finish" target="end"/>
          </state>
          <final id="end"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.child" target="done"/>
  </state>"#,
    );
    let (ok, out) = run(&["check", "-l", "c11"], &document);
    assert!(!ok, "c11 has no lowering for it yet:\n{out}");
    assert!(
        out.contains("generate/unsupported-feature") && out.contains("no C11 lowering yet"),
        "expected the unsupported-feature refusal naming C11:\n{out}"
    );
    assert!(
        out.contains(r#"an <invoke id=\"child\"> of a child that declares <sce:action>s"#),
        "it names the invoke:\n{out}"
    );
}

/// A child written beside the document, declaring `start: uint32`.
const CHILD_BESIDE: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="busy" datamodel="sce-static">
  <datamodel><data id="start" sce:type="uint32" expr="0"/></datamodel>
  <state id="busy"><transition event="finish" target="end"/></state>
  <final id="end"/>
</scxml>
"##;

/// `invoking`, with the child in a document beside the parent (`src`) rather
/// than written inside the `<invoke>`. The `<param>` is on line 10 here too.
fn invoking_beside(params: &str) -> String {
    machine(&format!(
        r#"<state id="s">
    <invoke type="scxml" id="child" src="child.scxml">
      {params}
    </invoke>
    <transition event="done.invoke.child" target="done"/>
  </state>"#
    ))
}

#[test]
fn a_child_written_beside_the_document_is_held_to_its_own_variables() {
    let siblings = [("child.scxml", CHILD_BESIDE)];
    for lang in ["rust", "kotlin"] {
        let (ok, out) = run_beside(
            &["check", "-l", lang],
            &invoking_beside(r#"<param name="start" expr="count"/>"#),
            &siblings,
        );
        assert!(ok, "{lang}: the child beside declares `start`:\n{out}");
    }
    let (ok, out) = run_beside(
        &["check"],
        &invoking_beside(r#"<param name="other" expr="count"/>"#),
        &siblings,
    );
    assert!(!ok, "the child beside declares no `other`:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 10);
}

#[test]
fn a_value_the_child_has_no_variable_for_is_refused_on_its_param() {
    for (what, param, says) in [
        (
            "a name the child does not declare",
            r#"<param name="other" expr="count"/>"#,
            "declares no top-level",
        ),
        (
            "two values for one name",
            r#"<param name="start" expr="count"/><param name="start" expr="7"/>"#,
            "handed this name twice",
        ),
        (
            "a param whose value is blank",
            r#"<param name="start" expr=" "/>"#,
            "names none",
        ),
    ] {
        let (ok, out) = run(&["check"], &invoking(param));
        assert!(!ok, "{what}: the value would never arrive:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 10);
        assert!(out.contains(says), "{what}: it says why:\n{out}");
    }
}

#[test]
fn a_value_of_another_type_than_the_childs_variable_is_refused() {
    // The value is held to the variable's type as an `<assign>` to it would be:
    // a bool does not land in a `uint32`.
    let (ok, out) = run(
        &["check"],
        &invoking(r#"<param name="start" expr="ready"/>"#),
    );
    assert!(!ok, "a bool is not a uint32:\n{out}");
    assert!(out.contains("ready"), "it names what was written:\n{out}");
}

#[test]
fn a_child_under_another_data_model_is_given_no_typed_value() {
    // Nothing here knows the type of a variable an engine holds, so the value
    // has no typed place to arrive in.
    let document = invoking_with(
        "",
        r#"<param name="start" expr="count"/>"#,
        r#"<data id="start" expr="0"/>"#,
    )
    .replacen(
        r#"initial="busy" datamodel="sce-static""#,
        r#"initial="busy" datamodel="ecmascript""#,
        1,
    );
    let (ok, out) = run(&["check"], &document);
    assert!(!ok, "an ecmascript child has no typed variable:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 10);
}

#[test]
fn a_param_is_still_given_to_a_child_under_ecmascript() {
    // The refusals above are of this data model's lacking a typed field to
    // arrive in, not of `<param>`: under `ecmascript` the value is seeded into
    // the child's datamodel.
    let document = invoking(r#"<param name="start" expr="count"/>"#)
        .replace(r#"datamodel="sce-static""#, r#"datamodel="ecmascript""#)
        .replace(r#" sce:type="uint32""#, "")
        .replace(r#" sce:type="bool""#, "");
    let (ok, out) = run(&["check"], &document);
    assert!(ok, "ecmascript hands a param to its child:\n{out}");
}

// ── A <finalize> is run by no machine of this data model ────────────────
//
// The model keeps an `<invoke>`'s `<finalize>` as script text (the parser
// transpiles its actions to one string), so no type rule reaches it, and the
// generated code hands that string to a script engine this data model never
// builds: the Rust body is an empty block and Kotlin reads the engine
// `executeFinalizeForChildEvent` finds absent. Measured 2026-10-01 on the
// generated code of a `sce-static` parent: the document was accepted, `check`
// said nothing, and the assignment never ran.
//
// Running one natively is not a matter of lowering the body: a `<finalize>` runs
// before the next event from ANY of the child's events is processed, to read that
// event's `_event.data`, and no type rule reaches a payload that arrives from
// whichever event comes next. A body that reads no payload has no consumer, so it
// stays refused until a typed form of the payload exists.

/// The control and the refusal's document: `invoking` with a `<finalize>` that
/// assigns `count`, written on the line after the `<invoke>` opens (line 10).
fn finalizing() -> String {
    invoking(r#"<finalize><assign location="count" expr="count + 1"/></finalize>"#)
}

#[test]
fn a_finalize_no_machine_here_would_run_is_refused_on_its_invoke() {
    let (ok, out) = run(&["check"], &finalizing());
    assert!(!ok, "no generated code runs it:\n{out}");
    // The model records the element the `<finalize>` is written in, not the
    // `<finalize>` itself, so the refusal sits on the `<invoke>` (line 9).
    assert_refused_at(&out, "scxml/static-datamodel-rule", 9);
}

#[test]
fn the_finalize_refusal_names_the_invoke_and_says_what_would_happen() {
    let (_, out) = run(&["check"], &finalizing());
    let record = out
        .lines()
        .find(|l| l.contains("\"code\":\"scxml/static-datamodel-rule\""))
        .unwrap_or_else(|| panic!("expected a static-datamodel-rule refusal:\n{out}"));
    assert!(
        record.contains("finalize"),
        "it names the element:\n{record}"
    );
    assert!(record.contains("child"), "it names the invoke:\n{record}");
    assert!(
        record.contains("never run"),
        "it says the body would be accepted and not executed:\n{record}"
    );
}

#[test]
fn an_empty_finalize_beside_a_param_is_refused_as_the_update_it_stands_for() {
    // §scxml-6.5.2 gives an empty `<finalize/>` the meaning "update each
    // `<param location>` from the event's data of that name", and the model
    // writes that update out as script text. Now that a child's `<param>` is
    // accepted, the pair reaches the same refusal a written body does, rather
    // than being accepted and never run.
    let (ok, out) = run(
        &["check"],
        &invoking(r#"<param name="start" location="count"/><finalize/>"#),
    );
    assert!(!ok, "the update it stands for is never run:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 9);
    assert!(out.contains("finalize"), "it names the element:\n{out}");

    // An empty `<finalize/>` with nothing to update is inert, as the clause
    // leaves it.
    let (ok, out) = run(&["check"], &invoking("<finalize/>"));
    assert!(ok, "there is nothing for it to update:\n{out}");
}

#[test]
fn a_finalize_is_still_run_under_ecmascript() {
    // The refusal is of this data model's having no engine to run the text, not
    // of `<finalize>`: `ecmascript` builds one.
    let document = finalizing()
        .replace(r#"datamodel="sce-static""#, r#"datamodel="ecmascript""#)
        .replace(r#" sce:type="uint32""#, "")
        .replace(r#" sce:type="bool""#, "");
    let (ok, out) = run(&["check"], &document);
    assert!(ok, "ecmascript runs a finalize:\n{out}");
}

#[test]
fn a_host_run_invokes_param_is_still_accepted() {
    // The refusal is of a CHILD SESSION's `<param>`. A host-run invoke's is part
    // of the request the host receives, so it has somewhere to arrive, and is
    // judged as a typed expression like any other.
    let host = machine(
        r#"<state id="s">
    <invoke type="x-sce-host" id="h"><param name="k" expr="count"/></invoke>
    <transition event="done.invoke.h" target="done"/>
  </state>"#,
    );
    let (ok, out) = run(&["check"], &host);
    assert!(ok, "a param addressed to the host is not refused:\n{out}");

    // ... and it is judged: a name nothing declares is still refused there.
    let (ok, out) = run(
        &["check"],
        &host.replace(r#"expr="count""#, r#"expr="cuont""#),
    );
    assert!(!ok, "`cuont` is declared nowhere:\n{out}");
    assert!(out.contains("cuont"), "it names what was written:\n{out}");
}

// ── What a <param> carries across to the host ───────────────────────────
//
// A `<param>` of a `<send>` or of an `<invoke>` the host runs is a typed
// expression, read from the machine's fields when the element runs, and it
// crosses as text and as a JSON value. The values every backend spells alike
// are carried; any other is refused where it is written, not dropped or carried
// differently by two backends.

/// A `sce-static` machine whose `<state>` (line 8) sends to the host from its
/// `<onentry>`, with `params` on line 11.
fn sending(params: &str) -> String {
    machine(&format!(
        r#"<state id="s">
    <onentry>
      <send type="x-sce-host" event="notify">
        {params}
      </send>
    </onentry>
    <transition event="go" target="done"/>
  </state>"#
    ))
}

#[test]
fn every_value_a_param_can_carry_is_accepted() {
    for expr in ["count", "ready", "count + 1", "7", "1.5", "'text'"] {
        let params = format!(r#"<param name="k" expr="{expr}"/>"#);
        let (ok, out) = run(&["check"], &sending(&params));
        assert!(
            ok,
            "`{expr}` is a bool, a string, a narrow integer or a real:\n{out}"
        );
    }
    // A location names a variable and is read as one.
    let (ok, out) = run(
        &["check"],
        &sending(r#"<param name="k" location="count"/>"#),
    );
    assert!(ok, "a location names a variable:\n{out}");
}

#[test]
fn a_param_whose_value_has_no_wire_spelling_is_refused_on_its_line() {
    // `big` is a 64-bit integer: a backend that reads numbers through a double
    // would carry one past 2^53 with its low bits wrong, and say nothing.
    let document = sending(r#"<param name="k" expr="big"/>"#).replace(
        r#"<data id="ready" sce:type="bool" expr="false"/>"#,
        r#"<data id="big" sce:type="int64" expr="0"/>"#,
    );
    let (ok, out) = run(&["check"], &document);
    assert!(
        !ok,
        "a 64-bit integer has no wire spelling every backend shares:\n{out}"
    );
    assert_refused_at(&out, "scxml/static-datamodel-rule", 11);
    assert!(out.contains("64-bit"), "it says why:\n{out}");
}

#[test]
fn a_param_reading_the_events_payload_is_refused_on_its_line() {
    // Reading it needs the payload channel's guard around the whole element,
    // which the lowering of a `<param>` does not yet put there.
    let (ok, out) = run_record(
        &["check"],
        &record(
            EVERY_FIELD,
            r#"<state id="s"><transition event="day.picked" type="internal"><send type="x-sce-host" event="forward"><param name="d" expr="_event.data.dayOfMonth"/></send></transition></state>"#,
        ),
    );
    assert!(!ok, "a payload read in a param has no lowering yet:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 12);
    assert!(out.contains("payload"), "it says why:\n{out}");
}

// ── A `<donedata>` param is the same value on the same wire ─────────────

/// A `sce-static` machine whose `<final>` (line 9) carries a `<donedata>`
/// (line 10) with `params` on line 11.
fn finishing(params: &str) -> String {
    machine(r#"<state id="s"><transition event="go" target="done"/></state>"#).replace(
        r#"<final id="done"/>"#,
        &format!(
            "<final id=\"done\">\n    <donedata>\n      {params}\n    </donedata>\n  </final>"
        ),
    )
}

#[test]
fn every_value_a_donedata_param_can_carry_is_accepted() {
    for expr in ["count", "ready", "count + 1", "7", "1.5", "'text'"] {
        let params = format!(r#"<param name="k" expr="{expr}"/>"#);
        let (ok, out) = run(&["check"], &finishing(&params));
        assert!(
            ok,
            "`{expr}` is a bool, a string, a narrow integer or a real:\n{out}"
        );
    }
    // A location names a variable and is read as one.
    let (ok, out) = run(
        &["check"],
        &finishing(r#"<param name="k" location="count"/>"#),
    );
    assert!(ok, "a location names a variable:\n{out}");
}

#[test]
fn a_donedata_param_whose_value_has_no_wire_spelling_is_refused_on_its_line() {
    // The rule is the one a `<send>`'s param is held to, stated once.
    let document = finishing(r#"<param name="k" expr="big"/>"#).replace(
        r#"<data id="ready" sce:type="bool" expr="false"/>"#,
        r#"<data id="big" sce:type="int64" expr="0"/>"#,
    );
    let (ok, out) = run(&["check"], &document);
    assert!(
        !ok,
        "a 64-bit integer has no wire spelling every backend shares:\n{out}"
    );
    assert_refused_at(&out, "scxml/static-datamodel-rule", 11);
    assert!(out.contains("64-bit"), "it says why:\n{out}");
}

#[test]
fn a_donedata_param_naming_a_variable_that_is_not_declared_is_refused_on_its_line() {
    // Only an `expr` was judged once; a `location` reached an engine unread.
    let (ok, out) = run(
        &["check"],
        &finishing(r#"<param name="k" location="missing"/>"#),
    );
    assert!(
        !ok,
        "a location names a variable the machine declares:\n{out}"
    );
    assert_refused_at(&out, "expression/unknown-identifier", 11);
}

#[test]
fn a_donedata_param_is_lowered_to_native_code_and_not_handed_to_an_engine() {
    // Measured 2026-10-03: judged, accepted and never lowered. The Rust machine
    // called `ensure_script_engine`, which a sce-static machine does not have,
    // and the Kotlin machine raised its done event with no data at all. The
    // manifest says `needs_script_engine:false` either way, so what is read is
    // the generated machine.
    let document = finishing(r#"<param name="k" expr="count + 1"/>"#);
    for (language, extension, native, engine) in [
        ("rust", "rs", "json_parts", "ensure_script_engine"),
        ("kotlin", "kt", "doneParams", "evaluateExpr"),
        (
            "go",
            "go",
            "ScriptValueToJSON(sceValue)",
            "EvaluateExpression",
        ),
        // The generated module keeps its engine helpers whatever the document
        // is, so the spelling that reads a donedata param is the one looked for:
        // a pair whose value is the engine's answer, joined to its name.
        (
            "python",
            "py",
            "_ScriptValue.of(",
            "+ engine._script_engine.evaluate_expression(",
        ),
    ] {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run(
            &[
                "generate",
                "-l",
                language,
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            &document,
        );
        assert!(ok, "{language}: the machine generates:\n{out}");
        let generated: Vec<_> = std::fs::read_dir(out_dir.path())
            .expect("the output directory")
            .map(|entry| entry.expect("an entry").path())
            .filter(|path| path.extension().is_some_and(|e| e == extension))
            .collect();
        assert_eq!(generated.len(), 1, "{language}: one machine: {generated:?}");
        let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
        assert!(
            source.contains(native),
            "{language}: the donedata is built from the lowered value"
        );
        assert!(
            !source.contains(engine),
            "{language}: no script engine reads a donedata param"
        );
    }
}

#[test]
fn a_go_send_param_is_lowered_to_native_code_and_not_handed_to_an_engine() {
    // Go refused a <send> carrying a <param> by name until its value was lowered
    // as the other targets lower it. The manifest says `needs_script_engine:false`
    // either way, so what is read is the generated machine: the value is computed
    // into a local from the machine's own fields and no engine evaluates it.
    let document = machine(
        r#"<state id="s">
    <transition event="go" type="internal">
      <send event="note"><param name="k" expr="count + 1"/></send>
    </transition>
  </state>"#,
    );
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &document,
    );
    assert!(ok, "the machine generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":false"),
        "a sce-static machine's params are read from its fields:\n{out}"
    );
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("sce.EventDataParam{Name: \"k\", Value: sceValue}"),
        "the pair is built from the lowered value"
    );
    assert!(
        !source.contains("EvaluateExpression"),
        "no script engine reads a send param"
    );
}

#[test]
fn a_python_send_param_is_lowered_to_native_code_and_not_handed_to_an_engine() {
    // Python refused a <send> carrying a <param> by name until its value was
    // lowered as the other targets lower it. The generated module keeps the
    // helper that evaluates a payload through the engine whatever the document
    // is, and a host-run `<invoke>`'s helper calls it too, so what is looked for
    // is the call a `<send>` makes: absent, the pair is built from the lowered
    // value.
    let document = machine(
        r#"<state id="s">
    <transition event="go" type="internal">
      <send event="note"><param name="k" expr="count + 1"/></send>
    </transition>
  </state>"#,
    );
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "python",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &document,
    );
    assert!(ok, "the machine generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":false"),
        "a sce-static machine's params are read from its fields:\n{out}"
    );
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "py"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("self._put_param(_data, _data_repeats, \"k\", "),
        "the pair is built from the lowered value"
    );
    assert!(
        !source.contains("_data = self._eval_send_payload("),
        "no script engine reads a send param"
    );
}

#[test]
fn a_cpp_send_param_is_lowered_to_native_code_and_not_handed_to_an_engine() {
    // C++ refused a <send> carrying a <param> or a <content> by name. The value is
    // now computed into a `ScriptValue` from the machine's own fields and put in
    // the typed map the event's JSON is built from; the manifest says
    // `needs_script_engine:false` either way, so what is read is the machine.
    let document = machine(
        r#"<state id="s">
    <transition event="go" type="internal">
      <send event="note"><param name="k" expr="count + 1"/></send>
    </transition>
  </state>"#,
    );
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "cpp",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &document,
    );
    assert!(ok, "the machine generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":false"),
        "a sce-static machine's params are read from its fields:\n{out}"
    );
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        // The C++ generator writes a machine's code to the `.inl` its header
        // includes, so that is the file a send's code is in.
        .filter(|path| path.extension().is_some_and(|e| e == "inl"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("typedParams[\"k\"].push_back(std::move(paramValue))"),
        "the pair is built from the lowered value"
    );
    assert!(
        !source.contains("scriptEngine.evaluateExpression("),
        "no script engine reads a send param"
    );
}

#[test]
fn a_basic_http_send_carrying_a_param_is_refused_by_name_in_cpp() {
    // A BasicHTTP send carries each value as the text a form does, which the C++
    // machine does not spell from a typed value; it is named, not left to
    // generate a request with no body. The refusal is decided by the construct,
    // not by where the request would go, so the target names no endpoint of the
    // suite (the fixture port is spelled once, in `basic_http_test_endpoint.h`).
    let document = machine(
        r#"<state id="s">
    <transition event="go" type="internal">
      <send type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor" target="http://example.invalid/hook" event="note"><param name="k" expr="count"/></send>
    </transition>
  </state>"#,
    );
    let (ok, out) = run(&["check", "-l", "cpp"], &document);
    assert!(!ok, "C++ has no lowering for it yet:\n{out}");
    assert!(
        out.contains("generate/unsupported-feature")
            && out.contains("a BasicHTTP <send> carrying a <param>"),
        "expected the unsupported-feature refusal naming the construct:\n{out}"
    );
}

#[test]
fn a_python_host_action_argument_that_can_fail_is_received_where_the_call_stands() {
    // Python refused a `<sce:action>` by name. A failing argument is an
    // exception, so the call stands in a `try`: the exception leaves it before the
    // host is called and `error.execution` is raised in its place, without ending
    // the block (as on the other backends). A call that reads the event's payload
    // is more than one line, and sits in a block that checks the delivery
    // carried one — the shape no scenario runs, so what is read here is the
    // machine, and Python itself reads it as source.
    let (ok, out_dir) = {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run_record(
            &[
                "generate",
                "-l",
                "python",
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            &record(
                EVERY_FIELD,
                r#"<state id="s">
    <transition event="day.picked" type="internal">
      <sce:action name="show"><sce:arg name="d" expr="_event.data.dayOfMonth + 1"/></sce:action>
    </transition>
  </state>"#,
            ),
        );
        assert!(ok, "the machine generates:\n{out}");
        (ok, out_dir)
    };
    assert!(ok);
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "py"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    let lines: Vec<&str> = source.lines().map(str::trim).collect();
    let call = lines
        .iter()
        .position(|line| line.starts_with("self._actions.show("))
        .expect("the host action is called");
    assert_eq!(lines[call - 1], "try:", "the call stands in a try");
    assert!(
        lines[call + 1].starts_with("except sce_algorithm.AlgorithmFailure:"),
        "a failed argument is received where the call stands: {}",
        lines[call + 1]
    );
    assert!(
        lines[..call]
            .iter()
            .rev()
            .take(3)
            .any(|line| line.starts_with("if self._pending_day_picked_payload is not None:")),
        "the call reads the payload only for a delivery that carried one"
    );
    let parsed = Command::new("python3")
        .arg("-c")
        .arg("import ast, sys; ast.parse(open(sys.argv[1]).read())")
        .arg(&generated[0])
        .output()
        .expect("run python3");
    assert!(
        parsed.status.success(),
        "the generated machine is not Python:\n{}",
        String::from_utf8_lossy(&parsed.stderr)
    );
}

#[test]
fn a_send_content_is_the_text_it_spells_and_no_engine_reads_it() {
    // Measured 2026-10-03: Go, Python and C++ refused a literal `<content>` by
    // name, and Kotlin accepted it and generated
    // `evaluateSendContent(ScriptSource.lua(…))` — a call to the script engine in
    // a machine whose manifest says it needs none, so the data depended on an
    // engine nothing had given the machine. Rust already wrote the text out.
    //
    // Under a data model with no engine the text is the value (the rung Go and
    // Rust take when `needs_script_engine` is false): the string it spells,
    // whitespace-normalised and JSON-quoted. What is read is the generated
    // machine, since the manifest says `needs_script_engine:false` either way.
    let document = machine(
        r#"<state id="s">
    <transition event="go" type="internal">
      <send event="note"><content>two   words</content></send>
    </transition>
    <transition event="note" type="internal"/>
  </state>"#,
    );
    // The wire text, as each language spells the string literal carrying it.
    let wire = r#""\"two words\"""#;
    for (language, extension, native, engine) in [
        ("kotlin", "kt", wire, "evaluateSendContent("),
        ("rust", "rs", wire, "evaluate_expression"),
        ("go", "go", wire, "EvaluateExpression"),
        (
            "python",
            "py",
            "_data = \"two words\"",
            "_data = self._eval_send_payload(",
        ),
        // C++ hands the normalised text to the helper `<donedata>` takes with no
        // data model, which quotes it as the others do here.
        (
            "cpp",
            "inl",
            "emitContentLiteral(\"two words\"",
            "DoneDataHelper::evaluateContent(",
        ),
    ] {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run(
            &[
                "generate",
                "-l",
                language,
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            &document,
        );
        assert!(ok, "{language}: the machine generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: a sce-static machine's content needs no engine:\n{out}"
        );
        let generated: Vec<_> = std::fs::read_dir(out_dir.path())
            .expect("the output directory")
            .map(|entry| entry.expect("an entry").path())
            .filter(|path| path.extension().is_some_and(|e| e == extension))
            .collect();
        assert_eq!(generated.len(), 1, "{language}: one machine: {generated:?}");
        let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
        assert!(
            source.contains(native),
            "{language}: the event's data is the text the content spells"
        );
        assert!(
            !source.contains(engine),
            "{language}: no script engine reads a send content"
        );
    }
}

#[test]
fn what_a_host_run_invoke_evaluates_besides_its_params_has_no_typed_form() {
    // Its `srcexpr`, `namelist` and `<content expr>` are script text a backend
    // would hand to an engine this data model does not have; they are refused,
    // as the same attributes of every other element are.
    for invoke in [
        r#"<invoke type="x-sce-host" id="h" srcexpr="'a'"/>"#,
        r#"<invoke type="x-sce-host" id="h" namelist="count"/>"#,
        r#"<invoke type="x-sce-host" id="h"><content expr="count"/></invoke>"#,
    ] {
        let document = machine(&format!(
            "<state id=\"s\">\n    {invoke}\n    <transition event=\"go\" target=\"done\"/>\n  </state>"
        ));
        let (ok, out) = run(&["check"], &document);
        assert!(!ok, "{invoke} has no typed form:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 9);
    }
}

#[test]
fn a_static_machine_that_sends_and_invokes_the_host_needs_no_script_engine() {
    // The manifest says which backends need an engine to run the machine. The
    // `<param>`s are lowered to native code, so this one needs none; the same
    // document under `ecmascript` evaluates them in an engine and still does.
    let document = machine(
        r#"<state id="s">
    <onentry><send type="x-sce-host" event="notify"><param name="k" expr="count"/></send></onentry>
    <invoke type="x-sce-host" id="h"><param name="k" expr="count"/></invoke>
    <transition event="done.invoke.h" target="done"/>
  </state>"#,
    );
    let generate = |language: &str, document: &str| {
        let out_dir = tempdir().expect("tempdir");
        run(
            &[
                "generate",
                "-l",
                language,
                "-o",
                out_dir.path().to_str().expect("a path"),
                "--host-processor",
                "x-sce-host",
                "--host-invoker",
                "x-sce-host",
            ],
            document,
        )
    };
    for language in ["rust", "kotlin"] {
        let (ok, out) = generate(language, &document);
        assert!(ok, "{language}: the machine generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: a sce-static machine's params are read from its fields:\n{out}"
        );
    }
    let ecmascript = document
        .replace(r#"datamodel="sce-static""#, r#"datamodel="ecmascript""#)
        .replace(r#" sce:type="uint32""#, "")
        .replace(r#" sce:type="bool""#, "");
    let (ok, out) = generate("kotlin", &ecmascript);
    assert!(ok, "the same document under ecmascript generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":true"),
        "under ecmascript a param is evaluated by an engine:\n{out}"
    );
}

// ── A variable is published by sce:direction="out" ──────────────────────

#[test]
fn a_variable_the_host_would_write_is_refused() {
    // `out` publishes a variable and `internal` keeps it the machine's own;
    // `in` would hand the host a write nothing lowers.
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="count" sce:type="uint32" expr="0" sce:direction="in"/>"#,
        ),
    );
    assert!(
        !ok,
        "a sce-static variable is written by the machine:\n{out}"
    );
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn a_published_and_an_internal_variable_are_accepted() {
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &doc(
            "sce-static",
            r#"<data id="count" sce:type="uint32" expr="0" sce:direction="out"/>
    <data id="step" sce:type="uint32" expr="1" sce:direction="internal"/>"#,
        ),
    );
    assert!(ok, "out and internal both mean something here:\n{out}");
}

// ── A list variable starts empty, is appended to and cleared ────────────

/// A list variable, `days: list<uint8>` of capacity 4, on line 5.
const DAYS: &str = r#"<data id="days" sce:type="list&lt;uint8&gt;" sce:capacity="4"/>"#;

/// A machine under `datamodel` whose first variable is `data` (line 5),
/// followed by `ready: bool` (line 6), and whose `states` open on line 8.
fn list_doc(datamodel: &str, data: &str, states: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="{datamodel}">
  <datamodel>
    {data}
    <data id="ready" sce:type="bool" expr="false"/>
  </datamodel>
  {states}
</scxml>
"##
    )
}

#[test]
fn a_list_is_appended_to_and_cleared() {
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &list_doc(
            "sce-static",
            DAYS,
            r#"<state id="s"><onentry><sce:append target="days" expr="7"/><sce:clear target="days"/></onentry></state>"#,
        ),
    );
    assert!(
        ok,
        "a list filled and emptied by its two statements:\n{out}"
    );
}

#[test]
fn a_list_without_a_capacity_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            r#"<data id="days" sce:type="list&lt;uint8&gt;"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "a list declares its bound:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn a_list_capacity_that_is_not_a_positive_count_is_refused_by_its_rule() {
    // `sce:capacity` is untyped in the schema — the event queue's bound on
    // `<scxml>` shares the name — so this rule is the list's only guard.
    for written in ["0", "many"] {
        let (ok, out) = run(
            &["check"],
            &list_doc(
                "sce-static",
                &format!(
                    r#"<data id="days" sce:type="list&lt;uint8&gt;" sce:capacity="{written}"/>"#
                ),
                r#"<state id="s"/>"#,
            ),
        );
        assert!(!ok, "sce:capacity=\"{written}\" is no count:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
    }
}

#[test]
fn a_list_with_an_initial_value_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            r#"<data id="days" sce:type="list&lt;uint8&gt;" sce:capacity="4" expr="0"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "a list starts empty:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn a_capacity_on_a_variable_that_is_not_a_list_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            r#"<data id="days" sce:type="uint8" sce:capacity="4" expr="0"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "only a list or a string has a capacity:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

// ── A string is bounded as a list is ─────────────────────────────────────

#[test]
fn a_string_without_a_capacity_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            r#"<data id="label" sce:type="string" expr="'ab'"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "a string declares its bound:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
    assert!(
        out.contains("UTF-8 bytes"),
        "the refusal says what the bound counts:\n{out}"
    );
}

#[test]
fn a_string_capacity_that_is_not_a_positive_count_is_refused() {
    for written in ["0", "many", "-1"] {
        let (ok, out) = run(
            &["check"],
            &list_doc(
                "sce-static",
                &format!(
                    r#"<data id="label" sce:type="string" sce:capacity="{written}" expr="''"/>"#
                ),
                r#"<state id="s"/>"#,
            ),
        );
        assert!(!ok, "sce:capacity=\"{written}\" is no count:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
    }
}

#[test]
fn a_string_that_fits_its_capacity_in_bytes_is_accepted() {
    // `é` is two bytes and `€` three: the bound counts bytes, so two characters
    // of two bytes fit four, and a literal as long as the bound fits it.
    for (written, capacity) in [("abcd", 4), ("\u{e9}\u{e9}", 4), ("", 1)] {
        let (ok, out) = run(
            &["check"],
            &list_doc(
                "sce-static",
                &format!(
                    r#"<data id="label" sce:type="string" sce:capacity="{capacity}" expr="'{written}'"/>"#
                ),
                r#"<state id="s"/>"#,
            ),
        );
        assert!(ok, "'{written}' fits {capacity} bytes:\n{out}");
    }
}

#[test]
fn a_string_that_starts_past_its_capacity_is_refused_in_bytes() {
    // Two characters, five bytes: a count of characters would accept it.
    for (written, capacity) in [("abcde", 4), ("\u{e9}\u{20ac}", 4)] {
        let (ok, out) = run(
            &["check"],
            &list_doc(
                "sce-static",
                &format!(
                    r#"<data id="label" sce:type="string" sce:capacity="{capacity}" expr="'{written}'"/>"#
                ),
                r#"<state id="s"/>"#,
            ),
        );
        assert!(!ok, "'{written}' is past {capacity} bytes:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
        assert!(
            out.contains("past the sce:capacity"),
            "the refusal names the bound:\n{out}"
        );
    }
}

#[test]
fn a_string_starts_at_a_literal_and_not_at_another_variable() {
    // The machine is built with no error to raise, so a value that could fail
    // to fit is refused where it is written rather than copied at run time.
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            r#"<data id="first" sce:type="string" sce:capacity="8" expr="'ab'"/>
    <data id="second" sce:type="string" sce:capacity="4" expr="first"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "a string starts at a literal:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 6);
}

#[test]
fn a_list_of_an_element_with_a_length_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            r#"<data id="days" sce:type="list&lt;string&gt;" sce:capacity="4"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "a list element has a fixed width:\n{out}");
    assert_refused_at(&out, "validation/attribute-rule-violated", 5);
}

#[test]
fn an_append_to_a_variable_that_is_not_a_list_is_refused_at_its_target() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            DAYS,
            r#"<state id="s"><onentry><sce:append target="ready" expr="1"/></onentry></state>"#,
        ),
    );
    assert!(!ok, "`ready` is a bool:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 8);
    assert!(
        out.contains("one of days"),
        "the refusal names the lists:\n{out}"
    );
}

#[test]
fn an_element_of_another_kind_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            DAYS,
            r#"<state id="s"><onentry><sce:append target="days" expr="true"/></onentry></state>"#,
        ),
    );
    assert!(!ok, "a bool is not a uint8 element:\n{out}");
    assert_refused_at(&out, "expression/type-mismatch", 8);
}

#[test]
fn a_list_read_as_a_value_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            DAYS,
            r#"<state id="s"><transition event="go" cond="days" target="s"/></state>"#,
        ),
    );
    assert!(
        !ok,
        "a list is read by the host, not by an expression:\n{out}"
    );
    assert_refused_at(&out, "expression/unsupported-construct", 8);
}

#[test]
fn a_list_is_measured_by_len() {
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &list_doc(
            "sce-static",
            &format!("{DAYS}\n    <data id=\"n\" sce:type=\"uint32\" expr=\"0\"/>"),
            r#"<state id="s">
    <onentry><assign location="n" expr="len(days)"/></onentry>
    <transition event="go" cond="len(days) &gt; 2" target="s"/>
  </state>"#,
        ),
    );
    assert!(
        ok,
        "len(days) reads a list's length, in a value and in a guard:\n{out}"
    );
}

#[test]
fn a_list_passed_to_a_host_action_is_refused() {
    // A host method takes values, and a list is not one — refused with the
    // same wording as in any other expression, not as an undeclared name.
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            DAYS,
            r#"<state id="s"><onentry><sce:action name="show"><sce:arg name="d" expr="days"/></sce:action></onentry></state>"#,
        ),
    );
    assert!(!ok, "a list is no host-method argument:\n{out}");
    assert!(
        out.contains("expression/unsupported-construct") && out.contains("reading the list `days`"),
        "{out}"
    );
}

#[test]
fn an_assignment_to_a_whole_list_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            DAYS,
            r#"<state id="s"><onentry><assign location="days" expr="0"/></onentry></state>"#,
        ),
    );
    assert!(
        !ok,
        "a list is filled by append and emptied by clear:\n{out}"
    );
    assert_refused_at(&out, "expression/unsupported-construct", 8);
}

#[test]
fn a_list_statement_under_another_data_model_is_refused() {
    // Before, the parser skipped an `sce:` element it did not know, so this
    // statement was dropped without a word.
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "ecmascript",
            r#"<data id="n" expr="0"/>"#,
            r#"<state id="s"><onentry><sce:append target="n" expr="1"/></onentry></state>"#,
        ),
    );
    assert!(!ok, "a list exists only under sce-static:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 8);
}

// ── An imported algorithm is called as `Alias(args)` ────────────────────

/// A scalar algorithm: `clamp(n, top)`.
const ALGORITHM_CLAMP: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="clamp" version="1.0">
  <sce:signature>
    <sce:param name="n" type="uint32"/>
    <sce:param name="top" type="uint32"/>
    <sce:return type="uint32"/>
  </sce:signature>
  <sce:body>
    <sce:var name="r" type="uint32" init="n"/>
    <sce:if cond="r &gt; top"><sce:assign target="r" expr="top"/></sce:if>
    <sce:return expr="r"/>
  </sce:body>
</scxml>
"#;

/// An algorithm returning a list — callable only by a host.
const ALGORITHM_LIST: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="upto" version="1.0">
  <sce:signature>
    <sce:param name="n" type="uint32"/>
    <sce:return type="list&lt;uint32&gt;" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="list&lt;uint32&gt;" capacity="4"/>
    <sce:append target="out" expr="n"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A machine under `datamodel` importing `algorithm_clamp.scxml` as `Clamp`
/// on line 4 (and `algorithm_upto.scxml` as `Upto` on line 5), with
/// `count: uint32` and `states` from line 10.
fn calling_doc(datamodel: &str, states: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="{datamodel}">
  <sce:import kind="algorithm" src="algorithm_clamp.scxml" as="Clamp"/>
  <sce:import kind="algorithm" src="algorithm_upto.scxml" as="Upto"/>
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>

  {states}
</scxml>
"##
    )
}

fn run_calling(args: &[&str], doc: &str) -> (bool, String) {
    run_beside(
        args,
        doc,
        &[
            ("algorithm_clamp.scxml", ALGORITHM_CLAMP),
            ("algorithm_upto.scxml", ALGORITHM_LIST),
        ],
    )
}

#[test]
fn an_imported_algorithm_is_called_in_a_guard_and_an_assignment() {
    let (ok, out) = run_calling(
        &["check", "-l", "kotlin"],
        &calling_doc(
            "sce-static",
            r#"<state id="s">
    <transition event="tick" cond="Clamp(count + 1, 5) &gt; count" type="internal">
      <assign location="count" expr="Clamp(count + 1, 5)"/>
    </transition>
    <transition event="list" type="internal"><sce:action name="show"><sce:arg name="n" expr="Clamp(count, 3)"/></sce:action></transition>
  </state>"#,
        )
        // Upto is imported to be refused below; here it is not in the file.
        .replace(
            r#"  <sce:import kind="algorithm" src="algorithm_upto.scxml" as="Upto"/>
"#,
            "",
        ),
    );
    assert!(ok, "a scalar algorithm is called like a function:\n{out}");
}

#[test]
fn an_imported_algorithm_nothing_calls_is_refused_at_its_import() {
    let (ok, out) = run_calling(
        &["check"],
        &calling_doc(
            "sce-static",
            r#"<state id="s"><transition event="tick" type="internal"><assign location="count" expr="Clamp(count, 5)"/></transition></state>"#,
        ),
    );
    assert!(!ok, "`Upto` is imported and never called:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
    assert!(out.contains("Upto"), "the refusal names the import:\n{out}");
}

#[test]
fn a_list_returning_algorithm_is_refused_where_it_is_called() {
    let (ok, out) = run_calling(
        &["check"],
        &calling_doc(
            "sce-static",
            r#"<state id="s"><transition event="tick" type="internal">
      <assign location="count" expr="Clamp(count, 5)"/>
      <log label="l" expr="Upto(count)"/>
    </transition></state>"#,
        ),
    );
    assert!(
        !ok,
        "only a host calls an algorithm returning a list:\n{out}"
    );
    assert!(
        out.contains("list<uint32>"),
        "the refusal names the slot:\n{out}"
    );
}

/// A `sce-static` machine receives the failures of what it runs (SCE
/// Accepted Subset §2.15, SCE_FORGE.md §3.4.1): a `may-fail` algorithm is
/// called, and its failure becomes `error.execution` in place of the
/// statement — on every backend that lowers the model.
///
/// ⚠ This test used to hold the opposite, that such a call is refused: the
/// machine had no failure channel, so the call could only be lowered as if
/// it returned a value. E12 D5 gave it one.
#[test]
fn a_may_fail_algorithm_is_received_where_a_static_machine_calls_it() {
    let may_fail_clamp = ALGORITHM_CLAMP.replace(
        r#"<sce:return type="uint32"/>"#,
        r#"<sce:return type="uint32" may-fail="true"/>"#,
    );
    let document = calling_doc(
        "sce-static",
        r#"<state id="s"><transition event="tick" type="internal"><assign location="count" expr="Clamp(count, 5)"/></transition></state>"#,
    )
    .replace(
        r#"  <sce:import kind="algorithm" src="algorithm_upto.scxml" as="Upto"/>
"#,
        "",
    );
    for lang in ["kotlin", "rust", "python"] {
        let (ok, out) = run_beside(
            &["check", "-l", lang],
            &document,
            &[("algorithm_clamp.scxml", may_fail_clamp.as_str())],
        );
        assert!(
            ok,
            "--lang {lang}: the machine receives Clamp's failure:\n{out}"
        );
    }
}

/// A machine calling `Clamp`, importing it alone: the document the tests below
/// generate for the backends whose imports are written by the machine.
fn calling_clamp_alone() -> String {
    calling_doc(
        "sce-static",
        r#"<state id="s"><transition event="tick" type="internal"><assign location="count" expr="Clamp(count, 5)"/></transition></state>"#,
    )
    .replace(
        r#"  <sce:import kind="algorithm" src="algorithm_upto.scxml" as="Upto"/>
"#,
        "",
    )
}

#[test]
fn a_go_machine_calls_the_package_its_algorithms_generation_put_it_in() {
    // The call is the package's name and the algorithm's exported symbol, and
    // the import is the module path the packages live under: the identity a forge
    // kind importing the same algorithm derives, so the two agree on where it is.
    // A prefix's trailing slash is the same prefix.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run_beside(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
            "--go-module-prefix",
            "github.com/acme/gen/",
        ],
        &calling_clamp_alone(),
        &[("algorithm_clamp.scxml", ALGORITHM_CLAMP)],
    );
    assert!(ok, "the machine generates:\n{out}");
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("\t\"github.com/acme/gen/clamp\"\n"),
        "the machine imports the algorithm's package by the module path"
    );
    assert!(
        source.contains("clamp.Clamp(p.vCount, 5)"),
        "the call names the package and the exported function"
    );
}

#[test]
fn a_python_machine_calls_the_module_its_algorithms_generation_put_it_in() {
    // The call is the module's name and the algorithm's function, and the import
    // is the sibling-module one a forge kind importing the same algorithm
    // writes: the two agree on where the algorithm is.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run_beside(
        &[
            "generate",
            "-l",
            "python",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &calling_clamp_alone(),
        &[("algorithm_clamp.scxml", ALGORITHM_CLAMP)],
    );
    assert!(ok, "the machine generates:\n{out}");
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "py"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("\nfrom . import clamp\n"),
        "the machine imports the algorithm's module as a sibling"
    );
    assert!(
        source.contains("clamp.clamp(self.v_count, 5)"),
        "the call names the module and the function"
    );
}

#[test]
fn a_go_machine_calling_an_algorithm_needs_the_module_its_packages_live_under() {
    // A Go import path has no valid bare form, so without the module path there
    // is nothing to write the import from. It is a configuration error naming the
    // flag, not a construct the backend lacks: the same refusal a forge kind
    // importing another gives.
    let (ok, out) = run_beside(
        &["check", "-l", "go"],
        &calling_clamp_alone(),
        &[("algorithm_clamp.scxml", ALGORITHM_CLAMP)],
    );
    assert!(!ok, "no module path was given:\n{out}");
    assert!(
        out.contains("generate/invalid-config") && out.contains("go_module_prefix"),
        "the refusal names what is missing:\n{out}"
    );
    let (ok, out) = run_beside(
        &[
            "check",
            "-l",
            "go",
            "--go-module-prefix",
            "github.com/acme/gen",
        ],
        &calling_clamp_alone(),
        &[("algorithm_clamp.scxml", ALGORITHM_CLAMP)],
    );
    assert!(ok, "given the module path, the machine lowers:\n{out}");
}

#[test]
fn an_algorithm_import_whose_file_is_missing_is_refused_at_its_import() {
    let (ok, out) = run_beside(
        &["check"],
        &calling_doc(
            "sce-static",
            r#"<state id="s"><transition event="tick" type="internal"><assign location="count" expr="Clamp(count, 5)"/></transition></state>"#,
        ),
        &[("algorithm_upto.scxml", ALGORITHM_LIST)],
    );
    assert!(
        !ok,
        "algorithm_clamp.scxml is not beside the document:\n{out}"
    );
    assert_refused_at(&out, "import/file-not-found", 4);
    assert!(out.contains("algorithm_clamp.scxml"), "{out}");
}

#[test]
fn an_algorithm_import_under_another_data_model_is_refused() {
    let (ok, out) = run_calling(
        &["check"],
        &calling_doc("ecmascript", r#"<state id="s"/>"#)
            .replace(r#"sce:type="uint32" expr="0""#, r#"expr="0""#),
    );
    assert!(!ok, "only sce-static lowers an algorithm call:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 4);
}

// ── Every scenario is replayed on every backend that lowers the model ────

/// The driver that replays scenarios on a backend, and the text it names a
/// scenario by (`{s}`, the scenario file's own name — a machine can have
/// several). A backend that lowers sce-static and is not listed here fails
/// the test below until it has one.
const SCENARIO_DRIVERS: &[(&str, &str, &str)] = &[
    (
        "rust",
        "backends/rust/tests/tests/static_scenarios.rs",
        "static_datamodel/scenarios/{s}.json",
    ),
    (
        "kotlin",
        "backends/kotlin/tests/src/test/kotlin/com/sce/integration/StaticScenarioTest.kt",
        "scenario(\"{s}\")",
    ),
    (
        "cpp",
        "tests/integration/AStaticDatamodelRunsGeneratedCppTest.cpp",
        "replay(\"{s}\"",
    ),
    (
        "go",
        "backends/go/tests/integration/static_datamodel/static_scenarios_test.go",
        "replay(t, \"{s}\"",
    ),
    (
        "python",
        "backends/python/tests/integration/static_datamodel/test_static_scenarios.py",
        "replay(\"{s}\")",
    ),
    // Built by CMake from the fixtures, so there is no machine to commit.
    (
        "c11",
        "backends/c/tests/integration/test_static_scalars.c",
        "_scenario(\"{s}\"",
    ),
];

/// Where each backend's regen script commits a machine: the file that exists
/// only if it did.
const COMMITTED_MACHINES: &[(&str, &str)] = &[
    (
        "rust",
        "backends/rust/tests/src/integration/static_datamodel/{m}_sm.rs",
    ),
    (
        "kotlin",
        "backends/kotlin/tests/src/main/kotlin/com/sce/integration/{m}/{m}Sm.kt",
    ),
    (
        "go",
        "backends/go/tests/integration/static_datamodel/{m}/{m}_sm.go",
    ),
];

/// The Go mutation casefile declares the machines `scripts/regen_static_datamodel_go.sh`
/// commits as targets, because a mutated template reaches the tests only through
/// them, and it cannot derive that list. A machine the script commits and the
/// casefile does not name is one the round would regenerate and never restore.
///
/// A directory the script commits holds a machine (`<m>_sm.go`, with one
/// `<m>__sce_synth_invoke__<id>_sm.go` beside it for each child an `<invoke>`
/// declares in place) or an imported algorithm's package (`<m>.go`). Every
/// `*_sm.go` is a target, the children included: a child is generated from the
/// same templates as its parent, and one the casefile left out is one the round
/// would regenerate and never restore. An algorithm's package is not: its
/// generation reads neither a statechart template nor a lowering spelling, so it
/// comes back from the regeneration as it was.
#[test]
fn the_go_mutation_casefile_names_every_committed_go_machine() {
    let root = repo_root();
    let casefile = std::fs::read_to_string(root.join(
        "sce-build/tests/mutations/a_static_go_machine_ends_its_block_at_a_failure_go.cases",
    ))
    .expect("the Go casefile");
    let committed = root.join("backends/go/tests/integration/static_datamodel");
    let mut directories: Vec<String> = std::fs::read_dir(&committed)
        .expect("the committed Go machines")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.is_dir())
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    directories.sort();
    assert!(!directories.is_empty(), "no committed Go machine to judge");
    let mut machines = 0;
    for directory in directories {
        let mut generated: Vec<String> = std::fs::read_dir(committed.join(&directory))
            .expect("a committed Go directory")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| name.ends_with("_sm.go"))
            .collect();
        generated.sort();
        if generated.is_empty() {
            let algorithm_file = committed.join(&directory).join(format!("{directory}.go"));
            assert!(
                algorithm_file.exists(),
                "{directory} holds neither a machine nor an algorithm's package"
            );
            continue;
        }
        assert!(
            generated.contains(&format!("{directory}_sm.go")),
            "{directory} holds machine files {generated:?} and not the machine it is named for"
        );
        for file in generated {
            machines += 1;
            let target =
                format!("backends/go/tests/integration/static_datamodel/{directory}/{file}");
            assert!(
                casefile.contains(&target),
                "the Go casefile does not declare {target} as a mutation target"
            );
        }
    }
    assert!(machines > 0, "no committed Go machine to judge");
}

/// A scenario (`fixtures/static_datamodel/scenarios/<name>.json`) is the
/// behaviour of a sce-static machine, stated once and replayed by every
/// backend that lowers the model. Which backends those are is ASKED of the
/// generator — each scenario's machine is checked in every language — not
/// written here, so a backend that starts lowering the model is held to
/// every scenario at once. The regen scripts derive their machines from the
/// fixture directory, so what is held is that they do, and that the machine
/// is committed where each driver builds it from — or the driver would replay
/// a machine nobody regenerates.
#[test]
fn every_scenario_is_replayed_on_every_backend_that_lowers_its_machine() {
    let root = repo_root();
    let fixtures = root.join("sce-build/tests/fixtures/static_datamodel");
    let mut scenarios: Vec<PathBuf> = std::fs::read_dir(fixtures.join("scenarios"))
        .expect("the scenarios directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    scenarios.sort();
    assert!(!scenarios.is_empty(), "no scenario to judge");
    let regen: Vec<String> = ["rust", "kotlin", "go", "python"]
        .iter()
        .map(|lang| {
            std::fs::read_to_string(root.join(format!("scripts/regen_static_datamodel_{lang}.sh")))
                .expect("a regen script")
        })
        .collect();
    for scenario in &scenarios {
        let text = std::fs::read_to_string(scenario).expect("a scenario");
        let value: serde_json::Value = serde_json::from_str(&text).expect("a scenario is JSON");
        let machine = value["machine"]
            .as_str()
            .expect("a scenario names its machine");
        let document = std::fs::read_to_string(fixtures.join(format!("{machine}.scxml")))
            .unwrap_or_else(|_| panic!("{}: no fixture {machine}.scxml", scenario.display()));
        for script in &regen {
            assert!(
                script.contains(r#"grep -l 'datamodel="sce-static"'"#),
                "a regen script lists its machines by hand, so {machine} is one it may not commit"
            );
        }
        let stem = scenario
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("a scenario file has a name");
        // Beside its siblings, where its `<sce:import src>`s resolve: a probe
        // written alone would be refused for an import it cannot find, and
        // read as a machine no backend lowers.
        let siblings: Vec<(String, String)> = std::fs::read_dir(&fixtures)
            .expect("the fixture directory")
            .map(|entry| entry.expect("an entry").path())
            .filter(|path| path.extension().is_some_and(|e| e == "scxml"))
            .map(|path| {
                (
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    std::fs::read_to_string(&path).expect("a fixture"),
                )
            })
            .collect();
        let siblings: Vec<(&str, &str)> = siblings
            .iter()
            .map(|(name, text)| (name.as_str(), text.as_str()))
            .collect();
        let lowering: Vec<&str> = ["cpp", "c11", "go", "python", "kotlin", "rust"]
            .into_iter()
            .filter(|lang| run_beside(&["check", "-l", lang], &document, &siblings).0)
            .collect();
        assert!(
            !lowering.is_empty(),
            "{machine}: no backend lowers it, so no scenario runs it"
        );
        // A backend that lowers the machine commits it where its driver builds
        // it from; one that refuses it has nothing to commit.
        for (lang, committed) in COMMITTED_MACHINES {
            if !lowering.contains(lang) {
                continue;
            }
            let path = root.join(committed.replace("{m}", machine));
            assert!(
                path.exists(),
                "{machine}: its {lang} regen script has not committed {}",
                path.display()
            );
        }
        for lang in lowering {
            let (_, driver, marker) = SCENARIO_DRIVERS
                .iter()
                .find(|(l, _, _)| *l == lang)
                .unwrap_or_else(|| panic!("{lang} lowers sce-static but has no scenario driver"));
            let source = std::fs::read_to_string(root.join(driver)).expect("a driver");
            assert!(
                source.contains(&marker.replace("{s}", stem)),
                "{lang}: {driver} does not replay the scenario {stem}"
            );
        }
    }
}
