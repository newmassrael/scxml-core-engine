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
    let dir = tempdir().expect("tempdir");
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
