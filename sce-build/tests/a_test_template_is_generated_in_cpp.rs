// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4, §11.9,
//! §12.6 slice 3): the header of each accepted template declares `Value`, a
//! `match` that takes the template's formal parameters in order, and, for a
//! specific template alone, a `value`; each matching mechanism is spelled over
//! the record types the event schema's unit declares. A template whose type
//! reaches a union is refused on C++ as a schema unit's union field is, and
//! every backend that generates no template refuses one by name.
//!
//! What the generated headers do when they run is held by
//! `tests/integration/ATestTemplateMatchesItsValuesTest.cpp`, which compiles
//! them.

use std::path::{Path, PathBuf};

use sce_build::forge::codegen_matrix::TEST_TEMPLATE_BACKENDS;
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::forge::test_profile::NOT_GENERATED_YET;
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

fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some("example.com/generated".to_string()),
        ..Default::default()
    }
}

/// The kind's example, a template over the record of the event schema example.
fn kind_example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("kind-examples/test-template.scxml")
}

/// The C++ header the template at `path` generates: its one file, `<name>.h`,
/// named by the document's file.
fn header(path: &Path, name: &str) -> String {
    let output = sce_build::compile_forge_file(path, Language::Cpp, &[], &options())
        .unwrap_or_else(|e| panic!("{} generates on C++: {e:?}", path.display()));
    let names: Vec<&str> = output.files.iter().map(|(file, _)| file.as_str()).collect();
    assert_eq!(names, [format!("{name}.h").as_str()], "{name}");
    output.files[0].1.clone()
}

/// The header of the fixture `<name>.scxml` of `dir`, a template named `name`.
fn header_of(dir: &Path, name: &str) -> String {
    header(&dir.join(format!("{name}.scxml")), name)
}

/// Every accepted template the profile's fixtures hold that C++ generates: its
/// document, its name, the namespace of its unit, and whether it is specific
/// (docs/SCE_TEST_PROFILE.md §6.2).
fn templates() -> Vec<(PathBuf, &'static str, &'static str, bool)> {
    let at = |dir: PathBuf, name: &'static str| dir.join(format!("{name}.scxml"));
    vec![
        (
            at(profile_dir(), "elapsed_at_most"),
            "elapsed_at_most",
            "ElapsedAtMost",
            false,
        ),
        (
            at(profile_dir(), "every_matcher"),
            "every_matcher",
            "EveryMatcher",
            false,
        ),
        (
            at(profile_dir(), "job_report"),
            "job_report",
            "JobReport",
            true,
        ),
        (
            at(profile_dir(), "job_report_of"),
            "job_report_of",
            "JobReportOf",
            true,
        ),
        // Specific in itself, and no specific template through its reference.
        (
            at(profile_dir(), "report_within"),
            "report_within",
            "ReportWithin",
            false,
        ),
        (
            at(imports_dir(), "quantity_at_most"),
            "quantity_at_most",
            "QuantityAtMost",
            false,
        ),
        (
            at(imports_dir(), "quantity_two"),
            "quantity_two",
            "QuantityTwo",
            true,
        ),
        (
            at(imports_dir(), "order_line_exact"),
            "order_line_exact",
            "OrderLineExact",
            true,
        ),
        (
            at(imports_dir(), "order_line_bounded"),
            "order_line_bounded",
            "OrderLineBounded",
            false,
        ),
        (
            at(imports_dir(), "card_within"),
            "card_within",
            "CardWithin",
            false,
        ),
        (
            at(imports_dir(), "voucher_is"),
            "voucher_is",
            "VoucherIs",
            true,
        ),
        // A unit is named by the document's file (`test-template.scxml`), as
        // every forge unit is (`DocumentLabel::for_input_path`), not by its
        // `name`: its header and its namespace alike.
        (kind_example(), "test_template", "TestTemplate", false),
    ]
}

