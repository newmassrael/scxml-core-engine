// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Kotlin unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §11.9, §12.6 slice 6): the file of each accepted template, named by the
//! document's file, declares `Value`, a `matches` that takes the template's
//! formal parameters in order, and, for a specific template alone, a `value`;
//! each matching mechanism is spelled over the record types the event schema's
//! unit declares, every enum, record, union and referenced template named by
//! the package of its own document. A template whose type reaches a union is
//! generated on Kotlin, which lowers a union (docs/adr/0092), its case matched
//! by an `is` test of the case's class. A byte string is compared by its
//! octets, since Kotlin's `==` compares a `ByteArray` by its identity.
//!
//! What the generated units do when they run is held by
//! `backends/kotlin/tests/src/test/kotlin/com/sce/integration/ATestTemplateMatchesItsValuesTest.kt`,
//! which compiles them.

use std::path::{Path, PathBuf};

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

/// The Kotlin file the template at `path` generates: its one file,
/// `<Name>.kt`, named by the document's file.
fn unit(path: &Path, name: &str, file: &str) -> String {
    let output =
        sce_build::compile_forge_file(path, Language::Kotlin, &[], &ForgeCompileOptions::default())
            .unwrap_or_else(|e| panic!("{} generates on Kotlin: {e:?}", path.display()));
    let names: Vec<&str> = output.files.iter().map(|(file, _)| file.as_str()).collect();
    assert_eq!(names, [file], "{name}");
    output.files[0].1.clone()
}

/// The unit of the fixture `<name>.scxml` of `dir`, a template named `name`
/// whose file is `<file>`.
fn unit_of(dir: &Path, name: &str, file: &str) -> String {
    unit(&dir.join(format!("{name}.scxml")), name, file)
}

/// Each fragment of `parts` is in `text`.
fn holds_each(text: &str, parts: &[&str]) {
    for part in parts {
        assert!(text.contains(part), "`{part}` is not in:\n{text}");
    }
}

/// Every accepted template the profile's fixtures hold, the two over a union
/// among them: its document, its name, its file, and whether it is specific
/// (docs/SCE_TEST_PROFILE.md §6.2).
fn templates() -> Vec<(PathBuf, &'static str, &'static str, bool)> {
    let at = |dir: PathBuf, name: &'static str| dir.join(format!("{name}.scxml"));
    vec![
        (
            at(profile_dir(), "elapsed_at_most"),
            "elapsed_at_most",
            "ElapsedAtMost.kt",
            false,
        ),
        (
            at(profile_dir(), "every_matcher"),
            "every_matcher",
            "EveryMatcher.kt",
            false,
        ),
        (
            at(profile_dir(), "job_report"),
            "job_report",
            "JobReport.kt",
            true,
        ),
        (
            at(profile_dir(), "job_report_of"),
            "job_report_of",
            "JobReportOf.kt",
            true,
        ),
        // Specific in itself, and no specific template through its reference.
        (
            at(profile_dir(), "report_within"),
            "report_within",
            "ReportWithin.kt",
            false,
        ),
        (
            at(profile_dir(), "shape_is_circle"),
            "shape_is_circle",
            "ShapeIsCircle.kt",
            false,
        ),
        (
            at(imports_dir(), "quantity_at_most"),
            "quantity_at_most",
            "QuantityAtMost.kt",
            false,
        ),
        (
            at(imports_dir(), "quantity_two"),
            "quantity_two",
            "QuantityTwo.kt",
            true,
        ),
        (
            at(imports_dir(), "order_line_exact"),
            "order_line_exact",
            "OrderLineExact.kt",
            true,
        ),
        (
            at(imports_dir(), "order_line_bounded"),
            "order_line_bounded",
            "OrderLineBounded.kt",
            false,
        ),
        (
            at(imports_dir(), "card_within"),
            "card_within",
            "CardWithin.kt",
            false,
        ),
        (
            at(imports_dir(), "voucher_is"),
            "voucher_is",
            "VoucherIs.kt",
            true,
        ),
        (
            at(imports_dir(), "order_placed_match"),
            "order_placed_match",
            "OrderPlacedMatch.kt",
            false,
        ),
        // A unit is named by the document's file (`test-template.scxml`), as
        // every forge unit is (`DocumentLabel::for_input_path`), not by its
        // `name`.
        (kind_example(), "test_template", "TestTemplate.kt", false),
    ]
}

