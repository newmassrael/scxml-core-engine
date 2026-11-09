// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The rules of a test template that need the documents it imports
//! (docs/SCE_TEST_PROFILE.md §11.4): a matcher below a field of an imported
//! record or a case of an imported union is held to that field's or that
//! case's type, a `record-match` to the fields of its record, a `union-match`
//! to the cases of its union, a `template-ref` to the type and the formal
//! parameters of the template it names, and references to no cycle; and
//! whether a template is specific through the templates it references.
//!
//! The documents are under `sce-build/tests/fixtures/test_profile/imports/`:
//! the records, enums and unions the templates import, one accepted template
//! that reaches every shape a position takes, and one template per refusal,
//! each named for what it refuses. Each of them parses: what refuses it is the
//! check that reads its imports.

use std::path::{Path, PathBuf};

use sce_build::forge::codegen_matrix::{TEST_TEMPLATE_BACKENDS, UNION_BACKENDS};
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::forge::model::{ForgeDocument, ForgeImport, ParsedForge, TestTemplateModel};
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::forge::test_template_check;
use sce_build::forge::union_check::NOT_LOWERED_YET;
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// The directory of this file's documents.
fn imports_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/imports")
}

/// The directory of the documents each of which is held to what it states
/// alone (`a_test_profile_document_is_held_to_what_it_states.rs`).
fn profile_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn label(name: &str) -> DocumentLabel<'_> {
    DocumentLabel {
        identifier: name.strip_suffix(".scxml").unwrap_or(name),
        diagnostic_label: name,
    }
}

/// The document `name` of `dir`, parsed: it parses, whatever its imports say.
fn parse_in(dir: &Path, name: &str) -> ParsedForge {
    let text = read(&dir.join(name));
    parse_forge_with_imports(&text, label(name))
        .unwrap_or_else(|e| panic!("{name} parses: {e:?}"))
        .unwrap_or_else(|| panic!("{name} is a forge document"))
}

/// The document `name` of `dir`, held to the rules its imports decide.
fn checked_in(dir: &Path, name: &str) -> Result<(), Located<ForgeError>> {
    let parsed = parse_in(dir, name);
    test_template_check::check(&parsed, dir, name)
}

/// The template `name` of this file's directory, with its imports.
fn template(name: &str) -> (TestTemplateModel, Vec<ForgeImport>) {
    let parsed = parse_in(&imports_dir(), name);
    match parsed.document {
        ForgeDocument::TestTemplate(model) => (model, parsed.imports),
        other => panic!("{name} is a template, got {:?}", other.kind()),
    }
}

/// The first record of a refusal: its wire code, its `actual`, and its line.
fn record(err: &Located<ForgeError>) -> (&'static str, Option<String>, Option<u32>) {
    let diagnostic = &err.error.to_diagnostics()[0];
    (
        diagnostic.code.as_str(),
        diagnostic.actual.clone(),
        err.location.line,
    )
}

fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some("example.com/generated".to_string()),
        ..Default::default()
    }
}

/// A template over an imported record that holds a list of records, an
/// optional field matched with `ifpresent`, a nested record and a union, with
/// a parameterised reference and a reference to a template of the record
/// under another alias, is accepted; so is every template the documents held
/// to what they state alone accept.
#[test]
fn a_template_that_keeps_every_rule_of_its_imports_is_accepted() {
    if let Err(err) = checked_in(&imports_dir(), "order_placed_match.scxml") {
        panic!("order_placed_match.scxml is accepted: {err:?}");
    }
    for name in [
        "order_line_exact.scxml",
        "order_line_bounded.scxml",
        "quantity_at_most.scxml",
        "quantity_two.scxml",
    ] {
        if let Err(err) = checked_in(&imports_dir(), name) {
            panic!("{name} is accepted: {err:?}");
        }
    }
    for name in [
        "every_matcher.scxml",
        "job_report.scxml",
        "shape_is_circle.scxml",
        "elapsed_at_most.scxml",
    ] {
        if let Err(err) = checked_in(&profile_dir(), name) {
            panic!("{name} is accepted: {err:?}");
        }
    }
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("kind-examples");
    if let Err(err) = checked_in(&examples, "test-template.scxml") {
        panic!("the kind example is accepted: {err:?}");
    }
}

