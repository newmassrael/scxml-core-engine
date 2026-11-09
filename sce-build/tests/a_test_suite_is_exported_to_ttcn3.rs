// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A test suite is exported to TTCN-3 (docs/adr/0091; docs/SCE_TEST_PROFILE.md
//! §9.5): its types module (rule E1, ADR 0091 C2), the structural manifest of
//! the types module, a wire surface of its own
//! (`schemas/sce-ttcn3-types-manifest.v1.schema.json`), and the behaviour
//! module: the port types (E3), the templates (E2), the module parameters,
//! the component types and functions (E4 to E13), the test cases (E14) and
//! the control part (E15); and the correspondence witness of both modules
//! (§9.4, `schemas/sce-ttcn3-witness.v1.schema.json`).
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
    document, write, write_event_schema, write_suite, write_suite_of_component, write_suite_of_enum,
};
use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::ttcn3_export::behaviour::{Source, Statement};
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

    // The helper's control port is the complement of the tester's: what one
    // sends out, the other takes in.
    assert_eq!(names(&export.port_types), ["job_control_port", "job_port"]);
    assert_eq!(
        definition(&export.port_types, "job_control_port"),
        "type port job_control_port message {
  inout job_status;
  in event_schema;
}"
    );
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
        assert_eq!(
            first.module_parameters,
            second.module_parameters,
            "{}",
            suite.display()
        );
        assert_eq!(
            first.component_types,
            second.component_types,
            "{}",
            suite.display()
        );
        assert_eq!(first.functions, second.functions, "{}", suite.display());
        assert_eq!(
            first.feature_parameters,
            second.feature_parameters,
            "{}",
            suite.display()
        );
        assert_eq!(
            first.system_types,
            second.system_types,
            "{}",
            suite.display()
        );
        assert_eq!(first.test_cases, second.test_cases, "{}", suite.display());
        assert_eq!(first.control, second.control, "{}", suite.display());
        assert_eq!(first.witness, second.witness, "{}", suite.display());
    }
}

/// The files the export writes are the types module, its manifest, the
/// behaviour module and the witness, each named by the suite.
#[test]
fn an_export_writes_its_two_modules_the_manifest_and_the_witness() {
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
            (
                "job_suite.ttcn".to_string(),
                export.behaviour_module.clone()
            ),
            ("job_suite.witness.json".to_string(), export.witness.clone()),
        ]
    );
}

/// Every behaviour module the export writes holds each definition the export
/// computes, once, in the order of §9.5, and imports the types module whole.
#[test]
fn every_behaviour_module_holds_its_definitions_in_order() {
    for suite in suites() {
        let export = exported(&suite);
        let module = &export.behaviour_module;
        assert!(
            module.contains(&format!(
                "module {} {{\n\nimport from {}_Types all;\n",
                export.suite, export.suite
            )),
            "{}:\n{module}",
            suite.display()
        );
        let mut from = 0;
        for definition in export
            .port_types
            .iter()
            .chain(&export.templates)
            .chain(&export.module_parameters)
            .chain(&export.feature_parameters)
            .chain(&export.component_types)
            .chain(&export.system_types)
            .chain(&export.functions)
            .chain(&export.test_cases)
        {
            let at = module[from..].find(&definition.text).unwrap_or_else(|| {
                panic!(
                    "{}: {} is not where §9.5 puts it in\n{module}",
                    suite.display(),
                    definition.name
                )
            });
            from += at + definition.text.len();
        }
        assert!(
            module[from..].ends_with(&format!("\n{}\n\n}}", export.control)),
            "{}: the control part does not close the module\n{module}",
            suite.display()
        );
    }
}

// ── The behaviour (E4 to E13) ──────────────────────────────────

/// The kill of the PTC `reference` refers to, at the depth `pad`, as every
/// branch that exits its invoking state writes it.
fn kill(pad: &str, reference: &str) -> String {
    format!(
        "{pad}if ({reference} != null) {{
{pad}  {reference}.kill;
{pad}  {reference} := null;
{pad}}}
"
    )
}

/// The job suite: its module parameters, the tester that creates the helper
/// and connects it, and the helper. The tester's behaviour depends on its
/// configuration, since it creates a PTC; the helper's does not.
#[test]
fn the_job_suites_behaviour_is_exported() {
    let export = exported(&job_suite());
    assert_eq!(
        names(&export.module_parameters),
        ["elapsed_limit", "run_label", "key"]
    );
    assert_eq!(
        definition(&export.module_parameters, "elapsed_limit"),
        "modulepar uint32 elapsed_limit := 500;"
    );
    assert_eq!(
        definition(&export.module_parameters, "run_label"),
        "modulepar universal charstring run_label := \"nightly\"; // at most 8 bytes of UTF-8"
    );
    assert_eq!(
        definition(&export.module_parameters, "key"),
        "modulepar octetstring key := '0AFF'O; // at most 2 octets"
    );

    assert_eq!(names(&export.component_types), ["helper", "job_tester"]);
    assert_eq!(
        definition(&export.component_types, "helper"),
        "type component helper {
  port job_control_port ctl;
}"
    );
    assert_eq!(
        definition(&export.component_types, "job_tester"),
        "type component job_tester {
  port job_port sut;
  port job_port ctl;
  timer deadline := 2.0;
  timer settle;
  var uint32 elapsed := 0;
}"
    );

    assert_eq!(
        names(&export.functions),
        ["f_helper", "f_job_tester_in_tester_and_helper"]
    );
    assert_eq!(
        definition(&export.functions, "f_helper"),
        "function f_helper() runs on helper {
  goto listening;
  label listening;
  alt {
    [] ctl.receive {
      setverdict(pass);
      return;
    }
  }
}"
    );
    let kill_helper = kill("      ", "v_helper");
    assert_eq!(
        definition(&export.functions, "f_job_tester_in_tester_and_helper"),
        format!(
            "function f_job_tester_in_tester_and_helper() runs on job_tester {{
  var helper v_helper := null;
  var job_status v_job_status;
  sut.send(job_report);
  deadline.start;
  goto requesting;
  label requesting;
  if (v_helper == null) {{
    v_helper := helper.create(\"helper\") alive;
    connect(mtc:ctl, v_helper:ctl);
    v_helper.start(f_helper());
  }}
  alt {{
    [] sut.receive(every_matcher(elapsed_limit)) -> value v_job_status {{
{kill_helper}      deadline.stop;
      elapsed := v_job_status.elapsed_ms;
      ctl.send(job_report);
      settle.start(0.5);
      goto settling;
    }}
    [] deadline.timeout {{
{kill_helper}      setverdict(fail, \"no reply in time\");
      return;
    }}
  }}
  label settling;
  alt {{
    [] settle.timeout {{
      setverdict(pass);
      return;
    }}
  }}
}}"
        )
    );
}

