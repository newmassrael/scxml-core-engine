// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C11 unit of a test template (docs/SCE_TEST_PROFILE.md §6, §11.4,
//! §11.9, §12.6 slice 6): the header of each accepted template, named by the
//! document's file, declares `<name>_value_t`, a `<name>_matches` that takes
//! the value by pointer and the template's formal parameters in order, and, for
//! a specific template alone, a `<name>_value`; each matching mechanism is
//! spelled over the record types the event schema's unit declares, a field in
//! the type that unit holds it in, and every record, enum and referenced
//! template is reached through the header of its own document, which the unit
//! includes. A text is compared by `strcmp` and its length counted in
//! characters by the profile's runtime; a value that may be absent is tested by
//! `present` and read by `value`; a value a parameter of a composed type gives
//! is compared part by part; a value is built as the initializer of its
//! variable. C11 lowers no union (docs/adr/0092), so a template whose type
//! reaches one is refused as C++ refuses it, and it refuses a byte string of
//! one entry, which its event schema unit holds as a pointer with no length.
//!
//! What the generated units do when they run is held by
//! `backends/c/tests/integration/test_a_test_template_matches_its_values.c`,
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

/// The header the template at `path` generates: its one file, `<name>.h`,
/// named by the document's file.
fn unit(path: &Path, name: &str) -> String {
    let output =
        sce_build::compile_forge_file(path, Language::C11, &[], &ForgeCompileOptions::default())
            .unwrap_or_else(|e| panic!("{} generates on C11: {e:?}", path.display()));
    let names: Vec<&str> = output.files.iter().map(|(file, _)| file.as_str()).collect();
    assert_eq!(names, [format!("{name}.h").as_str()], "{name}");
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

/// Every accepted template the profile's fixtures hold that C11 generates: its
/// document, its name, and whether it is specific (docs/SCE_TEST_PROFILE.md
/// §6.2). The two over a union are refused
/// ([`a_template_that_reaches_a_union_is_refused_on_c11`]), and the one over a
/// byte string of one entry
/// ([`a_byte_string_of_one_entry_is_refused_and_one_in_a_list_is_compared_by_its_octets`]).
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
        // A unit is named by the document's file (`test-template.scxml`), as
        // every forge unit is (`DocumentLabel::for_input_path`), not by its
        // `name`.
        (kind_example(), "test_template", false),
    ]
}

/// Each template's unit declares its type and its `<name>_matches` over a
/// pointer to it, and a `<name>_value` exactly when the template is specific.
#[test]
fn each_template_declares_matches_and_a_value_exactly_when_specific() {
    for (path, name, specific) in templates() {
        let text = unit(&path, name);
        let guard = format!("#ifndef SCE_FORGE_{}_H\n", name.to_uppercase());
        let value_type = format!(" {name}_value_t;\n");
        let matches = format!("\nstatic inline bool {name}_matches(const {name}_value_t *value");
        holds_each(
            &text,
            &[
                guard.as_str(),
                "\ntypedef ",
                value_type.as_str(),
                matches.as_str(),
            ],
        );
        assert_eq!(
            text.contains(&format!("\nstatic inline {name}_value_t {name}_value(")),
            specific,
            "{name}: a value is declared exactly for a specific template:\n{text}"
        );
    }
}

/// The template that writes every matcher a record holds: each mechanism as
/// its position's type spells it, the header of every document it names, and
/// the runtime's length of a text where a matcher counts one.
#[test]
fn every_matcher_is_spelled_over_the_schemas_record() {
    let text = unit_of(&profile_dir(), "every_matcher");
    holds_each(
        &text,
        &[
            "typedef JobStatusPayload_t every_matcher_value_t;\n",
            "static inline bool every_matcher_matches(const every_matcher_value_t *value, \
             uint32_t param_limit) {\n",
            "    (void)param_limit;\n",
            // value-list of a specific value and an inclusive range.
            "((*value).elapsed_ms == 1 || ((*value).elapsed_ms >= 10 && \
             (*value).elapsed_ms <= param_limit))",
            // any-value with ifpresent and a length on an optional text, its
            // characters counted through the profile's runtime where the text
            // is present.
            "(!(*value).note.present || ((*value).note.present && (!(*value).note.present || \
             sce_test_text_length((*value).note.value.data) <= 8u)))",
            // list-match: its count, its elements in order, a complement as
            // the comparison that does not hold, and its length.
            "(*value).codes.len == 3u && (*value).codes.items[0] == 3 && \
             (*value).codes.items[2] != 0",
            "(*value).codes.len >= 1u && (*value).codes.len <= 4u",
            // omit.
            "!(*value).retry.present",
            // A reference binds the template's own parameter, and names the
            // referenced unit by its document.
            "elapsed_at_most_matches(&(*value).count, param_limit)",
            // The headers of the documents it names, and the runtime's.
            "#include \"job_status.h\"\n",
            "#include \"elapsed_at_most.h\"\n",
            "#include \"sce/test/text_length.h\"\n",
        ],
    );
    // any-value-or-none holds whatever the field holds.
    assert!(!text.contains("(*value).tag"), "{text}");
    // A matcher that reads no element (`any-value` in a list) compares none.
    assert!(!text.contains("(*value).codes.items[1]"), "{text}");
    // Nothing the template holds of its own may be absent.
    assert!(!text.contains("sce/forge/optional.h"), "{text}");
}

