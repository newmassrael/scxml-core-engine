// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The `feature-model` kind (ADR 0057 §2): what a document of it reads as,
//! what it refuses and where, and that it is a declaration that writes no file
//! on any backend.

use sce_build::forge::codegen_matrix::{self, EmitOutcome, KindClass};
use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::forge::model::{ForgeDocument, ForgeKind, ParsedForge, RuntimeDep};
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::generator::Language;
use sce_build::DocumentLabel;

fn document(features: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="feature-model" name="session_features">
{features}
</scxml>"#
    )
}

const LABEL: DocumentLabel<'static> = DocumentLabel {
    identifier: "session_features",
    diagnostic_label: "session_features.scxml",
};

fn parse(features: &str) -> ParsedForge {
    parse_forge_with_imports(&document(features), LABEL)
        .unwrap_or_else(|e| panic!("a valid model parses: {e:?}"))
        .expect("a feature model is a forge document")
}

/// The refusal of `features`, and the line it is placed on (the document's
/// first feature is on line 5).
fn refusal(features: &str) -> (ValidationError, Option<u32>) {
    let err =
        parse_forge_with_imports(&document(features), LABEL).expect_err("the model is refused");
    let line = err.location.line;
    match err.error {
        ForgeError::Validation(v) => (*v, line),
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn a_feature_model_reads_as_its_features_in_document_order() {
    let parsed = parse(
        r#"  <sce:feature name="session-unicast-accept" requires="session-unicast"/>
  <sce:feature name="session-unicast"/>
  <sce:feature name="session-auth" requires=" session-unicast  session-unicast-accept "/>"#,
    );
    let ForgeDocument::FeatureModel(model) = &parsed.document else {
        panic!("a feature-model document, got {:?}", parsed.document.kind());
    };
    assert_eq!(model.name, "session_features");
    let written: Vec<(&str, Vec<&str>)> = model
        .features
        .iter()
        .map(|f| {
            (
                f.name.as_str(),
                f.requires.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        written,
        [
            ("session-unicast-accept", vec!["session-unicast"]),
            ("session-unicast", vec![]),
            (
                "session-auth",
                vec!["session-unicast", "session-unicast-accept"]
            ),
        ]
    );
    // The model's order puts a feature after what it requires.
    let order = model.feature_model();
    let names: Vec<&str> = order.features().map(|f| order.name(f)).collect();
    assert_eq!(
        names,
        ["session-unicast", "session-unicast-accept", "session-auth"]
    );
}

#[test]
fn a_name_that_is_not_a_feature_name_is_refused_on_its_line() {
    let (error, line) = refusal(
        r#"  <sce:feature name="session-unicast"/>
  <sce:feature name="Session_Accept"/>"#,
    );
    assert!(
        matches!(&error, ValidationError::FeatureNameInvalid { name, .. } if name == "Session_Accept"),
        "{error:?}"
    );
    assert_eq!(line, Some(6));
}

#[test]
fn a_name_declared_twice_is_refused_at_its_second_declaration() {
    let (error, line) = refusal(
        r#"  <sce:feature name="a"/>
  <sce:feature name="b"/>
  <sce:feature name="a"/>"#,
    );
    assert!(
        matches!(&error, ValidationError::FeatureDeclaredTwice { name, .. } if name == "a"),
        "{error:?}"
    );
    assert_eq!(line, Some(7));
}

#[test]
fn a_requirement_the_model_does_not_declare_is_refused_at_the_feature_that_names_it() {
    let (error, line) = refusal(
        r#"  <sce:feature name="session-unicast"/>
  <sce:feature name="session-unicast-accept" requires="session-unicst"/>"#,
    );
    assert!(
        matches!(
            &error,
            ValidationError::FeatureRequirementUndeclared { feature, missing, .. }
                if feature == "session-unicast-accept" && missing == "session-unicst"
        ),
        "{error:?}"
    );
    assert_eq!(line, Some(6));
}

#[test]
fn features_that_require_each_other_are_refused_as_one_feature() {
    let (error, line) = refusal(
        r#"  <sce:feature name="c" requires="a"/>
  <sce:feature name="b" requires="c"/>
  <sce:feature name="a" requires="b"/>"#,
    );
    assert!(
        matches!(&error, ValidationError::FeatureModelCycle { cycle, .. } if cycle == &["a", "b", "c", "a"]),
        "{error:?}"
    );
    // Placed on the feature the cycle is written from: its least name.
    assert_eq!(line, Some(7));
}

#[test]
fn a_feature_without_a_name_is_refused_on_its_line() {
    // The schema requires `name`, and the parser refuses its absence too, for
    // a build that does not validate against the schema (`xsd` off).
    let err = parse_forge_with_imports(&document(r#"  <sce:feature requires="a"/>"#), LABEL)
        .expect_err("the model is refused");
    match &err.error {
        ForgeError::Xml(sce_build::forge::error::XmlError::SchemaValidation(errors)) => {
            assert_eq!(errors.diagnostics[0].line, Some(5), "{errors:?}");
        }
        ForgeError::Validation(v) => {
            assert!(
                matches!(&**v, ValidationError::MissingAttribute { attr, .. } if attr == "name"),
                "{v:?}"
            );
            assert_eq!(err.location.line, Some(5));
        }
        other => panic!("refused for another reason: {other:?}"),
    }
}

#[test]
fn a_feature_model_is_a_declaration_that_writes_no_file_on_any_backend() {
    assert!(ForgeKind::FeatureModel.is_supported());
    assert_eq!(ForgeKind::FeatureModel.to_string(), "feature-model");
    assert!(!ForgeKind::FeatureModel.needs_instance());
    assert!(!ForgeKind::FeatureModel.is_inline_eligible());
    assert_eq!(ForgeKind::FeatureModel.max_runtime_dep(), RuntimeDep::None);
    assert_eq!(
        codegen_matrix::kind_class(ForgeKind::FeatureModel),
        KindClass::Declaration
    );
    let parsed = parse(r#"  <sce:feature name="a"/>"#);
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    for language in Language::ALL.iter().copied() {
        assert_eq!(
            codegen_matrix::lookup(ForgeKind::FeatureModel, language),
            EmitOutcome::Declaration,
            "{language:?}"
        );
        let output = sce_build::compile_forge_from_parsed(
            &parsed,
            LABEL,
            language,
            &base,
            &sce_build::ForgeCompileOptions::default(),
        )
        .unwrap_or_else(|e| panic!("{language:?}: {e:?}"));
        assert!(
            output.files.is_empty(),
            "{language:?} wrote {:?}",
            output.files
        );
    }
}