// ── The behaviour module (E14, E15) ────────────────────────────

/// The comment the behaviour module of the suite `suite` opens with, and the
/// opening of the module, which imports the types module whole.
fn behaviour_header(suite: &str) -> String {
    format!(
        "// The behaviour of the test suite {suite}, exported from its
// documents (docs/SCE_TEST_PROFILE.md, section 9.5).
module {suite} {{

import from {suite}_Types all;
"
    )
}

/// The job suite's behaviour module, whole: the port types, the templates,
/// the suite's module parameters and then its features' in the order of
/// their names, the component types and then the configuration's system
/// component type, the functions, the test case and the control part, which
/// executes the test case under its guard where both its features are on.
#[test]
fn the_job_suites_behaviour_module_is_exported() {
    let export = exported(&job_suite());
    let expected = format!(
        "{}
type port job_control_port message {{
  inout job_status;
  in event_schema;
}}

type port job_port message {{
  inout job_status;
  out event_schema;
}}

template uint32 elapsed_at_most(uint32 limit) := (-infinity .. limit);

template job_status every_matcher(uint32 limit) := {{
  elapsed_ms := (1, (10 .. limit)),
  note := ? length(0 .. 8) ifpresent,
  codes := {{ 3, ?, complement(0) }} length(1 .. 4),
  tag := *,
  retry := omit,
  count := elapsed_at_most(limit)
}};

template job_status job_report := {{
  elapsed_ms := 0,
  note := \"started on time\",
  codes := {{ 7 }},
  tag := omit,
  retry := omit,
  count := 1
}};

modulepar uint32 elapsed_limit := 500;
modulepar universal charstring run_label := \"nightly\"; // at most 8 bytes of UTF-8
modulepar octetstring key := '0AFF'O; // at most 2 octets
modulepar boolean PICS_JOB_RETRY;
modulepar boolean PICS_JOB_TIMEOUT;

type component helper {{
  port job_control_port ctl;
}}

type component job_tester {{
  port job_port sut;
  port job_port ctl;
  timer deadline := 2.0;
  timer settle;
  var uint32 elapsed := 0;
}}

type component tester_and_helper {{
  port job_port sut;
}}

function f_helper() runs on helper {{
  goto listening;
  label listening;
  alt {{
    [] ctl.receive {{
      setverdict(pass);
      return;
    }}
  }}
}}

function f_job_tester_in_tester_and_helper() runs on job_tester {{
  var helper v_helper := null;
  var job_status v_job_status;
  sut.send(job_report);
  deadline.start;
  goto requesting;
  label requesting;
  if (v_helper == null) {{
    v_helper := helper.create(\"helper\") alive;
    connect(mtc:ctl, v_helper:ctl);
    v_helper.start(f_helper());
  }}
  alt {{
    [] sut.receive(every_matcher(elapsed_limit)) -> value v_job_status {{
{kill_helper}      deadline.stop;
      elapsed := v_job_status.elapsed_ms;
      ctl.send(job_report);
      settle.start(0.5);
      goto settling;
    }}
    [] deadline.timeout {{
{kill_helper}      setverdict(fail, \"no reply in time\");
      return;
    }}
  }}
  label settling;
  alt {{
    [] settle.timeout {{
      setverdict(pass);
      return;
    }}
  }}
}}

testcase completes_in_time() runs on job_tester system tester_and_helper {{
  map(mtc:sut, system:sut);
  f_job_tester_in_tester_and_helper();
}}

control {{
  if (PICS_JOB_TIMEOUT and PICS_JOB_RETRY) {{
    execute(completes_in_time(), 10.0);
  }}
}}

}}",
        behaviour_header("job_suite"),
        kill_helper = kill("      ", "v_helper"),
    );
    assert_eq!(export.behaviour_module, expected);
    assert_eq!(
        names(&export.feature_parameters),
        ["PICS_JOB_RETRY", "PICS_JOB_TIMEOUT"]
    );
    assert_eq!(names(&export.system_types), ["tester_and_helper"]);
    assert_eq!(names(&export.test_cases), ["completes_in_time"]);
}

/// The runtime's suite, whole: four test cases in three configurations, one
/// of which joins no port to the test system and so has an empty system
/// component type; a configuration two test cases run in is written once.
/// The control part executes each in the suite's order: the first with a
/// guard and no selection, the second with a selection and no guard, a
/// conjunction, and a disjunction under a guard of a tenth of a millisecond.
#[test]
fn the_runtime_suites_behaviour_module_is_exported() {
    let export = exported(&runtime_suite());
    let functions: Vec<&str> = export.functions.iter().map(|f| f.text.as_str()).collect();
    let expected = format!(
        "{}
type port exchange_port message {{
  out job_request;
  in job_reply;
}}

type port serving_port message {{
  in job_request;
  out job_reply;
}}

template job_request any_request := {{
  job := ?
}};

template job_reply reply_of(uint32 answers, uint32 code) := {{
  job := answers,
  code := code
}};

template job_reply reply_ok := {{
  job := ?,
  code := 0
}};

template job_request request_of(uint32 wanted) := {{
  job := wanted
}};

modulepar uint32 job := 7;
modulepar boolean PICS_GUARD;
modulepar boolean PICS_PTCS;
modulepar boolean PICS_REMOVAL;

type component answerer {{
  port exchange_port peer;
  var uint32 asked := 42;
}}

type component coordinator {{
  port serving_port peer;
  var uint32 requested := 0;
  var uint32 ok := 0;
}}

type component idler {{
  port exchange_port peer;
}}

type component param_asker {{
  port exchange_port sut;
  timer deadline := 1.0;
  var uint32 answered := 0;
}}

type component ptc_keeper {{
  timer patience := 0.01;
  timer linger := 0.005;
}}

type component asking_configuration {{
  port exchange_port sut;
}}

type component keeping_configuration {{
  port exchange_port quiet;
}}

type component serving_configuration {{
}}

{}

testcase answers_the_job() runs on param_asker system asking_configuration {{
  map(mtc:sut, system:sut);
  f_param_asker();
}}

testcase serves_a_ptc() runs on coordinator system serving_configuration {{
  f_coordinator_in_serving_configuration();
}}

testcase keeps_a_ptc() runs on ptc_keeper system keeping_configuration {{
  f_ptc_keeper_in_keeping_configuration();
}}

testcase answers_before_the_guard() runs on param_asker system asking_configuration {{
  map(mtc:sut, system:sut);
  f_param_asker();
}}

control {{
  execute(answers_the_job(), 10.0);
  if (PICS_PTCS) {{
    execute(serves_a_ptc());
  }}
  if (PICS_PTCS and PICS_REMOVAL) {{
    execute(keeps_a_ptc());
  }}
  if (PICS_GUARD or PICS_REMOVAL) {{
    execute(answers_before_the_guard(), 0.0001);
  }}
}}

}}",
        behaviour_header("runtime_suite"),
        functions.join("\n\n"),
    );
    // The functions are held to their text by
    // `the_runtime_suites_behaviour_is_exported`; here, to their place.
    assert_eq!(functions.len(), 5);
    assert_eq!(export.behaviour_module, expected);
    assert_eq!(
        names(&export.system_types),
        [
            "asking_configuration",
            "keeping_configuration",
            "serving_configuration"
        ]
    );
    assert_eq!(
        names(&export.test_cases),
        [
            "answers_the_job",
            "serves_a_ptc",
            "keeps_a_ptc",
            "answers_before_the_guard"
        ]
    );
}

