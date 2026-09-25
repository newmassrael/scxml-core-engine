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

use sce_build::forge::generator::lowers_may_fail;
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

/// Generate `text` (written as `<stem>.scxml` beside `support`) for `lang`.
fn generate(
    stem: &str,
    text: &str,
    support: &[(&str, &str)],
    lang: Language,
) -> Result<String, String> {
    let dir = tempfile::tempdir().expect("tempdir");
    for (file, body) in support {
        std::fs::write(dir.path().join(file), body).expect("write support document");
    }
    let path = dir.path().join(format!("{stem}.scxml"));
    std::fs::write(&path, text).expect("write document");
    sce_build::compile_forge_file(&path, lang, &[], &ForgeCompileOptions::default())
        .map(|out| out.files[0].1.clone())
        .map_err(|e| e.to_string())
}

/// A backend lowers `may-fail` in the commit that teaches it the checked
/// lowering, and refuses it until then; the refusal is of the declaration,
/// not of the document.
#[test]
fn only_a_backend_with_the_checked_lowering_accepts_the_algorithm() {
    assert!(
        lowers_may_fail(Language::Rust),
        "Rust lowers may-fail since the checked lowering landed"
    );
    for &lang in Language::ALL {
        let outcome = generate(
            "probe_may_fail",
            &document(r#" may-fail="true""#),
            &[],
            lang,
        );
        match (lowers_may_fail(lang), outcome) {
            (true, Ok(_)) => {}
            (true, Err(err)) => panic!("{lang:?} lowers may-fail and refused it: {err}"),
            (false, Ok(_)) => panic!("{lang:?} emitted a may-fail algorithm it does not check"),
            (false, Err(text)) => assert!(
                text.contains("feature unsupported") && text.contains("may-fail"),
                "{lang:?}: {text}"
            ),
        }
    }
}

/// The same increment, guarded so the range analysis proves it cannot pass
/// `uint32`'s maximum: an algorithm that needs no declaration.
fn guarded_document() -> String {
    document("").replace(
        r#"<sce:return expr="prev + 1"/>"#,
        r#"<sce:if cond="prev &lt; 4294967295">
      <sce:return expr="prev + 1"/>
    </sce:if>
    <sce:return expr="prev"/>"#,
    )
}

/// The contract, enforced (SCE_FORGE.md §3.4.1): an operation the analysis
/// cannot prove safe, in an algorithm that does not declare `may-fail`, is
/// refused — on every backend alike, at the operation as written — and the
/// same operation behind a guard that bounds it is accepted.
#[test]
fn an_undeclared_operation_that_can_fail_is_refused_where_it_is_written() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe_may_fail.scxml");
    std::fs::write(&path, document("")).expect("write document");
    for &lang in Language::ALL {
        let err = sce_build::compile_forge_file(&path, lang, &[], &ForgeCompileOptions::default())
            .err()
            .unwrap_or_else(|| panic!("{lang:?} accepted `prev + 1` without may-fail"));
        let text = err.to_string();
        assert!(
            text.contains("`prev + 1` can overflow (uint32)") && text.contains("may-fail"),
            "{lang:?}: {text}"
        );
        assert_eq!(
            err.location.line,
            Some(8),
            "{lang:?}: placed at the body's <sce:return>: {text}"
        );
    }
    std::fs::write(&path, guarded_document()).expect("write document");
    for &lang in Language::ALL {
        if let Err(err) =
            sce_build::compile_forge_file(&path, lang, &[], &ForgeCompileOptions::default())
        {
            panic!("{lang:?} refused the guarded increment the analysis proves: {err}");
        }
    }
}

/// Rust checks every integer operation at its declared width and returns
/// the value inside the `Result` the checks `?` out of; the same document
/// without the declaration emits the plain operator and a plain return.
#[test]
fn rust_checks_each_operation_and_returns_through_a_result() {
    let checked = generate(
        "probe_may_fail",
        &document(r#" may-fail="true""#),
        &[],
        Language::Rust,
    )
    .expect("Rust lowers may-fail");
    for needle in [
        "-> Result<u32, sce_forge_runtime::algorithm::AlgorithmError>",
        "sce_forge_runtime::algorithm::add::<u32>(prev, 1)?",
        "return Ok(",
    ] {
        assert!(checked.contains(needle), "missing `{needle}`:\n{checked}");
    }
    let plain = generate("probe_may_fail", &guarded_document(), &[], Language::Rust)
        .expect("a proven algorithm generates without the declaration");
    assert!(
        !plain.contains("sce_forge_runtime") && plain.contains("prev + 1"),
        "an undeclared algorithm is emitted as before:\n{plain}"
    );
}

/// An integer division that lands in a real is real division (SCE_FORGE.md
/// §3.4.1's table), so it is not an integer operation to check — while the
/// integer operation beside it still is.
#[test]
fn a_division_in_float_context_stays_real_division() {
    let text = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_float_ctx" version="1.0">
  <sce:signature>
    <sce:param name="a" type="int32"/>
    <sce:param name="b" type="int32"/>
    <sce:return type="int32" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="x" type="float64" init="a / b"/>
    <sce:return expr="a / b"/>
  </sce:body>
</scxml>
"#;
    let out = generate("probe_float_ctx", text, &[], Language::Rust).expect("Rust lowers it");
    assert_eq!(
        out.matches("sce_forge_runtime::algorithm::div::<i32>(a, b)?")
            .count(),
        1,
        "only the integer division is checked:\n{out}"
    );
    assert!(
        out.contains("a as f64 / b as f64"),
        "the real division stays real:\n{out}"
    );
}

/// No caller statement receives a failure yet, so another algorithm may not
/// call a `may-fail` one — the same host-only rule a list slot follows.
#[test]
fn another_algorithm_cannot_call_a_may_fail_algorithm() {
    let caller = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_caller" version="1.0">
  <sce:import kind="algorithm" src="probe_may_fail.scxml" as="tick"/>
  <sce:signature>
    <sce:param name="n" type="uint32"/>
    <sce:return type="uint32"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="tick(n)"/>
  </sce:body>
</scxml>
"#;
    let callee = document(r#" may-fail="true""#);
    let err = generate(
        "probe_caller",
        caller,
        &[("probe_may_fail.scxml", &callee)],
        Language::Rust,
    )
    .expect_err("a call to a may-fail algorithm is refused");
    assert!(
        err.contains("declares may-fail") && err.contains("only a host"),
        "{err}"
    );
}
