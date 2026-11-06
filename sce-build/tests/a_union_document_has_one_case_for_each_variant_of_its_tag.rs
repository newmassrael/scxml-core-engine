// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The `union` kind (docs/adr/0092, section 1.1): what a document of it reads
//! as, each refusal of its tag and of its cases and where it is placed, and that
//! every backend refuses a valid union by name until one lowers it.
//!
//! The rules a union states alone are the parser's and are judged on inline
//! documents; the rules that read the enum and the records it imports are judged
//! on the documents under `sce-build/tests/fixtures/union/`, compiled as an author
//! compiles one.

use std::path::{Path, PathBuf};

use sce_build::forge::codegen_matrix::{self, EmitOutcome, KindClass};
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located, ValidationError};
use sce_build::forge::model::{ForgeDocument, ForgeKind, ParsedForge, RuntimeDep};
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// A union document whose imports and datamodel are `body`. The first line of
/// `body` is line 5 of the document.
fn document(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="union" name="shape">
{body}
</scxml>"#
    )
}

const LABEL: DocumentLabel<'static> = DocumentLabel {
    identifier: "shape",
    diagnostic_label: "shape.scxml",
};

/// The imports every inline document below names its tag and records by.
const IMPORTS: &str = r#"  <sce:import kind="enum" src="shape_kind.scxml" as="ShapeKind"/>
  <sce:import kind="event-schema" src="shape_radius.scxml" as="Radius"/>
  <sce:import kind="event-schema" src="shape_rect.scxml" as="Rect"/>"#;

/// `IMPORTS`, then a datamodel of `datamodel`. The datamodel's first line is
/// line 8 of the document.
fn with_imports(datamodel: &str) -> String {
    document(&format!("{IMPORTS}\n{datamodel}"))
}

fn parse(text: &str) -> ParsedForge {
    parse_forge_with_imports(text, LABEL)
        .unwrap_or_else(|e| panic!("a valid union parses: {e:?}"))
        .expect("a union is a forge document")
}

/// The validation refusal of `text`, its wire code, and the line it is placed
/// on.
fn refusal(text: &str) -> (ValidationError, DiagnosticCode, Option<u32>) {
    let err = parse_forge_with_imports(text, LABEL).expect_err("the union is refused");
    validation(err)
}

fn validation(err: Located<ForgeError>) -> (ValidationError, DiagnosticCode, Option<u32>) {
    let code = err.error.to_diagnostics()[0].code;
    let line = err.location.line;
    match err.error {
        ForgeError::Validation(v) => (*v, code, line),
        other => panic!("expected a validation error, got {other:?}"),
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/union")
        .join(name)
}

/// What every backend needs to compile a document that imports: Go writes its
/// imports under a module path.
fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some("example.com/generated".to_string()),
        ..Default::default()
    }
}

/// The refusal `language` gives the fixture `name`.
fn compiled(name: &str, language: Language) -> Located<ForgeError> {
    match sce_build::compile_forge_file(&fixture(name), language, &[], &options()) {
        Ok(_) => panic!("{name} generated on {language:?}"),
        Err(refusal) => refusal,
    }
}

#[test]
fn a_union_reads_as_its_tag_and_its_cases_in_document_order() {
    let text = std::fs::read_to_string(fixture("union_shape.scxml")).expect("the fixture reads");
    let parsed = parse(&text);
    let ForgeDocument::Union(union) = &parsed.document else {
        panic!("a union document, got {:?}", parsed.document.kind());
    };
    assert_eq!(union.name, "shape");
    assert_eq!(union.tag, "kind");
    assert_eq!(union.tag_enum, "ShapeKind");
    let cases: Vec<(&str, Option<&str>)> = union
        .cases
        .iter()
        .map(|case| (case.variant.as_str(), case.record.as_deref()))
        .collect();
    assert_eq!(
        cases,
        [
            ("rect", Some("Rect")),
            ("dot", None),
            ("circle", Some("Radius")),
        ]
    );
    // The AST names a case's record, and an empty case has none.
    let json = serde_json::to_value(&parsed.document).expect("a document serialises");
    assert_eq!(json["kind"], "union");
    assert_eq!(json["cases"][0]["record"], "Rect");
    assert!(json["cases"][1].get("record").is_none(), "{json}");
}

