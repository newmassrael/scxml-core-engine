// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Interpreter's lowering of an algorithm over a union (docs/adr/0092,
//! section 7, the Interpreter row, and Landing order step 5), held to the
//! documents under `sce-build/tests/fixtures/union/` that the Rust, Kotlin, Go
//! and C++ lanes are held to (`a_rust_algorithm_narrows_a_union_by_its_tag.rs`):
//!
//! * `algorithm_shape_extent.scxml` takes a `union:Shape`, the plain object of
//!   its wire form, and reads its tag and each case's fields where a test of
//!   the tag proves the case, as the object's own members. Its three tests each
//!   return, so no case is left after them (section 6.1 item 4): the function
//!   is closed there by a throw that says the point is not reached.
//! * `algorithm_shape_grown.scxml` returns a union: the one it was given, by
//!   name, or a union local it builds as the object of the case its tag names.
//! * A statechart calls an algorithm whose union is a local of its own, and the
//!   function travels in the lowered statechart.
//! * The judge every backend's route runs refuses a body that leaves a case
//!   unreturned and a field read where no test proves its case, with its own
//!   diagnostic, located in the algorithm's document.
//!
//! What the functions do when the script engine runs them is held by
//! `tests/integration/AnAlgorithmRunsLoweredUnderTheInterpreterTest.cpp`
//! (`AnAlgorithmRunsLoweredUnderTheInterpreter.AUnionIsReadByItsTagAndBuiltAsItsCase`).

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located};
use sce_build::forge::static_js::{lower_algorithm_document, lower_file};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// What installs the fixture `name` and the algorithms it imports, lowered on
/// its own as the Interpreter's check of the numerical cases lowers one.
fn installed(name: &str) -> String {
    let path = fixtures().join(name);
    lower_algorithm_document(path.to_str().expect("a utf-8 path"))
        .unwrap_or_else(|e| panic!("{name} does not lower for the Interpreter: {e:?}"))
        .install
}

/// Whether `text` holds `needle`, with `text` in the failure.
fn holds(text: &str, needle: &str) {
    assert!(
        text.contains(needle),
        "the lowering holds `{needle}`:\n{text}"
    );
}

/// Pinned by `holds` and the end `lower` closes (`ends_unreached`,
/// `unreached_end_reason`) in `sce-build/src/forge/static_js_algorithm.rs`.
#[test]
fn an_interpreter_algorithm_reads_each_case_of_its_union_where_its_tag_is_tested() {
    let text = installed("algorithm_shape_extent.scxml");
    holds(
        &text,
        "SceStatic.algorithms.algorithm_shape_extent = function (shape) { ",
    );
    // The tag is the declared name of the case's variant, as a value of the
    // tag's enum is.
    holds(&text, "if (shape.kind === 'dot') { return 0; }");
    holds(
        &text,
        "if (shape.kind === 'circle') { return shape.radius; }",
    );
    holds(&text, "shape.width");
    holds(&text, "shape.height");
    // Every case returned before the end of the body, which no path reaches.
    holds(
        &text,
        "throw new Error('algorithm_shape_extent: every case of its union returned before the \
         end of its body'); };",
    );
}

/// Pinned by the `UnionVar` arm of `Body::statement` and the union names
/// `lower` returns whole (`sce-build/src/forge/static_js_algorithm.rs`).
#[test]
fn an_interpreter_algorithm_returns_a_union_it_was_given_or_built() {
    let text = installed("algorithm_shape_grown.scxml");
    holds(
        &text,
        "SceStatic.algorithms.algorithm_shape_grown = function (shape, radius) { ",
    );
    holds(&text, "if (shape.kind !== 'dot') { return shape; }");
    // The case's object: the tag beside the fields of its record.
    holds(&text, "var grown = {'kind': 'circle', 'radius': radius};");
    holds(&text, "return grown;");
    // Its last statement returns, so the body needs nothing after it.
    assert!(!text.contains("throw new Error("), "{text}");
}

/// The shape fixtures copied into a scratch directory, beside an algorithm
/// `probe_shape` whose signature is `signature` and whose body is `body`, and
/// beside it the statechart `statechart_probe_shape`, which calls it with
/// `uint16` 3 where its signature takes one. Answers the directory.
fn probe(signature: &str, body: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for fixture in [
        "shape_kind.scxml",
        "shape_radius_minimal.scxml",
        "shape_rect_minimal.scxml",
        "union_shape_minimal.scxml",
    ] {
        std::fs::copy(fixtures().join(fixture), dir.path().join(fixture)).expect("copy a fixture");
    }
    let algorithm = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_shape" version="1.0">
  <sce:import kind="enum" src="shape_kind.scxml" as="ShapeKind"/>
  <sce:import kind="event-schema" src="shape_radius_minimal.scxml" as="Radius"/>
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
    std::fs::write(dir.path().join("probe_shape.scxml"), algorithm).expect("write the probe");
    let statechart = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle" datamodel="sce-static" name="statechart_probe_shape">
  <sce:import kind="algorithm" src="probe_shape.scxml" as="Probe"/>
  <datamodel>
    <data id="extent" sce:type="uint16" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="grow" type="internal">
      <assign location="extent" expr="Probe(3)"/>
    </transition>
  </state>
</scxml>
"#;
    std::fs::write(dir.path().join("statechart_probe_shape.scxml"), statechart)
        .expect("write the statechart");
    dir
}

