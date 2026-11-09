// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A test suite is exported to TTCN-3 (docs/adr/0091; docs/SCE_TEST_PROFILE.md
//! §9.5): its types module (rule E1, ADR 0091 C2), the port types (E3) and
//! the templates (E2) the behaviour module will hold, and the structural
//! manifest of the types module, a wire surface of its own
//! (`schemas/sce-ttcn3-types-manifest.v1.schema.json`).
//!
//! The goldens are the export of the suites the profile's fixtures and the
//! kind examples hold, and of `fixtures/ttcn3_export/orders_suite.scxml`,
//! which reaches the template fixtures over a union, an enum, a nested record,
//! a list of records and a byte string. A name the export cannot write as it
//! is written, and a type it has no TTCN-3 form for, are refused on documents
//! each test writes for itself.

mod common;

use std::path::{Path, PathBuf};

use common::profile_suite::{
    document, write, write_event_schema, write_suite, write_suite_of_enum,
};
use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::ttcn3_export::{export_suite, Definition, ExportError, Ttcn3Export};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn job_suite() -> PathBuf {
    manifest_dir().join("tests/fixtures/test_profile/job_suite.scxml")
}

fn runtime_suite() -> PathBuf {
    manifest_dir().join("tests/fixtures/test_profile/runtime/runtime_suite.scxml")
}

fn kind_example_suite() -> PathBuf {
    manifest_dir().join("kind-examples/test-suite.scxml")
}

fn orders_suite() -> PathBuf {
    manifest_dir().join("tests/fixtures/ttcn3_export/orders_suite.scxml")
}

/// Every suite of the fixtures the export is held to.
fn suites() -> Vec<PathBuf> {
    vec![
        job_suite(),
        runtime_suite(),
        kind_example_suite(),
        orders_suite(),
    ]
}

fn exported(path: &Path) -> Ttcn3Export {
    export_suite(path, &[]).unwrap_or_else(|refused| match refused {
        ExportError::Refused(e) => panic!("{} is refused: {e:?}", path.display()),
        ExportError::NotASuite(what) => panic!("{what}"),
    })
}

/// The text of the definition named `name`.
fn definition<'d>(definitions: &'d [Definition], name: &str) -> &'d str {
    &definitions
        .iter()
        .find(|d| d.name == name)
        .unwrap_or_else(|| panic!("no definition {name} among {definitions:?}"))
        .text
}

fn names(definitions: &[Definition]) -> Vec<&str> {
    definitions.iter().map(|d| d.name.as_str()).collect()
}

/// The integer types every types module opens with.
const INTEGER_TYPES: &str = "type integer uint8 (0 .. 255);
type integer uint16 (0 .. 65535);
type integer uint32 (0 .. 4294967295);
type integer uint64 (0 .. 18446744073709551615);
type integer int8 (-128 .. 127);
type integer int16 (-32768 .. 32767);
type integer int32 (-2147483648 .. 2147483647);
type integer int64 (-9223372036854775808 .. 9223372036854775807);
";

/// The comment that opens the codec functions of every types module.
const CODEC_COMMENT: &str =
    "// The codec of each message type, declared here and implemented outside the
// export (ADR 0091, C2; docs/SCE_TEST_PROFILE.md, section 9.5).
";

