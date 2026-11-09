// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4, §11.9,
//! §12.6 slice 6): the file of each accepted template, named by the
//! document's file, declares `Value`, a `matches` that takes the template's
//! formal parameters in order, and, for a specific template alone, a `value`;
//! each matching mechanism is spelled over the record types the event schema's
//! unit declares, every enum, record and union named through the sibling
//! module of its own document. A template whose type reaches a union is
//! generated on Rust, which lowers a union (docs/adr/0092), its case matched by
//! the variant of the union's enum.
//!
//! What the generated units do when they run is held by
//! `backends/rust/tests/tests/a_test_template_matches_its_values.rs`, which
//! compiles them.

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

/// The Rust file the template at `path` generates: its one file, `<name>.rs`,
/// named by the document's file.
fn unit(path: &Path, name: &str) -> String {
    let output =
        sce_build::compile_forge_file(path, Language::Rust, &[], &ForgeCompileOptions::default())
            .unwrap_or_else(|e| panic!("{} generates on Rust: {e:?}", path.display()));
    let names: Vec<&str> = output.files.iter().map(|(file, _)| file.as_str()).collect();
    assert_eq!(names, [format!("{name}.rs").as_str()], "{name}");
    output.files[0].1.clone()
}

/// The unit of the fixture `<name>.scxml` of `dir`, a template named `name`.
fn unit_of(dir: &Path, name: &str) -> String {
    unit(&dir.join(format!("{name}.scxml")), name)
}

/// Each fragment of `parts` is in `text`.
fn holds_each(text: &str, parts: &[&str]) {
    for part in parts {
        assert!(text.contains(part), "`{part}` is not in:\n{text}");
    }
}

/// Every accepted template the profile's fixtures hold, the two over a union
/// among them: its document, its name, and whether it is specific
/// (docs/SCE_TEST_PROFILE.md §6.2).
fn templates() -> Vec<(PathBuf, &'static str, bool)> {
    let at = |dir: PathBuf, name: &'static str| (dir.join(format!("{name}.scxml")), name);
    vec![
        (at(profile_dir(), "elapsed_at_most"), false),
        (at(profile_dir(), "every_matcher"), false),
        (at(profile_dir(), "job_report"), true),
        (at(profile_dir(), "job_report_of"), true),
        // Specific in itself, and no specific template through its reference.
        (at(profile_dir(), "report_within"), false),
        (at(profile_dir(), "shape_is_circle"), false),
        (at(imports_dir(), "quantity_at_most"), false),
        (at(imports_dir(), "quantity_two"), true),
        (at(imports_dir(), "order_line_exact"), true),
        (at(imports_dir(), "order_line_bounded"), false),
        (at(imports_dir(), "card_within"), false),
        (at(imports_dir(), "voucher_is"), true),
        (at(imports_dir(), "order_placed_match"), false),
    ]
    .into_iter()
    .map(|((path, name), specific)| (path, name, specific))
    // A unit is named by the document's file (`test-template.scxml`), as every
    // forge unit is (`DocumentLabel::for_input_path`), not by its `name`.
    .chain(std::iter::once((kind_example(), "test_template", false)))
    .collect()
}

/// Each template's unit declares its type and its `matches`, and a `value`
/// exactly when the template is specific; it brings no module into scope.
#[test]
fn each_template_declares_matches_and_a_specific_one_value() {
    for (path, name, specific) in templates() {
        let text = unit(&path, name);
        holds_each(
            &text,
            &["pub type Value = ", "pub fn matches(value: &Value"],
        );
        assert_eq!(
            text.contains("pub fn value("),
            specific,
            "{name}: a value is declared exactly for a specific template:\n{text}"
        );
        assert!(!text.contains("\nuse "), "{name}:\n{text}");
    }
}

/// The template that writes every matcher a record holds: each mechanism as
/// its position's type spells it.
#[test]
fn every_matcher_is_spelled_over_the_schemas_record() {
    let text = unit_of(&profile_dir(), "every_matcher");
    holds_each(
        &text,
        &[
            "pub type Value = super::job_status::JobStatusPayload;",
            "pub fn matches(value: &Value, param_limit: u32) -> bool {",
            // value-list of a specific value and an inclusive range.
            "(value.elapsed_ms == 1 || (value.elapsed_ms >= 10 && value.elapsed_ms <= param_limit))",
            // any-value with ifpresent and a length on an optional text, its
            // characters counted where the text is present.
            "(value.note.is_none() || (value.note.is_some() && (match &value.note { \
             Some(present_0) => present_0.chars().count() <= 8, None => true })))",
            // list-match: its count, its elements in order, a complement as
            // the comparison that does not hold, and its length.
            "value.codes.len() == 3 && value.codes[0] == 3 && value.codes[2] != 0",
            "!value.codes.is_empty() && value.codes.len() <= 4",
            // omit.
            "value.retry.is_none()",
            // A reference binds the template's own parameter.
            "super::elapsed_at_most::matches(&value.count, param_limit)",
        ],
    );
    // any-value-or-none holds whatever the field holds.
    assert!(!text.contains("value.tag"), "{text}");
    // A matcher that reads no element (`any-value` in a list) compares none.
    assert!(!text.contains("value.codes[1]"), "{text}");
}

