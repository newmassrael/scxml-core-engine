// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Kotlin `sce-static` machine holds a variable of a record that holds a list of
//! records, and a `<sce:lift>` writes it whole (docs/adr/0087, "Landing order" item
//! 8), as the Rust machine does: the variable is the record's data class over `?`,
//! started at `null`; the lift reads the text by the composed reader of the record's
//! type through the runtime's `liftValue`, the record's defaults and each list's counts
//! with it; and a saved state writes the record and reads it back by its type, a list
//! held to its lower count where it is read back.
//!
//! `scenarios/static_lift_list.json` replays the machine the build generates
//! (`StaticScenarioTest`), and `ARecordThatHoldsAListIsSavedAndRestoredByItsTypeTest`
//! restores it; this file holds the text the generator writes, which is what those
//! two run.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-11-11: the Kotlin row of the one table of what each
//! backend lowers held no `records`, so such a variable, and every lift into it, was
//! refused by name on Kotlin.

use std::path::{Path, PathBuf};

use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The Kotlin machine of `static_lift_list.scxml`, generated where it lies, beside
/// the schemas it imports.
fn lift_list_machine() -> String {
    let path = repo_root().join("sce-build/tests/fixtures/static_datamodel/static_lift_list.scxml");
    let language = sce_build::generator::Language::Kotlin;
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("Kotlin lowers static_lift_list: {e:?}"));
    let names: Vec<String> = output.files.iter().map(|(name, _)| name.clone()).collect();
    output
        .files
        .into_iter()
        .find(|(name, _)| name.ends_with("Sm.kt"))
        .map(|(_, code)| code)
        .unwrap_or_else(|| panic!("the machine's Kotlin is generated among {names:?}"))
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

/// Pinned by `composed_record_variable_field` (`sce-build/src/forge/static_lowering.rs`)
/// and `KotlinComposed::record_def` (`sce-build/src/forge/composed_lowering/kotlin.rs`):
/// the variable is the machine's own record over `?`, started absent, and the record
/// declares the companion its saved form extends.
#[test]
fn the_variable_is_the_records_data_class_over_an_optional_started_absent() {
    let code = lift_list_machine();
    holds(
        &code,
        &[
            "data class StaticLiftListPlanRecord(val title: String, val items: \
             List<StaticLiftListItemRecord>, val priority: UByte) {\n    init {\n",
            "    companion object\n}",
            "private var vAnswer: StaticLiftListPlanRecord? = null",
        ],
    );
}

/// Pinned by `KotlinTarget::lift`'s `Lifted::Whole` arm
/// (`sce-build/src/forge/static_lowering.rs`) and `KotlinComposed::read_record`: the
/// text is read through `liftValue` at the depth of a record of a list of records, by
/// the reader a delivery's field of the record is read by, which reads the default of
/// the field the second revision added and holds the list to both its counts before an
/// item is read.
#[test]
fn the_lift_reads_the_record_whole_by_the_reader_of_its_type() {
    let code = lift_list_machine();
    holds(
        &code,
        &[
            "com.sce.runtime.EventPayload.liftValue(",
            ", 3, 256) { it.record(\"{\\\"priority\\\":3}\") { StaticLiftListPlanRecord(title = \
             it.field(\"title\").let { it.string(16) }, items = it.field(\"items\").let { \
             it.listWithin(1, 3) { it.record(null) { StaticLiftListItemRecord(title = \
             it.field(\"title\").let { it.string(16) }, at = it.field(\"at\").let { it.uint8() \
             }) } } }, priority = it.field(\"priority\").let { it.uint8() }) } }",
            "is com.sce.runtime.EventPayload.LiftOutcome.Lifted -> { vAnswer = sceLift.value; \
             vWhy = null }",
        ],
    );
    assert!(
        !code.contains("EventPayload.liftRecord("),
        "a record that holds a list is not read field by field:\n{code}"
    );
}

/// Pinned by `KotlinComposed::saved_record_ext` and `KotlinSavedReader`
/// (`sce-build/src/forge/composed_lowering/kotlin.rs`) and the `saved_kind` and
/// `saved_elem` of `composed_record_variable_field`: a saved state writes the variable
/// as `null` or as the object of its record's fields, each by its type, and reads it
/// back by its type, the list held to its lower count and each item read by its own
/// record's `fromSaved`.
#[test]
fn a_saved_state_writes_the_record_and_reads_it_back_by_its_type() {
    let code = lift_list_machine();
    holds(
        &code,
        &[
            "fun StaticLiftListPlanRecord.toSaved(): Any = linkedMapOf<String, Any?>(\"title\" \
             to SavedValues.of(this.title), \"items\" to SavedValues.list(this.items) { \
             sceSaved0 -> sceSaved0.toSaved() }, \"priority\" to SavedValues.of(this.priority))",
            "fun StaticLiftListPlanRecord.Companion.fromSaved(value: Any?, what: String): \
             StaticLiftListPlanRecord = StaticLiftListPlanRecord(title = \
             SavedValues.string(SavedValues.field(value, what, \"title\"), \"$what.title\", 16), \
             items = SavedValues.listWithin(SavedValues.field(value, what, \"items\"), \
             \"$what.items\", 1, 3) { sceSaved0, sceWhat0 -> \
             StaticLiftListItemRecord.fromSaved(sceSaved0, sceWhat0) }, priority = \
             SavedValues.uint8(SavedValues.field(value, what, \"priority\"), \"$what.priority\"))",
            "variables += \"answer\" to SavedValues.optional(vAnswer) { it.toSaved() }",
            "SavedValues.optional(savedGrown.variable(\"answer\"), \"answer\") { e, w -> \
             StaticLiftListPlanRecord.fromSaved(e, w) }",
        ],
    );
}
