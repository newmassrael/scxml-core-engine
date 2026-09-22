// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A document depends on the imports it names, and on no others.
//
// Measured 2026-09-22: a transform importing an enum it never read
// generated with exit 0, and its Go output carried
// `import ("…/enum_session_state")` with nothing using the package — which
// Go refuses to compile. The other backends emitted the same dependency and
// merely warned, so nothing else noticed.
//
// Both arms below read the same import from the same file. The one that
// names it proves the dependency text appears when it should, so the arm
// that does not name it is measuring an absence the generator could have
// produced, not one it never could.

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

const LANGUAGES: [Language; 6] = [
    Language::Cpp,
    Language::C11,
    Language::Rust,
    Language::Kotlin,
    Language::Go,
    Language::Python,
];

/// The imported enum's document name — every backend spells the
/// dependency on it with this snake form (an include, a `use`, a package
/// path, a module import).
const IMPORTED: &str = "enum_uds_nrc";

fn resource_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .join("tests/forge/resources")
}

/// A transform importing the NRC enum as `Nrc`, with its one input typed
/// `input_type`.
fn transform(input_type: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="import_use">
  <sce:import as="Nrc" src="{IMPORTED}.scxml" kind="enum"/>
  <datamodel>
    <data id="code" sce:type="{input_type}" sce:direction="in"/>
    <data id="out" sce:type="uint8" sce:direction="out" expr="1"/>
  </datamodel>
</scxml>"#
    )
}

/// Every file `scxml` generates for `language`, concatenated.
fn generated(scxml: &str, language: Language) -> String {
    let mut options = ForgeCompileOptions::default();
    if matches!(language, Language::Go) {
        options.go_module_prefix = Some("example.com/generated".to_string());
    }
    let output = sce_build::compile_forge_with_imports(
        scxml,
        DocumentLabel::symmetric("import_use"),
        language,
        &resource_dir(),
        &options,
    )
    .unwrap_or_else(|e| panic!("{language:?}: must generate: {e}"));
    output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn an_import_the_document_names_is_a_dependency_on_every_backend() {
    let scxml = transform("enum:Nrc");
    for language in LANGUAGES {
        let code = generated(&scxml, language);
        assert!(
            code.contains(IMPORTED),
            "{language:?}: a field typed by the import must depend on it:\n{code}"
        );
    }
}

#[test]
fn an_import_nothing_names_is_not_a_dependency_on_any_backend() {
    let scxml = transform("uint8");
    for language in LANGUAGES {
        let code = generated(&scxml, language);
        assert!(
            !code.contains(IMPORTED),
            "{language:?}: nothing names the import, so nothing may depend on it:\n{code}"
        );
    }
}