/// The kind examples: the features of the feature-model example, the test
/// case of the test-case example and the control part that selects it.
#[test]
fn the_kind_examples_test_case_and_control_are_exported() {
    let export = exported(&kind_example_suite());
    assert_eq!(
        names(&export.feature_parameters),
        [
            "PICS_SESSION_AUTH",
            "PICS_SESSION_UNICAST",
            "PICS_SESSION_UNICAST_ACCEPT"
        ]
    );
    assert_eq!(
        definition(&export.feature_parameters, "PICS_SESSION_UNICAST_ACCEPT"),
        "modulepar boolean PICS_SESSION_UNICAST_ACCEPT;"
    );
    assert_eq!(
        definition(&export.system_types, "test_configuration"),
        "type component test_configuration {
  port test_port_type sut;
}"
    );
    assert_eq!(
        definition(&export.test_cases, "test_case"),
        "testcase test_case() runs on test_component system test_configuration {
  map(mtc:sut, system:sut);
  f_test_component();
}"
    );
    assert_eq!(
        export.control,
        "control {
  if (PICS_SESSION_UNICAST) {
    execute(test_case(), 10.0);
  }
}"
    );
}

/// Each test case, system component type, module parameter of a feature and
/// statement of the control part names the construct it was translated from
/// and its rule (docs/SCE_TEST_PROFILE.md §9.4).
#[test]
fn each_test_case_and_execute_names_its_source() {
    let export = exported(&job_suite());
    let execution = &export.execution;

    let case = &execution.test_cases[0];
    assert_eq!(
        (case.source.rule, case.source.node.as_str()),
        ("E14", "scxml")
    );
    assert!(case.source.document.ends_with("completes_in_time.scxml"));
    assert_eq!(case.component, "job_tester");
    assert_eq!(case.system, "tester_and_helper");
    assert_eq!(case.function, "f_job_tester_in_tester_and_helper");

    let system = &execution.system_types[0];
    assert_eq!(system.source.rule, "E14");
    assert!(system.source.document.ends_with("tester_and_helper.scxml"));
    assert_eq!(
        (
            system.ports[0].source.rule,
            system.ports[0].source.node.as_str()
        ),
        ("E14", "sce:system-port 'sut'")
    );

    let features: Vec<(&str, &str)> = execution
        .features
        .iter()
        .map(|f| (f.source.rule, f.source.node.as_str()))
        .collect();
    assert_eq!(
        features,
        [
            ("E15", "sce:feature 'job-retry'"),
            ("E15", "sce:feature 'job-timeout'")
        ]
    );
    assert!(execution
        .features
        .iter()
        .all(|f| f.source.document.ends_with("job_features.scxml") && f.value.is_none()));

    let execute = &execution.control.executes[0];
    assert_eq!(
        (execute.source.rule, execute.source.node.as_str()),
        ("E15", "sce:execute 'CompletesInTime'")
    );
    assert!(execute.source.document.ends_with("job_suite.scxml"));
    assert!(execute.source.line.is_some());
    assert_eq!(execute.guard.as_deref(), Some("10.0"));
    assert_eq!(
        execute.condition.as_deref(),
        Some("PICS_JOB_TIMEOUT and PICS_JOB_RETRY")
    );
    assert_eq!(execution.control.source.rule, "E15");
}

/// The runtime's suite: a component that reads a module parameter and takes
/// an eventless choice, one that serves the PTC it creates over a connect,
/// one that maps the PTC it creates to the test system and removes it, and
/// the two PTCs.
#[test]
fn the_runtime_suites_behaviour_is_exported() {
    let export = exported(&runtime_suite());
    assert_eq!(
        definition(&export.module_parameters, "job"),
        "modulepar uint32 job := 7;"
    );
    assert_eq!(
        names(&export.component_types),
        [
            "answerer",
            "coordinator",
            "idler",
            "param_asker",
            "ptc_keeper"
        ]
    );
    assert_eq!(
        definition(&export.component_types, "coordinator"),
        "type component coordinator {
  port serving_port peer;
  var uint32 requested := 0;
  var uint32 ok := 0;
}"
    );
    assert_eq!(
        definition(&export.component_types, "param_asker"),
        "type component param_asker {
  port exchange_port sut;
  timer deadline := 1.0;
  var uint32 answered := 0;
}"
    );
    assert_eq!(
        definition(&export.component_types, "ptc_keeper"),
        "type component ptc_keeper {
  timer patience := 0.01;
  timer linger := 0.005;
}"
    );

    assert_eq!(
        names(&export.functions),
        [
            "f_answerer",
            "f_coordinator_in_serving_configuration",
            "f_idler",
            "f_param_asker",
            "f_ptc_keeper_in_keeping_configuration"
        ]
    );
    assert_eq!(
        definition(&export.functions, "f_answerer"),
        "function f_answerer() runs on answerer {
  peer.send(request_of(asked));
  log(\"asked\");
  goto asking;
  label asking;
  alt {
    [] peer.receive(reply_ok) {
      setverdict(pass);
      return;
    }
  }
}"
    );
    assert_eq!(
        definition(&export.functions, "f_idler"),
        "function f_idler() runs on idler {
  setverdict(inconc);
  goto idling;
  label idling;
  alt {
    [] peer.receive {
      return;
    }
  }
}"
    );
    // An eventless transition with a guard is a check taken before the
    // next, and one with none ends the state's checks: no alt follows it.
    assert_eq!(
        definition(&export.functions, "f_param_asker"),
        "function f_param_asker() runs on param_asker {
  var job_reply v_job_reply;
  sut.send(request_of(job));
  deadline.start;
  goto asking;
  label asking;
  alt {
    [] sut.receive(reply_ok) -> value v_job_reply {
      deadline.stop;
      answered := v_job_reply.job;
      goto checking;
    }
    [] deadline.timeout {
      setverdict(fail, \"no matching reply in time\");
      return;
    }
  }
  label checking;
  if (answered == job) {
    setverdict(pass);
    return;
  }
  setverdict(fail, \"the reply answers another job\");
  return;
}"
    );
    // The PTC is started at each atomic state its invoking state holds, and
    // the done branch of the invoking state is an alternative of each.
    let create_answerer = "  if (v_answerer == null) {
    v_answerer := answerer.create(\"answerer\") alive;
    connect(mtc:peer, v_answerer:peer);
    v_answerer.start(f_answerer());
  }
";
    let done = format!(
        "    [] v_answerer.done {{
{}      setverdict(pass);
      return;
    }}
