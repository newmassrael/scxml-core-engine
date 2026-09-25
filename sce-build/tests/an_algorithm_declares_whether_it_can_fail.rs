//! `<sce:return may-fail="true">` — the integer arithmetic contract's
//! declaration (SCE_FORGE.md §3.4.1).
//!
//! An algorithm whose integer arithmetic can overflow, divide by zero or
//! take a signed MIN / -1 says so on its signature's return, and its
//! callers receive the failure instead of a wrapped or trapped value. This
//! file holds the declaration itself: that the parser reads every spelling
//! the schema admits and refuses the rest instead of reading them as
//! `false`, that the page shows it and reads it back, and that a backend
//! which has not learned the checked lowering refuses the algorithm rather
//! than emitting an unchecked body behind a signature that promises a
//! failure channel.

use sce_build::forge::model::{AlgorithmModel, ForgeDocument};
use sce_build::forge::parser::parse_forge;
use sce_build::forge::unpseudo;
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// An algorithm whose `<sce:return>` carries `attr` verbatim.
fn document(attr: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_may_fail" version="1.0">
  <sce:signature>
    <sce:param name="prev" type="uint32"/>
    <sce:return type="uint32"{attr}/>
  </sce:signature>
  <sce:body>
    <sce:return expr="prev + 1"/>
  </sce:body>
</scxml>
"#
    )
}

fn label() -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: "probe_may_fail",
        diagnostic_label: "probe_may_fail",
    }
}

fn algorithm(text: &str) -> AlgorithmModel {
    match parse_forge(text, label()) {
        Ok(Some(ForgeDocument::Algorithm(m))) => m,
        other => panic!("expected an algorithm, got {other:?}"),
    }
}

#[test]
fn every_spelling_the_schema_admits_is_read() {
    for (attr, want) in [
        ("", false),
        (r#" may-fail="false""#, false),
        (r#" may-fail="0""#, false),
        (r#" may-fail="true""#, true),
        (r#" may-fail="1""#, true),
        (r#" may-fail=" true ""#, true),
    ] {
        assert_eq!(
            algorithm(&document(attr)).signature.may_fail,
            want,
            "<sce:return{attr}>"
        );
    }
}

/// A value outside `xs:boolean` is refused in either build: by the schema
/// when the `xsd` feature is on, and by the parser when it is off, where it
/// would otherwise vanish into `false` — the declaration dropped, and the
/// body lowered unchecked.
#[test]
fn a_value_the_schema_does_not_admit_is_refused_not_read_as_false() {
    let err = parse_forge(&document(r#" may-fail="yes""#), label())
        .expect_err("`yes` is not an xs:boolean");
    let text = err.to_string();
    assert!(
        text.contains("may-fail") && text.contains("yes"),
        "the refusal names the attribute and its value: {text}"
    );
}

/// Whether the algorithm can fail is the signature's to say; a body's
/// return saying it too would be a second answer that can disagree.
#[test]
fn a_body_return_cannot_declare_it() {
    let text = document("").replace(
        r#"<sce:return expr="prev + 1"/>"#,
        r#"<sce:return expr="prev + 1" may-fail="true"/>"#,
    );
    let err = parse_forge(&text, label()).expect_err("refused on a body's return");
    assert!(err.to_string().contains("may-fail"), "{err}");
    assert_eq!(
        err.location.line,
        Some(8),
        "placed at the body's <sce:return>: {err}"
    );
}

#[test]
fn the_page_shows_the_declaration_and_reads_it_back() {
    let document = ForgeDocument::Algorithm(algorithm(&document(r#" may-fail="true""#)));
    let rendered = sce_build::forge::pseudo::render(&document).expect("an algorithm renders");
    assert!(
        rendered
            .lines()
            .any(|l| l.starts_with("algorithm ") && l.trim_end().ends_with("-> uint32 may-fail")),
        "the signature line carries the clause:\n{rendered}"
    );
    let read_back = unpseudo::parse(&rendered).unwrap_or_else(|e| panic!("{e:?}\n{rendered}"));
    assert_eq!(
        unpseudo::ir_for_comparison(&document).expect("a model serialises"),
        unpseudo::ir_for_comparison(&read_back).expect("a model serialises"),
        "the declaration was lost between the renderer and the reader:\n{rendered}"
    );
}

/// No backend lowers the checked arithmetic yet, so every one refuses —
/// each joins `MAY_FAIL_BACKENDS` in the commit that teaches it.
#[test]
fn a_backend_without_the_checked_lowering_refuses_the_algorithm() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe_may_fail.scxml");
    std::fs::write(&path, document(r#" may-fail="true""#)).expect("write document");
    for &lang in Language::ALL {
        let err = sce_build::compile_forge_file(&path, lang, &[], &ForgeCompileOptions::default())
            .err()
            .unwrap_or_else(|| panic!("{lang:?} accepted a may-fail algorithm"));
        let text = err.to_string();
        assert!(
            text.contains("feature unsupported") && text.contains("may-fail"),
            "{lang:?}: {text}"
        );
    }
    // The same algorithm without the declaration is accepted: the refusal
    // is of the declaration, not of the document.
    std::fs::write(&path, document("")).expect("write document");
    for &lang in Language::ALL {
        if let Err(err) =
            sce_build::compile_forge_file(&path, lang, &[], &ForgeCompileOptions::default())
        {
            panic!("{lang:?} refused the undeclared algorithm: {err}");
        }
    }
}