/// Each template's unit is in the package of its document and declares its
/// type and its `matches`, and a `value` exactly when the template is
/// specific; it imports no package.
#[test]
fn each_template_declares_matches_and_a_specific_one_value() {
    for (path, name, file, specific) in templates() {
        let text = unit(&path, name, file);
        let package = format!("\npackage com.sce.generated.{name}\n");
        holds_each(
            &text,
            &[
                package.as_str(),
                "\ntypealias Value = ",
                "\nfun matches(value: Value",
            ],
        );
        assert_eq!(
            text.contains("\nfun value("),
            specific,
            "{name}: a value is declared exactly for a specific template:\n{text}"
        );
        assert!(!text.contains("\nimport "), "{name}:\n{text}");
    }
}

/// The template that writes every matcher a record holds: each mechanism as
/// its position's type spells it.
#[test]
fn every_matcher_is_spelled_over_the_schemas_record() {
    let text = unit_of(&profile_dir(), "every_matcher", "EveryMatcher.kt");
    holds_each(
        &text,
        &[
            "typealias Value = com.sce.generated.job_status.JobStatusPayload\n",
            "fun matches(value: Value, param_limit: UInt): Boolean =\n",
            // value-list of a specific value and an inclusive range.
            "(value.elapsed_ms == 1u || (value.elapsed_ms >= 10u && value.elapsed_ms <= param_limit))",
            // any-value with ifpresent and a length on an optional text, its
            // characters counted through the profile's runtime where the text
            // is present.
            "(value.note == null || (value.note != null && value.note.let { present_0 -> \
             present_0 == null || com.sce.runtime.test.textLength(present_0) <= 8 }))",
            // list-match: its count, its elements in order at their width, a
            // complement as the comparison that does not hold, and its length.
            "value.codes.size == 3 && value.codes[0] == 3u.toUByte() \
             && value.codes[2] != 0u.toUByte()",
            "value.codes.size >= 1 && value.codes.size <= 4",
            // omit.
            "value.retry == null",
            // A reference binds the template's own parameter, and names the
            // referenced unit by its package.
            "com.sce.generated.elapsed_at_most.matches(value.count, param_limit)",
        ],
    );
    // The match alone: the unit's `canonical` reads every field, the matched
    // ones and the others alike.
    let matches = text
        .split_once("\nfun matches(")
        .map(|(_, rest)| rest.split_once("\n\n").map_or(rest, |(body, _)| body))
        .unwrap_or_else(|| panic!("no `matches` function:\n{text}"));
    // any-value-or-none holds whatever the field holds.
    assert!(!matches.contains("value.tag"), "{text}");
    // A matcher that reads no element (`any-value` in a list) compares none.
    assert!(!matches.contains("value.codes[1]"), "{text}");
}

