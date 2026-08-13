// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The `datamodel` attribute selects a language, so it has to reach a
// decision.
//
// W3C SCXML §3.2 gives the attribute the values `"null"`, `"ecmascript"`,
// `"xpath"` "or other platform-defined values"; Appendix B says what each
// one means. SCE parsed it into a `String` that nothing subsequently
// consulted. Measured against the pin this suite was written on, every one
// of `null`, `ecmascript`, `xpath`, `lua` and an invented token produced
// byte-identical decisions — the same `needs_script_engine: true`, the
// same emitted sites — so a document could name a data model it was not
// evaluated in, and nothing anywhere would say so.
//
// That is what makes the rest of this file's assertions worth pinning
// rather than obvious: each one is a decision the attribute did not reach
// before.
//
// The two rejections stay separate on purpose. `xpath` is valid W3C SCXML
// that SCE has not implemented; an invented token is a value no processor
// defines. They reject alike and repair differently.

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

/// Run `check` over a document and return `(exit_ok, diagnostics)`.
///
/// Asks for the JSON form: the wire `code` is what a consumer dispatches
/// on, and the human rendering carries only the message. Asserting the
/// slug is most of the point — a diagnostic that renders the right prose
/// under the wrong code routes to the wrong repair.
fn check(doc: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .arg("check")
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// A document with the given root attributes and body.
fn doc(root_attrs: &str, body: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s" {root_attrs}>
  {body}
  <final id="done"/>
</scxml>
"##
    )
}

#[test]
fn a_data_model_sce_does_not_implement_is_refused_as_unimplemented() {
    let (ok, out) = check(&doc(
        r#"datamodel="xpath""#,
        r#"<state id="s"><transition event="go" target="done"/></state>"#,
    ));
    assert!(!ok, "datamodel=\"xpath\" must be refused, got success");
    assert!(
        out.contains("scxml/unsupported-datamodel"),
        "expected scxml/unsupported-datamodel, got:\n{out}"
    );
    assert!(
        out.contains("does not implement"),
        "the message must say SCE is the limitation, not that the value is \
         nonsense — `xpath` is valid W3C SCXML:\n{out}"
    );
}

#[test]
fn a_data_model_no_processor_defines_is_refused_as_undefined() {
    // `lua` is the token this repository's own mesh fixtures carried, in
    // the belief that it selected the Lua engine. It never did: the engine
    // is whichever `IScriptEngine` the deployment injects, and the
    // attribute reached no decision at all.
    let (ok, out) = check(&doc(
        r#"datamodel="lua""#,
        r#"<state id="s"><transition event="go" target="done"/></state>"#,
    ));
    assert!(!ok, "datamodel=\"lua\" must be refused, got success");
    assert!(
        out.contains("scxml/unsupported-datamodel"),
        "expected scxml/unsupported-datamodel, got:\n{out}"
    );
    assert!(
        out.contains("not a data model any processor defines"),
        "an invented token must not be reported as an SCE limitation:\n{out}"
    );
}

#[test]
fn an_absent_attribute_takes_the_declared_platform_default() {
    // §3.2 leaves the default platform-specific. SCE's choice is
    // ECMAScript, so a document that omits the attribute and writes a
    // value expression compiles rather than being refused for using a
    // language the Null data model lacks.
    let (ok, out) = check(&doc(
        "",
        r#"<datamodel><data id="n" expr="1"/></datamodel>
  <state id="s"><transition event="go" cond="n > 0" target="done"/></state>"#,
    ));
    assert!(
        ok,
        "an absent datamodel attribute must default to a model with a \
         value expression language:\n{out}"
    );
}

#[test]
fn the_null_data_model_admits_its_one_boolean_expression() {
    // §B.1.2: the boolean expression language consists of the In
    // predicate only. W3C test436 is exactly this document.
    let (ok, out) = check(&doc(
        r#"datamodel="null""#,
        r#"<state id="s"><transition cond="In('s')" target="done"/></state>"#,
    ));
    assert!(
        ok,
        "In() is the Null data model's boolean expression:\n{out}"
    );
}