/// The compile asks the same rules: the accepted template reaches the
/// generation, and a refused one is refused with its own code before it. Every
/// backend generates a template (`codegen_matrix::TEST_TEMPLATE_BACKENDS`), so
/// the generation that refuses the accepted one is a backend's that lowers no
/// union (`codegen_matrix::UNION_BACKENDS`), Swift here, refusing the union the
/// template reaches by name, which only the generation does.
#[test]
fn the_compile_holds_a_template_to_its_imports_before_it_refuses_to_generate_one() {
    assert!(
        Language::ALL
            .iter()
            .all(|language| TEST_TEMPLATE_BACKENDS.contains(language)),
        "a backend generates no template: the test's refusal is no longer the only one"
    );
    let refusing = Language::Swift;
    assert!(
        !UNION_BACKENDS.contains(&refusing),
        "Swift lowers a union: name a backend that still refuses one"
    );
    match sce_build::compile_forge_file(
        &imports_dir().join("order_placed_match.scxml"),
        refusing,
        &[],
        &options(),
    ) {
        Ok(_) => panic!("a test template generated"),
        Err(err) => match err.error {
            ForgeError::Generate(boxed) => match *boxed {
                GenerateError::UnsupportedFeature { detail, .. } => {
                    assert!(
                        detail.starts_with("test-template 'order_placed_match': `union:Payment`: "),
                        "{detail}"
                    );
                    assert!(detail.contains(NOT_LOWERED_YET), "{detail}");
                }
                other => panic!("the generation's refusal, got {other:?}"),
            },
            other => panic!("the generation's refusal, got {other:?}"),
        },
    }
    let Err(err) = sce_build::compile_forge_file(
        &imports_dir().join("refused_line_field_unknown.scxml"),
        Language::Rust,
        &[],
        &options(),
    ) else {
        panic!("a field the record does not declare is refused at the compile");
    };
    let (code, actual, _) = record(&err);
    assert_eq!(code, DiagnosticCode::TestProfileRecordFields.as_str());
    assert_eq!(actual.as_deref(), Some("size"));
}

/// Every refusal of a rule the imports decide, by the fixture that breaks it:
/// its code, and the value the record names as `actual` where the rule names
/// one.
const REFUSALS: &[(&str, DiagnosticCode, Option<&str>)] = &[
    // A record-match and the fields of its record.
    (
        "refused_line_field_missing.scxml",
        DiagnosticCode::TestProfileRecordFields,
        None,
    ),
    (
        "refused_line_field_unknown.scxml",
        DiagnosticCode::TestProfileRecordFields,
        Some("size"),
    ),
    // A matcher at a field's type.
    (
        "refused_lines_enum_literal.scxml",
        DiagnosticCode::TestProfileLiteral,
        Some("purple"),
    ),
    (
        "refused_line_omit_not_optional.scxml",
        DiagnosticCode::TestProfileMatcherType,
        Some("omit"),
    ),
    (
        "refused_line_ifpresent_not_optional.scxml",
        DiagnosticCode::TestProfileMatcherType,
        Some("ifpresent"),
    ),
    (
        "refused_line_length_on_integer.scxml",
        DiagnosticCode::TestProfileMatcherType,
        Some("length-max"),
    ),
    (
        "refused_line_range_on_text.scxml",
        DiagnosticCode::TestProfileMatcherType,
        Some("value-range"),
    ),
    (
        "refused_line_literal_out_of_range.scxml",
        DiagnosticCode::TestProfileLiteral,
        Some("70000"),
    ),
    (
        "refused_line_range_bound_out_of_range.scxml",
        DiagnosticCode::TestProfileLiteral,
        Some("70000"),
    ),
    (
        "refused_line_parameter_of_another_type.scxml",
        DiagnosticCode::TestProfileTemplateParameter,
        Some("count"),
    ),
    // A union-match and the cases of its union.
    (
        "refused_payment_unknown_variant.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("cheque"),
    ),
    // How many matchers a case holds is a count, no text of the document.
    (
        "refused_payment_case_without_matcher.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        None,
    ),
    (
        "refused_payment_empty_case_with_matcher.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        None,
    ),
    // A template-ref and the template it names. How many arguments it binds
    // is a count, and the cycle a path across documents: neither is text of
    // the line, and each is the message's.
    (
        "refused_line_reference_of_another_type.scxml",
        DiagnosticCode::TestProfileMatcherType,
        Some("QuantityAtMost"),
    ),
    (
        "refused_line_reference_arity.scxml",
        DiagnosticCode::TestProfileTemplateParameter,
        None,
    ),
    (
        "refused_line_reference_argument_type.scxml",
        DiagnosticCode::TestProfileTemplateParameter,
        Some("limit"),
    ),
    (
        "refused_reference_cycle_there.scxml",
        DiagnosticCode::ValidationCrossKindCircularDependency,
        None,
    ),
    (
        "refused_reference_cycle_back.scxml",
        DiagnosticCode::ValidationCrossKindCircularDependency,
        None,
    ),
];

