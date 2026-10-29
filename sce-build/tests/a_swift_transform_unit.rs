// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A transform (`sce:kind="transform"`, a formula per output), written for Swift: a static
// function of the caseless enum the document's `<Pascal>` names for each output, lowerCamelCase
// as Kotlin's, over the document's inputs, each named by the signature as the author wrote it.
//
// A transform that reads a value through `previous()` keeps it between activations in a holder
// object, which a Swift unit has not spelled: it is refused by name, never written without it.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_forge_file, ForgeCompileOptions};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The Swift source `document` (a path under the repository) is written as, or the
/// refusal it is answered with.
fn swift_of(document: &str) -> Result<String, String> {
    let output = compile_forge_file(
        &repo_root().join(document),
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let (_, code) = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file");
    Ok(code)
}

#[test]
fn each_output_is_a_static_function_of_the_enum_its_unit_declares() {
    let code = swift_of("tests/forge/resources/transform_multi_output.scxml")
        .expect("a transform is written");
    assert!(
        code.contains("public enum TransformMultiOutput {"),
        "{code}"
    );
    assert!(
        code.contains("public static func computeFahrenheit(_ celsius: Double) -> Double {"),
        "{code}"
    );
    assert!(
        code.contains("public static func computeKelvin(_ celsius: Double) -> Double {"),
        "{code}"
    );
}

#[test]
fn a_physical_quantity_is_documented_beside_the_function_that_reads_it() {
    let code = swift_of("tests/forge/resources/transform_quantity_celsius.scxml")
        .expect("a transform is written");
    assert!(
        code.contains("`raw_temp` carries unit `celsius`"),
        "the input's unit is in the doc comment: {code}"
    );
}

#[test]
fn a_transform_that_keeps_a_value_between_activations_is_refused_by_name() {
    let refusal = swift_of("tests/forge/resources/transform_previous_value.scxml")
        .expect_err("a holder is not written yet");
    assert!(
        refusal.contains("a value read through `previous()`"),
        "the refusal names the construct: {refusal}"
    );
}
