// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Go unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §11.9, §12.6 slice 6): the file of each accepted template, named by the
//! document's file, is the package of its document and declares `Value`, a
//! `Matches` that takes the template's formal parameters in order, and, for a
//! specific template alone, a `Specific`; each matching mechanism is spelled
//! over the record types the event schema's unit declares, every enum, record
//! and referenced template named through the package of its own document,
//! which the unit imports exactly where its text names it. A value that may
//! be absent is tested by `IsPresent` and read by `Proven`, and built with its
//! type named; a byte string is compared by its octets, since Go's `==` does
//! not compare slices. Go lowers no union (docs/adr/0092), so a template whose
//! type reaches one is refused as C++ refuses it.
//!
//! What the generated units do when they run is held by
//! `backends/go/tests/integration/test_profile/a_test_template_matches_its_values_test.go`,
//! which compiles them.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::forge::union_check::NOT_LOWERED_YET;
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

/// The module path the generated packages are imported under.
const PREFIX: &str = "example.com/generated";

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

fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some(PREFIX.to_string()),
        ..Default::default()
    }
}

/// The Go file the template at `path` generates: its one file, `<name>.go`,
/// named by the document's file.
fn unit(path: &Path, name: &str) -> String {
    let output = sce_build::compile_forge_file(path, Language::Go, &[], &options())
        .unwrap_or_else(|e| panic!("{} generates on Go: {e:?}", path.display()));
    let names: Vec<&str> = output.files.iter().map(|(file, _)| file.as_str()).collect();
    assert_eq!(names, [format!("{name}.go").as_str()], "{name}");
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

/// The import spec of the generated package of the document `snake`, under
/// the alias every Go unit imports a sibling package by.
fn imported(snake: &str) -> String {
    format!("\tsce_{snake} \"{PREFIX}/{snake}\"\n")
}

/// Every accepted template the profile's fixtures hold that Go generates: its
/// document, its name, and whether it is specific (docs/SCE_TEST_PROFILE.md
/// §6.2). The two over a union are refused
/// ([`a_template_that_reaches_a_union_is_refused_on_go`]).
fn templates() -> Vec<(PathBuf, &'static str, bool)> {
    let at = |dir: PathBuf, name: &'static str| dir.join(format!("{name}.scxml"));
    vec![
        (
            at(profile_dir(), "elapsed_at_most"),
            "elapsed_at_most",
            false,
        ),
        (at(profile_dir(), "every_matcher"), "every_matcher", false),
        (at(profile_dir(), "job_report"), "job_report", true),
        (at(profile_dir(), "job_report_of"), "job_report_of", true),
        // Specific in itself, and no specific template through its reference.
        (at(profile_dir(), "report_within"), "report_within", false),
        (
            at(imports_dir(), "quantity_at_most"),
            "quantity_at_most",
            false,
        ),
        (at(imports_dir(), "quantity_two"), "quantity_two", true),
        (
            at(imports_dir(), "order_line_exact"),
            "order_line_exact",
            true,
        ),
        (
            at(imports_dir(), "order_line_bounded"),
            "order_line_bounded",
            false,
        ),
        (at(imports_dir(), "card_within"), "card_within", false),
        (at(imports_dir(), "voucher_is"), "voucher_is", true),
        // A unit is named by the document's file (`test-template.scxml`), as
        // every forge unit is (`DocumentLabel::for_input_path`), not by its
        // `name`.
        (kind_example(), "test_template", false),
    ]
}

/// Each template's unit is the package of its document and declares its type
/// and its `Matches`, and a `Specific` exactly when the template is specific.
#[test]
fn each_template_declares_matches_and_a_specific_one_specific() {
    for (path, name, specific) in templates() {
        let text = unit(&path, name);
        let package = format!("\npackage {name}\n");
        holds_each(
            &text,
            &[
                package.as_str(),
                "\ntype Value = ",
                "\nfunc Matches(value Value",
            ],
        );
        assert_eq!(
            text.contains("\nfunc Specific("),
            specific,
            "{name}: a value is declared exactly for a specific template:\n{text}"
        );
    }
}

/// The template that writes every matcher a record holds: each mechanism as
/// its position's type spells it, and the imports of exactly the packages the
/// text names.
#[test]
fn every_matcher_is_spelled_over_the_schemas_record() {
    let text = unit_of(&profile_dir(), "every_matcher");
    let job_status = imported("job_status");
    let elapsed_at_most = imported("elapsed_at_most");
    holds_each(
        &text,
        &[
            "type Value = sce_job_status.JobStatusPayload\n",
            "func Matches(value Value, param_limit uint32) bool {\n",
            // value-list of a specific value and an inclusive range.
            "(value.ElapsedMs == 1 || (value.ElapsedMs >= 10 && value.ElapsedMs <= param_limit))",
            // any-value with ifpresent and a length on an optional text, its
            // characters counted through the profile's runtime where the text
            // is present.
            "(!value.Note.IsPresent() || (value.Note.IsPresent() && \
             (!value.Note.IsPresent() || testprofile.TextLength(value.Note.Proven()) <= 8)))",
            // list-match: its count, its elements in order, a complement as
            // the comparison that does not hold, and its length.
            "len(value.Codes) == 3 && value.Codes[0] == 3 && value.Codes[2] != 0",
            "len(value.Codes) >= 1 && len(value.Codes) <= 4",
            // omit.
            "!value.Retry.IsPresent()",
            // A reference binds the template's own parameter, and names the
            // referenced unit through its package.
            "sce_elapsed_at_most.Matches(value.Count, param_limit)",
            // The packages the text names, and the runtime that counts a
            // text's characters.
            "\t\"github.com/newmassrael/sce-go-runtime/testprofile\"\n",
            job_status.as_str(),
            elapsed_at_most.as_str(),
        ],
    );
    // any-value-or-none holds whatever the field holds.
    assert!(!text.contains("value.Tag"), "{text}");
    // A matcher that reads no element (`any-value` in a list) compares none.
    assert!(!text.contains("value.Codes[1]"), "{text}");
    // Nothing builds or names an optional value, and no byte string is
    // compared, so neither package is imported: Go refuses an unused one.
    assert!(!text.contains("sce-forge-runtime/optional"), "{text}");
    assert!(!text.contains("\"bytes\""), "{text}");
}

/// A specific template's value: every field by name, an omitted field absent
/// and a present one each built with its element's type named, a number as an
/// untyped constant, and a parameter where the template names one.
#[test]
fn a_specific_templates_value_is_its_record() {
    let report = unit_of(&profile_dir(), "job_report");
    holds_each(
        &report,
        &[
            "func Specific() Value {\n",
            "sce_job_status.JobStatusPayload{ElapsedMs: 0, \
             Note: optional.Some[string](\"started on time\"), Codes: []uint8{7}, \
             Tag: optional.None[uint8](), Retry: optional.None[uint8](), Count: 1}",
            "value.Note.IsPresent() && value.Note.Proven() == \"started on time\"",
            "len(value.Codes) == 1 && value.Codes[0] == 7",
            // The value names `optional`, so the unit imports it.
            "\t\"github.com/newmassrael/sce-forge-runtime/optional\"\n",
        ],
    );
    // No text is counted, so the runtime is not imported.
    assert!(!report.contains("sce-go-runtime/testprofile"), "{report}");

    let report_of = unit_of(&profile_dir(), "job_report_of");
    holds_each(
        &report_of,
        &[
            "func Specific(param_elapsed uint32, param_note string, param_code uint8) Value {\n",
            "value.ElapsedMs == param_elapsed",
            "value.Note.IsPresent() && value.Note.Proven() == param_note",
            "value.Codes[0] == param_code",
            "value.Codes[1] == 255",
            "value.Retry.IsPresent() && value.Retry.Proven() == 2",
            "value.Count == 4294967295",
            "Note: optional.Some[string](param_note),",
            "Codes: []uint8{param_code, 255},",
            "Retry: optional.Some[uint8](2),",
            "Count: 4294967295}",
        ],
    );
}

/// An enum literal is the variant of the enum the record's document imports,
/// named through that enum's own package, which the template imports though
/// it does not import the enum itself; a reference to a specific template
/// builds with its `Specific` and matches with its `Matches`; a byte string is
/// compared by its octets and built as the slice of them.
#[test]
fn an_enum_a_reference_and_bytes_are_spelled_by_their_units() {
    let exact = unit_of(&imports_dir(), "order_line_exact");
    let order_line = imported("order_line");
    let quantity_two = imported("quantity_two");
    let paint_color = imported("paint_color");
    holds_each(
        &exact,
        &[
            "value.Sku == \"A-1\"",
            "sce_quantity_two.Matches(value.Quantity)",
            "value.Color == sce_paint_color.PaintColorRed",
            "sce_order_line.OrderLinePayload{Sku: \"A-1\", \
             Quantity: sce_quantity_two.Specific(), Color: sce_paint_color.PaintColorRed}",
            order_line.as_str(),
            quantity_two.as_str(),
            paint_color.as_str(),
        ],
    );
    let voucher = unit_of(&imports_dir(), "voucher_is");
    holds_each(
        &voucher,
        &[
            "bytes.Equal(value.VoucherCode, []byte{0x00, 0xff, 0x10, 0xa0})",
            "VoucherCode: []byte{0x00, 0xff, 0x10, 0xa0}}",
            "\t\"bytes\"\n",
        ],
    );
    assert!(!voucher.contains("value.VoucherCode =="), "{voucher}");
}

/// A range's literal bound at its type's own limit is left out, since it
/// holds every value; a length counts a text's characters through the
/// profile's runtime; a template that names no other document imports no
/// package.
#[test]
fn a_range_bound_at_the_limit_is_left_out_and_a_text_length_counts_characters() {
    let example = unit(&kind_example(), "test_template");
    assert!(
        example.contains("\treturn value.ElapsedMs <= param_limit\n"),
        "{example}"
    );
    let card = unit_of(&imports_dir(), "card_within");
    holds_each(
        &card,
        &[
            "testprofile.TextLength(value.Last4) >= 2 \
             && testprofile.TextLength(value.Last4) <= 3",
            "value.Amount >= 1 && value.Amount <= param_most",
        ],
    );
    let at_most = unit_of(&profile_dir(), "elapsed_at_most");
    holds_each(
        &at_most,
        &["type Value = uint32\n", "\treturn value <= param_limit\n"],
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
        "{name} on Go: {err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{name} on Go: {other:?}"),
        },
        other => panic!("{name} on Go: {other:?}"),
    }
}