#[test]
fn the_null_data_model_refuses_a_boolean_expression_that_is_not_in() {
    // `cond="true"` is not `In(id)`. §B.1.2 admits no other form — a
    // literal is still a term in a language the model does not have.
    let (ok, out) = check(&doc(
        r#"datamodel="null""#,
        r#"<state id="s"><transition cond="true" target="done"/></state>"#,
    ));
    assert!(!ok, "cond=\"true\" under null must be refused:\n{out}");
    assert!(
        out.contains("scxml/null-datamodel-forbids-construct") && out.contains("B.1.2"),
        "the rejection must name §B.1.2, the sub-section that withholds \
         the language:\n{out}"
    );
}

#[test]
fn the_null_data_model_refuses_a_conjunction_built_around_in() {
    // The rule has to be "the condition *is* `In(id)`", not "the
    // condition *contains* one". `In('s') && n > 0` reads as an In()
    // predicate to any check that only looks for the substring, and the
    // half beside it is written in the value expression language §B.1.4
    // withholds. Widening the check is the mistake that admits it.
    let (ok, out) = check(&doc(
        r#"datamodel="null""#,
        r#"<state id="s"><transition cond="In('s') &amp;&amp; n > 0" target="done"/></state>"#,
    ));
    assert!(
        !ok,
        "a conjunction around In() is not the Null data model's boolean \
         expression language:\n{out}"
    );
    assert!(
        out.contains("B.1.2"),
        "expected the §B.1.2 rejection, got:\n{out}"
    );
}

#[test]
fn the_null_data_model_refuses_a_value_expression() {
    let (ok, out) = check(&doc(
        r#"datamodel="null""#,
        r#"<state id="s"><onentry><send event="go" delayexpr="'1s'"/></onentry></state>"#,
    ));
    assert!(!ok, "delayexpr under null must be refused:\n{out}");
    assert!(
        out.contains("B.1.4"),
        "a value expression must be reported under §B.1.4, not folded \
         into a generic 'null has nothing' message:\n{out}"
    );
}

#[test]
fn the_null_data_model_refuses_a_place_to_store_into() {
    let (ok, out) = check(&doc(
        r#"datamodel="null""#,
        r#"<datamodel><data id="n"/></datamodel>
  <state id="s"><transition event="go" target="done"/></state>"#,
    ));
    assert!(!ok, "<datamodel> under null must be refused:\n{out}");
    assert!(
        out.contains("B.1.1"),
        "the absent underlying data model is §B.1.1:\n{out}"
    );
}

#[test]
fn the_null_data_model_refuses_script_text_but_admits_a_native_block() {
    let with_text = check(&doc(
        r#"datamodel="null""#,
        r#"<state id="s"><onentry><script>x = 1</script></onentry></state>"#,
    ));
    assert!(
        !with_text.0 && with_text.1.contains("B.1.5"),
        "script text under null must be refused under §B.1.5:\n{}",
        with_text.1
    );

    // `<script><cpp>` is SCE's native host action (§2.11): the parser
    // lowers it into the generated language and no script engine
    // evaluates it, so §B.1.5 withholds nothing it uses. Refusing this
    // would push an honestly engine-free document onto the scripting
    // tier to satisfy a rule about a language it never used — which is
    // what the seven `examples/doom_wasm` documents would have suffered.
    // A body naming no context object, so this exercises §B.1.5 alone —
    // `<cpp>` referencing an object without an `<sce:context>` declaration
    // is a separate rule and would reject for a reason this test is not
    // about.
    let native = check(&doc(
        r#"datamodel="null""#,
        r#"<state id="s"><onentry><script><cpp>tick();</cpp></script></onentry></state>"#,
    ));
    assert!(
        native.0,
        "a native <script><cpp> block declares no data model language:\n{}",
        native.1
    );
}

#[test]
fn a_nested_inline_document_is_judged_by_its_own_declaration() {
    // The child inside `<content>` declares `ecmascript` and uses a value
    // expression; the parent declares `null` and uses none. Judging the
    // child by its parent's declaration would refuse a document that is
    // correct on its own terms.
    let (ok, out) = check(&doc(
        r#"datamodel="null""#,
        r#"<state id="s">
    <invoke type="scxml">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
               datamodel="ecmascript" initial="c">
          <datamodel><data id="n" expr="1"/></datamodel>
          <state id="c"><transition event="go" target="cdone"/></state>
          <final id="cdone"/>
        </scxml>
      </content>
    </invoke>
    <transition event="go" target="done"/>
  </state>"#,
    ));
    assert!(
        ok,
        "a nested inline document declares its own data model:\n{out}"
    );
}
