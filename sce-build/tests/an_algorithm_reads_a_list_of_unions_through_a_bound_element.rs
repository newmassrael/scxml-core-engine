// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An algorithm holds a list of unions (docs/adr/0099, "Still open" item 2):
//! a `list<union:U>` parameter, local and return, read by `len`, walked by a
//! `<sce:foreach>` whose item is a union a test of its tag narrows, and read at
//! an index only whole, by a union local copied from the element
//! (`<sce:var type="union:U" init="xs[i]"/>`, section 5.2). What is judged
//! before any backend is held here, on Rust, the backend every judge runs ahead
//! of; each backend's lowering is held by its own
//! `a_<backend>_algorithm_narrows_a_union_by_its_tag` test.
//!
//! * An element at an index is no narrowing subject (docs/adr/0092, section 6.1
//!   item 2): its tag and its fields are read nowhere inside an expression.
//! * A union local is copied from an element of a list of its own union, and
//!   only in a body that may fail, since the element is read by the checked
//!   read of its index.
//! * A union local takes its `init` or its `<sce:set>`s, never both.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located};
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// An algorithm `probe_shapes` written beside the shape fixtures, whose
/// signature is `signature` and whose body is `body`; what Rust makes of it.
fn probe(signature: &str, body: &str) -> Result<GeneratedOutput, Located<ForgeError>> {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for fixture in [
        "shape_kind.scxml",
        "shape_radius_minimal.scxml",
        "shape_rect_minimal.scxml",
        "union_shape_minimal.scxml",
    ] {
        std::fs::copy(fixtures().join(fixture), dir.path().join(fixture)).expect("copy a fixture");
    }
    let text = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_shapes" version="1.0">
  <sce:import kind="enum" src="shape_kind.scxml" as="ShapeKind"/>
  <sce:import kind="union" src="union_shape_minimal.scxml" as="Shape"/>
  <sce:signature>
{signature}
  </sce:signature>
  <sce:body>
{body}
  </sce:body>
</scxml>
"#
    );
    let path = dir.path().join("probe_shapes.scxml");
    std::fs::write(&path, text).expect("write the probe");
    sce_build::compile_forge_file(&path, Language::Rust, &[], &ForgeCompileOptions::default())
}

/// The code of the first record the refusal of `result` carries, and its text.
fn refused_code(result: Result<GeneratedOutput, Located<ForgeError>>) -> (DiagnosticCode, String) {
    let err = match result {
        Ok(_) => panic!("the probe generated"),
        Err(refusal) => refusal,
    };
    let record = err.error.to_diagnostics().remove(0);
    (record.code, err.error.to_string())
}

const TAKES_SHAPES: &str = r#"    <sce:param name="shapes" type="list&lt;union:Shape&gt;"/>
    <sce:param name="at" type="uint32"/>
    <sce:return type="uint16" may-fail="true"/>"#;

/// The body of `algorithm_shape_at.scxml` past its copy: every case returned.
const EVERY_CASE_OF_PICKED: &str = r#"    <sce:if cond="picked.kind === ShapeKind.dot">
      <sce:return expr="0"/>
    </sce:if>
    <sce:if cond="picked.kind === ShapeKind.circle">
      <sce:return expr="picked.radius"/>
    </sce:if>
    <sce:if cond="picked.kind === ShapeKind.rect">
      <sce:return expr="picked.width"/>
    </sce:if>"#;

/// The three fixtures of a list of unions are judged and generated on Rust: the
/// copy from an index, the loop whose item is narrowed, and the list built and
/// returned.
#[test]
fn a_list_of_unions_is_copied_from_walked_and_built_by_an_algorithm() {
    for name in [
        "algorithm_shape_at.scxml",
        "algorithm_shapes_widest.scxml",
        "algorithm_shapes_kept.scxml",
    ] {
        let path = fixtures().join(name);
        if let Err(refusal) = sce_build::compile_forge_file(
            &path,
            Language::Rust,
            &[],
            &ForgeCompileOptions::default(),
        ) {
            panic!("{name} is refused: {}", refusal.error);
        }
    }
}

/// The tag of an element at an index is no subject a test narrows, and a field
/// of one is read nowhere: each is refused at the element, naming the repair.
#[test]
fn an_element_of_a_list_of_unions_is_read_inside_no_expression() {
    for cond in [
        "shapes[at].kind === ShapeKind.dot",
        "shapes[at].radius &gt; 0",
    ] {
        let body = format!(
            r#"    <sce:if cond="{cond}">
      <sce:return expr="1"/>
    </sce:if>
    <sce:return expr="0"/>"#
        );
        let (code, why) = refused_code(probe(TAKES_SHAPES, &body));
        assert!(
            matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
            "{cond}: {code:?}: {why}"
        );
        assert!(
            why.contains("copy it whole into a union local"),
            "{cond}: {why}"
        );
    }
}

/// A union local copies an element of a list of its union, read at an index;
/// a number is no such element.
#[test]
fn a_union_local_copies_only_an_element_of_a_list_of_its_union() {
    let body = format!(
        "    <sce:var name=\"picked\" type=\"union:Shape\" init=\"at\"/>\n{EVERY_CASE_OF_PICKED}"
    );
    let (code, why) = refused_code(probe(TAKES_SHAPES, &body));
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(why.contains("copied whole from an element"), "{why}");
}

/// The element is read by the checked read of its index, whose `out-of-range`
/// only a body that may fail can hand on.
#[test]
fn a_union_local_is_copied_from_an_element_only_where_the_body_may_fail() {
    let signature = r#"    <sce:param name="shapes" type="list&lt;union:Shape&gt;"/>
    <sce:param name="at" type="uint32"/>
    <sce:return type="uint16"/>"#;
    let body = format!(
        "    <sce:var name=\"picked\" type=\"union:Shape\" init=\"shapes[at]\"/>\n{EVERY_CASE_OF_PICKED}"
    );
    let (code, why) = refused_code(probe(signature, &body));
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(why.contains("may-fail"), "{why}");
}

/// A union local is copied or built, never both.
#[test]
fn a_union_local_takes_its_init_or_its_sets_and_not_both() {
    let body = format!(
        r#"    <sce:var name="picked" type="union:Shape" init="shapes[at]">
      <sce:set name="kind" expr="ShapeKind.dot"/>
    </sce:var>
{EVERY_CASE_OF_PICKED}"#
    );
    let (code, why) = refused_code(probe(TAKES_SHAPES, &body));
    assert!(
        matches!(code, DiagnosticCode::ValidationAttributeRuleViolated),
        "{code:?}: {why}"
    );
}

/// A list of unions takes, by `<sce:append>`, a union of its own document by
/// the name that holds it, and nothing else.
#[test]
fn a_list_of_unions_is_appended_a_union_of_its_document_by_name() {
    let signature = r#"    <sce:param name="shapes" type="list&lt;union:Shape&gt;"/>
    <sce:param name="at" type="uint32"/>
    <sce:return type="list&lt;union:Shape&gt;[4]" may-fail="true"/>"#;
    let body = r#"    <sce:var name="kept" type="list&lt;union:Shape&gt;[4]"/>
    <sce:append target="kept" expr="at"/>
    <sce:return expr="kept"/>"#;
    let (code, why) = refused_code(probe(signature, body));
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(why.contains("appended by the name that holds it"), "{why}");
}