const TAKES_A_RADIUS: &str = r#"    <sce:param name="radius" type="uint16"/>
    <sce:return type="uint16"/>"#;

const TAKES_A_SHAPE: &str = r#"    <sce:param name="shape" type="union:Shape"/>
    <sce:return type="uint16"/>"#;

/// A union local that a test of its own tag then reads: built as a circle, and
/// read as one where the test proves it.
const BUILDS_A_CIRCLE: &str = r#"    <sce:var name="built" type="union:Shape">
      <sce:set name="kind" expr="ShapeKind.circle"/>
      <sce:set name="radius" expr="radius"/>
    </sce:var>
    <sce:if cond="built.kind === ShapeKind.circle">
      <sce:return expr="built.radius"/>
    </sce:if>"#;

/// The code of the first record the refusal carries, the file it is located
/// in, and its text.
fn refused(refusal: Located<ForgeError>) -> (DiagnosticCode, String, String) {
    let record = refusal.error.to_diagnostics().remove(0);
    (
        record.code,
        refusal.location.file.clone(),
        refusal.error.to_string(),
    )
}

/// A statechart's call of an algorithm whose union is a local of its own
/// crosses no union: the call is lowered as any other, and the function the
/// statechart installs builds the local and reads it where its tag is tested.
/// Pinned by `holds` and the `UnionVar` arm of `Body::statement`
/// (`sce-build/src/forge/static_js_algorithm.rs`).
#[test]
fn a_statechart_calls_an_algorithm_that_holds_a_union_of_its_own() {
    let body = format!("{BUILDS_A_CIRCLE}\n    <sce:return expr=\"0\"/>");
    let dir = probe(TAKES_A_RADIUS, &body);
    let path = dir.path().join("statechart_probe_shape.scxml");
    let out = lower_file(path.to_str().expect("a utf-8 path"), Vec::new())
        .unwrap_or_else(|e| panic!("the statechart does not lower: {e:?}"));
    let document = roxmltree::Document::parse(&out).expect("well-formed");
    let installer = document
        .descendants()
        .find(|n| n.attribute("id") == Some("SceStaticInstalled"))
        .and_then(|n| n.attribute("expr"))
        .unwrap_or_else(|| panic!("the library is installed:\n{out}"));
    holds(
        installer,
        "SceStatic.algorithms.probe_shape = function (radius) { var built = {'kind': \
         'circle', 'radius': radius}; if (built.kind === 'circle') { return built.radius; } \
         return 0; };",
    );
}

/// A chain of tests that leaves a case unreturned reaches the end of the body,
/// which then needs a return (docs/adr/0092, section 6.1 item 4): the judge
/// refuses it before the lowering reads it, in the algorithm's own document,
/// whether the algorithm is lowered on its own or for the statechart that
/// calls it. Pinned by `judge` (`sce-build/src/forge/static_js_algorithm.rs`)
/// and its call in `algorithm_functions` (`sce-build/src/forge/static_js.rs`).
#[test]
fn a_body_that_leaves_a_case_unreturned_is_refused_with_return_missing() {
    let dir = probe(TAKES_A_RADIUS, BUILDS_A_CIRCLE);
    let algorithm = dir.path().join("probe_shape.scxml");
    let (code, file, why) = refused(
        lower_algorithm_document(algorithm.to_str().expect("a utf-8 path"))
            .expect_err("a case is left unreturned"),
    );
    assert!(
        matches!(code, DiagnosticCode::AlgorithmReturnMissing),
        "{code:?}: {why}"
    );
    assert!(file.ends_with("probe_shape.scxml"), "{file}: {why}");

    let statechart = dir.path().join("statechart_probe_shape.scxml");
    let (code, file, why) = refused(
        lower_file(statechart.to_str().expect("a utf-8 path"), Vec::new())
            .expect_err("the algorithm the statechart calls leaves a case unreturned"),
    );
    assert!(
        matches!(code, DiagnosticCode::AlgorithmReturnMissing),
        "{code:?}: {why}"
    );
    // The fault is the algorithm's, and it is located there, not in the
    // statechart that calls it.
    assert!(
        file.ends_with("probe_shape.scxml") && !file.ends_with("statechart_probe_shape.scxml"),
        "{file}: {why}"
    );
}

/// A case's field read where no test of the tag proves a case that declares
/// it is refused by the judge, as on every backend.
#[test]
fn a_case_field_read_where_no_test_proves_its_case_is_refused() {
    let dir = probe(TAKES_A_SHAPE, r#"    <sce:return expr="shape.radius"/>"#);
    let algorithm = dir.path().join("probe_shape.scxml");
    let (code, _, why) = refused(
        lower_algorithm_document(algorithm.to_str().expect("a utf-8 path"))
            .expect_err("the read is unproven"),
    );
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnionReadUnproven),
        "{code:?}: {why}"
    );
}