",
        kill("      ", "v_answerer")
    );
    assert_eq!(
        definition(&export.functions, "f_coordinator_in_serving_configuration"),
        format!(
            "function f_coordinator_in_serving_configuration() runs on coordinator {{
  var answerer v_answerer := null;
  var job_request v_job_request;
  goto serving;
  label serving;
{create_answerer}  alt {{
    [] peer.receive(any_request) -> value v_job_request {{
      requested := v_job_request.job;
      peer.send(reply_of(requested, ok));
      goto served;
    }}
{done}  }}
  label served;
{create_answerer}  alt {{
{done}  }}
}}"
        )
    );
    assert_eq!(
        definition(&export.functions, "f_ptc_keeper_in_keeping_configuration"),
        format!(
            "function f_ptc_keeper_in_keeping_configuration() runs on ptc_keeper {{
  var idler v_idler := null;
  patience.start;
  goto watching;
  label watching;
  if (v_idler == null) {{
    v_idler := idler.create(\"idler\") alive;
    map(v_idler:peer, system:quiet);
    v_idler.start(f_idler());
  }}
  alt {{
    [] patience.timeout {{
{}      linger.start;
      goto lingering;
    }}
  }}
  label lingering;
  alt {{
    [] linger.timeout {{
      setverdict(pass);
      return;
    }}
  }}
}}",
            kill("      ", "v_idler")
        )
    );
}

/// The kind examples: a component that reads the suite's module parameter
/// as a template's argument.
#[test]
fn the_kind_examples_behaviour_is_exported() {
    let export = exported(&kind_example_suite());
    assert_eq!(
        definition(&export.module_parameters, "elapsed_limit"),
        "modulepar uint32 elapsed_limit := 500;"
    );
    assert_eq!(
        definition(&export.component_types, "test_component"),
        "type component test_component {
  port test_port_type sut;
  timer deadline := 2.0;
}"
    );
    assert_eq!(
        definition(&export.functions, "f_test_component"),
        "function f_test_component() runs on test_component {
  deadline.start;
  goto waiting;
  label waiting;
  alt {
    [] sut.receive(test_template(elapsed_limit)) {
      deadline.stop;
      setverdict(pass);
      return;
    }
    [] deadline.timeout {
      setverdict(fail, \"no reply in time\");
      return;
    }
  }
}"
    );
}

/// Every definition and statement of the behaviour names the construct it
/// was translated from and its rule (docs/SCE_TEST_PROFILE.md §9.4): the
/// component type the component, a PTC's creation and kill its `<invoke>`,
/// an alt its atomic state, a branch its transition.
#[test]
fn each_definition_and_statement_names_its_source() {
    let export = exported(&job_suite());
    let tester = export
        .behaviour
        .functions
        .iter()
        .find(|f| f.name == "f_job_tester_in_tester_and_helper")
        .expect("the tester's function");
    assert_eq!(tester.source.rule, "E5");
    assert!(tester.source.document.ends_with("job_tester.scxml"));
    assert_eq!(tester.configuration.as_deref(), Some("tester_and_helper"));
    let sources: Vec<(&str, &str)> = tester
        .body
        .iter()
        .map(|statement| {
            let source: &Source = match statement {
                Statement::Label { source, .. }
                | Statement::Goto { source, .. }
                | Statement::Return { source }
                | Statement::Send { source, .. }
                | Statement::StartTimer { source, .. }
                | Statement::StopTimer { source, .. }
                | Statement::Setverdict { source, .. }
                | Statement::Log { source, .. }
                | Statement::Assign { source, .. }
                | Statement::If { source, .. }
                | Statement::Create { source, .. }
                | Statement::Kill { source, .. }
                | Statement::Alt { source, .. } => source,
            };
            (source.rule, source.node.as_str())
        })
        .collect();
    assert_eq!(
        sources,
        [
            ("E9", "state 'requesting' / onentry 1 / send 1"),
            ("E10", "state 'requesting' / onentry 1 / sce_start_timer 2"),
            ("E8", "scxml"),
            ("E6", "state 'requesting'"),
            ("E11", "invoke 'helper'"),
            ("E7", "state 'requesting'"),
            ("E6", "state 'settling'"),
            ("E7", "state 'settling'"),
        ]
    );
    let Some(Statement::Alt { branches, .. }) = tester.body.get(5) else {
        panic!("the requesting state's alt: {:?}", tester.body);
    };
    let branch_sources: Vec<&str> = branches.iter().map(|b| b.source.node.as_str()).collect();
    assert_eq!(
        branch_sources,
        [
            "state 'requesting' / transition 1",
            "state 'requesting' / transition 2"
        ]
    );
    let Some(Statement::Kill { source, .. }) = branches[0].block.first() else {
        panic!(
            "the receive branch kills the helper first: {:?}",
            branches[0].block
        );
    };
    assert_eq!(
        (source.rule, source.node.as_str()),
        ("E11", "invoke 'helper'")
    );
    let component = export
        .behaviour
        .component_types
        .iter()
        .find(|c| c.name == "job_tester")
        .expect("the tester's type");
    assert_eq!(component.timers[0].source.node, "sce:test-timer 'deadline'");
    assert_eq!(component.timers[0].source.rule, "E4");
    assert_eq!(export.parameter_views[0].source.rule, "E15");
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
    violations(&wire_schema(), instance)
}

/// Why `instance` is not an instance of the draft-07 `schema`; empty when it
/// is one.
fn violations(schema: &serde_json::Value, instance: &serde_json::Value) -> Vec<String> {
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(schema)
        .expect("the schema compiles");
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

// ── The witness ────────────────────────────────────────────────

fn witness_schema() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../schemas/sce-ttcn3-witness.v1.schema.json"
    ))
    .expect("the witness's schema is JSON")
}

fn witness_of(export: &Ttcn3Export) -> serde_json::Value {
    serde_json::from_str(&export.witness).expect("a witness is JSON")
}

