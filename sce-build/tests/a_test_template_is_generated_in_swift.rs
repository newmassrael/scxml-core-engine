// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Swift unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §11.9, §12.6 slice 6): the file of each accepted template, named by the
//! document's file as every forge unit is, declares a caseless enum of the
//! document's name, whose members are `Value`, a `matches` that takes the
//! template's formal parameters in order, and, for a specific template alone,
//! a `value`. Each matching mechanism is spelled over the payload structs the
//! event schema's unit declares, every enum, record and referenced template
//! named bare, as its unit declares it in the one module. The tests are joined
//! in C's words; a value that may be absent is read through `map`, its absence
//! answered by `??`; a text's length is counted in characters through the
//! profile's runtime, which the unit imports exactly where it counts one.
//! Swift lowers no union (docs/adr/0092), so a template whose type reaches one
//! is refused as C++, Go, Python and C11 refuse it, and a record or an enum
//! whose own Swift unit is not written is refused by what that unit is
//! refused for.
//!
//! What the generated units do when they run is held by
//! `backends/swift/tests/Tests/StaticDatamodelTests/ATestTemplateMatchesItsValuesTests.swift`,
//! which compiles them.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::forge::union_check::NOT_LOWERED_YET;
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

/// The directory of the test profile's documents.
fn profile_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile")
}

/// The directory of the templates over imported records.
fn imports_dir() -> PathBuf {
    profile_dir().join("imports")
}

/// The kind's example, a template over the record of the event schema example.
fn kind_example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("kind-examples/test-template.scxml")
}

/// The Swift unit the template at `path` generates: its one file,
/// `<Unit>.swift`, named by the document's file.
fn unit(path: &Path, unit: &str) -> String {
    let output =
        sce_build::compile_forge_file(path, Language::Swift, &[], &ForgeCompileOptions::default())
            .unwrap_or_else(|e| panic!("{} generates on Swift: {e:?}", path.display()));
    let names: Vec<&str> = output.files.iter().map(|(file, _)| file.as_str()).collect();
    assert_eq!(names, [format!("{unit}.swift").as_str()], "{unit}");
    output.files[0].1.clone()
}

/// The unit of the fixture `<name>.scxml` of `dir`, written as `<Unit>.swift`.
fn unit_of(dir: &Path, name: &str, unit_name: &str) -> String {
    unit(&dir.join(format!("{name}.scxml")), unit_name)
}

/// Each fragment of `parts` is in `text`.
fn holds_each(text: &str, parts: &[&str]) {
    for part in parts {
        assert!(text.contains(part), "`{part}` is not in:\n{text}");
    }
}

/// The import of the profile's runtime, which counts a text's characters.
const TEST_RUNTIME_IMPORT: &str = "\nimport SceTestProfile\n";

/// Every accepted template the profile's fixtures hold that Swift generates:
/// its document, the unit it is written as, and whether it is specific
/// (docs/SCE_TEST_PROFILE.md §6.2). The two over a union are refused
/// ([`a_template_that_reaches_a_union_is_refused_on_swift`]).
fn templates() -> Vec<(PathBuf, &'static str, bool)> {
    let at = |dir: PathBuf, name: &'static str| dir.join(format!("{name}.scxml"));
    vec![
        (at(profile_dir(), "elapsed_at_most"), "ElapsedAtMost", false),
        (at(profile_dir(), "every_matcher"), "EveryMatcher", false),
        (at(profile_dir(), "job_report"), "JobReport", true),
        (at(profile_dir(), "job_report_of"), "JobReportOf", true),
        // Specific in itself, and no specific template through its reference.
        (at(profile_dir(), "report_within"), "ReportWithin", false),
        (
            at(imports_dir(), "quantity_at_most"),
            "QuantityAtMost",
            false,
        ),
        (at(imports_dir(), "quantity_two"), "QuantityTwo", true),
        (
            at(imports_dir(), "order_line_exact"),
            "OrderLineExact",
            true,
        ),
        (
            at(imports_dir(), "order_line_bounded"),
            "OrderLineBounded",
            false,
        ),
        (at(imports_dir(), "card_within"), "CardWithin", false),
        (at(imports_dir(), "voucher_is"), "VoucherIs", true),
        // A unit is named by the document's file (`test-template.scxml`), as
        // every forge unit is (`DocumentLabel::for_input_path`), not by its
        // `name`.
        (kind_example(), "TestTemplate", false),
    ]
}