/// A template whose type reaches a union, in itself or below a field, is
/// refused on Go by the union's own refusal, which a schema unit's union
/// field is refused by: Go lowers no union.
#[test]
fn a_template_that_reaches_a_union_is_refused_on_go() {
    for (path, alias) in [
        (profile_dir().join("shape_is_circle.scxml"), "Shape"),
        (imports_dir().join("order_placed_match.scxml"), "Payment"),
    ] {
        let name = path.file_stem().and_then(|s| s.to_str()).expect("a stem");
        let Err(err) = sce_build::compile_forge_file(&path, Language::Go, &[], &options()) else {
            panic!("{name} generated on Go");
        };
        let detail = unsupported(name, err);
        assert!(
            detail.starts_with(&format!("test-template '{name}': `union:{alias}`: ")),
            "{detail}"
        );
        assert!(detail.contains(NOT_LOWERED_YET), "{detail}");
    }
}

/// A byte string a parameter names, present or that may be absent, is
/// compared by its octets, the one that may be absent by `Equal`, and a value
/// is built from a copy of it, so the value holds no array its caller can
/// write into. The documents are a test's alone, written into a directory of
/// their own.
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
    let text = unit(&template, "blob_is");
    holds_each(
        &text,
        &[
            "func Matches(value Value, param_code []byte, param_spare optional.Of[[]byte]) bool {\n",
            "\treturn bytes.Equal(value.Code, param_code) && value.Spare.Equal(param_spare)\n",
            "sce_blob_record.BlobRecordPayload{Code: bytes.Clone(param_code), \
             Spare: optional.Map(param_spare, bytes.Clone)}",
        ],
    );
    assert!(!text.contains("value.Code =="), "{text}");
    assert!(!text.contains("value.Spare =="), "{text}");
}

/// A template that names another document's package needs the module path
/// the packages are imported under, and is refused by name without it, as
/// every Go unit that imports one is.
#[test]
fn a_template_that_names_another_package_needs_the_module_path() {
    let path = imports_dir().join("order_line_exact.scxml");
    let Err(err) =
        sce_build::compile_forge_file(&path, Language::Go, &[], &ForgeCompileOptions::default())
    else {
        panic!("order_line_exact generated on Go with no module path");
    };
    let text = format!("{err:?}");
    assert!(text.contains("go_module_prefix"), "{text}");
}