/// The header every types module opens with, for the suite `suite`.
fn header(suite: &str) -> String {
    format!(
        "// The types of the test suite {suite}, exported from its documents
// (docs/SCE_TEST_PROFILE.md, sections 6.3 and 9.5).
module {suite}_Types {{

"
    )
}

/// The codec functions of the message type `ty`.
fn codec(ty: &str) -> String {
    format!(
        "external function enc_{ty}(in {ty} msg) return octetstring;
external function dec_{ty}(inout octetstring octets, out {ty} msg) return integer;
"
    )
}

// ── Goldens ────────────────────────────────────────────────────

#[test]
fn the_job_suite_is_exported() {
    let export = exported(&job_suite());
    assert_eq!(export.suite, "job_suite");
    let expected = format!(
        "{}{INTEGER_TYPES}
type record event_schema {{
  uint32 elapsed_ms
}}

type record job_status {{
  uint32 elapsed_ms,
  universal charstring note optional, // at most 32 bytes of UTF-8
  record length(0 .. 4) of uint8 codes,
  uint8 tag optional,
  uint8 retry optional,
  uint32 count
}}

{CODEC_COMMENT}{}{}
}}",
        header("job_suite"),
        codec("event_schema"),
        codec("job_status"),
    );
    assert_eq!(export.types_module, expected);

    assert_eq!(names(&export.port_types), ["job_port"]);
    assert_eq!(
        definition(&export.port_types, "job_port"),
        "type port job_port message {
  inout job_status;
  out event_schema;
}"
    );

    assert_eq!(
        names(&export.templates),
        ["elapsed_at_most", "every_matcher", "job_report"]
    );
    assert_eq!(
        definition(&export.templates, "elapsed_at_most"),
        "template uint32 elapsed_at_most(uint32 limit) := (-infinity .. limit);"
    );
    assert_eq!(
        definition(&export.templates, "every_matcher"),
        "template job_status every_matcher(uint32 limit) := {
  elapsed_ms := (1, (10 .. limit)),
  note := ? length(0 .. 8) ifpresent,
  codes := { 3, ?, complement(0) } length(1 .. 4),
  tag := *,
  retry := omit,
  count := elapsed_at_most(limit)
};"
    );
    assert_eq!(
        definition(&export.templates, "job_report"),
        "template job_status job_report := {
  elapsed_ms := 0,
  note := \"started on time\",
  codes := { 7 },
  tag := omit,
  retry := omit,
  count := 1
};"
    );
}

/// The kind examples, whose files are named for their kinds: each name is the
/// unit name of the file, `test-port-type.scxml` the port type
/// `test_port_type`.
#[test]
fn the_kind_examples_suite_is_exported() {
    let export = exported(&kind_example_suite());
    assert_eq!(export.suite, "test_suite");
    let expected = format!(
        "{}{INTEGER_TYPES}
type record event_schema {{
  uint32 elapsed_ms
}}

{CODEC_COMMENT}{}
}}",
        header("test_suite"),
        codec("event_schema"),
    );
    assert_eq!(export.types_module, expected);
    assert_eq!(
        definition(&export.port_types, "test_port_type"),
        "type port test_port_type message {
  in event_schema;
}"
    );
    assert_eq!(
        definition(&export.templates, "test_template"),
        "template event_schema test_template(uint32 limit) := {
  elapsed_ms := (0 .. limit)
};"
    );
}

/// The suite of `fixtures/ttcn3_export`: a union whose case holds no record,
/// enums, a nested record, a list of records, a byte string, a bool, and
/// templates that match every one of them.
#[test]
fn the_orders_suite_is_exported() {
    let export = exported(&orders_suite());
    assert_eq!(export.suite, "orders_suite");
    let expected = format!(
        "{}{INTEGER_TYPES}
type record card_payment {{
  universal charstring last4, // at most 4 bytes of UTF-8
  uint32 amount
}}

type record order_customer {{
  universal charstring full_name, // at most 24 bytes of UTF-8
  boolean vip
}}

type record order_line {{
  universal charstring sku, // at most 16 bytes of UTF-8
  uint16 quantity,
  paint_color color
}}

type union order_payment {{
  card_payment card,
  record {{}} cash,
  voucher_payment voucher
}}

type record order_placed {{
  uint32 order_id,
  record length(0 .. 4) of order_line lines,
  universal charstring note optional, // at most 32 bytes of UTF-8
  uint8 priority optional,
  order_customer customer,
  order_payment payment
}}

type enumerated paint_color {{ red(0), green(1), blue(2) }}

type enumerated payment_kind {{ card(0), cash(1), voucher(2) }}

type record voucher_payment {{
  octetstring voucher_code // at most 4 octets
}}

{CODEC_COMMENT}{}{}
}}",
        header("orders_suite"),
        codec("order_placed"),
        codec("voucher_payment"),
    );
    assert_eq!(export.types_module, expected);

    assert_eq!(
        definition(&export.port_types, "orders_port"),
        "type port orders_port message {
  in order_placed;
  out voucher_payment;
}"
    );

    assert_eq!(
        names(&export.templates),
        [
            "order_line_exact",
            "order_placed_match",
            "quantity_at_most",
            "quantity_two",
            "voucher_is"
        ]
    );
    assert_eq!(
        definition(&export.templates, "order_line_exact"),
        "template order_line order_line_exact := {
  sku := \"A-1\",
  quantity := quantity_two,
  color := red
};"
    );
    assert_eq!(
        definition(&export.templates, "order_placed_match"),
        "template order_placed order_placed_match(uint16 limit) := {
  order_id := ?,
  lines := { { sku := \"A-1\", quantity := quantity_at_most(limit), color := green }, \
order_line_exact } length(0 .. 4),
  note := ? length(0 .. 8) ifpresent,
  priority := (1 .. 5) ifpresent,
  customer := { full_name := ?, vip := true },
  payment := ({ card := { last4 := ? length(4 .. infinity), amount := (1 .. infinity) } }, \
{ cash := {} })
};"
    );
    assert_eq!(
        definition(&export.templates, "quantity_at_most"),
        "template uint16 quantity_at_most(uint16 most) := (-infinity .. most);"
    );
    assert_eq!(
        definition(&export.templates, "quantity_two"),
        "template uint16 quantity_two := 2;"
    );
    assert_eq!(
        definition(&export.templates, "voucher_is"),
        "template voucher_payment voucher_is := {
  voucher_code := '00FF10A0'O
};"
    );
}