/// A specific template's value: every field by name, a text and a list built
/// to their bounds, an omitted field absent, an optional one present, and a
/// parameter where the template names one.
#[test]
fn a_specific_templates_value_is_its_record() {
    let report = unit_of(&profile_dir(), "job_report");
    holds_each(
        &report,
        &[
            "pub fn value() -> Value {",
            "super::job_status::JobStatusPayload { elapsed_ms: 0, \
             note: Some(::sce_portable_bytes::sce_text!(32, \"started on time\")), \
             codes: ::sce_portable_bytes::BoundedList::proven(&[7]), \
             tag: None, retry: None, count: 1 }",
            "(match &value.note { Some(present_0) => *present_0 == \"started on time\", \
             None => false })",
            "value.codes.len() == 1 && value.codes[0] == 7",
        ],
    );

    let report_of = unit_of(&profile_dir(), "job_report_of");
    holds_each(
        &report_of,
        &[
            // A number is passed by value, a text lent.
            "pub fn value(param_elapsed: u32, \
             param_note: &::sce_portable_bytes::SceString<32>, param_code: u8) -> Value {",
            "value.elapsed_ms == param_elapsed",
            "(match &value.note { Some(present_0) => *present_0 == *param_note, None => false })",
            "value.codes[0] == param_code",
            "value.codes[1] == 255",
            "(match &value.retry { Some(present_0) => *present_0 == 2, None => false })",
            "value.count == 4294967295",
            "note: Some(param_note.clone())",
            "codes: ::sce_portable_bytes::BoundedList::proven(&[param_code, 255])",
            "retry: Some(2)",
            "count: 4294967295",
        ],
    );
}

/// An enum literal is the variant of the enum the record's document imports,
/// named through that enum's own module; a reference to a specific template
/// builds with its `value` and matches with its `matches`; a byte string is
/// compared as its octets and built as the `Vec<u8>` its record holds.
#[test]
fn an_enum_a_reference_and_bytes_are_spelled_by_their_units() {
    let exact = unit_of(&imports_dir(), "order_line_exact");
    holds_each(
        &exact,
        &[
            "value.sku == \"A-1\"",
            "super::quantity_two::matches(&value.quantity)",
            "value.color == super::paint_color::PaintColor::Red",
            "sku: ::sce_portable_bytes::sce_text!(16, \"A-1\")",
            "quantity: super::quantity_two::value()",
            "color: super::paint_color::PaintColor::Red",
        ],
    );
    let voucher = unit_of(&imports_dir(), "voucher_is");
    holds_each(
        &voucher,
        &[
            r#"value.voucher_code[..] == b"\x00\xff\x10\xa0"[..]"#,
            r#"voucher_code: b"\x00\xff\x10\xa0".to_vec()"#,
        ],
    );
}

/// A range's literal bound at its type's own limit is left out, since it
/// holds every value; a length counts a text's characters.
#[test]
fn a_range_bound_at_the_limit_is_left_out_and_a_text_length_counts_characters() {
    let example = unit(&kind_example(), "test_template");
    assert!(
        example.contains("    value.elapsed_ms <= param_limit\n"),
        "{example}"
    );
    let card = unit_of(&imports_dir(), "card_within");
    holds_each(
        &card,
        &[
            "value.last4.chars().count() >= 2 && value.last4.chars().count() <= 3",
            "value.amount >= 1 && value.amount <= param_most",
        ],
    );
    let at_most = unit_of(&profile_dir(), "elapsed_at_most");
    holds_each(
        &at_most,
        &["pub type Value = u32;", "    *value <= param_limit\n"],
    );
}