/// Each template's unit declares the enum of its document, its type and its
/// `matches` as members of it, and a `value` exactly when the template is
/// specific.
#[test]
fn each_template_declares_matches_and_a_specific_one_value() {
    for (path, unit_name, specific) in templates() {
        let text = unit(&path, unit_name);
        let declared = format!("\npublic enum {unit_name} {{\n");
        holds_each(
            &text,
            &[
                declared.as_str(),
                "\n    public typealias Value = ",
                "\n    public static func matches(_ value: Value",
            ],
        );
        assert_eq!(
            text.contains("\n    public static func value("),
            specific,
            "{unit_name}: a value is declared exactly for a specific template:\n{text}"
        );
    }
}

/// The template that writes every matcher a record holds: each mechanism as
/// its position's type spells it, joined in C's words, an optional text read
/// through `map`, and the import of the runtime that counts its characters.
#[test]
fn every_matcher_is_spelled_over_the_schemas_record() {
    let text = unit_of(&profile_dir(), "every_matcher", "EveryMatcher");
    holds_each(
        &text,
        &[
            "\n    public typealias Value = JobStatusPayload\n",
            "\n    public static func matches(_ value: Value, _ param_limit: UInt32) -> Bool {\n",
            // value-list of a specific value and an inclusive range.
            "(value.elapsed_ms == 1 || (value.elapsed_ms >= 10 && value.elapsed_ms <= param_limit))",
            // any-value with ifpresent and a length on an optional text, its
            // characters counted through the profile's runtime where the text
            // is present, which `map` binds.
            "(value.note == nil || (value.note != nil && value.note.map { present_0 -> Bool in \
             SceTestProfile.textLength(present_0) <= 8 } ?? true))",
            // list-match: its count, its elements in order, a complement as
            // the comparison that does not hold, and its length.
            "value.codes.count == 3 && value.codes[0] == 3 && value.codes[2] != 0",
            "value.codes.count >= 1 && value.codes.count <= 4",
            // omit.
            "value.retry == nil",
            // A reference binds the template's own parameter, and calls the
            // referenced unit through its enum.
            "ElapsedAtMost.matches(value.count, param_limit)",
            TEST_RUNTIME_IMPORT,
        ],
    );
    // any-value-or-none holds whatever the field holds.
    assert!(!text.contains("value.tag"), "{text}");
    // A matcher that reads no element (`any-value` in a list) compares none.
    assert!(!text.contains("value.codes[1]"), "{text}");
}

/// A specific template's value: the payload struct's initializer of every
/// field, an omitted field `nil` and a present one as its value, and a
/// parameter where the template names one.
#[test]
fn a_specific_templates_value_is_its_record() {
    let report = unit_of(&profile_dir(), "job_report", "JobReport");
    holds_each(
        &report,
        &[
            "\n    public static func value() -> Value {\n",
            "JobStatusPayload(elapsed_ms: 0, note: \"started on time\", codes: [7], tag: nil, \
             retry: nil, count: 1)",
            "value.note.map { present_0 -> Bool in present_0 == \"started on time\" } ?? false",
            "value.codes.count == 1 && value.codes[0] == 7",
            "value.tag == nil && value.retry == nil && value.count == 1",
        ],
    );
    // No text is counted, so the runtime is not imported.
    assert!(!report.contains("import SceTestProfile"), "{report}");
    assert!(!report.contains("\nimport "), "{report}");

    let report_of = unit_of(&profile_dir(), "job_report_of", "JobReportOf");
    holds_each(
        &report_of,
        &[
            "\n    public static func matches(_ value: Value, _ param_elapsed: UInt32, \
             _ param_note: String, _ param_code: UInt8) -> Bool {\n",
            "\n    public static func value(_ param_elapsed: UInt32, _ param_note: String, \
             _ param_code: UInt8) -> Value {\n",
            "value.elapsed_ms == param_elapsed",
            "value.note.map { present_0 -> Bool in present_0 == param_note } ?? false",
            "value.codes[0] == param_code",
            "value.codes[1] == 255",
            "value.retry.map { present_0 -> Bool in present_0 == 2 } ?? false",
            "value.count == 4294967295",
            "JobStatusPayload(elapsed_ms: param_elapsed, note: param_note, codes: [param_code, 255], \
             tag: nil, retry: 2, count: 4294967295)",
        ],
    );
}

