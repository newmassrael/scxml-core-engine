// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

/// Pinned by the Python row of `ComposedLowering::of`, `PythonTarget::lift`'s
/// `Lifted::Whole` arm, and `PythonComposed::read_list` and
/// `PythonComposed::saved_record_ext` (`sce-build/src/forge/composed_lowering/python.rs`):
/// the variable is the machine's frozen record class in `Optional`, the lift reads it
/// whole through `lift_value` by the runtime's name of the record's type, the list held
/// to both its counts, and the class is given `to_saved` and `from_saved`, the list read
/// back held to its lower count.
#[test]
fn a_python_machine_holds_the_record_lifts_it_whole_and_saves_it_by_its_type() {
    let code = machine(Language::Python, "_sm.py");
    holds(
        &code,
        &[
            "class StaticLiftListPlanRecord:\n",
            "Optional[StaticLiftListPlanRecord]",
            "sce_event_payload.lift_value(",
            "sce_event_payload.ListOf(",
            ", 3, least=1)",
            "StaticLiftListPlanRecord.to_saved = _StaticLiftListPlanRecord_to_saved\n",
            "StaticLiftListPlanRecord.from_saved = \
             classmethod(_StaticLiftListPlanRecord_from_saved)",
            "sce_saved.saved_read_list_within(sce_saved0, sce_what0, 1, 3, \
             StaticLiftListItemRecord.from_saved)",
        ],
    );
    assert!(
        !code.contains("sce_event_payload.lift_record("),
        "a record that holds a list is not read field by field:\n{code}"
    );
}

/// Pinned by the C++ row of `ComposedLowering::of`, `CppTarget::lift`'s `Lifted::Whole`
/// arm, and `CppComposed::read_list` and `CppComposed::saved_record_ext`
/// (`sce-build/src/forge/composed_lowering/cpp.rs`): the variable is the machine's own
/// struct in `std::optional`, the lift reads it whole through `liftValue`, the list held
/// to both its counts, and a saved state writes it by `<Type>ToSaved` and reads it back
/// by `<Type>FromSaved`, the list held to its lower count.
#[test]
fn a_cpp_machine_holds_the_record_lifts_it_whole_and_saves_it_by_its_type() {
    let code = machine(Language::Cpp, "_sm.h");
    holds(
        &code,
        &[
            "struct StaticLiftListPlanRecord {\n",
            "std::optional<StaticLiftListPlanRecord>",
            "::SCE::Common::EventPayloadFields::liftValue(",
            "sceValue.readListWithin(1u, 3u, sceInto, ",
            "inline ::SCE::Saved::JsonValue StaticLiftListPlanRecordToSaved(const \
             StaticLiftListPlanRecord &value) {",
            "inline StaticLiftListPlanRecord StaticLiftListPlanRecordFromSaved(const \
             ::SCE::Saved::JsonValue &value, const std::string &what) {",
            "::SCE::Saved::savedReadOwnedListWithin<3>(sceSaved0, sceWhat0, 1u, \
             StaticLiftListItemRecordFromSaved)",
        ],
    );
    assert!(
        !code.contains("EventPayloadFields::liftRecord("),
        "a record that holds a list is not read field by field:\n{code}"
    );
}

/// Pinned by the Swift row of `ComposedLowering::of`, `SwiftTarget::lift`'s
/// `Lifted::Whole` arm, and `SwiftComposed::read_list`, `SwiftComposed::record_def` and
/// `SwiftComposed::saved_record_ext` (`sce-build/src/forge/composed_lowering/swift.rs`):
/// the variable is the machine's own struct, built whole by its memberwise initializer
/// (docs/adr/0099, section 4), over `?`; the lift reads it whole through `liftValue`,
/// the list held to both its counts; and the struct's extension writes it by
/// `toSaved` and reads it back by `fromSaved`, the list held to its lower count.
#[test]
fn a_swift_machine_holds_the_record_lifts_it_whole_and_saves_it_by_its_type() {
    let code = machine(Language::Swift, "_sm.swift");
    holds(
        &code,
        &[
            "public struct StaticLiftListPlanRecord: Equatable, Sendable {\n",
            "StaticLiftListPlanRecord?",
            "liftValue(",
            "try sceValue.list(least: 1, bound: 3, ",
            "extension StaticLiftListPlanRecord {\n",
            "public static func fromSaved(_ value: JSONValue, _ what: String) throws -> \
             StaticLiftListPlanRecord {",
            "try SavedValues.list(try SavedValues.field(value, what, \"items\"), \
             \"\\(what).items\", least: 1, capacity: 3) { sceSaved0, sceWhat0 in try \
             StaticLiftListItemRecord.fromSaved(sceSaved0, sceWhat0) }",
            "SavedValues.optional(answer) { $0.toSaved() }",
        ],
    );
    assert!(
        !code.contains("liftRecord("),
        "a record that holds a list is not read field by field:\n{code}"
    );
}

/// Pinned by the C11 row of `ComposedLowering::of`, `composed_record_variable_field`
/// (the optional value of the record declared after it, `ComposedSpelling::take_declared`),
/// `CTarget::lift`'s `Lifted::Whole` arm, and `C11Composed::read_list` and
/// `C11Composed::saved_record_ext` (`sce-build/src/forge/composed_lowering/c11.rs`): the
/// variable is the struct of a flag and the machine's own record, the lift reads it
/// whole through `sce_payload_lift_value`, the list opened held to both its counts, and
/// a saved state writes the record by its type and reads it back, the list held to its
/// lower count.
#[test]
fn a_c11_machine_holds_the_record_lifts_it_whole_and_saves_it_by_its_type() {
    let code = machine(Language::C11, "_sm.h");
    holds(
        &code,
        &[
            "} static_lift_list_record_plan_t;",
            "sce_optional_record_static_lift_list_record_plan_t",
            "sce_payload_lift_value(",
            "sce_payload_list_open_within(sce_value, 1u, 3u, &sce_cursor)",
            "static inline void static_lift_list_record_plan_t_saved_write(sce_jw_t *w, const \
             static_lift_list_record_plan_t *value) {",
            "static inline bool static_lift_list_record_plan_t_saved_read(sce_jv_t v, const char \
             *what, unsigned at, static_lift_list_record_plan_t *out,",
            "sce_saved_list_within(member, member_what, 1u, 3u, &count_",
        ],
    );
    assert!(
        !code.contains("sce_payload_lift("),
        "a record that holds a list is not read field by field:\n{code}"
    );
}

/// Pinned by the Interpreter's row (`ComposedLowering::ECMASCRIPT`,
/// `sce-build/src/forge/event_schema_check.rs`) and `JsTarget::lift`'s
/// `Lifted::Whole` arm (`sce-build/src/forge/static_js.rs`): the Interpreter lowers
/// the machine, the lift handing the text to the library's `lift` with the reader of
/// the record's whole type and writing the variable with the value it answers, the
/// plain object a scenario compares; the Interpreter saves no state. The
/// Interpreter's replay finds `scenarios/static_lift_list.json` by scanning its
/// directory (`AStaticDatamodelRunsLoweredUnderTheInterpreterTest`).
#[test]
fn the_interpreter_holds_the_record_and_lifts_it_whole() {
    let path = fixture();
    let lowered =
        sce_build::forge::static_js::lower_file(path.to_str().expect("a utf-8 path"), Vec::new())
            .unwrap_or_else(|e| panic!("the Interpreter lowers static_lift_list: {e:?}"));
    holds(
        &lowered,
        &[
            "SceStatic.lift(_event.data.text, ",
            "if (sceLift.refusal === null) { answer = sceLift.value; why = null; }",
        ],
    );
}
