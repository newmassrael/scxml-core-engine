// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Python unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §11.9, §12.6 slice 6): the module of each accepted template, named by the
//! document's file, declares `Value`, a `matches` that takes the template's
//! formal parameters in order, and, for a specific template alone, a `value`;
//! each matching mechanism is spelled over the record classes the event
//! schema's unit declares, every enum, record and referenced template named
//! through the module of its own document, which the unit imports exactly
//! where its text names it. The tests are joined in Python's words (`and`,
//! `or`, `not`, `True`, `False`); a value that may be absent is tested by
//! `is None` and read as it is. Python lowers no union (docs/adr/0092), so a
//! template whose type reaches one is refused as C++ and Go refuse it.
//!
//! What the generated units do when they run is held by
//! `backends/python/tests/integration/static_datamodel/test_a_test_template_matches_its_values.py`,
//! which imports them.

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

/// The Python module the template at `path` generates: its one file,
/// `<name>.py`, named by the document's file.
fn unit(path: &Path, name: &str) -> String {
    let output =
        sce_build::compile_forge_file(path, Language::Python, &[], &ForgeCompileOptions::default())
            .unwrap_or_else(|e| panic!("{} generates on Python: {e:?}", path.display()));
    let names: Vec<&str> = output.files.iter().map(|(file, _)| file.as_str()).collect();
    assert_eq!(names, [format!("{name}.py").as_str()], "{name}");
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

/// The import of the generated module of the document `snake`, under the alias
/// every Python unit imports a sibling module by.
fn imported(snake: &str) -> String {
    format!("\nfrom . import {snake} as sce_{snake}\n")
}

/// The import of the profile's runtime function that counts a text's
/// characters.
const TEXT_LENGTH_IMPORT: &str = "\nfrom sce_runtime.testprofile import text_length\n";

/// Every accepted template the profile's fixtures hold that Python generates:
/// its document, its name, and whether it is specific (docs/SCE_TEST_PROFILE.md
/// §6.2). The two over a union are refused
/// ([`a_template_that_reaches_a_union_is_refused_on_python`]).
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

/// Each template's unit declares its type and its `matches`, and a `value`
/// exactly when the template is specific.
#[test]
fn each_template_declares_matches_and_a_specific_one_value() {
    for (path, name, specific) in templates() {
        let text = unit(&path, name);
        holds_each(&text, &["\nValue = ", "\ndef matches(value: Value"]);
        assert_eq!(
            text.contains("\ndef value("),
            specific,
            "{name}: a value is declared exactly for a specific template:\n{text}"
        );
    }
}

/// The template that writes every matcher a record holds: each mechanism as
/// its position's type spells it, joined in Python's words, and the imports of
/// exactly the modules the text names.
#[test]
fn every_matcher_is_spelled_over_the_schemas_record() {
    let text = unit_of(&profile_dir(), "every_matcher");
    let job_status = imported("job_status");
    let elapsed_at_most = imported("elapsed_at_most");
    holds_each(
        &text,
        &[
            "\nValue = sce_job_status.JobStatusPayload\n",
            "\ndef matches(value: Value, param_limit: int) -> bool:\n",
            // value-list of a specific value and an inclusive range.
            "(value.elapsed_ms == 1 or (value.elapsed_ms >= 10 and value.elapsed_ms <= param_limit))",
            // any-value with ifpresent and a length on an optional text, its
            // characters counted through the profile's runtime where the text
            // is present.
            "(value.note is None or (value.note is not None and \
             (value.note is None or text_length(value.note) <= 8)))",
            // list-match: its count, its elements in order, a complement as
            // the comparison that does not hold, and its length.
            "len(value.codes) == 3 and value.codes[0] == 3 and value.codes[2] != 0",
            "len(value.codes) >= 1 and len(value.codes) <= 4",
            // omit.
            "value.retry is None",
            // A reference binds the template's own parameter, and names the
            // referenced unit through its module.
            "sce_elapsed_at_most.matches(value.count, param_limit)",
            // The modules the text names, and the runtime that counts a
            // text's characters.
            TEXT_LENGTH_IMPORT,
            job_status.as_str(),
            elapsed_at_most.as_str(),
        ],
    );
    // any-value-or-none holds whatever the field holds.
    assert!(!text.contains("value.tag"), "{text}");
    // A matcher that reads no element (`any-value` in a list) compares none.
    assert!(!text.contains("value.codes[1]"), "{text}");
    // No connective of C's is left in the unit.
    assert!(!text.contains("&&"), "{text}");
    assert!(!text.contains("||"), "{text}");
    // No type may hold no value, so `Optional` is not imported.
    assert!(!text.contains("from typing import Optional"), "{text}");
}

/// A specific template's value: every field by its keyword, an omitted field
/// `None` and a present one as its value, and a parameter where the template
/// names one.
#[test]
fn a_specific_templates_value_is_its_record() {
    let report = unit_of(&profile_dir(), "job_report");
    holds_each(
        &report,
        &[
            "\ndef value() -> Value:\n",
            "sce_job_status.JobStatusPayload(elapsed_ms=0, note=\"started on time\", \
             codes=[7], tag=None, retry=None, count=1)",
            "value.note is not None and value.note == \"started on time\"",
            "len(value.codes) == 1 and value.codes[0] == 7",
            "value.tag is None and value.retry is None and value.count == 1",
        ],
    );
    // No text is counted, so the runtime is not imported.
    assert!(!report.contains("sce_runtime.testprofile"), "{report}");

    let report_of = unit_of(&profile_dir(), "job_report_of");
    holds_each(
        &report_of,
        &[
            "\ndef value(param_elapsed: int, param_note: str, param_code: int) -> Value:\n",
            "value.elapsed_ms == param_elapsed",
            "value.note is not None and value.note == param_note",
            "value.codes[0] == param_code",
            "value.codes[1] == 255",
            "value.retry is not None and value.retry == 2",
            "value.count == 4294967295",
            "note=param_note,",
            "codes=[param_code, 255],",
            "retry=2,",
            "count=4294967295)",
        ],
    );
}

/// An enum literal is the member of the enum the record's document imports,
/// named through that enum's own module, which the template imports though it
/// does not import the enum itself; a reference to a specific template builds
/// with its `value` and matches with its `matches`; a byte string is compared
/// and built as the bytes literal of its octets.
#[test]
fn an_enum_a_reference_and_bytes_are_spelled_by_their_units() {
    let exact = unit_of(&imports_dir(), "order_line_exact");
    let order_line = imported("order_line");
    let quantity_two = imported("quantity_two");
    let paint_color = imported("paint_color");
    holds_each(
        &exact,
        &[
            "value.sku == \"A-1\"",
            "sce_quantity_two.matches(value.quantity)",
            "value.color == sce_paint_color.PaintColor.RED",
            "sce_order_line.OrderLinePayload(sku=\"A-1\", \
             quantity=sce_quantity_two.value(), color=sce_paint_color.PaintColor.RED)",
            order_line.as_str(),
            quantity_two.as_str(),
            paint_color.as_str(),
        ],
    );
    let voucher = unit_of(&imports_dir(), "voucher_is");
    holds_each(
        &voucher,
        &[
            "value.voucher_code == b\"\\x00\\xff\\x10\\xa0\"",
            "sce_voucher_payment.VoucherPaymentPayload(voucher_code=b\"\\x00\\xff\\x10\\xa0\")",
            imported("voucher_payment").as_str(),
        ],
    );
}

/// A range's literal bound at its type's own limit is left out, since it holds
/// every value; a length counts a text's characters through the profile's
/// runtime; a template that names no other document imports no module.
#[test]
fn a_range_bound_at_the_limit_is_left_out_and_a_text_length_counts_characters() {
    let example = unit(&kind_example(), "test_template");
    assert!(
        example.contains("\n    return value.elapsed_ms <= param_limit\n"),
        "{example}"
    );
    let card = unit_of(&imports_dir(), "card_within");
    holds_each(
        &card,
        &[
            "text_length(value.last4) >= 2 and text_length(value.last4) <= 3",
            "value.amount >= 1 and value.amount <= param_most",
            TEXT_LENGTH_IMPORT,
        ],
    );
    let at_most = unit_of(&profile_dir(), "elapsed_at_most");
    holds_each(
        &at_most,
        &["\nValue = int\n", "\n    return value <= param_limit\n"],
    );
    assert!(!at_most.contains("\nfrom "), "{at_most}");
    assert!(!at_most.contains("\nimport "), "{at_most}");
}

/// The detail of a `generate/unsupported-feature` refusal.
fn unsupported(name: &str, err: Located<ForgeError>) -> String {
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{name} on Python: {err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{name} on Python: {other:?}"),
        },
        other => panic!("{name} on Python: {other:?}"),
    }
}

