// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An algorithm holds a union (docs/adr/0092, Landing order step 5), held to
//! the documents under `sce-build/tests/fixtures/union/`:
//!
//! * `algorithm_shape_extent.scxml` takes a `union:Shape` and reads each case's
//!   fields where a test of the tag proves the case. Its three tests each
//!   return, so no case is left after them and its body needs no return there
//!   (section 6.1 item 4): Rust closes the body with `unreachable!`.
//! * `algorithm_shape_grown.scxml` returns a union: the one it was given, by
//!   name, or a union local it builds as the case its tag names.
//! * Both compile under `rustc -D warnings` beside the union's unit and the
//!   units it names.
//!
//! A body that leaves a case unreturned is refused with
//! `algorithm/return-missing`, a case's field read where no test proves its case
//! with `expression/union-read-unproven`, a union taken as a value, a write of a
//! union local and a tag that is no variant literal each by name; every backend
//! of `ALGORITHM_UNION_BACKENDS` generates the algorithm, and one not listed
//! refuses it by name.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::ALGORITHM_UNION_BACKENDS;
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located};
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// What `language` generates for the document at `path`.
fn generated(path: &Path, language: Language) -> Result<GeneratedOutput, Located<ForgeError>> {
    sce_build::compile_forge_file(path, language, &[], &ForgeCompileOptions::default())
}

/// The Rust unit of the fixture `name`, whose file is named for its stem, as
/// `(file, text)`.
fn unit(name: &str) -> (String, String) {
    let output = generated(&fixtures().join(name), Language::Rust)
        .unwrap_or_else(|e| panic!("{name} does not generate on Rust: {}", e.error));
    let stem = name.trim_end_matches(".scxml");
    output
        .files
        .into_iter()
        .find(|(file, _)| file.strip_suffix(".rs") == Some(stem))
        .unwrap_or_else(|| panic!("{name}: no Rust unit named for it"))
}

/// Whether `text` holds `needle`, with `text` in the failure.
fn holds(text: &str, needle: &str) {
    assert!(text.contains(needle), "the unit holds `{needle}`:\n{text}");
}

const SHAPE: &str = "union_shape_minimal::UnionShapeMinimal";

#[test]
fn an_algorithm_reads_each_case_of_its_union_where_its_tag_is_tested() {
    let (_, text) = unit("algorithm_shape_extent.scxml");
    holds(&text, "use super::union_shape_minimal;\n");
    holds(
        &text,
        &format!("pub fn algorithm_shape_extent(shape: {SHAPE}) -> u16"),
    );
    // The tag is read by its method and compared with the algorithm's own enum.
    holds(&text, "shape.kind()");
    holds(&text, "shape_kind::ShapeKind::Dot");
    holds(&text, "shape_kind::ShapeKind::Rect");
    holds(&text, "shape.proven_circle().radius");
    holds(&text, "shape.proven_rect().width");
    holds(&text, "shape.proven_rect().height");
    // Every case returned before the end of the body, which no path reaches.
    holds(
        &text,
        "unreachable!(\"algorithm_shape_extent: every case of its union returned before the \
         end of its body\")",
    );
}

#[test]
fn an_algorithm_returns_a_union_it_was_given_or_built() {
    let (_, text) = unit("algorithm_shape_grown.scxml");
    // The case's record is named by no expression, and is imported all the same.
    holds(&text, "use super::shape_radius_minimal;\n");
    holds(&text, &format!("radius: u16) -> {SHAPE}"));
    holds(&text, "return shape;");
    holds(
        &text,
        &format!(
            "let grown = {SHAPE}::Circle(shape_radius_minimal::ShapeRadiusMinimalPayload {{ \
             radius: radius }});"
        ),
    );
    holds(&text, "return grown;");
    // Its last statement returns, so the body needs nothing after it.
    assert!(!text.contains("unreachable!("), "{text}");
}

/// A module that calls both algorithms, the one's union handed to the other.
const PROBE: &str = r#"use super::algorithm_shape_extent::algorithm_shape_extent;
use super::algorithm_shape_grown::algorithm_shape_grown;
use super::union_shape_minimal::UnionShapeMinimal;

pub fn grown_extent(shape: UnionShapeMinimal) -> u16 {
    algorithm_shape_extent(algorithm_shape_grown(shape, 3))
}

pub fn dot() -> UnionShapeMinimal {
    UnionShapeMinimal::Dot
}
"#;