/// An enum literal is the case of the enum the record's document imports,
/// named by its type; a reference to a specific template builds with its
/// `value` and matches with its `matches`, each through the referenced
/// template's enum; a byte string is compared and built as the array of its
/// octets.
#[test]
fn an_enum_a_reference_and_bytes_are_spelled_by_their_units() {
    let exact = unit_of(&imports_dir(), "order_line_exact", "OrderLineExact");
    holds_each(
        &exact,
        &[
            "\n    public typealias Value = OrderLinePayload\n",
            "value.sku == \"A-1\"",
            "QuantityTwo.matches(value.quantity)",
            "value.color == PaintColor.red",
            "OrderLinePayload(sku: \"A-1\", quantity: QuantityTwo.value(), color: PaintColor.red)",
        ],
    );
    let voucher = unit_of(&imports_dir(), "voucher_is", "VoucherIs");
    holds_each(
        &voucher,
        &[
            "value.voucher_code == [0x00, 0xff, 0x10, 0xa0]",
            "VoucherPaymentPayload(voucher_code: [0x00, 0xff, 0x10, 0xa0])",
        ],
    );
}

/// A range's literal bound at its type's own limit is left out, since it holds
/// every value; a length counts a text's characters through the profile's
/// runtime; a template that counts no text imports nothing.
#[test]
fn a_range_bound_at_the_limit_is_left_out_and_a_text_length_counts_characters() {
    let example = unit(&kind_example(), "TestTemplate");
    holds_each(
        &example,
        &[
            // A unit is named by its file's stem, so the record of the
            // example `event-schema.scxml` is `EventSchemaPayload`.
            "\n    public typealias Value = EventSchemaPayload\n",
            "\n        return value.elapsed_ms <= param_limit\n",
        ],
    );
    let card = unit_of(&imports_dir(), "card_within", "CardWithin");
    holds_each(
        &card,
        &[
            "SceTestProfile.textLength(value.last4) >= 2 && \
             SceTestProfile.textLength(value.last4) <= 3",
            "value.amount >= 1 && value.amount <= param_most",
            TEST_RUNTIME_IMPORT,
        ],
    );
    let at_most = unit_of(&profile_dir(), "elapsed_at_most", "ElapsedAtMost");
    holds_each(
        &at_most,
        &[
            "\n    public typealias Value = UInt32\n",
            "\n        return value <= param_limit\n",
        ],
    );
    assert!(!at_most.contains("\nimport "), "{at_most}");
}

/// The detail of a `generate/unsupported-feature` refusal.
fn unsupported(name: &str, err: Located<ForgeError>) -> String {
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{name} on Swift: {err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{name} on Swift: {other:?}"),
        },
        other => panic!("{name} on Swift: {other:?}"),
    }
}

/// The refusal of the template at `path` on Swift, which must be refused.
fn refused(path: &Path) -> String {
    let name = path.file_stem().and_then(|s| s.to_str()).expect("a stem");
    let Err(err) =
        sce_build::compile_forge_file(path, Language::Swift, &[], &ForgeCompileOptions::default())
    else {
        panic!("{name} generated on Swift");
    };
    unsupported(name, err)
}

/// A template whose type reaches a union, in itself or below a field, is
/// refused on Swift by the union's own refusal, which a schema unit's union
/// field is refused by: Swift lowers no union. What the unit spells once it
/// does is held by the tests of `forge::generator::swift_test_template`.
#[test]
fn a_template_that_reaches_a_union_is_refused_on_swift() {
    for (path, alias) in [
        (profile_dir().join("shape_is_circle.scxml"), "Shape"),
        (imports_dir().join("order_placed_match.scxml"), "Payment"),
    ] {
        let name = path.file_stem().and_then(|s| s.to_str()).expect("a stem");
        let detail = refused(&path);
        assert!(
            detail.starts_with(&format!("test-template '{name}': `union:{alias}`: ")),
            "{detail}"
        );
        assert!(detail.contains(NOT_LOWERED_YET), "{detail}");
    }
}