/// Two exports of one suite are one export, byte for byte
/// (docs/SCE_CODEGEN_DETERMINISM.md).
#[test]
fn an_export_is_the_same_bytes_every_time() {
    for suite in suites() {
        let first = exported(&suite);
        let second = exported(&suite);
        assert_eq!(first.files(), second.files(), "{}", suite.display());
        assert_eq!(first.port_types, second.port_types, "{}", suite.display());
        assert_eq!(first.templates, second.templates, "{}", suite.display());
    }
}

/// The files the export writes are the types module and its manifest, named
/// by the suite, and no part of the behaviour module.
#[test]
fn an_export_writes_the_types_module_and_its_manifest() {
    let export = exported(&job_suite());
    assert_eq!(export.types_module_name(), "job_suite_Types");
    assert_eq!(
        export.files(),
        [
            (
                "job_suite_Types.ttcn".to_string(),
                export.types_module.clone()
            ),
            (
                "job_suite_Types.manifest.json".to_string(),
                export.manifest.clone()
            ),
        ]
    );
}

// ── The manifest ───────────────────────────────────────────────

fn wire_schema() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../schemas/sce-ttcn3-types-manifest.v1.schema.json"
    ))
    .expect("the manifest's schema is JSON")
}

/// Why `instance` is not a manifest the export writes; empty when it is one.
fn wire_violations(instance: &serde_json::Value) -> Vec<String> {
    let schema = wire_schema();
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("the manifest's schema compiles");
    let outcome = validator.validate(instance);
    match outcome {
        Ok(()) => Vec::new(),
        Err(errors) => errors.map(|e| e.to_string()).collect(),
    }
}

fn manifest_of(export: &Ttcn3Export) -> serde_json::Value {
    serde_json::from_str(&export.manifest).expect("a manifest is JSON")
}

/// Every manifest the export writes is one the wire schema states, and states
/// the module beside it: every definition it names is a definition of the
/// module, of the kind it says, and every codec function a declaration.
#[test]
fn every_manifest_the_export_writes_validates_against_the_wire_schema() {
    for suite in suites() {
        let export = exported(&suite);
        let manifest = manifest_of(&export);
        assert_eq!(
            wire_violations(&manifest),
            Vec::<String>::new(),
            "{}",
            suite.display()
        );
        assert_eq!(manifest["suite"], export.suite.as_str());
        assert_eq!(manifest["module"], export.types_module_name().as_str());
        let types = manifest["types"].as_array().expect("types");
        assert!(types.len() >= 9, "{}: {types:?}", suite.display());
        for ty in types {
            let name = ty["name"].as_str().expect("a name");
            let opening = match ty["kind"].as_str().expect("a kind") {
                "integer" => format!("type integer {name} ("),
                "enumerated" => format!("type enumerated {name} {{"),
                "record" => format!("type record {name} {{"),
                "union" => format!("type union {name} {{"),
                other => panic!("a kind {other}"),
            };
            assert!(
                export.types_module.contains(&opening),
                "{}: the module does not hold `{opening}`:\n{}",
                suite.display(),
                export.types_module
            );
        }
        for function in manifest["codec"].as_array().expect("codec") {
            for key in ["encode", "decode"] {
                let declared = format!(
                    "external function {}(",
                    function[key].as_str().expect("a function")
                );
                assert!(
                    export.types_module.contains(&declared),
                    "{}: {declared}",
                    suite.display()
                );
            }
        }
    }
}