/// A specific template's value: every field by name, an omitted field absent,
/// an optional one present, a number at its type's width, and a parameter
/// where the template names one.
#[test]
fn a_specific_templates_value_is_its_record() {
    let report = unit_of(&profile_dir(), "job_report", "JobReport.kt");
    holds_each(
        &report,
        &[
            "fun value(): Value =\n",
            "com.sce.generated.job_status.JobStatusPayload(elapsed_ms = 0u, \
             note = \"started on time\", codes = listOf(7u.toUByte()), tag = null, \
             retry = null, count = 1u)",
            "value.note.let { present_0 -> present_0 != null && present_0 == \"started on time\" }",
            "value.codes.size == 1 && value.codes[0] == 7u.toUByte()",
        ],
    );

    let report_of = unit_of(&profile_dir(), "job_report_of", "JobReportOf.kt");
    holds_each(
        &report_of,
        &[
            "fun value(param_elapsed: UInt, param_note: String, param_code: UByte): Value =\n",
            "value.elapsed_ms == param_elapsed",
            "value.note.let { present_0 -> present_0 != null && present_0 == param_note }",
            "value.codes[0] == param_code",
            "value.codes[1] == 255u.toUByte()",
            "value.retry.let { present_0 -> present_0 != null && present_0 == 2u.toUByte() }",
            "value.count == 4294967295u",
            "note = param_note,",
            "codes = listOf(param_code, 255u.toUByte())",
            "retry = 2u.toUByte()",
            "count = 4294967295u)",
        ],
    );
}

/// An enum literal is the variant of the enum the record's document imports,
/// named by that enum's own package; a reference to a specific template
/// builds with its `value` and matches with its `matches`; a byte string is
/// compared by its octets and built as an array of them.
#[test]
fn an_enum_a_reference_and_bytes_are_spelled_by_their_units() {
    let exact = unit_of(&imports_dir(), "order_line_exact", "OrderLineExact.kt");
    holds_each(
        &exact,
        &[
            "value.sku == \"A-1\"",
            "com.sce.generated.quantity_two.matches(value.quantity)",
            "value.color == com.sce.generated.paint_color.PaintColor.RED",
            "com.sce.generated.order_line.OrderLinePayload(sku = \"A-1\", \
             quantity = com.sce.generated.quantity_two.value(), \
             color = com.sce.generated.paint_color.PaintColor.RED)",
        ],
    );
    let voucher = unit_of(&imports_dir(), "voucher_is", "VoucherIs.kt");
    let octets = "byteArrayOf(0x00.toByte(), 0xff.toByte(), 0x10.toByte(), 0xa0.toByte())";
    let compared = format!("value.voucher_code.contentEquals({octets})");
    let built = format!("voucher_code = {octets})");
    holds_each(&voucher, &[compared.as_str(), built.as_str()]);
}

/// A range's literal bound at its type's own limit is left out, since it
/// holds every value; a length counts a text's characters through the
/// profile's runtime.
#[test]
fn a_range_bound_at_the_limit_is_left_out_and_a_text_length_counts_characters() {
    let example = unit(&kind_example(), "test_template", "TestTemplate.kt");
    assert!(
        example.contains("    value.elapsed_ms <= param_limit\n"),
        "{example}"
    );
    let card = unit_of(&imports_dir(), "card_within", "CardWithin.kt");
    holds_each(
        &card,
        &[
            "com.sce.runtime.test.textLength(value.last4) >= 2 \
             && com.sce.runtime.test.textLength(value.last4) <= 3",
            "value.amount >= 1u && value.amount <= param_most",
        ],
    );
    let at_most = unit_of(&profile_dir(), "elapsed_at_most", "ElapsedAtMost.kt");
    holds_each(
        &at_most,
        &["typealias Value = UInt\n", "    value <= param_limit\n"],
    );
}