/// The record schema the documents of a test of its own import: two `bool`
/// fields, a byte string, and a number that may be absent, each a field the
/// Swift unit of an event schema writes.
const BLOB_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="blob_record"
       sce:event-name="blob.record">
  <datamodel>
    <data id="ready" sce:type="bool" sce:direction="in"/>
    <data id="done" sce:type="bool" sce:direction="in"/>
    <data id="code" sce:type="bytes[4]" sce:direction="in"/>
    <data id="spare" sce:type="optional&lt;uint8&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// Writes `text` as `<name>.scxml` into `dir`, and answers its path.
fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(format!("{name}.scxml"));
    std::fs::write(&path, text).unwrap_or_else(|e| panic!("{name}: {e}"));
    path
}

/// Writes the schema and the template `text` as `<name>.scxml` into `dir`,
/// and answers the template's path.
fn write_template(dir: &Path, name: &str, text: &str) -> PathBuf {
    write(dir, "blob_record", BLOB_SCHEMA);
    write(dir, name, text)
}

/// A `bool` compared with a literal is read, or read under `!`, a literal of
/// one is `true` or `false`, and a matcher that holds whatever the value is
/// returns `true`. The documents are a test's alone, written into a directory
/// of their own.
#[test]
fn a_bool_and_a_test_that_always_holds_are_spelled_in_swift() {
    let dir = tempfile::tempdir().expect("a directory for the documents");
    let flags = write_template(
        dir.path(),
        "flags_set",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="flags_set"
       sce:type="record:Blob">
  <sce:import kind="event-schema" src="blob_record.scxml" as="Blob"/>
  <sce:record-match>
    <sce:specific field="ready" value="true"/>
    <sce:specific field="done" value="false"/>
    <sce:specific field="code" value="0a0b0c0d"/>
    <sce:omit field="spare"/>
  </sce:record-match>
</scxml>
"#,
    );
    let text = unit(&flags, "FlagsSet");
    holds_each(
        &text,
        &[
            "\n        return value.ready && !value.done && \
             value.code == [0x0a, 0x0b, 0x0c, 0x0d] && value.spare == nil\n",
            "BlobRecordPayload(ready: true, done: false, code: [0x0a, 0x0b, 0x0c, 0x0d], \
             spare: nil)",
        ],
    );
    let anything = write_template(
        dir.path(),
        "any_blob",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="any_blob"
       sce:type="record:Blob">
  <sce:import kind="event-schema" src="blob_record.scxml" as="Blob"/>
  <sce:record-match>
    <sce:any-value field="ready"/>
    <sce:any-value field="done"/>
    <sce:any-value field="code"/>
    <sce:any-value-or-none field="spare"/>
  </sce:record-match>
</scxml>
"#,
    );
    let text = unit(&anything, "AnyBlob");
    assert!(text.contains("\n        return true\n"), "{text}");
}

/// A byte string a parameter names, and a number that may be absent, are
/// compared by `==`, which compares a Swift value by its contents and holds
/// two absent values equal, and a value holds each as given, since each is a
/// value type. The documents are a test's alone, written into a directory of
/// their own.
#[test]
fn a_byte_string_and_an_optional_parameter_are_compared_by_their_contents() {
    let dir = tempfile::tempdir().expect("a directory for the documents");
    let template = write_template(
        dir.path(),
        "blob_is",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="blob_is"
       sce:type="record:Blob">
  <sce:import kind="event-schema" src="blob_record.scxml" as="Blob"/>
  <datamodel>
    <data id="code" sce:type="bytes[4]" sce:direction="in"/>
    <data id="spare" sce:type="optional&lt;uint8&gt;" sce:direction="in"/>
  </datamodel>
  <sce:record-match>
    <sce:specific field="ready" value="true"/>
    <sce:specific field="done" value="true"/>
    <sce:specific field="code" param="code"/>
    <sce:specific field="spare" param="spare"/>
  </sce:record-match>
</scxml>
"#,
    );
    let text = unit(&template, "BlobIs");
    holds_each(
        &text,
        &[
            "\n    public static func matches(_ value: Value, _ param_code: [UInt8], \
             _ param_spare: UInt8?) -> Bool {\n",
            "value.code == param_code && value.spare == param_spare\n",
            "BlobRecordPayload(ready: true, done: true, code: param_code, spare: param_spare)",
        ],
    );
}

