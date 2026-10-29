// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// An interpolation (`sce:kind="interpolation"`, a table interpolated between its breakpoints),
// written for Swift: a static function of the caseless enum the document's `<Pascal>` names,
// over `[Double]` arrays the enum holds, read by the runtime's `Interpolation.linear` or
// `bilinear`, which it names by its module.
//
// Every backend clamps an input outside its axis whatever policy the document asks for, so a
// document that asks for `extrapolate` or `error` is answered with a clamp it did not ask for.
// Swift writes what the document says or refuses it by name.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{
    compile_forge_file, compile_forge_from_string, DocumentLabel, ForgeCompileOptions,
};

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
fn a_table_of_one_axis_is_read_by_the_runtimes_linear_function() {
    let code = swift_of("sce-build/tests/fixtures/static_datamodel/forge_call_limit.scxml")
        .expect("an interpolation is written");
    assert!(code.contains("public enum ForgeCallLimit {"), "{code}");
    assert!(
        code.contains("private static let sceAxisRpm: [Double] = [0.0, 1000.0, 2000.0]"),
        "{code}"
    );
    assert!(
        code.contains("public static func lookup(_ rpm: UInt16) -> Double {"),
        "{code}"
    );
    assert!(
        code.contains("SceForgeRuntime.Interpolation.linear("),
        "the runtime is named by its module: {code}"
    );
}

#[test]
fn a_table_of_two_axes_is_read_by_the_runtimes_bilinear_function() {
    let code = swift_of("tests/forge/resources/interpolation_2d_bilinear.scxml")
        .expect("an interpolation is written");
    assert!(
        code.contains("private static let sceValues: [[Double]] = ["),
        "{code}"
    );
    assert!(
        code.contains("public static func lookup(_ rpm: UInt16, _ load: UInt8) -> Double {"),
        "{code}"
    );
    assert!(
        code.contains("SceForgeRuntime.Interpolation.bilinear("),
        "{code}"
    );
}

#[test]
fn a_policy_other_than_clamp_is_refused_by_name() {
    for policy in ["extrapolate", "error"] {
        let document = format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
          version="1.0" sce:kind="interpolation" name="engine_limit">
  <datamodel>
    <data id="rpm" sce:type="uint16" sce:direction="in"/>
    <data id="limit" sce:type="float64" sce:direction="out"
          sce:interpolation="linear" sce:out-of-bounds="{policy}"
          sce:axis-rpm="0 1000 2000">
      100.0 150.0 120.0
    </data>
  </datamodel></scxml>"#
        );
        let error = compile_forge_from_string(
            &document,
            DocumentLabel {
                identifier: "engine_limit",
                diagnostic_label: "engine_limit.scxml",
            },
            Language::Swift,
        )
        .err()
        .unwrap_or_else(|| panic!("Swift answered a `{policy}` table with a clamp"));
        let text = format!("{error:?}");
        assert!(
            text.contains(&format!("an out-of-range policy of `{policy}`")),
            "{policy}: the refusal names the policy: {text}"
        );
    }
}