/// A specific template's value: the initializer of its record, every field by
/// its designator, an omitted field absent and a present one present, a text
/// through the fill function of its buffer, and a parameter where the template
/// names one.
#[test]
fn a_specific_templates_value_is_its_records_initializer() {
    let report = unit_of(&profile_dir(), "job_report");
    holds_each(
        &report,
        &[
            "static inline job_report_value_t job_report_value(void) {\n",
            "    const job_report_value_t value = { .elapsed_ms = 0, .note = { .present = true, \
             .value = sce_static_string_32_of(\"started on time\") }, .codes = { .len = 1u, \
             .items = { 7 } }, .tag = { .present = false }, .retry = { .present = false }, \
             .count = 1 };\n",
            "    return value;\n",
            "(*value).note.present && strcmp((*value).note.value.data, \"started on time\") == 0",
            "(*value).codes.len == 1u && (*value).codes.items[0] == 7",
        ],
    );
    // No text is counted, so the runtime is not included.
    assert!(!report.contains("sce/test/text_length.h"), "{report}");

    let report_of = unit_of(&profile_dir(), "job_report_of");
    holds_each(
        &report_of,
        &[
            "static inline job_report_of_value_t job_report_of_value(uint32_t param_elapsed, \
             sce_static_string_32_t param_note, uint8_t param_code) {\n",
            "(*value).elapsed_ms == param_elapsed",
            "(*value).note.present && strcmp((*value).note.value.data, param_note.data) == 0",
            "(*value).codes.items[0] == param_code",
            "(*value).codes.items[1] == 255",
            "(*value).retry.present && (*value).retry.value == 2",
            "(*value).count == 4294967295",
            ".note = { .present = true, .value = sce_static_string_32_of(param_note.data) }",
            ".codes = { .len = 2u, .items = { param_code, 255 } }",
            ".retry = { .present = true, .value = 2 }",
            ".count = 4294967295 };",
            // The buffer the text parameter is held in, declared under the
            // guard every header of its bound shares.
            "#ifndef SCE_STATIC_STRING_32\n",
            "} sce_static_string_32_t;\n",
        ],
    );
}

/// An enum literal is the constant the enum's document declares, which the
/// record's header brings in; a reference to a specific template builds with
/// its `<name>_value` and matches with its `<name>_matches`.
#[test]
fn an_enum_and_a_reference_are_spelled_by_their_units() {
    let exact = unit_of(&imports_dir(), "order_line_exact");
    holds_each(
        &exact,
        &[
            "strcmp((*value).sku.data, \"A-1\") == 0",
            "quantity_two_matches(&(*value).quantity)",
            "(*value).color == PAINT_COLOR_RED",
            "{ .sku = sce_static_string_16_of(\"A-1\"), .quantity = quantity_two_value(), \
             .color = PAINT_COLOR_RED }",
            "#include \"order_line.h\"\n",
            "#include \"quantity_two.h\"\n",
        ],
    );
    let two = unit_of(&imports_dir(), "quantity_two");
    holds_each(
        &two,
        &[
            "typedef uint16_t quantity_two_value_t;\n",
            "    return (*value) == 2;\n",
            "    const quantity_two_value_t value = 2;\n",
        ],
    );
}

/// A range's literal bound at its type's own limit is left out, since it
/// holds every value; a length counts a text's characters through the
/// profile's runtime; a template that names no other document includes no
/// header of one.
#[test]
fn a_range_bound_at_the_limit_is_left_out_and_a_text_length_counts_characters() {
    let example = unit(&kind_example(), "test_template");
    assert!(
        example.contains("    return (*value).elapsed_ms <= param_limit;\n"),
        "{example}"
    );
    let card = unit_of(&imports_dir(), "card_within");
    holds_each(
        &card,
        &[
            "sce_test_text_length((*value).last4.data) >= 2u \
             && sce_test_text_length((*value).last4.data) <= 3u",
            "(*value).amount >= 1 && (*value).amount <= param_most",
        ],
    );
    let at_most = unit_of(&profile_dir(), "elapsed_at_most");
    holds_each(
        &at_most,
        &[
            "typedef uint32_t elapsed_at_most_value_t;\n",
            "    return (*value) <= param_limit;\n",
        ],
    );
    assert!(!at_most.contains("#include \""), "{at_most}");
}