/// The algorithms beside a probe that calls each, compiled with the union's
/// unit and the units it names: the reads, the build, the returns and the
/// closed end are Rust the compiler accepts with every warning an error.
#[test]
fn the_rust_of_an_algorithm_over_a_union_compiles_under_deny_warnings() {
    let Some(rustc) = sce_build::toolchain::require_any_or_skip(
        &["rustc"],
        "compile a generated algorithm over a union",
    ) else {
        return;
    };
    let dir = tempfile::tempdir().expect("a scratch directory");
    let mut modules = String::new();
    for name in [
        "shape_kind.scxml",
        "shape_radius_minimal.scxml",
        "shape_rect_minimal.scxml",
        "union_shape_minimal.scxml",
        "algorithm_shape_extent.scxml",
        "algorithm_shape_grown.scxml",
    ] {
        let (file, text) = unit(name);
        std::fs::write(dir.path().join(&file), text).expect("write the unit");
        modules.push_str(&format!("pub mod {};\n", file.trim_end_matches(".rs")));
    }
    std::fs::write(dir.path().join("probe.rs"), PROBE).expect("write the probe");
    modules.push_str("pub mod probe;\n");
    let root = dir.path().join("lib.rs");
    std::fs::write(&root, modules).expect("write the crate root");
    let out = Command::new(&rustc)
        .args([
            "--edition=2021",
            "--crate-type=lib",
            "--emit=metadata",
            "-D",
            "warnings",
            "--out-dir",
        ])
        .arg(dir.path().join("out"))
        .arg(&root)
        .output()
        .expect("rustc runs");
    assert!(
        out.status.success(),
        "the generated Rust does not compile:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// An algorithm `probe_shape` written beside the shape fixtures, whose
/// signature is `signature` and whose body is `body`; what `language` makes of
/// it.
fn probe(
    signature: &str,
    body: &str,
    language: Language,
) -> Result<GeneratedOutput, Located<ForgeError>> {
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
    let path = dir.path().join("probe_shape.scxml");
    std::fs::write(&path, text).expect("write the probe");
    generated(&path, language)
}

/// The code of the first record the refusal of `result` carries.
fn refused_code(result: Result<GeneratedOutput, Located<ForgeError>>) -> (DiagnosticCode, String) {
    let err = match result {
        Ok(_) => panic!("the probe generated"),
        Err(refusal) => refusal,
    };
    let record = err.error.to_diagnostics().remove(0);
    (record.code, err.error.to_string())
}

const TAKES_A_SHAPE: &str = r#"    <sce:param name="shape" type="union:Shape"/>
    <sce:return type="uint16"/>"#;

/// A chain of tests that leaves `rect` unreturned reaches the end of the body,
/// which then needs a return (docs/adr/0092, section 6.1 item 4).
#[test]
fn a_body_that_leaves_a_case_unreturned_is_refused_with_return_missing() {
    let body = r#"    <sce:if cond="shape.kind === ShapeKind.dot">
      <sce:return expr="0"/>
    </sce:if>
    <sce:if cond="shape.kind === ShapeKind.circle">
      <sce:return expr="shape.radius"/>
    </sce:if>"#;
    let (code, why) = refused_code(probe(TAKES_A_SHAPE, body, Language::Rust));
    assert!(
        matches!(code, DiagnosticCode::AlgorithmReturnMissing),
        "{code:?}: {why}"
    );
}

/// A case's field read where no test of the tag proves a case that declares it.
#[test]
fn a_case_field_read_where_no_test_proves_its_case_is_refused() {
    let body = r#"    <sce:return expr="shape.radius"/>"#;
    let (code, why) = refused_code(probe(TAKES_A_SHAPE, body, Language::Rust));
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnionReadUnproven),
        "{code:?}: {why}"
    );
}

/// A union is read through its tag and its cases' fields: it is no value of a
/// number's place, a union local is not written, and the tag of one it builds
/// is a variant literal.
#[test]
fn a_union_is_taken_whole_only_by_a_return_of_its_type() {
    for body in [
        r#"    <sce:var name="copy" type="uint16" init="shape"/>
    <sce:return expr="copy"/>"#,
        r#"    <sce:var name="built" type="union:Shape">
      <sce:set name="kind" expr="ShapeKind.dot"/>
    </sce:var>
    <sce:assign target="built.radius" expr="1"/>
    <sce:return expr="0"/>"#,
        r#"    <sce:var name="built" type="union:Shape">
      <sce:set name="kind" expr="shape.kind"/>
    </sce:var>
    <sce:return expr="0"/>"#,
    ] {
        let (code, why) = refused_code(probe(TAKES_A_SHAPE, body, Language::Rust));
        assert!(
            matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
            "{code:?}: {why}"
        );
    }
}

/// Every backend either lowers an algorithm's union, listed in
/// `ALGORITHM_UNION_BACKENDS` and generating the algorithm, or refuses it by
/// the union's name (`generate/unsupported-feature`), after the body was
/// judged as it is on every backend. Every generated backend lowers one today,
/// so the refusing side holds a backend added to `Language::ALL` before its
/// lowering lands. Go is handed the module path its imports are under, which
/// every other backend ignores.
#[test]
fn a_backend_that_lowers_no_union_in_an_algorithm_refuses_one_by_name() {
    let path = fixtures().join("algorithm_shape_extent.scxml");
    let options = ForgeCompileOptions {
        go_module_prefix: Some("example.com/generated".to_string()),
        ..ForgeCompileOptions::default()
    };
    for language in Language::ALL.iter().copied() {
        let result = sce_build::compile_forge_file(&path, language, &[], &options);
        if ALGORITHM_UNION_BACKENDS.contains(&language) {
            if let Err(refusal) = result {
                panic!(
                    "{language:?} lowers a union in an algorithm and refused one: {}",
                    refusal.error
                );
            }
            continue;
        }
        let (code, why) = refused_code(result);
        assert!(
            matches!(code, DiagnosticCode::GenerateUnsupportedFeature),
            "{language:?}: {code:?}: {why}"
        );
        assert!(
            why.contains("union:Shape") || why.contains("<sce:import as=\"Shape\">"),
            "{language:?}: {why}"
        );
    }
}
