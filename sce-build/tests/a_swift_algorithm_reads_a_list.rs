// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// An algorithm over a `list<T>` parameter (SCE_FORGE.md §4.12), written for Swift: the
// list is an `Array` of the element, read-only, and `<sce:foreach>` walks it. A list
// an algorithm builds, appends to or returns is refused by that construct and never
// emitted wrong.

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
fn a_list_of_records_is_an_array_of_the_schemas_struct() {
    let code = swift_of(&format!("{FIXTURES}/algorithm_ledger_has.scxml"))
        .expect("an algorithm that reads a list of records is written");
    assert!(
        code.contains("func ledgerHas(_ entries: [SchemaEntryPayload], _ id: UInt32) -> Bool"),
        "{code}"
    );
    assert!(code.contains("for e in entries {"), "{code}");
}

#[test]
fn a_list_of_numbers_is_an_array_of_the_number() {
    let code = swift_of(&format!("{FIXTURES}/algorithm_ids_above.scxml"))
        .expect("an algorithm that reads a list of numbers is written");
    assert!(
        code.contains("func idsAbove(_ ids: [UInt32], _ floor: UInt32) -> Bool"),
        "{code}"
    );
    assert!(code.contains("for id in ids {"), "{code}");
}

#[test]
fn a_standard_rule_over_a_list_of_texts_compares_them_by_their_bytes() {
    let code = swift_of("stdlib/text/text_slot_holds.scxml")
        .expect("a standard rule that reads a list of records is written");
    assert!(
        code.contains(
            "func textSlotHolds(_ slots: [TextSlotPayload], _ probe: TextSlotPayload) -> Bool"
        ),
        "{code}"
    );
    assert!(
        code.contains("s.text.utf8.elementsEqual(probe.text.utf8)"),
        "Swift's `==` takes two spellings of a character for one text: {code}"
    );
}

#[test]
fn a_list_an_algorithm_builds_or_returns_is_refused_by_name() {
    // `day_run` appends to a list local and returns it, and fails.
    let refusal = swift_of(&format!("{FIXTURES}/algorithm_day_run.scxml"))
        .expect_err("a list a body builds is not lowered for Swift yet");
    assert!(
        refusal.contains("Swift") && refusal.contains("list<uint8>"),
        "{refusal}"
    );
}