/// A template over a union matches the union's case by its variant, the
/// case's record bound for the tests of its matcher; a case its matcher reads
/// nothing of, and a case that holds no record, are matched by the variant
/// alone. Rust lowers a union, so the template is generated, where C++
/// refuses it.
#[test]
fn a_union_is_matched_by_the_variant_of_its_case() {
    let circle = unit_of(&profile_dir(), "shape_is_circle");
    holds_each(
        &circle,
        &[
            "pub type Value = super::union_shape_minimal::UnionShapeMinimal;",
            "    matches!(value, super::union_shape_minimal::UnionShapeMinimal::Circle(_))\n",
        ],
    );
    let placed = unit_of(&imports_dir(), "order_placed_match");
    holds_each(
        &placed,
        &[
            "pub fn matches(value: &Value, param_limit: u16) -> bool {",
            "(matches!(&value.payment, super::order_payment::OrderPayment::Card(case_0) \
             if case_0.last4.chars().count() >= 4 && case_0.amount >= 1) \
             || matches!(&value.payment, super::order_payment::OrderPayment::Cash))",
            // A list of records, each element's fields read in place.
            "value.lines.len() == 2 && value.lines[0].sku == \"A-1\" \
             && super::quantity_at_most::matches(&value.lines[0].quantity, param_limit) \
             && value.lines[0].color == super::paint_color::PaintColor::Green \
             && super::order_line_exact::matches(&value.lines[1]) && value.lines.len() <= 4",
            // A range with ifpresent on an optional number.
            "(value.priority.is_none() || (match &value.priority { \
             Some(present_0) => *present_0 >= 1 && *present_0 <= 5, None => false }))",
            // A `bool` literal is the field itself.
            " && value.customer.vip && ",
        ],
    );
}

/// The runtime's writers of the canonical form of a value in the test log.
const CANONICAL: &str = "::sce_rust_runtime::test::canonical";

/// docs/SCE_TEST_PROFILE.md §12.5: a template over a record, the type of a
/// message, writes its value's canonical form in the test log by the types the
/// schema declares, each field by its declared id, an optional one absent where
/// it is; a template over another type writes none.
#[test]
fn a_template_over_a_record_writes_its_values_canonical_form() {
    for (path, name, _) in templates() {
        let text = unit(&path, name);
        let over_record = !matches!(
            name,
            "elapsed_at_most" | "shape_is_circle" | "quantity_at_most" | "quantity_two"
        );
        assert_eq!(
            text.contains("pub fn canonical(value: &Value) -> ::sce_rust_runtime::json::Value {"),
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
                &[format!("pub const TYPE: &str = \"{record}\";").as_str()],
            );
        }
    }
    let report = unit_of(&profile_dir(), "job_report");
    let c = CANONICAL;
    let written = format!(
        "{c}::record([(\"elapsed_ms\", Some({c}::integer(value.elapsed_ms))), \
         (\"note\", value.note.as_ref().map(|present_0| {c}::text(present_0.as_str()))), \
         (\"codes\", Some({c}::list(value.codes.as_slice().iter().map(|item_0| \
         {c}::integer(*item_0))))), \
         (\"tag\", value.tag.as_ref().map(|present_0| {c}::integer(*present_0))), \
         (\"retry\", value.retry.as_ref().map(|present_0| {c}::integer(*present_0))), \
         (\"count\", Some({c}::integer(value.count)))])"
    );
    holds_each(&report, &[written.as_str()]);
}

/// The canonical form of a record that holds a list of records, an enum, a
/// nested record and a union: each element's fields read through its binding,
/// an enum and a case by their declared names, a byte string as its octets.
#[test]
fn a_records_canonical_form_reaches_every_type_it_holds() {
    let placed = unit_of(&imports_dir(), "order_placed_match");
    let c = CANONICAL;
    let payment = "super::order_payment::OrderPayment";
    let lines = format!(
        "(\"lines\", Some({c}::list(value.lines.as_slice().iter().map(|item_0| \
         {c}::record([(\"sku\", Some({c}::text(item_0.sku.as_str()))), \
         (\"quantity\", Some({c}::integer(item_0.quantity))), \
         (\"color\", Some({c}::enumerated(item_0.color.declared_name())))])))))"
    );
    let customer = format!(
        "(\"customer\", Some({c}::record([(\"full_name\", \
         Some({c}::text(value.customer.full_name.as_str()))), \
         (\"vip\", Some({c}::boolean(value.customer.vip)))])))"
    );
    let paid = format!(
        "(\"payment\", Some(match &value.payment {{ \
         {payment}::Card(case_0) => {c}::union_case(\"card\", \
         Some({c}::record([(\"last4\", Some({c}::text(case_0.last4.as_str()))), \
         (\"amount\", Some({c}::integer(case_0.amount)))]))), \
         {payment}::Cash => {c}::union_case(\"cash\", None), \
         {payment}::Voucher(case_0) => {c}::union_case(\"voucher\", \
         Some({c}::record([(\"voucher_code\", \
         Some({c}::bytes(case_0.voucher_code.as_slice())))]))) }}))"
    );
    holds_each(&placed, &[lines.as_str(), customer.as_str(), paid.as_str()]);
}
