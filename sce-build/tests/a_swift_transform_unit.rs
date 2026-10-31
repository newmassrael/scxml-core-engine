// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A transform (`sce:kind="transform"`, a formula per output), written for Swift: a static
// function of the caseless enum the document's `<Pascal>` names for each output, lowerCamelCase
// as Kotlin's, over the document's inputs, each named by the signature as the author wrote it.
//
// A transform that reads a value through `previous()` keeps it between activations: its unit is a
// final class holding the kept values, with `reset()` and `update(...)` returning a record of the
// outputs. A value the host restores (`sce:retain`) has no Swift constructor yet: it is refused by
// name, never written without it.

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
fn a_transform_that_keeps_a_value_between_activations_is_a_final_class_with_reset_and_update() {
    let code = swift_of("tests/forge/resources/transform_previous_value.scxml")
        .expect("a holder is written");
    assert!(
        code.contains("public final class TransformPreviousValue {"),
        "the unit is the holder: {code}"
    );
    assert!(code.contains("public init() {}"), "{code}");
    assert!(code.contains("public func reset() {"), "{code}");
    assert!(
        code.contains(") -> TransformPreviousValueOutputs {"),
        "an activation answers a record of the outputs: {code}"
    );
    assert!(
        code.contains("public struct TransformPreviousValueOutputs: Equatable, Sendable {"),
        "{code}"
    );
}

#[test]
fn a_value_the_host_restores_is_refused_by_name() {
    // A retained count read through `previous()`: the holder would start from a stored value.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("a_swift_transform_unit");
    std::fs::create_dir_all(&dir).expect("a directory under the target tree");
    let document = dir.join("stored.scxml");
    std::fs::write(
        &document,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="stored">
  <datamodel>
    <data id="step" sce:type="int32" sce:direction="in"/>
    <data id="count" sce:type="int32" sce:direction="out" sce:retain="nvm"
          sce:initial="0" expr="previous(count) + step"/>
  </datamodel>
</scxml>
"#,
    )
    .expect("write the document");
    let refusal = compile_forge_file(
        &document,
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .map(|_| ())
    .map_err(|e| format!("{e:?}"));
    std::fs::remove_dir_all(&dir).expect("remove the document");
    let refusal = refusal.expect_err("a restored holder is not written yet");
    assert!(
        refusal.contains("`sce:retain`"),
        "the refusal names the construct: {refusal}"
    );
}
