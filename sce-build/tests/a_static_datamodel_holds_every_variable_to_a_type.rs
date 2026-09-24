// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
    assert!(
        record.contains(&format!("\"line\":{line},")),
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
fn every_backend_that_does_not_lower_the_model_refuses_to_generate_it() {
    let document = doc(
        "sce-static",
        r#"<data id="count" sce:type="uint32" expr="0"/>"#,
    );
    for lang in ["rust", "cpp", "c11", "go", "python"] {
        let (ok, out) = run(&["check", "-l", lang], &document);
        assert!(
            !ok,
            "--lang {lang}: a backend that does not lower sce-static must \
             refuse, not evaluate forge expressions in a script engine:\n{out}"
        );
        assert!(
            out.contains("generate/unsupported-feature") && out.contains("sce-static"),
            "--lang {lang}: expected the unsupported-feature refusal naming \
             the data model:\n{out}"
        );
    }
}

#[test]
fn kotlin_lowers_the_model_with_no_script_engine() {
    // Kotlin holds the variables as fields and lowers every expression
    // natively, so the machine it generates carries no engine — the manifest
    // says so, and the Kotlin integration suite (StaticDatamodelTest.kt)
    // compiles and drives the committed machine.
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &machine(
            r#"<state id="s">
    <transition event="tick" cond="count &lt; 10 &amp;&amp; In('s')" type="internal">
      <assign location="count" expr="count + 1"/>
    </transition>
  </state>"#,
        ),
    );
    assert!(ok, "Kotlin lowers sce-static:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":false"),
        "a sce-static machine needs no script engine:\n{out}"
    );
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
fn an_assignment_to_a_whole_record_is_refused() {
    let (ok, out) = run_record(
        &["check"],
        &record(
            EVERY_FIELD,
            r#"<state id="s"><transition event="day.picked" type="internal"><assign location="shown" expr="_event.data"/></transition></state>"#,
        ),
    );
    assert!(!ok, "a record is updated a field at a time:\n{out}");
    assert_refused_at(&out, "expression/unsupported-construct", 12);
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
    assert!(!ok, "only a list has a capacity:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
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