fn entries(witness: &serde_json::Value) -> &Vec<serde_json::Value> {
    witness["entries"].as_array().expect("entries")
}

/// One place of a witness: its module, its path, and the rule of its entry.
struct Place {
    module: String,
    path: serde_json::Value,
    rule: String,
}

/// Every place of `witness`, entry by entry.
fn places(witness: &serde_json::Value) -> Vec<Place> {
    entries(witness)
        .iter()
        .flat_map(|entry| {
            entry["produced"]
                .as_array()
                .expect("produced")
                .iter()
                .map(move |location| Place {
                    module: location["module"].as_str().expect("a module").to_string(),
                    path: location["path"].clone(),
                    rule: entry["rule"].as_str().expect("a rule").to_string(),
                })
        })
        .collect()
}

/// The statements `body` holds, every statement inside them, and each
/// alternative of an `alt`.
fn statements_in(body: &[Statement]) -> usize {
    body.iter()
        .map(|statement| {
            1 + match statement {
                Statement::If {
                    then, otherwise, ..
                } => statements_in(then) + statements_in(otherwise),
                Statement::Alt { branches, .. } => branches
                    .iter()
                    .map(|branch| 1 + statements_in(&branch.block))
                    .sum::<usize>(),
                _ => 0,
            }
        })
        .sum()
}

/// A definition of a module as the witness names it: the module, the kind
/// and the name of the definition, and the rule that wrote it.
type Named = (String, String, String, String);

/// Every witness the export writes is one the wire schema states, of the
/// suite and the two modules beside it.
#[test]
fn every_witness_the_export_writes_validates_against_the_wire_schema() {
    let schema = witness_schema();
    for suite in suites() {
        let export = exported(&suite);
        let witness = witness_of(&export);
        assert_eq!(
            violations(&schema, &witness),
            Vec::<String>::new(),
            "{}",
            suite.display()
        );
        assert_eq!(witness["schema"], "sce-ttcn3-witness.v1");
        assert_eq!(witness["suite"], export.suite.as_str());
        assert_eq!(
            witness["modules"],
            serde_json::json!([export.types_module_name(), export.suite])
        );
    }
}

/// Every definition, member of a component type and statement the export
/// writes is one place of the witness, counted from the model the modules
/// are written from, and no place is named twice; each definition of the two
/// modules is named, by the rule of its kind (docs/SCE_TEST_PROFILE.md §9.4).
#[test]
fn every_definition_and_statement_has_one_place_in_the_witness() {
    for suite in suites() {
        let export = exported(&suite);
        let manifest = manifest_of(&export);
        let witness = witness_of(&export);
        let found = places(&witness);

        let types = manifest["types"].as_array().expect("types");
        let codec = manifest["codec"].as_array().expect("codec");
        let components: usize = export
            .behaviour
            .component_types
            .iter()
            .chain(&export.execution.system_types)
            .map(|c| 1 + c.ports.len() + c.timers.len() + c.variables.len())
            .sum();
        let functions: usize = export
            .behaviour
            .functions
            .iter()
            .map(|f| 1 + f.locals.len() + statements_in(&f.body))
            .sum();
        let expected = types.len()
            + 2 * codec.len()
            + export.port_types.len()
            + export.templates.len()
            + export.module_parameters.len()
            + export.feature_parameters.len()
            + components
            + functions
            + export.test_cases.len()
            + 1
            + export.execution.control.executes.len();
        assert_eq!(found.len(), expected, "{}", suite.display());

        let mut unique: Vec<(String, String)> = found
            .iter()
            .map(|place| (place.module.clone(), place.path.to_string()))
            .collect();
        unique.sort();
        unique.dedup();
        assert_eq!(
            unique.len(),
            found.len(),
            "{}: a place is named twice",
            suite.display()
        );

        let mut definitions: Vec<Named> = found
            .iter()
            .filter(|place| place.path.as_array().is_some_and(|p| p.len() == 1))
            .map(|place| {
                let (kind, name) = place.path[0]
                    .as_object()
                    .and_then(|segment| segment.iter().next())
                    .expect("a segment of one member");
                (
                    place.module.clone(),
                    kind.clone(),
                    name.as_str().expect("a name").to_string(),
                    place.rule.clone(),
                )
            })
            .collect();
        definitions.sort();
        let types_module = export.types_module_name();
        let module = export.suite.clone();
        let named = |module: &str, kind: &str, name: &str, rule: &str| -> Named {
            (
                module.to_string(),
                kind.to_string(),
                name.to_string(),
                rule.to_string(),
            )
        };
        let mut stated: Vec<Named> = Vec::new();
        for ty in types {
            stated.push(named(
                &types_module,
                "type",
                ty["name"].as_str().expect("a name"),
                "E1",
            ));
        }
        for function in codec {
            for side in ["encode", "decode"] {
                stated.push(named(
                    &types_module,
                    "external-function",
                    function[side].as_str().expect("a name"),
                    "E1",
                ));
            }
        }
        let groups: [(&[Definition], &str, &str); 8] = [
            (&export.port_types, "port-type", "E3"),
            (&export.templates, "template", "E2"),
            (&export.module_parameters, "modulepar", "E15"),
            (&export.feature_parameters, "modulepar", "E15"),
            (&export.component_types, "component-type", "E4"),
            (&export.system_types, "component-type", "E14"),
            (&export.functions, "function", "E5"),
            (&export.test_cases, "testcase", "E14"),
        ];
        for (group, kind, rule) in groups {
            for definition in group {
                stated.push(named(&module, kind, &definition.name, rule));
            }
        }
        stated.push(named(&module, "control", &module, "E15"));
        stated.sort();
        assert_eq!(definitions, stated, "{}", suite.display());
    }
}

/// Every construct and rule has one entry, in the order of the document,
/// the row, the node and the rule, and every document is named relative to
/// the suite's directory, though the suite is read by an absolute path.
#[test]
fn every_construct_and_rule_has_one_entry() {
    type Key = (String, Option<u64>, String, String);
    for suite in suites() {
        let witness = witness_of(&exported(&suite));
        let keys: Vec<Key> = entries(&witness)
            .iter()
            .map(|entry| {
                let construct = &entry["construct"];
                (
                    construct["document"]
                        .as_str()
                        .expect("a document")
                        .to_string(),
                    construct["line"].as_u64(),
                    construct["node"].as_str().expect("a node").to_string(),
                    entry["rule"].as_str().expect("a rule").to_string(),
                )
            })
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(keys, sorted, "{}", suite.display());
        for (document, ..) in &keys {
            assert!(
                !Path::new(document).is_absolute(),
                "{}: {document} is named by an absolute path",
                suite.display()
            );
        }
    }
    let witness = witness_of(&exported(&job_suite()));
    let documents: Vec<&str> = entries(&witness)
        .iter()
        .map(|entry| entry["construct"]["document"].as_str().expect("a document"))
        .collect();
    assert!(documents.contains(&"job_suite.scxml"), "{documents:?}");
    assert!(documents.contains(&"job_tester.scxml"), "{documents:?}");
}

