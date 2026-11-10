// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `<sce:lift>` (docs/adr/0087, section 2.1; docs/SCE_ACCEPTED_SUBSET.md §2.15, "A text
// read as a record"): a text a statechart holds read into a record of a schema, or the
// refusal of it kept as a record of the standard refusal document. Each rule of the
// statement is refused on its own code and on the row of the element, and a lift that
// keeps every rule is lowered by every backend, the Interpreter's included.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

use common::composed_lowering::{lowering, refusing_or_skip};
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

/// Run `sce-codegen <args…> probe.scxml` with JSON diagnostics, `siblings` written
/// beside the document where its imports resolve, and return `(exit_ok, output)`.
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

/// The schema a text is read as: one field of each kind a lift reads now.
fn answer_schema(title: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="answer" sce:event-name="answer">
  <datamodel>
    <data id="{title}" sce:type="string[16]" sce:direction="in"/>
    <data id="count" sce:type="uint32" sce:direction="in"/>
    <data id="note" sce:type="optional&lt;string[8]&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#
    )
}

/// A schema whose field holds a list, which a lift does not read yet.
const LISTED_SCHEMA: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="listed" sce:event-name="listed">
  <datamodel>
    <data id="tags" sce:type="list&lt;string[8]&gt;[3]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A copy of the standard refusal record in another file: another record.
const COPIED_REFUSAL: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="refusal" sce:event-name="lift.copied">
  <sce:import kind="enum" src="sce:std/lift/refusal_code.scxml" as="LiftRefusalCode"/>
  <datamodel>
    <data id="code" sce:type="enum:LiftRefusalCode" sce:direction="in"/>
    <data id="path" sce:type="string[256]" sce:direction="in"/>
    <data id="bound" sce:type="optional&lt;uint32&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// The event a text arrives on.
const GIVEN_SCHEMA: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="given" sce:event-name="text.given">
  <datamodel>
    <data id="text" sce:type="string[64]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// The files every statechart below imports, the answer's first field named `title`.
fn siblings() -> Vec<(&'static str, String)> {
    vec![
        ("answer.scxml", answer_schema("title")),
        ("listed.scxml", LISTED_SCHEMA.to_string()),
        ("copied_refusal.scxml", COPIED_REFUSAL.to_string()),
        ("given.scxml", GIVEN_SCHEMA.to_string()),
    ]
}

/// A statechart whose `text.given` transition holds `body`; `datamodel` its data model.
/// The `<sce:lift>` a body holds stands on line 19.
fn statechart(datamodel: &str, body: &str) -> String {
    statechart_with(datamodel, true, body)
}

/// [`statechart`], with the list schema and the copied refusal record imported or, for
/// a document every backend generates, comments on their rows.
fn statechart_with(datamodel: &str, probes: bool, body: &str) -> String {
    let (listed, copied, other) = if probes {
        (
            r#"<sce:import kind="event-schema" src="listed.scxml" as="Listed"/>"#,
            r#"<sce:import kind="event-schema" src="copied_refusal.scxml" as="Copied"/>"#,
            r#"<data id="other" sce:type="optional&lt;record:Copied&gt;" expr="null"/>"#,
        )
    } else {
        (
            "<!-- no list schema -->",
            "<!-- no copied refusal -->",
            "<!-- no other -->",
        )
    };
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="{datamodel}" name="probe">
  <sce:import kind="event-schema" src="answer.scxml" as="Answer"/>
  {listed}
  {copied}
  <sce:import kind="event-schema" src="given.scxml" as="Given"/>
  <sce:import kind="event-schema" src="sce:std/lift/refusal.scxml" as="LiftRefusal"/>
  <datamodel>
    <data id="said" sce:type="string[64]" expr="''"/>
    <data id="maybe" sce:type="optional&lt;string[64]&gt;" expr="null"/>
    <data id="count" sce:type="uint32" expr="0"/>
    <data id="answer" sce:type="optional&lt;record:Answer&gt;" expr="null"/>
    <data id="why" sce:type="optional&lt;record:LiftRefusal&gt;" expr="null"/>
    {other}
  </datamodel>
  <state id="s">
    <transition event="text.given" type="internal">
{body}
    </transition>
  </state>
</scxml>
"#
    )
}

fn check(body: &str) -> (bool, String) {
    let siblings = siblings();
    let borrowed: Vec<(&str, &str)> = siblings.iter().map(|(n, t)| (*n, t.as_str())).collect();
    run(&["check"], &statechart("sce-static", body), &borrowed)
}