/// The detail of a `generate/unsupported-feature` refusal.
fn unsupported(name: &str, err: Located<ForgeError>) -> String {
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{name} on C11: {err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{name} on C11: {other:?}"),
        },
        other => panic!("{name} on C11: {other:?}"),
    }
}

/// The refusal of the template at `path` on C11.
fn refused(path: &Path) -> String {
    let name = path.file_stem().and_then(|s| s.to_str()).expect("a stem");
    let Err(err) =
        sce_build::compile_forge_file(path, Language::C11, &[], &ForgeCompileOptions::default())
    else {
        panic!("{name} generated on C11");
    };
    unsupported(name, err)
}

/// A template whose type reaches a union, in itself or below a field, is
/// refused on C11 by the union's own refusal, which a schema unit's union field
/// is refused by: C11 lowers no union.
#[test]
fn a_template_that_reaches_a_union_is_refused_on_c11() {
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

/// The schema of a record that holds a byte string of one entry and a list of
/// byte strings.
const BLOB_RECORD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="blob_record"
       sce:event-name="blob.record">
  <datamodel>
    <data id="code" sce:type="bytes[4]" sce:direction="in"/>
    <data id="spares" sce:type="list&lt;bytes[2]&gt;[2]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A template over the blob record whose `code` matcher is `code` and whose
/// first spare is the byte string `00ff`.
fn blob_template(name: &str, code: &str, parameters: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="{name}"
       sce:type="record:Blob">
  <sce:import kind="event-schema" src="blob_record.scxml" as="Blob"/>
  {parameters}
  <sce:record-match>
    {code}
    <sce:list-match field="spares">
      <sce:specific value="00ff"/>
      <sce:any-value/>
    </sce:list-match>
  </sce:record-match>
</scxml>
"#
    )
}

/// A byte string of one entry is held by the C11 event schema unit as a
/// pointer with no length, so a template that compares, counts or builds one is
/// refused by name: a literal of it, a length restriction on it, and a record
/// that holds one compared whole. One inside a list is the composition's,
/// which holds its length, and is compared by its length and its octets. The
/// documents are a test's alone, written into a directory of their own.
#[test]
fn a_byte_string_of_one_entry_is_refused_and_one_in_a_list_is_compared_by_its_octets() {
    let detail = refused(&imports_dir().join("voucher_is.scxml"));
    assert!(
        detail.starts_with("test-template 'voucher_is': `bytes[4]` is a byte string the C11 unit"),
        "{detail}"
    );

    let dir = tempfile::tempdir().expect("a directory for the documents");
    std::fs::write(dir.path().join("blob_record.scxml"), BLOB_RECORD).expect("the schema");
    let write = |name: &str, text: String| {
        let path = dir.path().join(format!("{name}.scxml"));
        std::fs::write(&path, text).expect("the template");
        path
    };

    let any_code = write(
        "blob_spare",
        blob_template("blob_spare", r#"<sce:any-value field="code"/>"#, ""),
    );
    let text = unit(&any_code, "blob_spare");
    holds_each(
        &text,
        &[
            "(*value).spares.len == 2u",
            "((*value).spares.items[0].len == sizeof(\"\\000\\377\") - 1u && \
             memcmp((*value).spares.items[0].data, \"\\000\\377\", sizeof(\"\\000\\377\") - 1u) \
             == 0)",
        ],
    );
    assert!(!text.contains("(*value).code"), "{text}");

    for (name, code, parameters) in [
        (
            "blob_code_is",
            r#"<sce:specific field="code" value="00ff10a0"/>"#,
            "",
        ),
        (
            "blob_code_short",
            r#"<sce:any-value field="code" length-max="2"/>"#,
            "",
        ),
    ] {
        let path = write(name, blob_template(name, code, parameters));
        let detail = refused(&path);
        assert!(
            detail.starts_with(&format!(
                "test-template '{name}': `bytes[4]` is a byte string the C11 unit"
            )),
            "{detail}"
        );
    }

    let whole = write(
        "blob_is",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="blob_is"
       sce:type="record:Blob">
  <sce:import kind="event-schema" src="blob_record.scxml" as="Blob"/>
  <datamodel>
    <data id="expected" sce:type="record:Blob" sce:direction="in"/>
  </datamodel>
  <sce:specific param="expected"/>
</scxml>
"#
        .to_string(),
    );
    let detail = refused(&whole);
    assert!(
        detail.starts_with("test-template 'blob_is': `bytes[4]` is a byte string the C11 unit"),
        "{detail}"
    );
}

/// A record, a list and a value that may be absent a parameter gives are
/// compared part by part, since C compares no two structs, and a specific
/// template built from them holds each as given. The document is a test's
/// alone, written into a directory of its own, over the profile's job status.
#[test]
fn a_parameter_of_a_composed_type_is_compared_part_by_part() {
    let dir = tempfile::tempdir().expect("a directory for the documents");
    let schema = profile_dir().join("job_status.scxml");
    let path = dir.path().join("job_like.scxml");
    std::fs::write(
        &path,
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="job_like"
       sce:type="record:JobStatus">
  <sce:import kind="event-schema" src="{schema}" as="JobStatus"/>
  <datamodel>
    <data id="codes" sce:type="list&lt;uint8&gt;[4]" sce:direction="in"/>
    <data id="tag" sce:type="optional&lt;uint8&gt;" sce:direction="in"/>
  </datamodel>
  <sce:record-match>
    <sce:specific field="elapsed_ms" value="0"/>
    <sce:omit field="note"/>
    <sce:specific field="codes" param="codes"/>
    <sce:specific field="tag" param="tag"/>
    <sce:omit field="retry"/>
    <sce:specific field="count" value="1"/>
  </sce:record-match>
</scxml>
"#,
            schema = schema.display()
        ),
    )
    .expect("the template");
    let text = unit(&path, "job_like");
    holds_each(
        &text,
        &[
            "static inline bool job_like_matches(const job_like_value_t *value, \
             sce_list_4_uint8_t param_codes, sce_optional_uint8_t param_tag) {\n",
            "((*value).codes.len == param_codes.len && ((*value).codes.len <= 0u || \
             (*value).codes.items[0] == param_codes.items[0]) && ((*value).codes.len <= 1u || \
             (*value).codes.items[1] == param_codes.items[1]) && ((*value).codes.len <= 2u || \
             (*value).codes.items[2] == param_codes.items[2]) && ((*value).codes.len <= 3u || \
             (*value).codes.items[3] == param_codes.items[3]))",
            "((*value).tag.present == param_tag.present && (!(*value).tag.present || \
             (*value).tag.value == param_tag.value))",
            ".codes = param_codes, .tag = param_tag,",
            // The list's type, declared once under its guard, and the
            // runtime's optional values.
            "#ifndef SCE_LIST_4_UINT8_DECLARED\n",
            "#include \"sce/forge/optional.h\"",
        ],
    );
    assert!(!text.contains("(*value).codes =="), "{text}");
}

