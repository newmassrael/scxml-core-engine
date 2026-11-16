// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Each lane's slice of docs/adr/0087, "Landing order" item 8, beside the Rust and
//! Kotlin slices (`a_lift_reads_a_text_into_a_record_or_keeps_why_not.rs`,
//! `a_kotlin_machine_lifts_a_text_into_a_record_that_holds_a_list.rs`): a
//! `sce-static` machine holds a variable of `optional<record:R>` whose record holds a
//! list of records, started absent; a `<sce:lift>` writes it whole, read by the
//! composed reader of the record's type through the runtime's whole-value lift (the
//! reader a delivery's field of that type is read by, the record's defaults and the
//! list's two counts with it); and a saved state writes the record and reads it back
//! by its type, the list held to its lower count where it is read back.
//!
//! Held to `sce-build/tests/fixtures/static_datamodel/static_lift_list.scxml`, whose
//! scenario (`scenarios/static_lift_list.json`) each lane's driver replays; this file
//! holds the text the generator writes, which is what those drivers run.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-11-16: the rows of the one table of what each backend
//! lowers (`ComposedLowering::of`) held `records` on Rust and Kotlin only, so such a
//! variable, and every lift into it, was refused by name on every other lane, although
//! each lane's composed walk already read, copied and saved the records a union holds
//! (docs/adr/0099, "Still open (2026-11-16)" item 1).

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

/// The fixture, where it lies beside the schemas it imports.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/static_datamodel/static_lift_list.scxml")
}

/// The machine `language` generates of the fixture: every file of it, one after the
/// other, the file whose name ends in `machine_file` among them.
fn machine(language: Language, machine_file: &str) -> String {
    let output = compile_scxml_lang_typed(
        fixture().to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("{language:?} lowers static_lift_list: {e:?}"));
    let names: Vec<String> = output.files.iter().map(|(name, _)| name.clone()).collect();
    assert!(
        names.iter().any(|name| name.ends_with(machine_file)),
        "the machine's `{machine_file}` is generated among {names:?}"
    );
    output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

/// Pinned by the Go row of `ComposedLowering::of`
/// (`sce-build/src/forge/event_schema_check.rs`), `composed_record_variable_field`
/// and `GoTarget::lift`'s `Lifted::Whole` arm (`sce-build/src/forge/static_lowering.rs`),
/// and `GoComposed::read_list` and `GoComposed::saved_record_ext`
/// (`sce-build/src/forge/composed_lowering/go.rs`): the variable is the machine's own
/// struct in `optional.Of`, the lift reads it whole through `sce.LiftValue`, the list
/// held to both its counts before an item is read, and a saved state writes it by its
/// type and reads it back through `<Type>FromSaved`, the list held to its lower count.
#[test]
fn a_go_machine_holds_the_record_lifts_it_whole_and_saves_it_by_its_type() {
    let code = machine(Language::Go, "_sm.go");
    holds(
        &code,
        &[
            "type StaticLiftListPlanRecord struct {\n",
            "optional.Of[StaticLiftListPlanRecord]",
            "sce.LiftValue(",
            "sce.ReadListWithin(",
            ", 1, 3, func(",
            "func StaticLiftListPlanRecordFromSaved(value any, what string) \
             (StaticLiftListPlanRecord, error) {",
            "sce.SavedReadListWithin(sceSaved0, sceWhat0, 1, 3, StaticLiftListItemRecordFromSaved)",
            "\"answer\", StaticLiftListPlanRecordFromSaved, \
             optional.None[StaticLiftListPlanRecord], optional.Some[StaticLiftListPlanRecord])",
        ],
    );
    assert!(
        !code.contains("sce.LiftRecord("),
        "a record that holds a list is not read field by field:\n{code}"
    );
}