/// Each template's header declares its type and its `match` in the namespace
/// of its name, and a `value` exactly when the template is specific.
#[test]
fn each_template_declares_match_and_a_specific_one_value() {
    for (path, name, namespace, specific) in templates() {
        let text = header(&path, name);
        assert!(
            text.contains(&format!("namespace SCE::Generated::{namespace} {{")),
            "{name}:\n{text}"
        );
        assert!(text.contains("using Value = "), "{name}:\n{text}");
        assert!(text.contains("inline bool match("), "{name}:\n{text}");
        assert_eq!(
            text.contains("inline Value value("),
            specific,
            "{name}: a value is declared exactly for a specific template:\n{text}"
        );
    }
}

/// The template that writes every matcher a record holds: each mechanism as
/// its position's type spells it.
#[test]
fn every_matcher_is_spelled_over_the_schemas_record() {
    let text = header_of(&profile_dir(), "every_matcher");
    for part in [
        "#include \"job_status.h\"",
        "#include \"elapsed_at_most.h\"",
        // A text's length is counted in characters by the profile's runtime.
        "#include \"test/Length.h\"",
        "using Value = SCE::Generated::JobStatus::JobStatusPayload;",
        "[[maybe_unused]] const uint32_t &param_limit",
        // value-list of a specific value and an inclusive range.
        "((value.elapsed_ms == 1) || ((value.elapsed_ms >= 10) && (value.elapsed_ms <= param_limit)))",
        // any-value with ifpresent and a length on an optional text.
        "(!value.note.has_value() || (value.note.has_value() && (!value.note.has_value() || \
         (SCE::Test::textLength((*value.note)) <= 8u))))",
        // list-match: its count, its elements in order, any element, a
        // complement, and its length.
        "(value.codes.size() == 3u)",
        "(value.codes[0] == 3)",
        "!(value.codes[2] == 0)",
        "(value.codes.size() >= 1u)",
        "(value.codes.size() <= 4u)",
        // omit.
        "!value.retry.has_value()",
        // A reference binds the template's own parameter.
        "SCE::Generated::ElapsedAtMost::match(value.count, param_limit)",
    ] {
        assert!(text.contains(part), "`{part}` is not in:\n{text}");
    }
    // any-value-or-none holds whatever the field holds.
    assert!(!text.contains("value.tag"), "{text}");
    // A matcher that reads no element (`any-value` in a list) compares none.
    assert!(!text.contains("value.codes[1]"), "{text}");
}

/// A specific template's value: every field in the order its schema declares
/// it, an omitted field absent, a list by its elements, and a parameter or a
/// literal where the template names one.
#[test]
fn a_specific_templates_value_is_its_record_in_declared_order() {
    let report = header_of(&profile_dir(), "job_report");
    assert!(
        report.contains(
            "return SCE::Generated::JobStatus::JobStatusPayload{/*elapsed_ms=*/0, \
             /*note=*/\"started on time\", \
             /*codes=*/decltype(SCE::Generated::JobStatus::JobStatusPayload::codes){7}, \
             /*tag=*/std::nullopt, /*retry=*/std::nullopt, /*count=*/1};"
        ),
        "{report}"
    );
    // No length is restricted, so the profile's runtime is not included.
    assert!(!report.contains("test/Length.h"), "{report}");

    let report_of = header_of(&profile_dir(), "job_report_of");
    for part in [
        "inline Value value([[maybe_unused]] const uint32_t &param_elapsed, \
         [[maybe_unused]] const std::string &param_note, [[maybe_unused]] const uint8_t &param_code)",
        "(value.elapsed_ms == param_elapsed)",
        "(value.note.has_value() && ((*value.note) == param_note))",
        "(value.codes[0] == param_code)",
        "(value.codes[1] == 255)",
        "(value.retry.has_value() && ((*value.retry) == 2))",
        "(value.count == 4294967295)",
        "/*note=*/param_note",
        "decltype(SCE::Generated::JobStatus::JobStatusPayload::codes){param_code, 255}",
        "/*retry=*/2",
        "/*count=*/4294967295",
    ] {
        assert!(report_of.contains(part), "`{part}` is not in:\n{report_of}");
    }
}