/// The witness of the job tester's `requesting` state, as the export writes
/// it: the label (E6); the alt (E7), which claims the transitions of its two
/// alternatives in textual order; and the receive branch (E8), whose entry
/// holds the alternative, which claims the exit of `requesting` and the
/// entry of `settling`, and the `goto` that ends its block. The timeout
/// branch enters the `<final>` `failed`, which ends the component (E12).
#[test]
fn the_witness_states_the_alt_of_the_job_testers_requesting_state() {
    let witness = witness_of(&exported(&job_suite()));
    let entry = |node: &str, rule: &str| {
        entries(&witness)
            .iter()
            .find(|e| e["construct"]["node"] == node && e["rule"] == rule)
            .cloned()
            .unwrap_or_else(|| panic!("no entry ({node}, {rule})"))
    };
    let tester = |node: &str, line: u32| serde_json::json!({"document": "job_tester.scxml", "node": node, "line": line});
    let function = serde_json::json!({"function": "f_job_tester_in_tester_and_helper"});
    let requesting = tester("state 'requesting'", 23);
    let receive = tester("state 'requesting' / transition 1", 29);
    let timeout = tester("state 'requesting' / transition 2", 35);

    assert_eq!(
        entry("state 'requesting'", "E6"),
        serde_json::json!({
            "construct": requesting,
            "rule": "E6",
            "produced": [
                {"module": "job_suite", "path": [function, {"label": "requesting"}]}
            ]
        })
    );
    assert_eq!(
        entry("state 'requesting'", "E7"),
        serde_json::json!({
            "construct": requesting,
            "rule": "E7",
            "produced": [{
                "module": "job_suite",
                "path": [function, {"label": "requesting"}, {"statement": 1}],
                "claim": {
                    "kind": "alt",
                    "state": requesting,
                    "alternatives": [receive, timeout]
                }
            }]
        })
    );
    assert_eq!(
        entry("state 'requesting' / transition 1", "E8"),
        serde_json::json!({
            "construct": receive,
            "rule": "E8",
            "produced": [
                {
                    "module": "job_suite",
                    "path": [
                        function, {"label": "requesting"}, {"statement": 1}, {"alternative": 0}
                    ],
                    "claim": {
                        "kind": "branch",
                        "state": requesting,
                        "transition": receive,
                        "exited": [requesting],
                        "entered": [tester("state 'settling'", 39)]
                    }
                },
                {
                    "module": "job_suite",
                    "path": [
                        function, {"label": "requesting"}, {"statement": 1}, {"alternative": 0},
                        {"statement": 5}
                    ]
                }
            ]
        })
    );
    let timed_out = entry("state 'requesting' / transition 2", "E8");
    assert_eq!(
        timed_out["produced"][0]["claim"]["entered"],
        serde_json::json!([tester("final 'failed'", 48)])
    );
    assert_eq!(
        entry("final 'failed'", "E12")["produced"],
        serde_json::json!([{
            "module": "job_suite",
            "path": [
                function, {"label": "requesting"}, {"statement": 1}, {"alternative": 1},
                {"statement": 2}
            ]
        }])
    );
}