/// A template whose type reaches a union, in itself or below a field, is
/// refused on Python by the union's own refusal, which a schema unit's union
/// field is refused by: Python lowers no union.
#[test]
fn a_template_that_reaches_a_union_is_refused_on_python() {
    for (path, alias) in [
        (profile_dir().join("shape_is_circle.scxml"), "Shape"),
        (imports_dir().join("order_placed_match.scxml"), "Payment"),
    ] {
        let name = path.file_stem().and_then(|s| s.to_str()).expect("a stem");
        let Err(err) = sce_build::compile_forge_file(
            &path,
            Language::Python,
            &[],
            &ForgeCompileOptions::default(),
        ) else {
            panic!("{name} generated on Python");
        };
        let detail = unsupported(name, err);
        assert!(
            detail.starts_with(&format!("test-template '{name}': `union:{alias}`: ")),
            "{detail}"
        );
        assert!(detail.contains(NOT_LOWERED_YET), "{detail}");
    }
}

/// The record schema the documents of a test of its own import: two `bool`
/// fields, a byte string, and a byte string that may be absent.
const FLAGS_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="blob_record"
       sce:event-name="blob.record">
  <datamodel>
    <data id="ready" sce:type="bool" sce:direction="in"/>
    <data id="done" sce:type="bool" sce:direction="in"/>
    <data id="code" sce:type="bytes[4]" sce:direction="in"/>
    <data id="spare" sce:type="optional&lt;bytes[4]&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// Writes the schema and the template `text` as `<name>.scxml` into `dir`,