/// Assert a refusal carries `code` and is placed on `line`.
fn assert_refused_at(out: &str, code: &str, line: u32) {
    let record = out
        .lines()
        .find(|l| l.contains(&format!("\"code\":\"{code}\"")))
        .unwrap_or_else(|| panic!("expected {code}, got:\n{out}"));
    assert!(
        record.contains(&format!("\"line\":{line},"))
            || record.contains(&format!("\"line\":{line}}}")),
        "{code} must be placed on line {line}:\n{record}"
    );
}

const LIFT_LINE: u32 = 19;

#[test]
fn a_lift_that_keeps_every_rule_is_accepted() {
    for text in ["said", "_event.data.text"] {
        let (ok, out) = check(&format!(
            r#"      <sce:lift text="{text}" schema="Answer" location="answer" refusal="why"/>"#
        ));
        assert!(ok, "a lift of `{text}` keeps every rule:\n{out}");
    }
    // The present value of an optional text, read where a test proves it present.
    let (ok, out) = check(
        r#"      <if cond="maybe !== null"><sce:lift text="maybe.value" schema="Answer" location="answer" refusal="why"/></if>"#,
    );
    assert!(ok, "a present optional text is a text:\n{out}");
}

#[test]
fn a_schema_the_document_does_not_import_is_an_invalid_attribute() {
    let (ok, out) =
        check(r#"      <sce:lift text="said" schema="Anwser" location="answer" refusal="why"/>"#);
    assert!(!ok);
    assert_refused_at(&out, "validation/invalid-attribute", LIFT_LINE);
    assert!(
        out.contains("Answer"),
        "the candidates name the imports:\n{out}"
    );
}

/// [`statechart`] under `sce-static` with `listed`, a variable of the record whose
/// field holds a list, in place of `other`, and `body` on the line of the lift.
fn listed_statechart(body: &str) -> String {
    statechart("sce-static", body).replace(
        r#"<data id="other" sce:type="optional&lt;record:Copied&gt;" expr="null"/>"#,
        r#"<data id="listed" sce:type="optional&lt;record:Listed&gt;" expr="null"/>"#,
    )
}

/// A lift of the record whose field holds a list into `listed`.
const LIFT_LISTED: &str =
    r#"      <sce:lift text="said" schema="Listed" location="listed" refusal="why"/>"#;

/// A schema whose field holds a list is lifted as any other is (docs/adr/0087,
/// "Landing order" item 8): the judge takes it, and each backend whose row of the
/// one table holds a variable of such a record (`ComposedLowering::records`) lowers
/// the lift into it.
#[test]
fn a_schema_that_holds_a_list_is_lifted_into_a_variable_of_its_record() {
    let siblings = siblings();
    let borrowed: Vec<(&str, &str)> = siblings.iter().map(|(n, t)| (*n, t.as_str())).collect();
    let doc = listed_statechart(LIFT_LISTED);
    assert!(doc.contains("location=\"listed\"") && doc.contains("<data id=\"listed\""));
    let (ok, out) = run(&["check"], &doc, &borrowed);
    assert!(
        ok,
        "the judge takes a lift of a record that holds a list:\n{out}"
    );
    let holding = lowering(|row| row.records);
    assert!(
        !holding.is_empty(),
        "some backend holds a variable of a record that holds a list"
    );
    for language in holding {
        let (ok, out) = run(&["check", "-l", language], &doc, &borrowed);
        assert!(ok, "{language} lifts a record that holds a list:\n{out}");
    }
}

/// Pinned by `ComposedLowering::of` (`sce-build/src/forge/event_schema_check.rs`):
/// each backend whose row holds no variable of a record that holds a list refuses
/// that variable by its own name where it is declared, and with it the lift that
/// writes it; so does the Interpreter's lowering, whose row holds none either.
#[test]
fn a_schema_that_holds_a_list_is_refused_by_name_where_its_variable_is_not_held() {
    let siblings = siblings();
    let borrowed: Vec<(&str, &str)> = siblings.iter().map(|(n, t)| (*n, t.as_str())).collect();
    let doc = listed_statechart(LIFT_LISTED);
    for language in refusing_or_skip(
        |row| row.records,
        "a_schema_that_holds_a_list_is_refused_by_name_where_its_variable_is_not_held",
    ) {
        let (ok, out) = run(&["check", "-l", language], &doc, &borrowed);
        assert!(!ok, "{language} holds no such variable yet:\n{out}");
        assert!(
            out.contains("\"code\":\"generate/unsupported-feature\""),
            "{language} refuses it by name:\n{out}"
        );
        assert!(
            out.contains("a statechart's variable"),
            "the refusal names the variable:\n{out}"
        );
        assert!(out.contains("tags"), "the refusal names the field:\n{out}");
        assert!(
            out.contains(&format!("has no {language} lowering")),
            "the refusal names the backend:\n{out}"
        );
    }
    let (ok, out) = run(&["lower"], &doc, &borrowed);
    assert!(!ok, "the Interpreter holds no such variable yet:\n{out}");
    assert!(
        out.contains("generate/unsupported-feature") && out.contains("has no ecmascript lowering"),
        "the Interpreter refuses it by its own name:\n{out}"
    );
}

#[test]
fn a_location_that_is_not_a_variable_of_the_record_is_refused() {
    for location in ["said", "nothing", "why"] {
        let (ok, out) = check(&format!(
            r#"      <sce:lift text="said" schema="Answer" location="{location}" refusal="why"/>"#
        ));
        assert!(!ok, "location {location}");
        assert_refused_at(&out, "validation/lift-location", LIFT_LINE);
        assert!(out.contains("\"actual\":\""), "{out}");
    }
}

#[test]
fn a_refusal_that_is_not_the_standard_refusal_record_is_refused() {
    // Another variable's type, a copy of the record in another file, no variable at all,
    // and the variable the location names.
    for refusal in ["count", "other", "nothing", "answer"] {
        let (ok, out) = check(&format!(
            r#"      <sce:lift text="said" schema="Answer" location="answer" refusal="{refusal}"/>"#
        ));
        assert!(!ok, "refusal {refusal}");
        assert_refused_at(&out, "validation/lift-location", LIFT_LINE);
        assert!(out.contains(&format!("\"actual\":\"{refusal}\"")), "{out}");
    }
}

#[test]
fn a_text_that_is_not_a_present_bounded_text_is_refused() {
    // A name of a number, of a record that may be absent, and of a text that may be
    // absent taken whole: each a name the attribute admits, of a type a lift does not
    // read. An expression is no name at all, refused first as the attribute's.
    for text in ["count", "answer", "maybe"] {
        let (ok, out) = check(&format!(
            r#"      <sce:lift text="{text}" schema="Answer" location="answer" refusal="why"/>"#
        ));
        assert!(!ok, "text {text}");
        assert_refused_at(&out, "validation/lift-text", LIFT_LINE);
    }
}

#[test]
fn a_schema_whose_longest_pointer_is_past_the_path_bound_is_refused() {
    // A field name of 300 bytes makes a pointer of 301, past the 256 of the path.
    let long = "t".repeat(300);
    let siblings = [
        ("answer.scxml", answer_schema(&long)),
        ("listed.scxml", LISTED_SCHEMA.to_string()),
        ("copied_refusal.scxml", COPIED_REFUSAL.to_string()),
        ("given.scxml", GIVEN_SCHEMA.to_string()),
    ];
    let borrowed: Vec<(&str, &str)> = siblings.iter().map(|(n, t)| (*n, t.as_str())).collect();
    let (ok, out) = run(
        &["check"],
        &statechart(
            "sce-static",
            r#"      <sce:lift text="said" schema="Answer" location="answer" refusal="why"/>"#,
        ),
        &borrowed,
    );
    assert!(!ok);
    assert_refused_at(&out, "validation/lift-path-bound", LIFT_LINE);
    assert!(out.contains("301 bytes"), "the length is named:\n{out}");
}

#[test]
fn a_lift_under_another_data_model_is_refused() {
    let siblings = siblings();
    let borrowed: Vec<(&str, &str)> = siblings.iter().map(|(n, t)| (*n, t.as_str())).collect();
    let (ok, out) = run(
        &["check"],
        &statechart(
            "ecmascript",
            r#"      <sce:lift text="said" schema="Answer" location="answer" refusal="why"/>"#,
        ),
        &borrowed,
    );
    assert!(!ok);
    assert!(
        out.contains("\"code\":\"scxml/static-datamodel-rule\""),
        "a lift exists only under sce-static:\n{out}"
    );
}

/// Every backend lowers a lift, and the Interpreter's lowering of the document carries
/// the library's reader of a text, not the element.
#[test]
fn every_backend_lowers_a_lift() {
    let siblings = siblings();
    let borrowed: Vec<(&str, &str)> = siblings.iter().map(|(n, t)| (*n, t.as_str())).collect();
    let doc = statechart_with(
        "sce-static",
        false,
        r#"      <sce:lift text="_event.data.text" schema="Answer" location="answer" refusal="why"/>"#,
    );
    for language in Language::ALL.iter().map(|l| l.canonical_name()) {
        let (ok, out) = run(&["check", "-l", language], &doc, &borrowed);
        assert!(ok, "{language} lowers a lift:\n{out}");
    }
    let (ok, out) = run(&["lower"], &doc, &borrowed);
    assert!(ok, "the Interpreter lowers a lift:\n{out}");
    assert!(out.contains("SceStatic.lift("), "{out}");
    assert!(
        !out.contains("<sce:lift"),
        "the element is replaced:\n{out}"
    );
}