/// An enum literal is the enumerator of the type the record's unit declares
/// for the field; a reference to a specific template builds with its `value`
/// and matches with its `match`; a byte string is its octets.
#[test]
fn an_enum_a_reference_and_bytes_are_spelled_by_their_units() {
    let exact = header_of(&imports_dir(), "order_line_exact");
    for part in [
        "#include \"quantity_two.h\"",
        "(value.color == decltype(SCE::Generated::OrderLine::OrderLinePayload::color)::Red)",
        "SCE::Generated::QuantityTwo::match(value.quantity)",
        "/*quantity=*/SCE::Generated::QuantityTwo::value()",
        "/*color=*/decltype(SCE::Generated::OrderLine::OrderLinePayload::color)::Red",
    ] {
        assert!(exact.contains(part), "`{part}` is not in:\n{exact}");
    }
    let voucher = header_of(&imports_dir(), "voucher_is");
    assert!(
        voucher.contains(
            "decltype(SCE::Generated::VoucherPayment::VoucherPaymentPayload::voucher_code)\
             {0x00, 0xff, 0x10, 0xA0}"
        ),
        "{voucher}"
    );
}

/// A range's literal bound at its type's own limit is left out, since it
/// holds every value; a length counts a text's characters.
#[test]
fn a_range_bound_at_the_limit_is_left_out_and_a_text_length_counts_characters() {
    let example = header(&kind_example(), "test_template");
    assert!(
        example.contains("return (value.elapsed_ms <= param_limit);"),
        "{example}"
    );
    let card = header_of(&imports_dir(), "card_within");
    for part in [
        "#include \"test/Length.h\"",
        "(SCE::Test::textLength(value.last4) >= 2u)",
        "(SCE::Test::textLength(value.last4) <= 3u)",
        "(value.amount >= 1)",
        "(value.amount <= param_most)",
    ] {
        assert!(card.contains(part), "`{part}` is not in:\n{card}");
    }
    let at_most = header_of(&profile_dir(), "elapsed_at_most");
    assert!(at_most.contains("using Value = uint32_t;"), "{at_most}");
    assert!(
        at_most.contains("return (value <= param_limit);"),
        "{at_most}"
    );
}

/// The detail of a `generate/unsupported-feature` refusal.
fn unsupported(name: &str, language: Language, err: Located<ForgeError>) -> String {
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{name} on {language:?}: {err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{name} on {language:?}: {other:?}"),
        },
        other => panic!("{name} on {language:?}: {other:?}"),
    }
}

/// A template whose type reaches a union, in itself or below a field, is
/// refused on C++ by the union's own refusal, which a schema unit's union
/// field is refused by.
#[test]
fn a_template_that_reaches_a_union_is_refused_on_cpp() {
    for (path, alias) in [
        (profile_dir().join("shape_is_circle.scxml"), "Shape"),
        (imports_dir().join("order_placed_match.scxml"), "Payment"),
    ] {
        let name = path.file_stem().and_then(|s| s.to_str()).expect("a stem");
        let Err(err) = sce_build::compile_forge_file(&path, Language::Cpp, &[], &options()) else {
            panic!("{name} generated on C++");
        };
        let detail = unsupported(name, Language::Cpp, err);
        assert!(
            detail.starts_with(&format!("test-template '{name}': `union:{alias}`: ")),
            "{detail}"
        );
        assert!(detail.contains(NOT_LOWERED_YET), "{detail}");
    }
}

/// Every backend that does not generate a template
/// (`codegen_matrix::TEST_TEMPLATE_BACKENDS`) refuses one by name, until the
/// profile's runtime lands there.
#[test]
fn every_other_backend_refuses_a_template_by_name() {
    let path = profile_dir().join("job_report.scxml");
    for &language in Language::ALL {
        if TEST_TEMPLATE_BACKENDS.contains(&language) {
            continue;
        }
        let Err(err) = sce_build::compile_forge_file(&path, language, &[], &options()) else {
            panic!("a test template generated on {language:?}");
        };
        let detail = unsupported("job_report", language, err);
        assert!(
            detail.starts_with("test-template 'job_report': "),
            "{detail}"
        );
        assert!(detail.contains(NOT_GENERATED_YET), "{detail}");
    }
}