/// The manifest of the job suite, as the export writes it: its record of a
/// list, its optional text and its codec, structure for structure.
#[test]
fn the_manifest_states_each_types_structure() {
    let manifest = manifest_of(&exported(&job_suite()));
    assert_eq!(manifest["v"], 1);
    let job_status = manifest["types"]
        .as_array()
        .expect("types")
        .iter()
        .find(|t| t["name"] == "job_status")
        .expect("job_status");
    assert_eq!(
        job_status,
        &serde_json::json!({
            "kind": "record",
            "name": "job_status",
            "event": "job.status",
            "fields": [
                {"name": "elapsed_ms", "type": {"kind": "ref", "name": "uint32"}, "optional": false},
                {"name": "note", "type": {"kind": "universal charstring", "max_utf8_bytes": 32}, "optional": true},
                {
                    "name": "codes",
                    "type": {
                        "kind": "record of",
                        "min_length": 0,
                        "max_length": 4,
                        "element": {"kind": "ref", "name": "uint8"}
                    },
                    "optional": false
                },
                {"name": "tag", "type": {"kind": "ref", "name": "uint8"}, "optional": true},
                {"name": "retry", "type": {"kind": "ref", "name": "uint8"}, "optional": true},
                {"name": "count", "type": {"kind": "ref", "name": "uint32"}, "optional": false}
            ]
        })
    );
    assert_eq!(
        manifest["types"][3],
        serde_json::json!({"kind": "integer", "name": "uint64", "min": "0", "max": "18446744073709551615"})
    );
    assert_eq!(
        manifest["codec"],
        serde_json::json!([
            {"type": "event_schema", "encode": "enc_event_schema", "decode": "dec_event_schema"},
            {"type": "job_status", "encode": "enc_job_status", "decode": "dec_job_status"}
        ])
    );
}

/// The wire schema refuses a manifest one member away from one the export
/// wrote, for each member a consumer reads.
#[test]
fn the_wire_schema_rejects_what_the_export_does_not_write() {
    let valid = manifest_of(&exported(&orders_suite()));
    assert_eq!(wire_violations(&valid), Vec::<String>::new());
    let position = |name: &str| {
        valid["types"]
            .as_array()
            .expect("types")
            .iter()
            .position(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("no type {name}"))
    };
    let placed = position("order_placed");
    let payment = position("order_payment");
    let color = position("paint_color");
    let changed = |change: &dyn Fn(&mut serde_json::Value)| {
        let mut instance = valid.clone();
        change(&mut instance);
        instance
    };
    let changes: [(&str, serde_json::Value); 8] = [
        (
            "another version",
            changed(&|m| m["v"] = serde_json::json!(2)),
        ),
        (
            "no codec functions",
            changed(&|m| {
                m.as_object_mut().expect("an object").remove("codec");
            }),
        ),
        (
            "a name that is no identifier",
            changed(&|m| m["types"][placed]["name"] = serde_json::json!("order-placed")),
        ),
        (
            "a bound of an integer type as a number",
            changed(&|m| m["types"][0]["min"] = serde_json::json!(0)),
        ),
        (
            "a list without its upper length",
            changed(&|m| {
                m["types"][placed]["fields"][1]["type"]
                    .as_object_mut()
                    .expect("a list")
                    .remove("max_length");
            }),
        ),
        (
            "a field that does not say whether it is optional",
            changed(&|m| {
                m["types"][placed]["fields"][0]
                    .as_object_mut()
                    .expect("a field")
                    .remove("optional");
            }),
        ),
        (
            "a union with no alternative",
            changed(&|m| m["types"][payment]["alternatives"] = serde_json::json!([])),
        ),
        (
            "a variant's number as a number",
            changed(&|m| m["types"][color]["variants"][0]["value"] = serde_json::json!(0)),
        ),
    ];
    for (what, instance) in changes {
        assert!(
            !wire_violations(&instance).is_empty(),
            "the schema accepts {what}: {instance}"
        );
    }
}