#[test]
fn a_union_of_no_tag_or_of_two_is_refused() {
    let (error, code, line) = refusal(&with_imports("  <datamodel/>"));
    assert!(
        matches!(code, DiagnosticCode::ValidationAttributeRuleViolated),
        "{code:?}"
    );
    assert!(
        matches!(&error, ValidationError::AttributeRuleViolated { attr, .. } if attr == "<data>"),
        "{error:?}"
    );
    assert_eq!(line, Some(8));

    let (error, code, line) = refusal(&with_imports(
        r#"  <datamodel>
    <data id="kind" sce:type="enum:ShapeKind">
      <sce:case variant="circle" record="Radius"/>
    </data>
    <data id="other" sce:type="enum:ShapeKind"/>
  </datamodel>"#,
    ));
    assert!(
        matches!(code, DiagnosticCode::ValidationAttributeRuleViolated),
        "{code:?}"
    );
    assert!(
        matches!(&error, ValidationError::AttributeRuleViolated { value, .. } if value == "other"),
        "{error:?}"
    );
    // At the second `<data>`.
    assert_eq!(line, Some(12));
}

#[test]
fn a_tag_that_is_no_value_of_an_enum_is_refused() {
    let (error, code, _) = refusal(&with_imports(
        r#"  <datamodel>
    <data id="kind" sce:type="uint8">
      <sce:case variant="circle" record="Radius"/>
    </data>
  </datamodel>"#,
    ));
    assert!(
        matches!(code, DiagnosticCode::ValidationAttributeRuleViolated),
        "{code:?}"
    );
    assert!(
        matches!(
            &error,
            ValidationError::AttributeRuleViolated { attr, value, .. }
                if attr == "sce:type" && value == "uint8"
        ),
        "{error:?}"
    );
}

