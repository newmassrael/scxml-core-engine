// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A lookup (`sce:kind="lookup"`, a table read by exact match), written for Swift: a static
// function of the caseless enum the document's `<Pascal>` names, over two arrays the enum
// holds, read by the runtime's `Lookup.find`. A table that errors on a miss answers `nil` and
// one with a default answers it. A table that answers a name answers a case of an enum the
// unit declares, and a validator that imports one is refused by name.

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

const FIXTURES: &str = "sce-build/tests/fixtures/static_datamodel";

#[test]
fn a_table_that_errors_on_a_miss_answers_an_optional_read_by_the_runtime() {
    let code = swift_of(&format!("{FIXTURES}/forge_call_code.scxml")).expect("a lookup is written");
    assert!(code.contains("public enum ForgeCallCode {"), "{code}");
    assert!(
        code.contains("private static let sceKeys: [UInt8] = [1, 2]"),
        "{code}"
    );
    assert!(
        code.contains("public static func lookupCode(_ gear: UInt8) -> Int32? {"),
        "a miss that is an error answers nothing: {code}"
    );
    assert!(
        code.contains("Lookup.find(sceKeys, sceValues, gear)"),
        "{code}"
    );
    assert!(code.contains("import SceForgeRuntime"), "{code}");
}

#[test]
fn a_table_with_a_default_answers_it_for_a_miss() {
    let code =
        swift_of(&format!("{FIXTURES}/forge_call_ratio.scxml")).expect("a lookup is written");
    assert!(
        code.contains("public static func lookupRatio(_ gear: UInt8) -> Int32 {"),
        "a default is a value, so nothing is optional: {code}"
    );
    assert!(
        code.contains("Lookup.find(sceKeys, sceValues, gear) ?? 0"),
        "{code}"
    );
}

#[test]
fn a_table_of_enum_variants_names_the_variants_the_units_declare() {
    let code =
        swift_of(&format!("{FIXTURES}/forge_call_weight.scxml")).expect("a lookup is written");
    assert!(
        code.contains("[ForgeCallStatus.open, ForgeCallStatus.held]"),
        "the keys are variants, never their numbers: {code}"
    );
    assert!(
        code.contains("-> ForgeCallLevel? {"),
        "the answer is a variant of the other enum: {code}"
    );
}

/// A table that answers a name declares an enum of the names, nested in the unit's own,
/// each case holding the name the document spells; a key it does not list answers its
/// default.
#[test]
fn a_table_that_answers_a_name_answers_a_case_of_a_nested_enum() {
    let code = swift_of("tests/forge/resources/lookup_engine_status.scxml")
        .expect("a table of names is written");
    for needle in [
        "public enum Status: String, Sendable {",
        // A case is spelled as the enum units spell theirs (`enum_naming::variant_ident`),
        // which leaves a name the document writes in capitals as it is.
        "case STOP = \"STOP\"",
        "case RUNNING = \"RUNNING\"",
        "public static func lookupStatus(_ engSta: UInt8) -> Status {",
        "return .RUNNING",
        "default:",
    ] {
        assert!(
            code.contains(needle),
            "the generated Swift holds `{needle}`: {code}"
        );
    }
    // A table of names needs no runtime of the forge.
    assert!(!code.contains("SceForgeRuntime.Lookup"), "{code}");
}