// ── Refusals ───────────────────────────────────────────────────

/// The first wire record of a refused export.
fn refusal(path: &Path) -> serde_json::Value {
    match export_suite(path, &[]) {
        Ok(export) => panic!("{} is exported:\n{}", path.display(), export.types_module),
        Err(ExportError::NotASuite(what)) => panic!("{what}"),
        Err(ExportError::Refused(refused)) => {
            let records = refused.to_diagnostics();
            serde_json::to_value(&records[0]).expect("a diagnostic is JSON")
        }
    }
}

/// Where a wire record stands: the file name of its location.
fn file_of(record: &serde_json::Value) -> String {
    let file = record["location"]["file"]
        .as_str()
        .unwrap_or_else(|| panic!("a record with a file: {record}"));
    Path::new(file)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(file)
        .to_string()
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir().expect("a scratch directory")
}

/// A variant named by a reserved word of TTCN-3 is refused at the variant, in
/// its enum, and not renamed. `inconc`, a verdict of TTCN-3, is a word no
/// backend reserves, so the export is what refuses it.
#[test]
fn a_reserved_word_is_refused_by_name() {
    let dir = scratch();
    let record = refusal(&write_suite_of_enum(dir.path(), "inconc"));
    assert_eq!(record["code"], "test-profile/export-name", "{record}");
    assert_eq!(record["actual"], "inconc", "{record}");
    assert_eq!(file_of(&record), "outcome.scxml", "{record}");
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|m| m.contains("the variant 'inconc' is a reserved word of TTCN-3")),
        "{record}"
    );
}

/// A document whose unit name is no TTCN-3 identifier is refused at the
/// document: a file stem that opens with a digit. The unit name is the
/// stem's, whatever the root's `name` says.
#[test]
fn a_name_that_is_no_identifier_is_refused_by_name() {
    let dir = scratch();
    write(
        dir.path(),
        "2nd_report",
        &document(
            "sce:kind=\"event-schema\" name=\"second_report\" sce:event-name=\"probe.report\"",
            "  <datamodel>\n    <data id=\"elapsed_ms\" sce:type=\"uint32\" \
             sce:direction=\"in\"/>\n  </datamodel>\n",
        ),
    );
    let record = refusal(&write_suite(dir.path(), "2nd_report", false));
    assert_eq!(record["code"], "test-profile/export-name", "{record}");
    assert_eq!(record["actual"], "2nd_report", "{record}");
    assert_eq!(file_of(&record), "2nd_report.scxml", "{record}");
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|m| m.contains("the record '2nd_report' is not a TTCN-3 identifier")),
        "{record}"
    );
}

/// A document named as a type the export makes is one identifier with it,
/// and is refused at the document, not at the export's own name.
#[test]
fn a_name_another_definition_holds_is_refused_by_name() {
    let dir = scratch();
    write_event_schema(
        dir.path(),
        "uint8",
        "probe.report",
        "",
        "    <data id=\"elapsed_ms\" sce:type=\"uint32\" sce:direction=\"in\"/>\n",
    );
    let record = refusal(&write_suite(dir.path(), "uint8", false));
    assert_eq!(record["code"], "test-profile/export-name", "{record}");
    assert_eq!(record["actual"], "uint8", "{record}");
    assert_eq!(file_of(&record), "uint8.scxml", "{record}");
    assert!(
        record["message"].as_str().is_some_and(|m| m
            .contains("the record 'uint8' is one TTCN-3 identifier with the integer type 'uint8'")),
        "{record}"
    );
}