/// A list holds a value that may be absent in its composition's own optional
/// type, not in the runtime's one a template of that type takes, so a
/// reference to such a template inside a list is refused by name. The
/// documents are a test's alone, written into a directory of their own.
#[test]
fn a_reference_inside_a_list_to_a_template_of_an_optional_type_is_refused() {
    let dir = tempfile::tempdir().expect("a directory for the documents");
    std::fs::write(
        dir.path().join("tag_record.scxml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="tag_record"
       sce:event-name="tag.record">
  <datamodel>
    <data id="tags" sce:type="list&lt;optional&lt;uint8&gt;&gt;[2]" sce:direction="in"/>
  </datamodel>
</scxml>
"#,
    )
    .expect("the schema");
    std::fs::write(
        dir.path().join("no_tag.scxml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="no_tag"
       sce:type="optional&lt;uint8&gt;">
  <sce:omit/>
</scxml>
"#,
    )
    .expect("the referenced template");
    let path = dir.path().join("tags_absent.scxml");
    std::fs::write(
        &path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="tags_absent"
       sce:type="record:Tags">
  <sce:import kind="event-schema" src="tag_record.scxml" as="Tags"/>
  <sce:import kind="test-template" src="no_tag.scxml" as="NoTag"/>
  <sce:record-match>
    <sce:list-match field="tags">
      <sce:template-ref template="NoTag"/>
      <sce:template-ref template="NoTag"/>
    </sce:list-match>
  </sce:record-match>
</scxml>
"#,
    )
    .expect("the template");
    let detail = refused(&path);
    assert!(
        detail.starts_with(
            "test-template 'tags_absent': <sce:template-ref template=\"NoTag\"> of \
             `optional<uint8>` stands inside a list"
        ),
        "{detail}"
    );
    // The referenced template itself is generated, over the runtime's optional.
    let referenced = unit(&dir.path().join("no_tag.scxml"), "no_tag");
    holds_each(
        &referenced,
        &[
            "typedef sce_optional_uint8_t no_tag_value_t;\n",
            "    return !(*value).present;\n",
            "#include \"sce/forge/optional.h\"",
        ],
    );
}