/// The wire schema refuses a witness one member away from one the export
/// wrote, for each member a validator reads.
#[test]
fn the_witness_schema_rejects_what_the_export_does_not_write() {
    let schema = witness_schema();
    let valid = witness_of(&exported(&job_suite()));
    assert_eq!(violations(&schema, &valid), Vec::<String>::new());
    let position = |rule: &str| {
        entries(&valid)
            .iter()
            .position(|e| e["rule"] == rule && e["produced"][0].get("claim").is_some())
            .unwrap_or_else(|| panic!("no entry of {rule} that claims"))
    };
    let alt = position("E7");
    let branch = position("E8");
    let changed = |change: &dyn Fn(&mut serde_json::Value)| {
        let mut instance = valid.clone();
        change(&mut instance);
        instance
    };
    let changes: Vec<(&str, serde_json::Value)> = vec![
        (
            "another shape",
            changed(&|w| w["schema"] = serde_json::json!("sce-ttcn3-witness.v2")),
        ),
        (
            "one module",
            changed(&|w| w["modules"] = serde_json::json!(["job_suite"])),
        ),
        (
            "a rule the profile does not have",
            changed(&|w| w["entries"][0]["rule"] = serde_json::json!("E16")),
        ),
        (
            "an entry that produced nothing",
            changed(&|w| w["entries"][0]["produced"] = serde_json::json!([])),
        ),
        (
            "a construct without its row",
            changed(&|w| {
                w["entries"][0]["construct"]
                    .as_object_mut()
                    .expect("a construct")
                    .remove("line");
            }),
        ),
        (
            "a document named by an absolute path",
            changed(&|w| {
                w["entries"][0]["construct"]["document"] =
                    serde_json::json!("/work/completes_in_time.scxml")
            }),
        ),
        (
            "a path that opens with a statement",
            changed(&|w| {
                w["entries"][alt]["produced"][0]["path"][0] = serde_json::json!({"statement": 0})
            }),
        ),
        (
            "a segment of two members",
            changed(&|w| {
                w["entries"][alt]["produced"][0]["path"][1] =
                    serde_json::json!({"label": "requesting", "statement": 1})
            }),
        ),
        (
            "a position below 0",
            changed(&|w| {
                w["entries"][alt]["produced"][0]["path"][2] = serde_json::json!({"statement": -1})
            }),
        ),
        (
            "an alt's claim under the kind of an alternative's",
            changed(&|w| {
                w["entries"][alt]["produced"][0]["claim"]["kind"] = serde_json::json!("branch")
            }),
        ),
        (
            "an alternative's claim without the states it enters",
            changed(&|w| {
                w["entries"][branch]["produced"][0]["claim"]
                    .as_object_mut()
                    .expect("a claim")
                    .remove("entered");
            }),
        ),
    ];
    for (what, instance) in changes {
        assert!(
            !violations(&schema, &instance).is_empty(),
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

// ── Refusals of the behaviour ──────────────────────────────────

/// The record `report.scxml`, one number, which every probe below receives.
fn write_report(dir: &Path) {
    write_event_schema(
        dir,
        "report",
        "probe.report",
        "",
        "    <data id=\"elapsed_ms\" sce:type=\"uint32\" sce:direction=\"in\"/>\n",
    );
}

/// A template `probe_match.scxml` of the record, which takes the formal
/// parameter `parameter`, when one is named, as the upper bound of the number.
fn write_match(dir: &Path, parameter: Option<&str>) {
    let (declared, matcher) = match parameter {
        Some(name) => (
            format!(
                "  <datamodel>\n    <data id=\"{name}\" sce:type=\"uint32\" \
                 sce:direction=\"in\"/>\n  </datamodel>\n"
            ),
            format!(
                "    <sce:value-range field=\"elapsed_ms\">\n      <sce:upper param=\"{name}\"/>\n    \
                 </sce:value-range>\n"
            ),
        ),
        None => (
            String::new(),
            "    <sce:any-value field=\"elapsed_ms\"/>\n".to_string(),
        ),
    };
    write(
        dir,
        "probe_match",
        &document(
            "sce:kind=\"test-template\" name=\"probe_match\" sce:type=\"record:Report\"",
            &format!(
                "  <sce:import kind=\"event-schema\" src=\"report.scxml\" as=\"Report\"/>\n\
                 {declared}  <sce:record-match>\n{matcher}  </sce:record-match>\n"
            ),
        ),
    );
}

/// The imports of a probe that matches with `probe_match.scxml` and reads
/// the record it receives.
const MATCH_IMPORTS: &str =
    "  <sce:import kind=\"test-template\" src=\"probe_match.scxml\" as=\"Match\"/>\n  \
     <sce:import kind=\"event-schema\" src=\"report.scxml\" as=\"Report\"/>\n";

fn message_of(record: &serde_json::Value) -> &str {
    record["message"].as_str().unwrap_or_default()
}

/// A module parameter and a template's formal parameter of one name are one
/// identifier in the scope of the template, which the module's definitions
/// hold (ES 201 873-1 §5.2.2): the formal parameter is refused, where it is
/// written.
#[test]
fn a_formal_parameter_named_as_a_module_parameter_is_refused_by_name() {
    let dir = scratch();
    write_report(dir.path());
    write_match(dir.path(), Some("limit"));
    let suite = write_suite_of_component(
        dir.path(),
        "report",
        "waiting",
        MATCH_IMPORTS,
        "  <datamodel>\n    <data id=\"limit\" sce:type=\"uint32\" \
         sce:module-parameter=\"true\"/>\n  </datamodel>\n  <state id=\"waiting\">\n    \
         <transition sce:receive=\"sut\" sce:test-template=\"Match\" sce:template-args=\"limit\" \
         target=\"done\"/>\n  </state>\n  <final id=\"done\"/>\n",
        "  <sce:module-parameter id=\"limit\" type=\"uint32\" value=\"500\"/>\n",
    );
    let record = refusal(&suite);
    assert_eq!(record["code"], "test-profile/export-name", "{record}");
    assert_eq!(record["actual"], "limit", "{record}");
    assert_eq!(file_of(&record), "probe_match.scxml", "{record}");
    assert!(
        message_of(&record).contains(
            "the formal parameter 'limit' is one TTCN-3 identifier with the module parameter \
             'limit'"
        ),
        "{record}"
    );
}

/// A component's variable is in the scope of its component type, which the
/// module's definitions hold: one named as a module parameter is refused at
/// the variable.
#[test]
fn a_variable_named_as_a_module_parameter_is_refused_by_name() {
    let dir = scratch();
    write_report(dir.path());
    let suite = write_suite_of_component(
        dir.path(),
        "report",
        "waiting",
        "",
        "  <datamodel>\n    <data id=\"key\" sce:type=\"uint32\" expr=\"0\"/>\n  </datamodel>\n  \
         <state id=\"waiting\">\n    <transition sce:receive=\"sut\" target=\"done\"/>\n  \
         </state>\n  <final id=\"done\"/>\n",
        "  <sce:module-parameter id=\"key\" type=\"uint32\" value=\"1\"/>\n",
    );
    let record = refusal(&suite);
    assert_eq!(record["code"], "test-profile/export-name", "{record}");
    assert_eq!(record["actual"], "key", "{record}");
    assert_eq!(file_of(&record), "probe_tester.scxml", "{record}");
    assert!(
        message_of(&record).contains(
            "the variable 'key' is one TTCN-3 identifier with the module parameter 'key'"
        ),
        "{record}"
    );
}

/// A function that runs on a component type sees the type's variables: one
/// named as a local the export makes in the function, the variable a received
/// message is stored in, is refused at the variable, which an author wrote.
#[test]
fn a_variable_named_as_a_local_of_its_function_is_refused_by_name() {
    let dir = scratch();
    write_report(dir.path());
    write_match(dir.path(), None);
    let suite = write_suite_of_component(
        dir.path(),
        "report",
        "waiting",
        MATCH_IMPORTS,
        "  <datamodel>\n    <data id=\"v_report\" sce:type=\"uint32\" expr=\"0\"/>\n  \
         </datamodel>\n  <state id=\"waiting\">\n    <transition sce:receive=\"sut\" \
         sce:test-template=\"Match\" target=\"done\">\n      <assign location=\"v_report\" \
         expr=\"_event.data.elapsed_ms\"/>\n    </transition>\n  </state>\n  \
         <final id=\"done\"/>\n",
        "",
    );
    let record = refusal(&suite);
    assert_eq!(record["code"], "test-profile/export-name", "{record}");
    assert_eq!(record["actual"], "v_report", "{record}");
    assert_eq!(file_of(&record), "probe_tester.scxml", "{record}");
    assert!(
        message_of(&record).contains(
            "the variable 'v_report' is one TTCN-3 identifier with the variable a received \
             message is stored in 'v_report'"
        ),
        "{record}"
    );
}

/// A name the export makes in a function, the variable a received message is
/// stored in, and a state of that name are one identifier: the refusal
/// stands at the state, which an author wrote.
#[test]
fn a_state_named_as_a_name_the_export_makes_is_refused_by_name() {
    let dir = scratch();
    write_report(dir.path());
    write_match(dir.path(), None);
    let suite = write_suite_of_component(
        dir.path(),
        "report",
        "v_report",
        MATCH_IMPORTS,
        "  <datamodel>\n    <data id=\"seen\" sce:type=\"uint32\" expr=\"0\"/>\n  </datamodel>\n  \
         <state id=\"v_report\">\n    <transition sce:receive=\"sut\" sce:test-template=\"Match\" \
         target=\"done\">\n      <assign location=\"seen\" expr=\"_event.data.elapsed_ms\"/>\n    \
         </transition>\n  </state>\n  <final id=\"done\"/>\n",
        "",
    );
    let record = refusal(&suite);
    assert_eq!(record["code"], "test-profile/export-name", "{record}");
    assert_eq!(record["actual"], "v_report", "{record}");
    assert_eq!(file_of(&record), "probe_tester.scxml", "{record}");
    assert!(
        message_of(&record).contains(
            "the state 'v_report' is one TTCN-3 identifier with the variable a received message \
             is stored in 'v_report'"
        ),
        "{record}"
    );
}

/// `sce-static` adds at the width of its operands and fails the statement on
/// an overflow, and a TTCN-3 integer has no width: an addition is refused by
/// name, never written as one that computes another value.
#[test]
fn an_arithmetic_expression_is_refused_by_name() {
    let dir = scratch();
    write_report(dir.path());
    let suite = write_suite_of_component(
        dir.path(),
        "report",
        "waiting",
        "",
        "  <datamodel>\n    <data id=\"seen\" sce:type=\"uint32\" expr=\"0\"/>\n  </datamodel>\n  \
         <state id=\"waiting\">\n    <transition sce:receive=\"sut\" target=\"done\">\n      \
         <assign location=\"seen\" expr=\"seen + 1\"/>\n    </transition>\n  </state>\n  \
         <final id=\"done\"/>\n",
        "",
    );
    let record = refusal(&suite);
    assert_eq!(record["code"], "generate/unsupported-feature", "{record}");
    assert_eq!(file_of(&record), "probe_tester.scxml", "{record}");
    assert!(
        message_of(&record).contains("`seen + 1`")
            && message_of(&record).contains("state 'waiting' / transition 1 / assign 1"),
        "{record}"
    );
}

// ── The test cases and the control part (E14, E15) ─────────────

/// The feature model `probe_features.scxml`, declaring `features`.
fn write_features(dir: &Path, features: &[&str]) {
    let declared: String = features
        .iter()
        .map(|feature| format!("  <sce:feature name=\"{feature}\"/>\n"))
        .collect();
    write(
        dir,
        "probe_features",
        &document(
            "sce:kind=\"feature-model\" name=\"probe_features\"",
            &declared,
        ),
    );
}

/// The probe suite of [`write_suite`] over the record `report.scxml`, written
/// again to import `probe_features.scxml` and to hold `body` after its
/// imports; answer its path.
fn write_suite_of_features(dir: &Path, body: &str) -> PathBuf {
    write_report(dir);
    write_suite(dir, "report", false);
    write(
        dir,
        "probe_suite",
        &document(
            "sce:kind=\"test-suite\" name=\"probe_suite\"",
            &format!(
                "  <sce:import kind=\"feature-model\" src=\"probe_features.scxml\" \
                 as=\"Features\"/>\n  <sce:import kind=\"test-case\" src=\"probe_case.scxml\" \
                 as=\"ProbeCase\"/>\n{body}"
            ),
        ),
    )
}

/// A guard is the exact decimal of its seconds, read from its whole number
/// of microseconds, and a selection that joins a disjunction inside a
/// conjunction is written with the disjunction in parentheses. A test case
/// the suite neither guards nor selects is executed bare.
#[test]
fn a_guard_and_a_selection_are_written_as_the_suite_writes_them() {
    let dir = scratch();
    write_features(dir.path(), &["probe", "fast", "slow"]);
    let export = exported(&write_suite_of_features(
        dir.path(),
        "  <sce:execute test-case=\"ProbeCase\" guard=\"1500ms\" \
         requires=\"all(probe, any(fast, slow))\"/>\n",
    ));
    assert_eq!(
        export.control,
        "control {
  if (PICS_PROBE and (PICS_FAST or PICS_SLOW)) {
    execute(probe_case(), 1.5);
  }
}"
    );
    assert_eq!(
        names(&export.feature_parameters),
        ["PICS_FAST", "PICS_PROBE", "PICS_SLOW"]
    );
    assert_eq!(
        definition(&export.system_types, "probe_configuration"),
        "type component probe_configuration {
  port probe_port sut;
}"
    );
    assert_eq!(
        definition(&export.test_cases, "probe_case"),
        "testcase probe_case() runs on probe_tester system probe_configuration {
  map(mtc:sut, system:sut);
  f_probe_tester();
}"
    );

    let bare = scratch();
    write_report(bare.path());
    let export = exported(&write_suite(bare.path(), "report", false));
    assert_eq!(
        export.control,
        "control {
  execute(probe_case());
}"
    );
    assert!(export.feature_parameters.is_empty());
    assert!(!export.behaviour_module.contains("modulepar"));
}

