// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A `<sce:const>` of an algorithm (ADR 0063), written for Swift as a `static let` member of
// the unit's caseless enum: a scalar as its type and a table as an `Array` of its element,
// folded where the unit is generated, and read by the unit's function unqualified. A
// `<sce:test-vector>` writes nothing beside the unit: the sidecar the other backends write
// has no Swift test target to receive it, and the conformance harness holds the unit to the
// same values.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_forge_file, ForgeCompileOptions};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Every file `document` (a path under the repository) is written as for Swift, or the
/// refusal it is answered with.
fn files_of(document: &str) -> Result<Vec<(String, String)>, String> {
    compile_forge_file(
        &repo_root().join(document),
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .map(|output| output.files)
    .map_err(|e| format!("{e:?}"))
}

fn unit_of(document: &str) -> String {
    let mut files = files_of(document).unwrap_or_else(|why| panic!("{document} is written: {why}"));
    assert_eq!(files.len(), 1, "one unit and nothing beside it: {files:?}");
    files.remove(0).1
}

#[test]
fn a_table_is_a_typed_static_member_of_the_unit_and_is_read_by_its_name() {
    let code = unit_of("tests/forge/resources/algorithm_crc16_table.scxml");
    assert!(
        code.contains("public static let CRC16_TABLE: [UInt16] = ["),
        "the table is a typed member of the enum: {code}"
    );
    assert!(
        code.contains("CRC16_TABLE["),
        "the body reads an entry by the table's name: {code}"
    );
    // A member of the enum, before the function that reads it.
    let table = code.find("CRC16_TABLE: [UInt16]").expect("the table");
    let function = code.find("public static func").expect("the function");
    assert!(table < function, "the table precedes the function: {code}");
}

#[test]
fn a_second_table_is_written_the_same_way() {
    let code = unit_of("tests/forge/resources/algorithm_lcg_table.scxml");
    assert!(
        code.contains("public static let LCG_TABLE: [UInt32] = ["),
        "the table is a typed member of the enum: {code}"
    );
}

#[test]
fn a_test_vector_writes_no_sidecar_beside_the_unit() {
    let code = unit_of("tests/forge/resources/algorithm_crc16.scxml");
    assert!(
        code.contains("public enum AlgorithmCrc16"),
        "the unit is written: {code}"
    );
}