#[test]
fn a_record_no_import_names_is_refused_with_the_records_there_are() {
    let (error, code, line) = refusal(&with_imports(
        r#"  <datamodel>
    <data id="kind" sce:type="enum:ShapeKind">
      <sce:case variant="circle" record="Circle"/>
    </data>
  </datamodel>"#,
    ));
    assert!(
        matches!(code, DiagnosticCode::ValidationInvalidAttribute),
        "{code:?}"
    );
    match &error {
        ValidationError::InvalidAttribute {
            attr,
            value,
            allowed,
            ..
        } => {
            assert_eq!(attr, "record");
            assert_eq!(value, "Circle");
            assert_eq!(allowed, &["Radius", "Rect"]);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(line, Some(10));
}

#[test]
fn a_variant_given_two_cases_is_refused_at_its_second_case() {
    let (error, code, line) = refusal(&with_imports(
        r#"  <datamodel>
    <data id="kind" sce:type="enum:ShapeKind">
      <sce:case variant="circle" record="Radius"/>
      <sce:case variant="rect" record="Rect"/>
      <sce:case variant="circle" record="Rect"/>
    </data>
  </datamodel>"#,
    ));
    assert!(
        matches!(code, DiagnosticCode::ValidationUnionCases),
        "{code:?}"
    );
    assert!(
        matches!(&error, ValidationError::UnionCases { variant, .. } if variant == "circle"),
        "{error:?}"
    );
    assert_eq!(line, Some(12));
}

#[test]
fn a_union_of_no_case_or_of_only_empty_cases_is_refused() {
    let (error, code, _) = refusal(&with_imports(
        r#"  <datamodel>
    <data id="kind" sce:type="enum:ShapeKind"/>
  </datamodel>"#,
    ));
    assert!(
        matches!(code, DiagnosticCode::ValidationUnionCases),
        "{code:?}"
    );
    assert!(
        matches!(&error, ValidationError::UnionCases { detail, .. } if detail.contains("no <sce:case>")),
        "{error:?}"
    );

    // Every case empty: the union is its enum, and one fact has one spelling.
    let (error, code, line) = refusal(&with_imports(
        r#"  <datamodel>
    <data id="kind" sce:type="enum:ShapeKind">
      <sce:case variant="dot"/>
      <sce:case variant="circle"/>
      <sce:case variant="rect"/>
    </data>
  </datamodel>"#,
    ));
    assert!(
        matches!(code, DiagnosticCode::ValidationUnionCases),
        "{code:?}"
    );
    assert!(
        matches!(&error, ValidationError::UnionCases { variant, detail, .. }
            if variant.is_empty() && detail.contains("every case is empty")),
        "{error:?}"
    );
    // At the tag.
    assert_eq!(line, Some(9));
}

#[test]
fn a_variant_with_no_case_is_refused_naming_it() {
    let (error, code, line) =
        validation(compiled("union_shape_missing_case.scxml", Language::Rust));
    assert!(
        matches!(code, DiagnosticCode::ValidationUnionCases),
        "{code:?}"
    );
    match &error {
        ValidationError::UnionCases {
            document,
            variant,
            detail,
        } => {
            assert_eq!(document, "union_shape_missing_case");
            // The missing variant is written only in the enum, so no line of
            // the union holds it: the refusal names it in `detail` and has no
            // `actual`.
            assert_eq!(variant, "");
            assert!(detail.contains("with no case: 'rect'"), "{detail}");
        }
        other => panic!("{other:?}"),
    }
    // The tag, on line 9 of the fixture.
    assert_eq!(line, Some(9));
}

#[test]
fn a_case_naming_no_variant_is_refused_at_that_case() {
    let (error, code, line) = validation(compiled("union_shape_stray_case.scxml", Language::Rust));
    assert!(
        matches!(code, DiagnosticCode::ValidationUnionCases),
        "{code:?}"
    );
    match &error {
        ValidationError::UnionCases {
            variant, detail, ..
        } => {
            assert_eq!(variant, "square");
            assert!(detail.contains("naming no variant"), "{detail}");
            assert!(!detail.contains("with no case"), "{detail}");
        }
        other => panic!("{other:?}"),
    }
    // The fourth case, on line 15 of the fixture.
    assert_eq!(line, Some(15));
}

#[test]
fn a_tag_of_an_open_enum_is_refused() {
    let (error, code, _) = validation(compiled("union_shape_open_tag.scxml", Language::Rust));
    assert!(
        matches!(code, DiagnosticCode::ValidationAttributeRuleViolated),
        "{code:?}"
    );
    assert!(
        matches!(
            &error,
            ValidationError::AttributeRuleViolated { attr, value, rule, .. }
                if attr == "sce:type" && value == "enum:ShapeKind" && rule.contains("open")
        ),
        "{error:?}"
    );
}

#[test]
fn a_case_whose_record_declares_the_tags_key_is_refused() {
    let (error, code, line) =
        validation(compiled("union_shape_tag_collision.scxml", Language::Rust));
    assert!(
        matches!(code, DiagnosticCode::ValidationUnionTagCollision),
        "{code:?}"
    );
    match &error {
        ValidationError::UnionTagCollision {
            case,
            record,
            field,
            ..
        } => {
            assert_eq!(case, "rect");
            assert_eq!(record, "Kinded");
            assert_eq!(field, "kind");
        }
        other => panic!("{other:?}"),
    }
    // The tag, whose `id` is the colliding key, on line 11 of the fixture.
    assert_eq!(line, Some(11));
}

#[test]
fn a_valid_union_is_refused_by_name_on_every_backend() {
    assert!(ForgeKind::Union.is_supported());
    assert_eq!(ForgeKind::Union.to_string(), "union");
    assert!(!ForgeKind::Union.needs_instance());
    assert!(!ForgeKind::Union.is_inline_eligible());
    assert_eq!(ForgeKind::Union.max_runtime_dep(), RuntimeDep::None);
    assert_eq!(
        codegen_matrix::kind_class(ForgeKind::Union),
        KindClass::Generic
    );
    for language in Language::ALL.iter().copied() {
        assert_eq!(
            codegen_matrix::lookup(ForgeKind::Union, language),
            EmitOutcome::RuntimeMissing,
            "{language:?}"
        );
        let err = compiled("union_shape.scxml", language);
        assert!(
            matches!(
                err.error.to_diagnostics()[0].code,
                DiagnosticCode::GenerateUnsupportedFeature
            ),
            "{language:?}: {err:?}"
        );
        match err.error {
            ForgeError::Generate(boxed) => match *boxed {
                GenerateError::UnsupportedFeature { detail, .. } => {
                    assert!(
                        detail.contains("union 'union_shape'"),
                        "{language:?}: {detail}"
                    );
                    assert!(
                        detail.contains("no backend lowers a union yet"),
                        "{language:?}: {detail}"
                    );
                }
                other => panic!("{language:?}: {other:?}"),
            },
            other => panic!("{language:?}: {other:?}"),
        }
    }
}
