// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// An algorithm that builds a `list<T>` and returns it (SCE_FORGE.md §4.12), written
// for Swift: the buffer is a local `Array` of the element with its capacity
// reserved, `<sce:append>` is `Array.append`, and the return is the array by value.
// A list of records holds the schema's struct, which is a value, so the list holds
// its own copy of what is appended.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_forge_file, ForgeCompileOptions};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

const FIXTURES: &str = "sce-build/tests/fixtures/static_datamodel";

/// The Swift source `document` (a path under the repository) is written as.
fn swift_of(document: &str) -> String {
    let output = compile_forge_file(
        &repo_root().join(document),
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .unwrap_or_else(|e| panic!("{document} is written for Swift: {e:?}"));
    let (_, code) = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file");
    code
}

#[test]
fn a_list_of_numbers_is_a_local_array_the_body_appends_to() {
    let code = swift_of(&format!("{FIXTURES}/algorithm_day_run.scxml"));
    assert!(
        code.contains("func dayRun(_ first: UInt8, _ count: UInt8) throws -> [UInt8]"),
        "{code}"
    );
    assert!(code.contains("var out: [UInt8] = []"), "{code}");
    // The capacity the document declares is reserved, as the C++ buffer's is.
    assert!(code.contains("out.reserveCapacity(8)"), "{code}");
    // The element is the value of the checked sum, narrowed to the element's type
    // by the sum itself.
    assert!(
        code.contains("out.append(Checked.add(sceFailure, first, i))"),
        "{code}"
    );
}

#[test]
fn the_buffer_is_returned_as_the_value_the_failure_is_read_before() {
    let code = swift_of(&format!("{FIXTURES}/algorithm_day_run.scxml"));
    let computed = code
        .find("let sceReturned: [UInt8] = out")
        .expect("the list is computed into a local, as any may-fail return is");
    let check = code[computed..]
        .find("try sceFailure.throwIfFailed()")
        .expect("the failure is read");
    let returned = code[computed..]
        .find("return sceReturned")
        .expect("the local is returned");
    assert!(check < returned, "{code}");
}

#[test]
fn a_list_of_records_holds_the_schemas_struct() {
    let code = swift_of(&format!("{FIXTURES}/algorithm_ledger_add.scxml"));
    assert!(
        code.contains(
            "func ledgerAdd(_ entries: [SchemaEntryPayload], _ id: UInt32, _ amount: Int32) \
             throws -> [SchemaEntryPayload]"
        ),
        "{code}"
    );
    assert!(
        code.contains("var out: [SchemaEntryPayload] = []"),
        "{code}"
    );
    // A record is appended by its name: the struct is a value, so the array keeps a
    // copy, and a loop's item is appended as it is.
    assert!(code.contains("out.append(fresh)"), "{code}");
    assert!(code.contains("out.append(e)"), "{code}");
}

#[test]
fn an_algorithm_that_does_not_declare_may_fail_returns_its_list_without_throwing() {
    // A list return is a plain return of the array: no thrown failure, no local
    // that a check stands between.
    let code = swift_of("stdlib/mesh/ordering_prune.scxml");
    assert!(code.contains("-> [OrderingSlotPayload]"), "{code}");
    assert!(!code.contains("throws"), "{code}");
    assert!(!code.contains("sceFailure"), "{code}");
    assert!(code.contains("return out"), "{code}");
}