/// A cycle of references is named in the message, by the `src` of each
/// import on it, and the refusal stands on the reference that leads into
/// it: the `<sce:template-ref>` of the checked template.
#[test]
fn a_reference_cycle_is_named_in_the_message_of_the_reference_that_closes_it() {
    let err =
        checked_in(&imports_dir(), "refused_reference_cycle_there.scxml").expect_err("refused");
    let diagnostic = &err.error.to_diagnostics()[0];
    assert_eq!(diagnostic.actual, None, "{err:?}");
    assert!(
        diagnostic.message.contains(
            "refused_reference_cycle_back.scxml → refused_reference_cycle_there.scxml → \
             refused_reference_cycle_back.scxml"
        ),
        "{}",
        diagnostic.message
    );
    assert_eq!(err.location.line, Some(9));
}

#[test]
fn each_rule_the_imports_decide_is_refused_with_its_own_code() {
    let mut wrong: Vec<String> = Vec::new();
    for (name, code, actual) in REFUSALS {
        let err = match checked_in(&imports_dir(), name) {
            Ok(()) => {
                wrong.push(format!("{name}: accepted"));
                continue;
            }
            Err(err) => err,
        };
        let (got_code, got_actual, line) = record(&err);
        if got_code != code.as_str() {
            wrong.push(format!(
                "{name}: {got_code}, expected {}: {err:?}",
                code.as_str()
            ));
            continue;
        }
        if got_actual.as_deref() != *actual {
            wrong.push(format!(
                "{name}: actual {got_actual:?}, expected {actual:?}"
            ));
        }
        if line.is_none() {
            wrong.push(format!("{name}: placed on no line"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Every refusal fixture in the directory is in the table, so a fixture whose
/// row is lost cannot pass by not being read.
#[test]
fn every_refusal_fixture_is_judged() {
    let mut on_disk: Vec<String> = std::fs::read_dir(imports_dir())
        .expect("the fixture directory reads")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.starts_with("refused_"))
        .collect();
    on_disk.sort();
    let mut judged: Vec<String> = REFUSALS
        .iter()
        .map(|(name, _, _)| (*name).to_string())
        .collect();
    judged.sort();
    assert_eq!(on_disk, judged);
}

/// A field the record does not declare is refused on the matcher that names
/// it; a field no matcher names is written in the record alone, and its
/// refusal stands on the record-match.
#[test]
fn a_field_set_refusal_stands_on_the_line_that_holds_it() {
    let unknown =
        checked_in(&imports_dir(), "refused_line_field_unknown.scxml").expect_err("refused");
    assert_eq!(unknown.location.line, Some(13));
    let missing =
        checked_in(&imports_dir(), "refused_line_field_missing.scxml").expect_err("refused");
    assert_eq!(missing.location.line, Some(9));
}

/// docs/SCE_TEST_PROFILE.md §6.2: a template is specific when its own
/// matchers are values and every template it references is specific in turn.
#[test]
fn a_template_is_specific_through_the_templates_it_references() {
    let (exact, imports) = template("order_line_exact.scxml");
    assert!(test_template_check::is_specific(
        &exact,
        &imports,
        &imports_dir()
    ));
    // Its own matchers are values, and the template its quantity references
    // matches a range.
    let (bounded, imports) = template("order_line_bounded.scxml");
    assert!(bounded.is_specific());
    assert!(!test_template_check::is_specific(
        &bounded,
        &imports,
        &imports_dir()
    ));
    let (placed, imports) = template("order_placed_match.scxml");
    assert!(!test_template_check::is_specific(
        &placed,
        &imports,
        &imports_dir()
    ));
    // A reference that comes back to a template on the way names no value.
    let (cycle, imports) = template("refused_reference_cycle_there.scxml");
    assert!(cycle.is_specific());
    assert!(!test_template_check::is_specific(
        &cycle,
        &imports,
        &imports_dir()
    ));
}

/// A schema of no field, written beside the two templates below: an event
/// that carries no data (docs/SCE_ACCEPTED_SUBSET.md, "Fieldless schema").
const FIELDLESS_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="job_nothing"
       sce:event-name="job.nothing">
  <datamodel sce:payload="none"/>
</scxml>
"#;

/// A template whose own type is the record of the schema of no field.
const OVER_FIELDLESS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="over_nothing"
       sce:type="record:Nothing">
  <sce:import kind="event-schema" src="job_nothing.scxml" as="Nothing"/>
  <sce:any-value/>
</scxml>
"#;

/// A template over a number with a formal parameter that may hold the record
/// of the schema of no field.
const PARAMETER_FIELDLESS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="nothing_given"
       sce:type="uint8">
  <sce:import kind="event-schema" src="job_nothing.scxml" as="Nothing"/>
  <datamodel>
    <data id="spare" sce:type="optional&lt;record:Nothing&gt;" sce:direction="in"/>
  </datamodel>
  <sce:any-value/>
</scxml>
"#;

/// The 1-based line of `text` the first occurrence of `token` stands on.
fn line_of(text: &str, token: &str) -> u32 {
    let at = text
        .find(token)
        .unwrap_or_else(|| panic!("`{token}` is in the document"));
    u32::try_from(text[..at].matches('\n').count()).expect("a line number") + 1
}

/// A schema of no field names an event that carries no data, and no backend
/// emits a payload type for it, so a template whose type, or a formal
/// parameter's, holds its record is refused by name before any backend
/// generates it (`validation/attribute-rule-violated`, on the `sce:type` that
/// reaches it), on every backend that generates a template
/// (`codegen_matrix::TEST_TEMPLATE_BACKENDS`: Rust, C++, Kotlin, Go, Python,
/// C11 and Swift). The documents are a test's alone, written into a directory
/// of their own.
#[test]
fn a_record_of_a_schema_with_no_field_is_refused_on_every_backend_that_generates_templates() {
    let dir = tempfile::tempdir().expect("a directory for the documents");
    std::fs::write(dir.path().join("job_nothing.scxml"), FIELDLESS_SCHEMA).expect("the schema");
    let cases = [
        (
            "over_nothing.scxml",
            OVER_FIELDLESS,
            "record:Nothing",
            line_of(OVER_FIELDLESS, "<scxml"),
        ),
        (
            "nothing_given.scxml",
            PARAMETER_FIELDLESS,
            "optional<record:Nothing>",
            line_of(PARAMETER_FIELDLESS, "<data id=\"spare\""),
        ),
    ];
    assert_eq!(
        TEST_TEMPLATE_BACKENDS,
        [
            Language::Rust,
            Language::Cpp,
            Language::Kotlin,
            Language::Go,
            Language::Python,
            Language::C11,
            Language::Swift
        ]
    );
    for (name, text, ty, line) in cases {
        let path = dir.path().join(name);
        std::fs::write(&path, text).expect("the template");
        // The shared check refuses it, whatever backend follows.
        let checked = test_template_check::check(&parse_in(dir.path(), name), dir.path(), name);
        let Err(err) = checked else {
            panic!("{name} holds a record of a schema of no field and is accepted");
        };
        assert_eq!(
            record(&err),
            (
                DiagnosticCode::ValidationAttributeRuleViolated.as_str(),
                Some(ty.to_string()),
                Some(line)
            ),
            "{name}: {err:?}"
        );
        let expected = err.error.to_diagnostics()[0].expected.clone();
        assert!(
            expected
                .as_ref()
                .is_some_and(|rules| rules[0].contains("`Nothing` declares none")),
            "{name}: {expected:?}"
        );
        for &language in TEST_TEMPLATE_BACKENDS {
            let Err(err) = sce_build::compile_forge_file(&path, language, &[], &options()) else {
                panic!("{name} generated on {language:?}");
            };
            assert_eq!(
                record(&err),
                (
                    DiagnosticCode::ValidationAttributeRuleViolated.as_str(),
                    Some(ty.to_string()),
                    Some(line)
                ),
                "{name} on {language:?}: {err:?}"
            );
        }
    }
}
