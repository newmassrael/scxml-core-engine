// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A condition (`sce:kind="condition"`, a named boolean guard), written for Swift: a static
// function of the caseless enum the document's `<Pascal>` names, lowerCamelCase as Kotlin's,
// qualified by the enum where it is called, so a member of the calling type cannot hide it.

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
fn a_condition_is_a_static_function_of_the_enum_its_unit_declares() {
    let code = swift_of("sce-build/tests/fixtures/static_datamodel/forge_call_high.scxml")
        .expect("a condition is written");
    assert!(code.contains("public enum ForgeCallHigh {"), "{code}");
    assert!(
        code.contains("public static func forgeCallHigh(_ level: Int32) -> Bool {"),
        "the function is lowerCamelCase and answers a `Bool`: {code}"
    );
    assert!(
        code.contains("level > 20"),
        "the expression is the document's: {code}"
    );
}

#[test]
fn a_condition_that_reads_no_runtime_imports_none() {
    let code = swift_of("sce-build/tests/fixtures/static_datamodel/forge_call_high.scxml")
        .expect("a condition is written");
    assert!(
        !code.contains("import SceForgeRuntime"),
        "a unit names the runtime only where its expression does: {code}"
    );
}