/// A feature's module parameter is a name the export makes in the module's
/// scope: a module parameter the suite declares under that name is one
/// identifier with it, and is refused where the suite writes it.
#[test]
fn a_module_parameter_named_as_a_features_is_refused_by_name() {
    let dir = scratch();
    write_features(dir.path(), &["probe"]);
    let suite = write_suite_of_features(
        dir.path(),
        "  <sce:module-parameter id=\"PICS_PROBE\" type=\"uint32\" value=\"1\"/>\n  \
         <sce:execute test-case=\"ProbeCase\" requires=\"probe\"/>\n",
    );
    let record = refusal(&suite);
    assert_eq!(record["code"], "test-profile/export-name", "{record}");
    assert_eq!(record["actual"], "PICS_PROBE", "{record}");
    assert_eq!(file_of(&record), "probe_suite.scxml", "{record}");
    assert!(
        message_of(&record).contains(
            "the module parameter 'PICS_PROBE' is one TTCN-3 identifier with the feature's \
             module parameter 'PICS_PROBE'"
        ),
        "{record}"
    );
}

/// A configuration's system component type is named by the configuration's
/// unit name, a definition of the module: a module parameter of that name is
/// one identifier with it.
#[test]
fn a_module_parameter_named_as_a_configuration_is_refused_by_name() {
    let dir = scratch();
    write_features(dir.path(), &["probe"]);
    let suite = write_suite_of_features(
        dir.path(),
        "  <sce:module-parameter id=\"probe_configuration\" type=\"uint32\" value=\"1\"/>\n  \
         <sce:execute test-case=\"ProbeCase\"/>\n",
    );
    let record = refusal(&suite);
    assert_eq!(record["code"], "test-profile/export-name", "{record}");
    assert_eq!(record["actual"], "probe_configuration", "{record}");
    assert_eq!(file_of(&record), "probe_configuration.scxml", "{record}");
    assert!(
        message_of(&record).contains(
            "the system component type 'probe_configuration' is one TTCN-3 identifier with \
             the module parameter 'probe_configuration'"
        ),
        "{record}"
    );
}