/// A template over a union matches the union's case by an `is` test of the
/// case's class, the union bound where the case's record is read; a case its
/// matcher reads nothing of, and a case that holds no record, are matched by
/// the test alone. Kotlin lowers a union, so the template is generated, where
/// C++ refuses it.
#[test]
fn a_union_is_matched_by_the_class_of_its_case() {
    let circle = unit_of(&profile_dir(), "shape_is_circle", "ShapeIsCircle.kt");
    holds_each(
        &circle,
        &[
            "typealias Value = com.sce.generated.union_shape_minimal.UnionShapeMinimal\n",
            "    value is com.sce.generated.union_shape_minimal.UnionShapeMinimal.Circle\n",
        ],
    );
    let placed = unit_of(&imports_dir(), "order_placed_match", "OrderPlacedMatch.kt");
    holds_each(
        &placed,
        &[
            "fun matches(value: Value, param_limit: UShort): Boolean =\n",
            "(value.payment.let { case_0 -> \
             case_0 is com.sce.generated.order_payment.OrderPayment.Card \
             && com.sce.runtime.test.textLength(case_0.value.last4) >= 4 \
             && case_0.value.amount >= 1u } \
             || value.payment is com.sce.generated.order_payment.OrderPayment.Cash)",
            // A list of records, each element's fields read in place.
            "value.lines.size == 2 && value.lines[0].sku == \"A-1\" \
             && com.sce.generated.quantity_at_most.matches(value.lines[0].quantity, param_limit) \
             && value.lines[0].color == com.sce.generated.paint_color.PaintColor.GREEN \
             && com.sce.generated.order_line_exact.matches(value.lines[1]) \
             && value.lines.size <= 4",
            // A range with ifpresent on an optional number.
            "(value.priority == null || value.priority.let { present_0 -> \
             present_0 != null && present_0 >= 1u.toUByte() && present_0 <= 5u.toUByte() })",
            // A `bool` literal is the field itself.
            " && value.customer.vip && ",
        ],
    );
}

