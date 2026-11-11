// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An algorithm hands a union it holds whole, by the name of the parameter or
//! the local that holds it, to another algorithm's parameter of the same union
//! document (docs/adr/0092, "Decided while handing a union from one algorithm
//! to another"), held to the documents under `sce-build/tests/fixtures/union/`
//! on every generated backend and the Interpreter:
//!
//! * `algorithm_shape_extent_at_least_one.scxml` hands the `union:Shape` it
//!   takes to `algorithm_shape_extent.scxml` and reads its own tag after the
//!   call. Every backend passes the union by value, the type the union's unit
//!   declares on both sides: Rust hands a clone, since its union is no `Copy`
//!   type and the caller reads its own afterwards, and every other backend
//!   hands the name as it is. The Rust compiles under `rustc -D warnings`.
//! * `algorithm_date_handed_as_shape.scxml` hands a `sce:std/time` date
//!   expression to the same parameter, which takes the shape union: refused
//!   by the judge every backend's route runs, naming both documents.
//! * An argument that is no name of a union (a number, a call that answers a
//!   union) is refused by the same rule, under either call form, and an
//!   algorithm that returns a union stays callable by a host alone, refused
//!   naming its return.
//!
//! What was wrong: an algorithm whose parameter held a union was called by a
//! host alone, so a consumer's algorithm that resolves a date expression it
//! takes (`sce:std/time/resolve_date`) was refused on every backend.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::ALGORITHM_UNION_BACKENDS;
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located};
use sce_build::forge::static_js::lower_algorithm_document;
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// The options every backend is asked with: Go is handed the module path its
/// imports are under, which every other backend ignores.
fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some("example.com/generated".to_string()),
        ..ForgeCompileOptions::default()
    }
}

/// What `language` generates for the document at `path`.
fn generated(path: &Path, language: Language) -> Result<GeneratedOutput, Located<ForgeError>> {
    sce_build::compile_forge_file(path, language, &[], &options())
}

