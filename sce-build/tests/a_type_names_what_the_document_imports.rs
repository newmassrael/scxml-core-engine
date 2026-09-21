// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A type-valued attribute names a type this document can use: a scalar,
// or `enum:<alias>` for an enum the document imports.
//
// Measured 2026-09-21 with the generator at ae8053923c: a transform field
// typed `enum:Nope` with no such import passed every stage and PANICKED
// in code generation, exit 101 on every backend — the panic's own text
// blamed an "import gate" that did not exist. And a misspelled type was
// offered the scalar names alone, so the one spelling that depends on the
// document, `enum:<alias>`, was never a candidate.

use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .join("tests/forge/resources")
}

fn compile(name: &str, scxml: &str) -> Result<(), ForgeError> {
    sce_build::compile_forge_with_imports(
        scxml,
        DocumentLabel::symmetric(name),
        Language::Rust,
        &resource_dir(),
        &ForgeCompileOptions::default(),
    )
    .map(|_| ())
    .map_err(|located| located.error)
}

/// The validation error `scxml` is refused with.
fn refusal(name: &str, scxml: &str) -> ValidationError {
    match compile(name, scxml) {
        Ok(()) => panic!("{name}: generated, but must be refused"),
        Err(ForgeError::Validation(boxed)) => *boxed,
        Err(other) => panic!("{name}: refused, but not by validation: {other:?}"),
    }
}

/// The value and candidates of an `InvalidAttribute` refusal.
fn invalid_type(name: &str, scxml: &str) -> (String, Vec<String>) {
    match refusal(name, scxml) {
        ValidationError::InvalidAttribute { value, allowed, .. } => (value, allowed),
        other => panic!("{name}: expected InvalidAttribute, got {other:?}"),
    }
}

/// A transform with one input typed `input_type`, importing the NRC enum
/// as `Nrc` when `import` is set.
fn transform(name: &str, input_type: &str, import: bool) -> String {
    let import = if import {
        r#"<sce:import as="Nrc" src="enum_uds_nrc.scxml" kind="enum"/>"#
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="{name}">
  {import}
  <datamodel>
    <data id="code" sce:type="{input_type}" sce:direction="in"/>
    <data id="out" sce:type="uint8" sce:direction="out" expr="1"/>
  </datamodel>
</scxml>"#
    )
}

#[test]
fn an_enum_the_document_imports_is_a_type() {
    compile("enum_ok", &transform("enum_ok", "enum:Nrc", true))
        .expect("an imported enum is a type this document can name");
}

/// The case that panicked: refused as a type, with nothing enum-shaped to
/// offer because the document imports no enum.
#[test]
fn an_enum_the_document_does_not_import_is_refused_not_a_panic() {
    let (value, allowed) = invalid_type("enum_none", &transform("enum_none", "enum:Nope", false));
    assert_eq!(value, "enum:Nope");
    assert!(allowed.contains(&"uint8".to_string()), "{allowed:?}");
    assert!(
        !allowed.iter().any(|a| a.starts_with("enum:")),
        "no enum is imported, so none can be offered: {allowed:?}"
    );
}

#[test]
fn a_misspelled_enum_alias_is_offered_the_imported_one() {
    let (value, allowed) = invalid_type("enum_typo", &transform("enum_typo", "enum:Nrcc", true));
    assert_eq!(value, "enum:Nrcc");
    assert!(allowed.contains(&"enum:Nrc".to_string()), "{allowed:?}");
}

// ⚠ Two cases are NOT here, and are held by the parser's own unit tests
// (`type_attr_tests`) instead: a misspelled scalar offered the document's
// enums, and an enum at a position the schema types scalar-only (an
// algorithm's parameter). Where the XSD is available it answers both
// first, so no compile reaches the parser's reading of them — and that
// reading is what an XSD-less build has.

/// A `<sce:const>` has no enum type, imported or not: its rule names
/// none, and the schema leaves the attribute a free string, so the parser
/// is what refuses it.
#[test]
fn a_const_is_refused_an_enum_type_even_an_imported_one() {
    let constant = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" version="1.0" name="c">
  <sce:import as="Nrc" src="enum_uds_nrc.scxml" kind="enum"/>
  <sce:const name="DEFAULT" type="enum:Nrc" init="0"/>
  <sce:signature>
    <sce:param name="x" type="uint8"/>
    <sce:return type="uint8"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="x"/>
  </sce:body>
</scxml>"#;
    match refusal("const", constant) {
        ValidationError::AttributeRuleViolated { attr, value, .. } => {
            assert_eq!(attr, "type");
            assert_eq!(value, "enum:Nrc");
        }
        other => panic!("expected AttributeRuleViolated on the const type, got {other:?}"),
    }
}
