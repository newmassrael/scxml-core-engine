// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// An algorithm over a record of an event schema (SCE_FORGE.md §4.12), written for
// Swift: the record is the struct the schema's own unit declares
// (`<Schema>Payload`), taken and returned by value, and an algorithm that holds a
// record of a schema Swift does not write, or goes past a record into a construct it
// does not lower yet, is refused by that construct and never emitted wrong.

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
fn a_record_is_taken_as_the_struct_of_its_schemas_unit() {
    let code = swift_of(&format!("{FIXTURES}/algorithm_ledger_amount.scxml"))
        .expect("an algorithm of a record in and a number out is written");
    assert!(
        code.contains("public static func ledgerAmount(_ line: SchemaEntryPayload) -> Int32"),
        "{code}"
    );
    assert!(code.contains("return line.amount"), "{code}");
}

#[test]
fn a_record_is_built_from_its_fields_and_returned() {
    let code = swift_of("stdlib/text/text_slot_latest.scxml")
        .expect("an algorithm of records in and a record out is written");
    assert!(
        code.contains(
            "func textSlotLatest(_ a: TextSlotPayload, _ b: TextSlotPayload) -> TextSlotPayload"
        ),
        "{code}"
    );
    // Built whole through the struct's labelled `init`, in the schema's order, and
    // a `var` because a field is written afterwards.
    assert!(
        code.contains(
            "var out: TextSlotPayload = TextSlotPayload(slot: a.slot, version: a.version, text: a.text)"
        ),
        "{code}"
    );
    assert!(code.contains("out.version = b.version"), "{code}");
    assert!(code.contains("return out"), "{code}");
}

#[test]
fn a_text_is_compared_by_its_bytes_and_not_by_swifts_equality() {
    let code = swift_of("stdlib/text/text_slot_latest.scxml").expect("written");
    assert!(
        code.contains("b.text.utf8.elementsEqual(a.text.utf8)"),
        "Swift's `==` takes two spellings of a character for one text: {code}"
    );
}

#[test]
fn a_schema_swift_does_not_write_is_named_as_the_reason() {
    // `schema_goal` declares an optional field, which Swift writes no struct for yet,
    // so no algorithm is written against its payload type.
    let refusal = swift_of(&format!("{FIXTURES}/algorithm_goal_earliest.scxml"))
        .expect_err("an algorithm that imports a schema Swift does not write is refused");
    assert!(
        refusal.contains("Swift") && refusal.contains("optional field"),
        "{refusal}"
    );
}

#[test]
fn an_optional_record_return_is_refused_by_the_return() {
    let refusal = swift_of(&format!("{FIXTURES}/algorithm_item_pick_first.scxml"))
        .expect_err("an optional record is not lowered for Swift yet");
    assert!(
        refusal.contains("Swift") && refusal.contains("optional<record:Item>"),
        "{refusal}"
    );
}