/// Every file `language` generates for the document at `path`, joined.
fn text_of(path: &Path, language: Language) -> String {
    let output = generated(path, language).unwrap_or_else(|e| {
        panic!(
            "{} does not generate on {language:?}: {}",
            path.display(),
            e.error
        )
    });
    output
        .files
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The argument `language` hands a union held as `name` in: a clone on Rust,
/// the name on every other backend (`handed_union`,
/// `sce-build/src/forge/expr.rs`), each between the call's parentheses.
fn handed(language: Language, name: &str) -> String {
    match language {
        Language::Rust => format!("({name}.clone())"),
        Language::Cpp
        | Language::Kotlin
        | Language::Go
        | Language::Python
        | Language::C11
        | Language::Swift => format!("({name})"),
    }
}

/// The code and the text of the refusal `result` carries.
fn refused(result: Result<GeneratedOutput, Located<ForgeError>>) -> (DiagnosticCode, String) {
    let err = match result {
        Ok(_) => panic!("the document generated"),
        Err(refusal) => refusal,
    };
    let record = err.error.to_diagnostics().remove(0);
    (record.code, err.error.to_string())
}

/// Pinned by the union slot of `algorithm_record_slots` (`sce-build/src/lib.rs`)
/// and `handed_union` (`sce-build/src/forge/expr.rs`): with the slot mapped to
/// no union, the callee is refused as called by a host alone on every backend;
/// with the clone left out, the Rust arm spells the bare name.
#[test]
fn a_union_parameter_is_handed_to_another_algorithm_on_every_backend() {
    let path = fixtures().join("algorithm_shape_extent_at_least_one.scxml");
    for language in Language::ALL.iter().copied() {
        if !ALGORITHM_UNION_BACKENDS.contains(&language) {
            let (code, why) = refused(generated(&path, language));
            assert!(
                matches!(code, DiagnosticCode::GenerateUnsupportedFeature),
                "{language:?}: {code:?}: {why}"
            );
            continue;
        }
        let text = text_of(&path, language);
        let argument = handed(language, "shape");
        assert!(
            text.contains(&argument),
            "{language:?} hands the union as `{argument}`:\n{text}"
        );
        if language != Language::Rust {
            assert!(
                !text.contains("shape.clone()"),
                "{language:?} hands the union by value as it is:\n{text}"
            );
        }
    }
}

/// Pinned by `lower_union_arguments` and the Interpreter's call of a callable
/// algorithm (`sce-build/src/forge/static_js_algorithm.rs`): the union is the
/// wire's own object, handed as it is.
#[test]
fn the_interpreter_hands_a_union_parameter_as_it_is() {
    let path = fixtures().join("algorithm_shape_extent_at_least_one.scxml");
    let install = lower_algorithm_document(path.to_str().expect("a utf-8 path"))
        .unwrap_or_else(|e| panic!("the caller does not lower for the Interpreter: {e:?}"))
        .install;
    assert!(
        install.contains("SceStatic.algorithms.algorithm_shape_extent(shape)"),
        "the Interpreter hands the union by its name:\n{install}"
    );
}

/// The Rust unit of the fixture `name`, whose file is named for its stem, as
/// `(file, text)`.
fn rust_unit(name: &str) -> (String, String) {
    let output = generated(&fixtures().join(name), Language::Rust)
        .unwrap_or_else(|e| panic!("{name} does not generate on Rust: {}", e.error));
    let stem = name.trim_end_matches(".scxml");
    output
        .files
        .into_iter()
        .find(|(file, _)| file.strip_suffix(".rs") == Some(stem))
        .unwrap_or_else(|| panic!("{name}: no Rust unit named for it"))
}

/// The caller beside the callee, the union's unit and the units it names,
/// compiled with every warning an error: the caller reads its own union after
/// handing it, which compiles only because it handed a clone. Pinned by the
/// Rust arm of `handed_union` (`sce-build/src/forge/expr.rs`): the bare name
/// moves the union, and the read after it is a use of a moved value.
#[test]
fn the_rust_of_a_union_handed_on_compiles_under_deny_warnings() {
    let Some(rustc) = sce_build::toolchain::require_any_or_skip(
        &["rustc"],
        "compile a generated algorithm that hands a union on",
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
        "algorithm_shape_extent_at_least_one.scxml",
    ] {
        let (file, text) = rust_unit(name);
        std::fs::write(dir.path().join(&file), text).expect("write the unit");
        modules.push_str(&format!("pub mod {};\n", file.trim_end_matches(".rs")));
    }
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

/// Pinned by the identity test of `judge_union_argument`
/// (`sce-build/src/forge/expr.rs`), which the judge every backend's route runs
/// reads (`judge_narrowed_use`): a union of another document stands in the
/// parameter only if the identity is not compared.
#[test]
fn a_union_of_another_document_is_refused_where_one_is_handed() {
    let path = fixtures().join("algorithm_date_handed_as_shape.scxml");
    for &language in ALGORITHM_UNION_BACKENDS {
        let (code, why) = refused(generated(&path, language));
        assert!(
            matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
            "{language:?}: {code:?}: {why}"
        );
        for piece in ["`Extent`", "`date_expression`", "`union_shape_minimal`"] {
            assert!(why.contains(piece), "{language:?} names {piece}: {why}");
        }
    }
    let refusal = lower_algorithm_document(path.to_str().expect("a utf-8 path"))
        .expect_err("the Interpreter's route runs the same judge");
    let record = refusal.error.to_diagnostics().remove(0);
    assert!(
        matches!(record.code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{:?}: {}",
        record.code,
        refusal.error
    );
}

/// The shape fixtures and the two algorithms over them copied into a scratch
/// directory, beside an algorithm `probe_shape` that takes a `union:Shape` and
/// returns a `uint16`, whose body is `body`; what `language` makes of it.
fn probe(body: &str, language: Language) -> Result<GeneratedOutput, Located<ForgeError>> {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for fixture in [
        "shape_kind.scxml",
        "shape_radius_minimal.scxml",
        "shape_rect_minimal.scxml",
        "union_shape_minimal.scxml",
        "algorithm_shape_extent.scxml",
        "algorithm_shape_grown.scxml",
    ] {
        std::fs::copy(fixtures().join(fixture), dir.path().join(fixture)).expect("copy a fixture");
    }
    let text = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_shape" version="1.0">
  <sce:import kind="enum" src="shape_kind.scxml" as="ShapeKind"/>
  <sce:import kind="union" src="union_shape_minimal.scxml" as="Shape"/>
  <sce:import kind="algorithm" src="algorithm_shape_extent.scxml" as="Extent"/>
  <sce:import kind="algorithm" src="algorithm_shape_grown.scxml" as="Grown"/>
  <sce:signature>
    <sce:param name="shape" type="union:Shape"/>
    <sce:return type="uint16"/>
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

/// Every file of a probe that generated, joined.
fn probe_text(body: &str, language: Language) -> String {
    let output = probe(body, language).unwrap_or_else(|e| panic!("{language:?}: {}", e.error));
    output
        .files
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A union local is handed as a parameter is. Pinned by `lower_union_arguments`
/// (`sce-build/src/forge/expr.rs`), which spells a local by the name its
/// declaration takes.
#[test]
fn a_union_local_is_handed_as_a_parameter_is() {
    let body = r#"    <sce:var name="dot" type="union:Shape">
      <sce:set name="kind" expr="ShapeKind.dot"/>
    </sce:var>
    <sce:return expr="Extent(dot)"/>"#;
    for &language in ALGORITHM_UNION_BACKENDS {
        let text = probe_text(body, language);
        let argument = handed(language, "dot");
        assert!(
            text.contains(&argument),
            "{language:?} hands the local as `{argument}`:\n{text}"
        );
    }
}

/// `<sce:call>` hands a union as the expression form does. Pinned by
/// `transpile_union_argument` (`sce-build/src/forge/expr.rs`) and its call in
/// the lowering of `<sce:call>` (`sce-build/src/forge/generator.rs`).
#[test]
fn a_call_statement_hands_a_union_as_an_expression_does() {
    let body = r#"    <sce:call target="Extent" args="shape"/>
    <sce:return expr="0"/>"#;
    let text = probe_text(body, Language::Rust);
    assert!(
        text.contains("(shape.clone());"),
        "the statement hands a clone:\n{text}"
    );
}

/// What is no name of a union is refused in a parameter that takes one, under
/// either form, naming the document the parameter takes. Pinned by the
/// unnamed arm of `judge_union_argument` (`sce-build/src/forge/expr.rs`) and
/// its call for `<sce:call>` in the judge (`sce-build/src/forge/narrowing.rs`).
#[test]
fn what_is_no_name_of_a_union_is_refused_in_a_union_parameter() {
    for body in [
        r#"    <sce:return expr="Extent(3)"/>"#,
        r#"    <sce:return expr="Extent(Grown(shape, 3))"/>"#,
        r#"    <sce:call target="Extent" args="3"/>
    <sce:return expr="0"/>"#,
    ] {
        let (code, why) = refused(probe(body, Language::Rust));
        assert!(
            matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
            "{body}: {code:?}: {why}"
        );
        assert!(
            why.contains("`union_shape_minimal`") && why.contains("by the name"),
            "{body}: {why}"
        );
    }
}

/// An algorithm that returns a union is called by a host alone: no place of an
/// algorithm takes whole a union a call answers. The refusal names the return,
/// not the union parameter, which another algorithm fills. Pinned by the
/// return arm of `algorithm_record_slots` and `call_refusal`
/// (`sce-build/src/lib.rs`).
#[test]
fn an_algorithm_that_returns_a_union_is_refused_naming_its_return() {
    let body = r#"    <sce:call target="Grown" args="shape, 3"/>
    <sce:return expr="0"/>"#;
    let (code, why) = refused(probe(body, Language::Rust));
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(
        why.contains("returns union:Shape") && !why.contains("takes union:Shape"),
        "{why}"
    );
}