/// and answers the template's path.
fn write_template(dir: &Path, name: &str, text: &str) -> PathBuf {
    std::fs::write(dir.join("blob_record.scxml"), FLAGS_SCHEMA).expect("the record's schema");
    let path = dir.join(format!("{name}.scxml"));
    std::fs::write(&path, text).expect("the template");
    path
}

/// A `bool` compared with a literal is read, or read under `not`, a literal of
/// one is `True` or `False`, and a matcher that holds whatever the value is
/// returns `True`: Python's spellings of what C's write `!`, `true` and
/// `false`. The documents are a test's alone, written into a directory of
/// their own.
#[test]
fn a_bool_and_a_test_that_always_holds_are_spelled_in_pythons_words() {
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
    let text = unit(&flags, "flags_set");
    holds_each(
        &text,
        &[
            "\n    return value.ready and not value.done and \
             value.code == b\"\\x0a\\x0b\\x0c\\x0d\" and value.spare is None\n",
            "sce_blob_record.BlobRecordPayload(ready=True, done=False, \
             code=b\"\\x0a\\x0b\\x0c\\x0d\", spare=None)",
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
    let text = unit(&anything, "any_blob");
    assert!(text.contains("\n    return True\n"), "{text}");
    assert!(!text.contains("return true"), "{text}");
}

/// A byte string a parameter names, present or that may be absent, is compared
/// by `==`, which compares Python's `bytes` by their octets and holds two
/// absent values equal, and a value holds it as given, since a `bytes` is
/// immutable. A parameter that may be absent is typed `Optional`, which the
/// unit imports. The documents are a test's alone, written into a directory of
/// their own.
#[test]
fn a_byte_string_parameter_is_compared_by_its_octets_and_held_as_given() {
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
    <data id="spare" sce:type="optional&lt;bytes[4]&gt;" sce:direction="in"/>
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
    let text = unit(&template, "blob_is");
    holds_each(
        &text,
        &[
            "\ndef matches(value: Value, param_code: bytes, param_spare: Optional[bytes]) -> bool:\n",
            "value.code == param_code and value.spare == param_spare\n",
            "sce_blob_record.BlobRecordPayload(ready=True, done=True, \
             code=param_code, spare=param_spare)",
            "\nfrom typing import Optional\n",
            imported("blob_record").as_str(),
        ],
    );
}