/// A float, which the profile leaves out, has no type in the types module.
#[test]
fn a_float_field_is_refused_by_name() {
    let dir = scratch();
    write_event_schema(
        dir.path(),
        "reading",
        "probe.reading",
        "",
        "    <data id=\"level\" sce:type=\"float32\" sce:direction=\"in\"/>\n",
    );
    let record = refusal(&write_suite(dir.path(), "reading", false));
    assert_eq!(record["code"], "generate/unsupported-feature", "{record}");
    assert_eq!(file_of(&record), "reading.scxml", "{record}");
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|m| m.contains("the field 'level'") && m.contains("float32")),
        "{record}"
    );
}

/// An enum of an open set of values holds every number of its underlying
/// type, and a TTCN-3 `enumerated` its declared values alone.
#[test]
fn an_open_enum_is_refused_by_name() {
    let dir = scratch();
    write(
        dir.path(),
        "level_kind",
        &document(
            "sce:kind=\"enum\" name=\"level_kind\" sce:underlying-type=\"uint8\" \
             sce:strict-variants=\"false\"",
            "  <datamodel>\n    <data id=\"variants\">\n      <sce:variant name=\"low\" \
             value=\"0\"/>\n      <sce:variant name=\"high\" value=\"1\"/>\n    </data>\n  \
             </datamodel>\n",
        ),
    );
    write_event_schema(
        dir.path(),
        "reading",
        "probe.reading",
        "  <sce:import kind=\"enum\" src=\"level_kind.scxml\" as=\"LevelKind\"/>\n",
        "    <data id=\"level\" sce:type=\"enum:LevelKind\" sce:direction=\"in\"/>\n",
    );
    let record = refusal(&write_suite(dir.path(), "reading", false));
    assert_eq!(record["code"], "generate/unsupported-feature", "{record}");
    assert_eq!(file_of(&record), "level_kind.scxml", "{record}");
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|m| m.contains("open")),
        "{record}"
    );
}

/// A template of a list type has no type name to be declared by in TTCN-3,
/// and is refused by name where the suite reaches it.
#[test]
fn a_template_of_a_list_is_refused_by_name() {
    let dir = scratch();
    write_event_schema(
        dir.path(),
        "batch",
        "probe.batch",
        "",
        "    <data id=\"codes\" sce:type=\"list&lt;uint8&gt;[4]\" sce:direction=\"in\"/>\n    \
         <data id=\"count\" sce:type=\"uint8\" sce:direction=\"in\"/>\n",
    );
    write(
        dir.path(),
        "probe_codes",
        &document(
            "sce:kind=\"test-template\" name=\"probe_codes\" \
             sce:type=\"list&lt;uint8&gt;[4]\"",
            "  <sce:list-match>\n    <sce:specific value=\"1\"/>\n  </sce:list-match>\n",
        ),
    );
    write(
        dir.path(),
        "probe_match",
        &document(
            "sce:kind=\"test-template\" name=\"probe_match\" sce:type=\"record:Batch\"",
            "  <sce:import kind=\"event-schema\" src=\"batch.scxml\" as=\"Batch\"/>\n  \
             <sce:import kind=\"test-template\" src=\"probe_codes.scxml\" as=\"Codes\"/>\n  \
             <sce:record-match>\n    <sce:template-ref field=\"codes\" template=\"Codes\"/>\n    \
             <sce:any-value field=\"count\"/>\n  </sce:record-match>\n",
        ),
    );
    let record = refusal(&write_suite(dir.path(), "batch", true));
    assert_eq!(record["code"], "generate/unsupported-feature", "{record}");
    assert_eq!(file_of(&record), "probe_codes.scxml", "{record}");
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|m| m.contains("test-template 'probe_codes'") && m.contains("is a list")),
        "{record}"
    );
}

/// The export reads a test suite and refuses every other document by saying
/// what it is.
#[test]
fn a_document_that_is_no_suite_is_not_exported() {
    let template = manifest_dir().join("tests/fixtures/test_profile/job_report.scxml");
    match export_suite(&template, &[]) {
        Err(ExportError::NotASuite(what)) => {
            assert!(what.contains("sce:kind=\"test-template\""), "{what}")
        }
        other => panic!("a template is exported as a suite: {other:?}"),
    }
}