/// A record whose schema holds a byte string that may be absent, which the
/// Swift unit of an event schema does not write.
const SPARE_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="spare_record"
       sce:event-name="spare.record">
  <datamodel>
    <data id="code" sce:type="bytes[4]" sce:direction="in"/>
    <data id="spare" sce:type="optional&lt;bytes[4]&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// An enum of an open set of variants, which no record, algorithm or machine
/// of Swift holds a value of.
const OPEN_LEVEL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="level_open"
       sce:underlying-type="uint8" sce:strict-variants="false">
  <datamodel>
    <data id="variants">
      <sce:variant name="low" value="0"/>
      <sce:variant name="high" value="1"/>
    </data>
  </datamodel>
</scxml>
"#;

/// A record whose one field is a value of the open enum.
const GAUGE_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="gauge_record"
       sce:event-name="gauge.record">
  <sce:import kind="enum" src="level_open.scxml" as="Level"/>
  <datamodel>
    <data id="level" sce:type="enum:Level" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A record whose one field is a list of the record with the byte string that
/// may be absent.
const HOLDER_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="holder_record"
       sce:event-name="holder.record">
  <sce:import kind="event-schema" src="spare_record.scxml" as="Spare"/>
  <datamodel>
    <data id="items" sce:type="list&lt;record:Spare&gt;[2]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A template of the type `ty` that imports `import` and holds `matcher`.
fn template_over(name: &str, ty: &str, import: &str, matcher: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="{name}"
       sce:type="{ty}">
  {import}
  {matcher}
</scxml>
"#
    )
}

/// A template whose type holds, at any depth, a record or an enum whose own
/// Swift unit is not written is refused by name, by what that unit is refused
/// for, rather than written against a type nothing declares: a record with a
/// byte string that may be absent, an enum of an open set of variants, a
/// record with a field of one, and a record below a list in another record.
/// The documents are a test's alone, written into a directory of their own.
#[test]
fn a_record_or_an_enum_swift_does_not_write_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("a directory for the documents");
    write(dir.path(), "spare_record", SPARE_SCHEMA);
    write(dir.path(), "level_open", OPEN_LEVEL);
    write(dir.path(), "gauge_record", GAUGE_SCHEMA);
    write(dir.path(), "holder_record", HOLDER_SCHEMA);
    let open = "an open set of variants (`sce:strict-variants=\"false\"`)";
    let cases = [
        (
            "over_spare",
            "record:Spare",
            r#"<sce:import kind="event-schema" src="spare_record.scxml" as="Spare"/>"#,
            r#"<sce:record-match>
    <sce:any-value field="code"/>
    <sce:omit field="spare"/>
  </sce:record-match>"#,
            "`record:Spare`, a record with the optional field `spare`, has no Swift lowering yet"
                .to_string(),
        ),
        (
            "open_level",
            "enum:Level",
            r#"<sce:import kind="enum" src="level_open.scxml" as="Level"/>"#,
            "<sce:any-value/>",
            format!("`enum:Level`, an enum with {open}, has no Swift lowering yet"),
        ),
        (
            "gauge_any",
            "record:Gauge",
            r#"<sce:import kind="event-schema" src="gauge_record.scxml" as="Gauge"/>"#,
            r#"<sce:record-match>
    <sce:any-value field="level"/>
  </sce:record-match>"#,
            format!(
                "`record:Gauge`, a record with the field `level` of an enum with {open}, has no \
                 Swift lowering yet"
            ),
        ),
        (
            "holder_any",
            "record:Holder",
            r#"<sce:import kind="event-schema" src="holder_record.scxml" as="Holder"/>"#,
            r#"<sce:record-match>
    <sce:any-value field="items"/>
  </sce:record-match>"#,
            "`record:Spare`, a record with the optional field `spare`, has no Swift lowering yet"
                .to_string(),
        ),
    ];
    for (name, ty, import, matcher, why) in cases {
        let path = write(dir.path(), name, &template_over(name, ty, import, matcher));
        assert_eq!(
            refused(&path),
            format!("test-template '{name}': {why}"),
            "{name}"
        );
    }
}