/// A byte string a parameter names, present or that may be absent, is
/// compared by its octets, since Kotlin's `==` compares a `ByteArray` by its
/// identity, and a value is built from a copy of it, so the value holds no
/// array its caller can write into. The documents are a test's alone, written
/// into a directory of their own.
#[test]
fn a_byte_string_parameter_is_compared_by_its_octets_and_built_from_a_copy() {
    let dir = tempfile::tempdir().expect("a directory for the documents");
    std::fs::write(
        dir.path().join("blob_record.scxml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="blob_record"
       sce:event-name="blob.record">
  <datamodel>
    <data id="code" sce:type="bytes[4]" sce:direction="in"/>
    <data id="spare" sce:type="optional&lt;bytes[4]&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#,
    )
    .expect("the record's schema");
    let template = dir.path().join("blob_is.scxml");
    std::fs::write(
        &template,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="blob_is"
       sce:type="record:Blob">
  <sce:import kind="event-schema" src="blob_record.scxml" as="Blob"/>
  <datamodel>
    <data id="code" sce:type="bytes[4]" sce:direction="in"/>
    <data id="spare" sce:type="optional&lt;bytes[4]&gt;" sce:direction="in"/>
  </datamodel>
  <sce:record-match>
    <sce:specific field="code" param="code"/>
    <sce:specific field="spare" param="spare"/>
  </sce:record-match>
</scxml>
"#,
    )
    .expect("the template");
    let text = unit(&template, "blob_is", "BlobIs.kt");
    holds_each(
        &text,
        &[
            "fun matches(value: Value, param_code: ByteArray, param_spare: ByteArray?): Boolean =\n",
            "    value.code.contentEquals(param_code) && value.spare.contentEquals(param_spare)\n",
            "com.sce.generated.blob_record.BlobRecordPayload(code = param_code.copyOf(), \
             spare = param_spare?.copyOf())",
        ],
    );
    assert!(!text.contains("value.code =="), "{text}");
    assert!(!text.contains("value.spare =="), "{text}");
}

/// The runtime's writers of the canonical form of a value in the test log.
const CANONICAL: &str = "com.sce.runtime.test.Canonical";

/// docs/SCE_TEST_PROFILE.md §12.5: a template over a record, the type of a
/// message, writes its value's canonical form in the test log by the types the
/// schema declares, each field by its declared id, an optional one absent where
/// it is; a template over another type writes none. The type's name is the one
/// the Rust unit writes for the same record.
#[test]
fn a_template_over_a_record_writes_its_values_canonical_form() {
    for (path, name, file, _) in templates() {
        let text = unit(&path, name, file);
        let over_record = !matches!(
            name,
            "elapsed_at_most" | "shape_is_circle" | "quantity_at_most" | "quantity_two"
        );
        assert_eq!(
            text.contains("\nfun canonical(value: Value): Any =\n"),
            over_record,
            "{name}: the canonical form is written exactly for a template over a record:\n{text}"
        );
        // The unit names the SCE runtime exactly where it writes the form, and
        // its header says so.
        let runtime = if over_record { "sce_runtime" } else { "none" };
        assert!(
            text.contains(&format!("// Runtime: {runtime}\n")),
            "{name}: the header states the runtime the unit names:\n{text}"
        );
        // A message's type, as the TTCN-3 export's types module names its
        // record: the unit name of the event schema's file.
        let record = match name {
            "every_matcher" | "job_report" | "job_report_of" | "report_within" => {
                Some("job_status")
            }
            "order_line_exact" | "order_line_bounded" => Some("order_line"),
            "card_within" => Some("card_payment"),
            "voucher_is" => Some("voucher_payment"),
            "order_placed_match" => Some("order_placed"),
            // The kind example's schema is the file `event-schema.scxml`.
            "test_template" => Some("event_schema"),
            _ => None,
        };
        assert_eq!(
            record.is_some(),
            over_record,
            "{name}: a record template is named here"
        );
        if let Some(record) = record {
            holds_each(
                &text,
                &[format!("\nconst val TYPE: String = \"{record}\"\n").as_str()],
            );
        }
    }
    let report = unit_of(&profile_dir(), "job_report", "JobReport.kt");
    let c = CANONICAL;
    let written = format!(
        "{c}.record(\"elapsed_ms\" to {c}.integer(value.elapsed_ms), \
         \"note\" to value.note?.let {{ present_0 -> {c}.text(present_0) }}, \
         \"codes\" to {c}.list(value.codes.map {{ item_0 -> {c}.integer(item_0) }}), \
         \"tag\" to value.tag?.let {{ present_0 -> {c}.integer(present_0) }}, \
         \"retry\" to value.retry?.let {{ present_0 -> {c}.integer(present_0) }}, \
         \"count\" to {c}.integer(value.count))"
    );
    holds_each(&report, &[written.as_str()]);
}

/// The canonical form of a record that holds a list of records, an enum, a
/// nested record and a union: each element's fields read through its lambda's
/// parameter, an enum and a case by their declared names, a case's record
/// through the `when` that binds the union, a byte string as its octets.
#[test]
fn a_records_canonical_form_reaches_every_type_it_holds() {
    let placed = unit_of(&imports_dir(), "order_placed_match", "OrderPlacedMatch.kt");
    let c = CANONICAL;
    let payment = "com.sce.generated.order_payment.OrderPayment";
    let lines = format!(
        "\"lines\" to {c}.list(value.lines.map {{ item_0 -> \
         {c}.record(\"sku\" to {c}.text(item_0.sku), \
         \"quantity\" to {c}.integer(item_0.quantity), \
         \"color\" to {c}.enumerated(item_0.color.declaredName())) }})"
    );
    let customer = format!(
        "\"customer\" to {c}.record(\"full_name\" to {c}.text(value.customer.full_name), \
         \"vip\" to {c}.boolean(value.customer.vip))"
    );
    let paid = format!(
        "\"payment\" to when (val case_0 = value.payment) {{ \
         is {payment}.Card -> {c}.unionCase(\"card\", \
         {c}.record(\"last4\" to {c}.text(case_0.value.last4), \
         \"amount\" to {c}.integer(case_0.value.amount))); \
         is {payment}.Cash -> {c}.unionCase(\"cash\", null); \
         is {payment}.Voucher -> {c}.unionCase(\"voucher\", \
         {c}.record(\"voucher_code\" to {c}.bytes(case_0.value.voucher_code))) }}"
    );
    holds_each(&placed, &[lines.as_str(), customer.as_str(), paid.as_str()]);
}
