// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The type `union:<alias>` (docs/adr/0092, section 1.1): an event schema's
//! field holds one at any depth, named through `<sce:import kind="union">`; an
//! alias no import names and a bracket on one are refused at the attribute; the
//! scope a document's types are read in holds the union, its cases in its tag
//! enum's order, and sizes it by its largest case; and every backend refuses a
//! field of one by name until one lowers it (section 7).

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located, ValidationError};
use sce_build::forge::model::ForgeDocument;
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::forge::value_type::{
    cbor_max_len, container_depth, json_max_len, longest_pointer, Bound, ValueType,
};
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

fn shape() -> ValueType {
    ValueType::Union {
        alias: "Shape".into(),
    }
}

/// An event schema of revision 2 that imports the shape union and declares
/// one field `f` of the type `ty` (as the document spells it, `<` escaped)
/// with `extra` attributes.
fn schema_with(ty: &str, extra: &str) -> String {
    let ty = ty.replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="probe" sce:event-name="probe.sent" sce:revision="2">
  <sce:import kind="union" src="union_shape.scxml" as="Shape"/>
  <datamodel>
    <data id="first" sce:type="uint8" sce:direction="in"/>
    <data id="f" sce:type="{ty}" {extra} sce:direction="in"/>
  </datamodel>
</scxml>
"#
    )
}

/// The refusal of `document`, read as the forge parser reads it, and its code.
fn refusal(document: &str) -> (ValidationError, DiagnosticCode) {
    let err = parse_forge_with_imports(document, DocumentLabel::symmetric("probe"))
        .expect_err("the schema is refused");
    let code = err.error.to_diagnostics()[0].code;
    match err.error {
        ForgeError::Validation(v) => (*v, code),
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn an_event_field_holds_a_union_at_any_depth() {
    let path = fixtures().join("event_shapes.scxml");
    let text = std::fs::read_to_string(&path).expect("the fixture reads");
    let label = path.to_string_lossy().into_owned();
    let parsed = parse_forge_with_imports(&text, DocumentLabel::for_input_path(&label))
        .expect("the schema parses")
        .expect("a forge document");
    let ForgeDocument::EventSchema(schema) = &parsed.document else {
        panic!("an event schema, got {:?}", parsed.document.kind());
    };
    let types: Vec<(&str, &ValueType)> = schema
        .fields
        .iter()
        .map(|f| (f.id.as_str(), &f.sce_type))
        .collect();
    assert_eq!(
        types,
        [
            ("one", &shape()),
            (
                "many",
                &ValueType::List {
                    elem: Box::new(shape()),
                    max: Bound::of(3).at_least(1),
                }
            ),
            (
                "maybe",
                &ValueType::Optional {
                    elem: Box::new(shape()),
                }
            ),
        ]
    );
    // Each is a composition, which a lowering of one entry per field names by
    // the union.
    let composed = schema.entry_schema().expect_err("a composed schema");
    assert_eq!(composed.field, "one");
    assert_eq!(composed.composition.constructor, "union");
}

#[test]
fn the_scope_of_a_document_holds_a_union_in_its_tag_enums_order_and_sizes_it() {
    let path = fixtures().join("event_shapes.scxml");
    let text = std::fs::read_to_string(&path).expect("the fixture reads");
    let label = path.to_string_lossy().into_owned();
    let parsed = parse_forge_with_imports(&text, DocumentLabel::for_input_path(&label))
        .expect("the schema parses")
        .expect("a forge document");
    let scope = sce_build::forge::import_source::type_scope(&parsed.imports, &fixtures());
    let union = &scope.unions["Shape"];
    assert_eq!(union.tag, "kind");
    assert_eq!(union.tag_enum, "ShapeKind");
    // The document writes rect, dot, circle; the enum declares dot, circle,
    // rect, and its numbers are the cases' own.
    let cases: Vec<(&str, i128, Option<&str>)> = union
        .cases
        .iter()
        .map(|case| {
            (
                case.variant.name.as_str(),
                case.variant.value,
                case.record.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        cases,
        [
            ("dot", 0, None),
            ("circle", 1, Some("Radius")),
            ("rect", 2, Some("Rect")),
        ]
    );
    // The largest case: `{"kind":"rect","width":65535,"height":65535}`, and
    // on CBOR a map of three whose tag is the number 2.
    assert_eq!(json_max_len(&shape(), &scope), Ok(44));
    assert_eq!(cbor_max_len(&shape(), &scope), Ok(26));
    // One level, as a record is; the longest place is a field of a case, the
    // first of the longest in the enum's order.
    assert_eq!(container_depth(&shape(), &scope), Ok(1));
    assert_eq!(
        longest_pointer(&shape(), &scope),
        Ok(("/radius".to_string(), 7))
    );
}

#[test]
fn a_union_no_import_names_is_refused_with_the_unions_there_are() {
    let (error, code) = refusal(&schema_with("list<union:Nope>[2]", ""));
    assert!(
        matches!(code, DiagnosticCode::ValidationInvalidAttribute),
        "{code:?}"
    );
    match &error {
        ValidationError::InvalidAttribute { allowed, .. } => {
            assert_eq!(allowed, &["union:Shape"]);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_bracket_on_a_union_is_refused_as_on_a_record() {
    let (error, code) = refusal(&schema_with("union:Shape[2]", ""));
    assert!(
        matches!(code, DiagnosticCode::ValidationAttributeRuleViolated),
        "{code:?}"
    );
    assert!(
        matches!(&error, ValidationError::AttributeRuleViolated { attr, .. } if attr == "sce:type"),
        "{error:?}"
    );
}

#[test]
fn a_union_a_later_revision_adds_is_declared_optional() {
    let (error, code) = refusal(&schema_with("union:Shape", r#"sce:since="2""#));
    assert!(
        matches!(code, DiagnosticCode::ValidationAttributeRuleViolated),
        "{code:?}"
    );
    assert!(
        matches!(&error, ValidationError::AttributeRuleViolated { attr, .. } if attr == "sce:since"),
        "{error:?}"
    );
    // Declared optional, it reads absent.
    assert!(parse_forge_with_imports(
        &schema_with("optional<union:Shape>", r#"sce:since="2""#),
        DocumentLabel::symmetric("probe"),
    )
    .is_ok());
}

/// The refusal of `language` of the fixture event schema.
fn compiled(language: Language) -> Located<ForgeError> {
    let options = ForgeCompileOptions {
        go_module_prefix: Some("example.com/generated".to_string()),
        ..Default::default()
    };
    match sce_build::compile_forge_file(
        &fixtures().join("event_shapes.scxml"),
        language,
        &[],
        &options,
    ) {
        Ok(_) => panic!("{language:?} generated a field of a union"),
        Err(refusal) => refusal,
    }
}

#[test]
fn every_backend_refuses_a_field_of_a_union_by_name() {
    for language in Language::ALL.iter().copied() {
        let err = compiled(language);
        assert!(
            matches!(
                err.error.to_diagnostics()[0].code,
                DiagnosticCode::GenerateUnsupportedFeature
            ),
            "{language:?}: {err:?}"
        );
        let ForgeError::Generate(boxed) = err.error else {
            panic!("{language:?}: not a generation refusal");
        };
        match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => {
                assert!(detail.contains("union:Shape"), "{language:?}: {detail}");
            }
            other => panic!("{language:?}: {other:?}"),
        }
    }
}
