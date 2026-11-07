// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The six kinds of the test profile (docs/adr/0091; their forms are
//! docs/SCE_TEST_PROFILE.md §11): what a document of each reads as, each
//! refusal of what one document states alone and the code it is refused with,
//! that a test component's profile is read in a test component and in no
//! other statechart, and that a backend refuses to generate a document of the
//! profile by name until its runtime lands there for the document's kind.
//!
//! The documents are under `sce-build/tests/fixtures/test_profile/`: one per
//! accepted form, and one per refusal, each named for what it refuses. Of the
//! rules that need an imported document, a test component's are here (the
//! arguments, the port and the specificity of a template it uses, the PTC it
//! invokes); the declaration kinds' are not this file's.

use std::path::{Path, PathBuf};

use sce_build::forge::codegen_matrix::{self, EmitOutcome, KindClass};
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::forge::model::{
    ForgeDocument, ForgeKind, ParsedForge, RuntimeDep, TestMatcherForm, TestMessageDirection,
    TestValue,
};
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::generator::Language;
use sce_build::parser::SCXMLParser;
use sce_build::test_component::{TestStatement, Verdict};
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/test_profile")
        .join(name)
}

fn example(kind: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("kind-examples")
        .join(format!("{kind}.scxml"))
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

/// The fixture `name`, parsed as the forge document it is.
fn parse(name: &str) -> ParsedForge {
    let text = read(&fixture(name));
    parse_forge_with_imports(&text, label(name))
        .unwrap_or_else(|e| panic!("{name} parses: {e:?}"))
        .unwrap_or_else(|| panic!("{name} is a forge document"))
}

/// The fixture `name`, parsed as the statechart it is.
fn statechart(name: &str) -> Result<sce_build::model::SCXMLModel, Located<ForgeError>> {
    let path = fixture(name);
    SCXMLParser::new().parse_file(path.to_str().expect("a UTF-8 path"))
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

#[test]
fn a_port_type_reads_as_its_messages_and_their_directions() {
    let parsed = parse("job_port.scxml");
    let ForgeDocument::TestPortType(port) = &parsed.document else {
        panic!("a port type, got {:?}", parsed.document.kind());
    };
    let messages: Vec<(&str, TestMessageDirection)> = port
        .messages
        .iter()
        .map(|m| (m.schema.as_str(), m.direction))
        .collect();
    assert_eq!(
        messages,
        [
            ("JobStatus", TestMessageDirection::Inout),
            ("JobCompleted", TestMessageDirection::Out),
        ]
    );
    assert_eq!(port.in_list().collect::<Vec<_>>(), ["JobStatus"]);
    assert_eq!(
        port.out_list().collect::<Vec<_>>(),
        ["JobStatus", "JobCompleted"]
    );
}

#[test]
fn a_template_reads_as_its_type_its_parameters_and_its_matcher_tree() {
    let parsed = parse("every_matcher.scxml");
    let ForgeDocument::TestTemplate(template) = &parsed.document else {
        panic!("a template, got {:?}", parsed.document.kind());
    };
    assert_eq!(template.template_type.to_string(), "record:JobStatus");
    assert_eq!(template.parameters.len(), 1);
    assert_eq!(template.parameters[0].id, "limit");
    assert_eq!(template.parameters[0].parameter_type.to_string(), "uint32");
    let TestMatcherForm::RecordMatch { fields } = &template.matcher.form else {
        panic!("a record match, got {:?}", template.matcher.form);
    };
    let named: Vec<(&str, &str)> = fields
        .iter()
        .map(|f| (f.field.as_deref().unwrap_or_default(), f.form.element()))
        .collect();
    assert_eq!(
        named,
        [
            ("elapsed_ms", "value-list"),
            ("note", "any-value"),
            ("codes", "list-match"),
            ("tag", "any-value-or-none"),
            ("retry", "omit"),
            ("count", "template-ref"),
        ]
    );
    // The attributes every matcher may carry.
    assert!(fields[1].ifpresent);
    assert_eq!(fields[1].length_max, Some(8));
    assert_eq!(
        (fields[2].length_min, fields[2].length_max),
        (Some(1), Some(4))
    );
    // A value list's items, and a bound that is a formal parameter.
    let TestMatcherForm::ValueList { items } = &fields[0].form else {
        panic!("{:?}", fields[0].form);
    };
    assert!(matches!(
        &items[0].form,
        TestMatcherForm::Specific { value: TestValue::Value(v) } if v == "1"
    ));
    assert!(matches!(
        &items[1].form,
        TestMatcherForm::ValueRange {
            lower: Some(TestValue::Value(lower)),
            upper: Some(TestValue::Param(upper)),
        } if lower == "10" && upper == "limit"
    ));
    let TestMatcherForm::TemplateRef {
        template: alias,
        args,
    } = &fields[5].form
    else {
        panic!("{:?}", fields[5].form);
    };
    assert_eq!(alias, "ElapsedAtMost");
    assert_eq!(args, &["limit"]);
    // Matching symbols make a template no specific one (§6.2); a template of
    // values alone is.
    assert!(!template.is_specific());
    let report = parse("job_report.scxml");
    let ForgeDocument::TestTemplate(report) = &report.document else {
        panic!("a template");
    };
    assert!(report.is_specific());
}

#[test]
fn a_template_over_a_union_reads_as_the_case_it_matches() {
    let parsed = parse("shape_is_circle.scxml");
    let ForgeDocument::TestTemplate(template) = &parsed.document else {
        panic!("a template");
    };
    let TestMatcherForm::UnionMatch { variant, case } = &template.matcher.form else {
        panic!("{:?}", template.matcher.form);
    };
    assert_eq!(variant, "circle");
    assert!(matches!(
        case.as_deref().map(|m| &m.form),
        Some(TestMatcherForm::RecordMatch { .. })
    ));
}

#[test]
fn a_configuration_reads_as_its_components_and_how_their_ports_are_joined() {
    let parsed = parse("tester_and_helper.scxml");
    let ForgeDocument::TestConfiguration(configuration) = &parsed.document else {
        panic!("a configuration, got {:?}", parsed.document.kind());
    };
    assert_eq!(configuration.mtc, "Tester");
    assert_eq!(configuration.ptcs.len(), 1);
    assert_eq!(configuration.ptcs[0].id, "helper");
    assert_eq!(configuration.ptcs[0].component, "Helper");
    assert_eq!(configuration.system_ports[0].port_type, "JobPort");
    assert_eq!(configuration.maps[0].component, "mtc");
    assert_eq!(configuration.maps[0].system_port, "sut");
    let ends: Vec<(&str, &str)> = configuration.connects[0]
        .endpoints
        .iter()
        .map(|e| (e.component.as_str(), e.port.as_str()))
        .collect();
    assert_eq!(ends, [("mtc", "control"), ("helper", "control")]);
    assert!(configuration.names_component("mtc"));
    assert!(configuration.names_component("helper"));
    assert!(!configuration.names_component("Helper"));
}

#[test]
fn a_test_case_and_a_suite_read_as_what_they_name() {
    let parsed = parse("completes_in_time.scxml");
    let ForgeDocument::TestCase(case) = &parsed.document else {
        panic!("a test case, got {:?}", parsed.document.kind());
    };
    assert_eq!(case.configuration, "TesterAndHelper");

    let parsed = parse("job_suite.scxml");
    let ForgeDocument::TestSuite(suite) = &parsed.document else {
        panic!("a suite, got {:?}", parsed.document.kind());
    };
    assert_eq!(suite.features.as_deref(), Some("Features"));
    let parameters: Vec<(&str, String, &str)> = suite
        .module_parameters
        .iter()
        .map(|p| {
            (
                p.id.as_str(),
                p.parameter_type.to_string(),
                p.value.as_str(),
            )
        })
        .collect();
    assert_eq!(
        parameters,
        [
            ("limit", "uint32".to_string(), "500"),
            ("label", "string[8]".to_string(), "nightly"),
            ("key", "bytes[2]".to_string(), "0aff"),
        ]
    );
    assert_eq!(suite.executes.len(), 1);
    assert_eq!(suite.executes[0].test_case, "CompletesInTime");
    assert_eq!(suite.executes[0].guard_us, Some(10_000_000));
    assert_eq!(
        suite.executes[0].requires.as_deref(),
        Some("all(job-timeout, job-retry)")
    );
}

/// Every refusal of what one document of the profile states alone, by the
/// fixture that breaks it: its code, and the value the record names as
/// `actual` where the rule names one.
const REFUSALS: &[(&str, DiagnosticCode, Option<&str>)] = &[
    // §11.2, §11.3: the port type.
    (
        "refused_port_type_no_message.scxml",
        DiagnosticCode::ValidationMissingElement,
        None,
    ),
    (
        "refused_port_type_message_twice.scxml",
        DiagnosticCode::ValidationDuplicateId,
        Some("JobStatus"),
    ),
    (
        "refused_port_type_unimported_message.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("JobState"),
    ),
    (
        "refused_port_type_datamodel.scxml",
        DiagnosticCode::TestProfileDatamodel,
        Some("ecmascript"),
    ),
    // §11.4: the template.
    (
        "refused_template_no_type.scxml",
        DiagnosticCode::ValidationMissingAttribute,
        None,
    ),
    (
        "refused_template_float.scxml",
        DiagnosticCode::TestProfileTemplateFloat,
        Some("list<float32>[2]"),
    ),
    (
        "refused_template_two_matchers.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        Some("2"),
    ),
    (
        "refused_template_no_matcher.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        Some("0"),
    ),
    (
        "refused_template_parameter_out.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        Some("out"),
    ),
    (
        "refused_template_parameter_expr.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        Some("5"),
    ),
    (
        "refused_template_parameter_twice.scxml",
        DiagnosticCode::ValidationDuplicateId,
        Some("limit"),
    ),
    (
        "refused_template_value_list_of_one.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        Some("1"),
    ),
    (
        "refused_template_empty_complement.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        Some("0"),
    ),
    (
        "refused_template_field_missing.scxml",
        DiagnosticCode::ValidationMissingAttribute,
        None,
    ),
    (
        "refused_template_field_outside_record.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        Some("elapsed_ms"),
    ),
    (
        "refused_template_field_twice.scxml",
        DiagnosticCode::ValidationDuplicateId,
        Some("elapsed_ms"),
    ),
    (
        "refused_template_length_min_past_max.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        Some("5"),
    ),
    (
        "refused_template_literal_out_of_range.scxml",
        DiagnosticCode::TestProfileLiteral,
        Some("300"),
    ),
    (
        "refused_template_range_bound_not_integer.scxml",
        DiagnosticCode::TestProfileLiteral,
        Some("0x10"),
    ),
    (
        "refused_template_unknown_parameter.scxml",
        DiagnosticCode::TestProfileTemplateParameter,
        Some("limt"),
    ),
    (
        "refused_template_parameter_of_another_type.scxml",
        DiagnosticCode::TestProfileTemplateParameter,
        Some("wide"),
    ),
    (
        "refused_template_omit_not_optional.scxml",
        DiagnosticCode::TestProfileMatcherType,
        Some("omit"),
    ),
    (
        "refused_template_any_value_or_none_in_list.scxml",
        DiagnosticCode::TestProfileMatcherType,
        Some("any-value-or-none"),
    ),
    (
        "refused_template_ifpresent_not_optional.scxml",
        DiagnosticCode::TestProfileMatcherType,
        Some("ifpresent"),
    ),
    (
        "refused_template_specific_value_and_param.scxml",
        DiagnosticCode::ValidationExactlyOneAttribute,
        None,
    ),
    (
        "refused_template_unimported_reference.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("ElapsedAtLeast"),
    ),
    (
        "refused_template_union_case_of_two.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        Some("2"),
    ),
    (
        "refused_template_range_bound_twice.scxml",
        DiagnosticCode::ValidationDuplicateId,
        Some("lower"),
    ),
    // §11.6: the configuration.
    (
        "refused_configuration_no_mtc.scxml",
        DiagnosticCode::ValidationMissingElement,
        None,
    ),
    (
        "refused_configuration_two_mtcs.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        None,
    ),
    (
        "refused_configuration_ptc_named_mtc.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        Some("mtc"),
    ),
    (
        "refused_configuration_ptc_twice.scxml",
        DiagnosticCode::ValidationDuplicateId,
        Some("helper"),
    ),
    (
        "refused_configuration_unimported_component.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("Testor"),
    ),
    (
        "refused_configuration_map_unknown_component.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("ghost"),
    ),
    (
        "refused_configuration_map_unknown_system_port.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("system"),
    ),
    (
        "refused_configuration_system_port_unmapped.scxml",
        DiagnosticCode::TestProfilePortConnection,
        Some("sut"),
    ),
    (
        "refused_configuration_system_port_mapped_twice.scxml",
        DiagnosticCode::TestProfilePortConnection,
        Some("sut"),
    ),
    (
        "refused_configuration_port_joined_twice.scxml",
        DiagnosticCode::TestProfilePortConnection,
        Some("sut"),
    ),
    (
        "refused_configuration_connect_of_one.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        Some("1"),
    ),
    (
        "refused_configuration_system_port_twice.scxml",
        DiagnosticCode::ValidationDuplicateId,
        Some("sut"),
    ),
    // §11.7: the test case.
    (
        "refused_case_no_configuration.scxml",
        DiagnosticCode::ValidationMissingAttribute,
        None,
    ),
    (
        "refused_case_unimported_configuration.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("TesterAlone"),
    ),
    // §11.8: the suite.
    (
        "refused_suite_guard_no_unit.scxml",
        DiagnosticCode::TestProfileDuration,
        Some("10"),
    ),
    (
        "refused_suite_guard_below_a_microsecond.scxml",
        DiagnosticCode::TestProfileDuration,
        Some("0.0000005s"),
    ),
    (
        "refused_suite_test_case_twice.scxml",
        DiagnosticCode::ValidationDuplicateId,
        Some("CompletesInTime"),
    ),
    (
        "refused_suite_unimported_test_case.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("CompletesLate"),
    ),
    (
        "refused_suite_parameter_literal.scxml",
        DiagnosticCode::TestProfileLiteral,
        Some("300"),
    ),
    (
        "refused_suite_parameter_float.scxml",
        DiagnosticCode::ValidationAttributeRuleViolated,
        Some("float32"),
    ),
    (
        "refused_suite_parameter_twice.scxml",
        DiagnosticCode::ValidationDuplicateId,
        Some("limit"),
    ),
    (
        "refused_suite_condition_malformed.scxml",
        DiagnosticCode::ValidationFeatureConditionMalformed,
        Some("all(job-timeout"),
    ),
    (
        "refused_suite_condition_without_model.scxml",
        DiagnosticCode::ValidationFeatureUndeclared,
        Some("job-timeout"),
    ),
    (
        "refused_suite_two_feature_models.scxml",
        DiagnosticCode::ValidationFeatureModelImportedTwice,
        Some("../../../kind-examples/feature-model.scxml"),
    ),
];

#[test]
fn each_rule_one_document_states_is_refused_with_its_own_code() {
    let mut wrong: Vec<String> = Vec::new();
    for (name, code, actual) in REFUSALS {
        let text = read(&fixture(name));
        let err = match parse_forge_with_imports(&text, label(name)) {
            Ok(_) => {
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
        if let Some(actual) = actual {
            if got_actual.as_deref() != Some(*actual) {
                wrong.push(format!(
                    "{name}: actual {got_actual:?}, expected {actual:?}"
                ));
            }
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
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile");
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
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
        .chain(
            COMPONENT_REFUSALS
                .iter()
                .map(|(name, _, _)| (*name).to_string()),
        )
        .chain(["refused_component_datamodel.scxml"].map(str::to_string))
        .collect();
    judged.sort();
    assert_eq!(on_disk, judged);
}

/// A refusal is placed on the row of the value it names.
#[test]
fn a_literal_outside_its_type_is_refused_on_its_own_row() {
    let text = read(&fixture("refused_template_literal_out_of_range.scxml"));
    let err = parse_forge_with_imports(&text, label("refused_template_literal_out_of_range.scxml"))
        .expect_err("refused");
    assert_eq!(err.location.line, Some(8));
}

#[test]
fn a_test_component_records_its_declarations_branches_sends_and_statements() {
    let model = statechart("job_tester.scxml").unwrap_or_else(|e| panic!("parses: {e:?}"));
    assert_eq!(model.kind(), ForgeKind::TestComponent);
    let profile = model
        .test_component
        .as_ref()
        .expect("a test component's profile");
    let ports: Vec<(&str, &str)> = profile
        .ports
        .iter()
        .map(|p| (p.id.as_str(), p.port_type.as_str()))
        .collect();
    assert_eq!(ports, [("sut", "JobPort"), ("control", "JobPort")]);
    let timers: Vec<(&str, Option<&str>)> = profile
        .timers
        .iter()
        .map(|t| (t.id.as_str(), t.duration.as_deref()))
        .collect();
    assert_eq!(timers, [("reply", Some("2s")), ("settle", None)]);
    assert_eq!(profile.module_parameters, ["limit"]);
    assert_eq!(profile.invocations.len(), 1);
    assert_eq!(profile.invocations[0].id.as_deref(), Some("helper"));
    assert_eq!(profile.invocations[0].src.as_deref(), Some("helper.scxml"));
    assert_eq!(profile.invocations[0].state, "requesting");

    // Each statement is an action of the block that holds it, in its place
    // among the block's other actions (P4), so it runs where it is written.
    let types = |actions: &[sce_build::model::Action]| -> Vec<String> {
        actions.iter().map(|a| a.action_type.clone()).collect()
    };
    let statement = |actions: &[sce_build::model::Action], at: usize| -> TestStatement {
        actions[at]
            .test_statement
            .clone()
            .unwrap_or_else(|| panic!("action {at} is a statement: {:?}", types(actions)))
    };
    let requesting = &model.states["requesting"];
    let settling = &model.states["settling"];
    let entry = &requesting.on_entry_blocks[0];
    let received = &requesting.transitions[0].actions;
    let timed_out = &requesting.transitions[1].actions;
    assert_eq!(types(entry), ["send", "sce_start_timer"]);
    assert_eq!(types(received), ["sce_stop_timer", "assign", "send"]);
    assert_eq!(types(timed_out), ["sce_setverdict"]);
    assert_eq!(types(&settling.on_entry_blocks[0]), ["sce_start_timer"]);
    assert_eq!(types(&settling.transitions[0].actions), ["sce_setverdict"]);
    assert_eq!(
        statement(entry, 1),
        TestStatement::StartTimer {
            timer: "reply".into(),
            duration: None
        }
    );
    assert_eq!(
        statement(received, 0),
        TestStatement::StopTimer {
            timer: "reply".into()
        }
    );
    assert_eq!(
        statement(timed_out, 0),
        TestStatement::Setverdict {
            verdict: Verdict::Fail,
            reason: Some("'no reply in time'".into())
        }
    );
    assert_eq!(
        statement(&settling.on_entry_blocks[0], 0),
        TestStatement::StartTimer {
            timer: "settle".into(),
            duration: Some("500ms".into())
        }
    );
    assert_eq!(
        statement(&settling.transitions[0].actions, 0),
        TestStatement::Setverdict {
            verdict: Verdict::Pass,
            reason: None
        }
    );

    let receive = requesting.transitions[0]
        .test_branch
        .as_ref()
        .expect("a receive branch");
    assert_eq!(receive.receive.as_deref(), Some("sut"));
    assert_eq!(receive.template.template.as_deref(), Some("EveryMatcher"));
    assert_eq!(receive.template.args, ["limit"]);
    let timeout = requesting.transitions[1]
        .test_branch
        .as_ref()
        .expect("a timeout branch");
    assert_eq!(timeout.timeout.as_deref(), Some("reply"));
    assert!(timeout.receive.is_none());

    let send = requesting
        .on_entry_blocks
        .iter()
        .flatten()
        .find(|a| a.action_type == "send")
        .expect("the entry sends");
    let used = send
        .test_template
        .as_ref()
        .expect("the send names its template");
    assert_eq!(used.template.as_deref(), Some("JobReport"));
    assert!(used.args.is_empty());
}

#[test]
fn a_test_component_declares_the_static_data_model() {
    let err = statechart("refused_component_datamodel.scxml").expect_err("refused");
    let (code, actual, _) = record(&err);
    assert_eq!(code, DiagnosticCode::TestProfileDatamodel.as_str());
    assert_eq!(actual.as_deref(), Some("ecmascript"));
}

/// Every refusal of a test component (docs/adr/0091 §3;
/// docs/SCE_TEST_PROFILE.md §8.2, §11.5), by the fixture that breaks it: its
/// code, and the value the record names as `actual` where the rule names one.
/// Each fixture is read from its directory, so the rules that read the port
/// types, templates and components it names are asked too.
const COMPONENT_REFUSALS: &[(&str, DiagnosticCode, Option<&str>)] = &[
    // §8.2: the elements a component does not hold.
    (
        "refused_component_parallel.scxml",
        DiagnosticCode::TestProfileComponentConstruct,
        Some("parallel"),
    ),
    // §11.8: feature conditions appear only on a suite's <sce:execute>.
    (
        "refused_component_feature_model.scxml",
        DiagnosticCode::TestProfileComponentConstruct,
        Some("feature-model"),
    ),
    (
        "refused_component_history.scxml",
        DiagnosticCode::TestProfileComponentConstruct,
        Some("history"),
    ),
    (
        "refused_component_script.scxml",
        DiagnosticCode::TestProfileComponentConstruct,
        Some("script"),
    ),
    (
        "refused_component_donedata.scxml",
        DiagnosticCode::TestProfileComponentConstruct,
        Some("donedata"),
    ),
    (
        "refused_component_raise.scxml",
        DiagnosticCode::TestProfileComponentConstruct,
        Some("raise"),
    ),
    (
        "refused_component_cancel.scxml",
        DiagnosticCode::TestProfileComponentConstruct,
        Some("cancel"),
    ),
    // ADR 0091 §3 and §4.4: native code, and the message read outside a
    // receive branch's content.
    (
        "refused_component_native_condition.scxml",
        DiagnosticCode::TestProfileNativeCode,
        Some("cpp:ready()"),
    ),
    (
        "refused_component_cond_reads_message.scxml",
        DiagnosticCode::TestProfileMessageRead,
        Some("_event.data.count > 0"),
    ),
    (
        "refused_component_timeout_reads_message.scxml",
        DiagnosticCode::TestProfileMessageRead,
        Some("_event.data.count"),
    ),
    // ES 201 873-1 §22.2.2, restriction d: a receive any branch stores no
    // message.
    (
        "refused_component_receive_any_reads_message.scxml",
        DiagnosticCode::TestProfileMessageRead,
        Some("_event.data.count"),
    ),
    // §11.5: a message is read through the record the component imports, and
    // held to that record's fields.
    (
        "refused_component_received_record_unimported.scxml",
        DiagnosticCode::TestProfileMessageRead,
        Some("EveryMatcher"),
    ),
    (
        "refused_component_received_field_unknown.scxml",
        DiagnosticCode::ExpressionUnknownMember,
        Some("_event.data.attempts"),
    ),
    // §8.2, §11.5: the send.
    (
        "refused_component_send_type.scxml",
        DiagnosticCode::TestProfileSend,
        Some("http://www.w3.org/TR/scxml/#SCXMLEventProcessor"),
    ),
    (
        "refused_component_send_internal.scxml",
        DiagnosticCode::TestProfileSend,
        Some("#_internal"),
    ),
    (
        "refused_component_send_delay.scxml",
        DiagnosticCode::TestProfileSend,
        Some("delay"),
    ),
    (
        "refused_component_send_event.scxml",
        DiagnosticCode::TestProfileSend,
        Some("event"),
    ),
    (
        "refused_component_send_param.scxml",
        DiagnosticCode::TestProfileSend,
        Some("param"),
    ),
    (
        "refused_component_send_unknown_port.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("sutt"),
    ),
    (
        "refused_component_send_no_template.scxml",
        DiagnosticCode::ValidationMissingAttribute,
        None,
    ),
    // §8.2, §11.5: the invocation of a PTC.
    (
        "refused_component_invoke_type.scxml",
        DiagnosticCode::TestProfileInvoke,
        Some("http://www.w3.org/TR/scxml/"),
    ),
    (
        "refused_component_invoke_autoforward.scxml",
        DiagnosticCode::TestProfileInvoke,
        Some("autoforward"),
    ),
    (
        "refused_component_invoke_idlocation.scxml",
        DiagnosticCode::TestProfileInvoke,
        Some("idlocation"),
    ),
    (
        "refused_component_invoke_finalize.scxml",
        DiagnosticCode::TestProfileInvoke,
        Some("finalize"),
    ),
    (
        "refused_component_invoke_no_id.scxml",
        DiagnosticCode::ValidationMissingAttribute,
        None,
    ),
    (
        "refused_component_invoke_not_component.scxml",
        DiagnosticCode::TestProfileInvoke,
        Some("job_port.scxml"),
    ),
    // §4.2, §8.2: the branches.
    (
        "refused_component_branch_event.scxml",
        DiagnosticCode::TestProfileBranch,
        Some("go"),
    ),
    (
        "refused_component_branch_two_kinds.scxml",
        DiagnosticCode::TestProfileBranch,
        Some("sce:timeout"),
    ),
    (
        "refused_component_branch_done_of_another_state.scxml",
        DiagnosticCode::TestProfileBranch,
        Some("done.invoke.helper"),
    ),
    (
        "refused_component_branch_template_without_receive.scxml",
        DiagnosticCode::TestProfileBranch,
        Some("sce:test-template"),
    ),
    (
        "refused_component_done_branch_stays.scxml",
        DiagnosticCode::TestProfileDoneBranch,
        Some("running"),
    ),
    (
        "refused_component_done_branch_no_target.scxml",
        DiagnosticCode::TestProfileDoneBranch,
        None,
    ),
    // §11.5: the declarations and what names them.
    (
        "refused_component_port_twice.scxml",
        DiagnosticCode::ValidationDuplicateId,
        Some("sut"),
    ),
    (
        "refused_component_timer_twice.scxml",
        DiagnosticCode::ValidationDuplicateId,
        Some("reply"),
    ),
    (
        "refused_component_port_type_unimported.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("JobPrt"),
    ),
    (
        "refused_component_timer_duration.scxml",
        DiagnosticCode::TestProfileDuration,
        Some("0.0000005s"),
    ),
    (
        "refused_component_start_timer_duration.scxml",
        DiagnosticCode::TestProfileDuration,
        Some("2"),
    ),
    (
        "refused_component_start_timer_no_duration.scxml",
        DiagnosticCode::ValidationMissingAttribute,
        None,
    ),
    (
        "refused_component_unknown_timer.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("replay"),
    ),
    (
        "refused_component_unknown_receive_port.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("ctl"),
    ),
    (
        "refused_component_unimported_template.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("JobReprot"),
    ),
    (
        "refused_component_unknown_template_argument.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("limt"),
    ),
    (
        "refused_component_module_parameter_expr.scxml",
        DiagnosticCode::TestProfileModuleParameterWritten,
        Some("5"),
    ),
    (
        "refused_component_module_parameter_assigned.scxml",
        DiagnosticCode::TestProfileModuleParameterWritten,
        Some("limit"),
    ),
    // §6.2, §11.5: a template against the documents it is used with.
    (
        "refused_component_template_args_count.scxml",
        DiagnosticCode::TestProfileTemplateParameter,
        Some("limit"),
    ),
    (
        "refused_component_template_args_type.scxml",
        DiagnosticCode::TestProfileTemplateParameter,
        Some("label"),
    ),
    (
        "refused_component_receive_not_in.scxml",
        DiagnosticCode::TestProfilePortDirection,
        Some("CompletedWithin"),
    ),
    (
        "refused_component_send_not_out.scxml",
        DiagnosticCode::TestProfilePortDirection,
        Some("JobReport"),
    ),
    (
        "refused_component_send_not_specific.scxml",
        DiagnosticCode::TestProfileSpecificTemplate,
        Some("EveryMatcher"),
    ),
    (
        "refused_component_send_not_specific_by_reference.scxml",
        DiagnosticCode::TestProfileSpecificTemplate,
        Some("ReportWithin"),
    ),
    // §11.5: a verdict's reason is an expression of type string, typed where
    // every expression of the data model is.
    (
        "refused_component_verdict_reason.scxml",
        DiagnosticCode::ExpressionTypeMismatch,
        Some("limit"),
    ),
];

#[test]
fn each_rule_of_a_test_component_is_refused_with_its_own_code() {
    let mut wrong: Vec<String> = Vec::new();
    for (name, code, actual) in COMPONENT_REFUSALS {
        let err = match statechart(name) {
            Ok(_) => {
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

/// A refusal of a test component is placed on the row of what it names: the
/// element a component does not hold, and the attribute that names an
/// undeclared timer.
#[test]
fn a_test_components_refusal_is_placed_on_its_own_row() {
    let row = |name: &str| statechart(name).expect_err("refused").location.line;
    assert_eq!(row("refused_component_raise.scxml"), Some(10));
    assert_eq!(row("refused_component_unknown_timer.scxml"), Some(14));
    assert_eq!(row("refused_component_done_branch_stays.scxml"), Some(11));
}

/// The components the profile's examples are, and the fixtures every other
/// test reads, hold to every rule a test component is asked: what they write
/// and what the port types, templates and components they name say.
#[test]
fn the_accepted_test_components_hold_every_rule() {
    for name in ["job_tester.scxml", "helper.scxml"] {
        statechart(name).unwrap_or_else(|e| panic!("{name}: {e:?}"));
    }
    let example = example("test-component");
    SCXMLParser::new()
        .parse_file(example.to_str().expect("a UTF-8 path"))
        .unwrap_or_else(|e| panic!("the test-component example: {e:?}"));
}

/// docs/SCE_TEST_PROFILE.md §11.5: the message a receive branch takes is an
/// event of its template's record, read through the record the component
/// imports, whatever alias each gives it; the branch's `_event.data` is that
/// record, and every other transition's payload stays its own event's.
#[test]
fn a_receive_branch_reads_the_message_as_its_templates_record() {
    let model = statechart("job_tester.scxml").unwrap_or_else(|e| panic!("parses: {e:?}"));
    assert_eq!(
        model
            .received_records
            .iter()
            .map(|(t, r)| (t.as_str(), r.as_str()))
            .collect::<Vec<_>>(),
        [("EveryMatcher", "Status")]
    );
    let received = model.received_events();
    let triggers: Vec<(&str, &str)> = model.states["requesting"]
        .transitions
        .iter()
        .map(|t| {
            let branch = t.test_branch.as_ref().expect("a branch");
            let kind = if branch.receive.is_some() {
                "receive"
            } else {
                "timeout"
            };
            (kind, t.payload_event(&received))
        })
        .collect();
    assert_eq!(triggers, [("receive", "job.status"), ("timeout", "")]);
}

/// In a statechart that is not a test component nothing reads the profile's
/// attributes, so each is refused as unread, on the row that holds it.
///
/// The document is written here, not committed as a fixture: the tree
/// commits no document carrying an `sce:` attribute nothing reads
/// (`an_sce_attribute_nothing_reads_is_refused_not_ignored`).
#[test]
fn a_statechart_that_is_no_test_component_does_not_read_a_branch() {
    let text = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="statechart" name="no_test_component"
       datamodel="null" initial="waiting">
  <state id="waiting">
    <transition event="go" sce:receive="sut" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;
    let err = SCXMLParser::new()
        .parse_string(text, "no_test_component")
        .expect_err("refused");
    let (code, _, line) = record(&err);
    assert_eq!(
        (code, line),
        (
            DiagnosticCode::ValidationSceAttributeUnread.as_str(),
            Some(6)
        )
    );
}

/// A statement of the profile in a statechart that is not a test component is
/// an element its executable content does not take, refused on its row rather
/// than skipped, so a verdict the author wrote is never dropped in silence.
#[test]
fn a_statechart_that_is_no_test_component_refuses_a_statement() {
    for (statement, child) in [
        (r#"<sce:setverdict value="pass"/>"#, "sce:setverdict"),
        (r#"<sce:start-timer timer="t"/>"#, "sce:start-timer"),
        (r#"<sce:stop-timer timer="t"/>"#, "sce:stop-timer"),
    ] {
        let text = format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="statechart" name="no_test_component"
       datamodel="null" initial="waiting">
  <state id="waiting">
    <onentry>
      {statement}
    </onentry>
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#
        );
        let err = SCXMLParser::new()
            .parse_string(&text, "no_test_component")
            .expect_err("refused");
        let (code, actual, line) = record(&err);
        assert_eq!(
            (code, actual.as_deref(), line),
            (
                DiagnosticCode::ValidationUnexpectedChildElement.as_str(),
                Some(child),
                Some(7)
            ),
            "{statement}"
        );
    }
}

/// The pseudocode page of a test component says what its profile declares,
/// a transition's branch and a send's template.
#[test]
fn a_test_components_page_shows_its_profile() {
    let model = statechart("job_tester.scxml").unwrap_or_else(|e| panic!("parses: {e:?}"));
    let page = sce_build::forge::pseudo::render(&ForgeDocument::Statechart(Box::new(model)))
        .unwrap_or_else(|e| panic!("renders: {e:?}"));
    for line in [
        "test-port sut type JobPort",
        "test-timer reply duration 2s",
        "module-parameter limit",
        "branch receive sut template EveryMatcher args limit",
        "branch timeout reply",
        "test-template JobReport",
        "start-timer reply",
        "stop-timer reply",
        "setverdict fail reason 'no reply in time'",
        "start-timer settle duration 500ms",
        "setverdict pass",
    ] {
        assert!(page.contains(line), "`{line}` is not on the page:\n{page}");
    }
}

fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some("example.com/generated".to_string()),
        ..Default::default()
    }
}

fn not_generated(kind: &str, language: Language, err: Located<ForgeError>) {
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{kind} on {language:?}: {err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => {
                assert!(detail.starts_with(&format!("{kind} '")), "{detail}");
                assert!(
                    detail.contains(
                        "a backend generates one of its kind only once the profile's runtime \
                         lands there for that kind"
                    ),
                    "{detail}"
                );
            }
            other => panic!("{kind} on {language:?}: {other:?}"),
        },
        other => panic!("{kind} on {language:?}: {other:?}"),
    }
}

/// docs/SCE_TEST_PROFILE.md §11.9: a backend refuses a document of the
/// profile by name until its runtime lands there for the document's kind, the
/// declaration kinds at the forge dispatch and the test component where a
/// machine is written. C++ generates every kind: a template
/// (`a_test_template_is_generated_in_cpp.rs`), a component
/// (`a_test_component_is_generated_in_cpp.rs`) and the tables of a test
/// (`a_test_suite_is_generated_in_cpp.rs`); each example generates there.
#[test]
fn a_document_of_the_profile_is_refused_by_name_where_its_kind_is_not_generated() {
    for &kind in ForgeKind::TEST_PROFILE {
        assert!(kind.is_supported(), "{kind:?}");
        assert!(!kind.needs_instance(), "{kind:?}");
        assert!(!kind.is_inline_eligible(), "{kind:?}");
        // A component's machine is a statechart's, on the SCE runtime, and the
        // tables of a test name those machines and the profile's runner.
        let tier = if matches!(
            kind,
            ForgeKind::TestComponent
                | ForgeKind::TestConfiguration
                | ForgeKind::TestCase
                | ForgeKind::TestSuite
        ) {
            RuntimeDep::SceRuntime
        } else {
            RuntimeDep::None
        };
        assert_eq!(kind.max_runtime_dep(), tier, "{kind:?}");
        assert_eq!(codegen_matrix::kind_class(kind), KindClass::Generic);
        assert_eq!(
            kind.reads_as_statechart(),
            kind == ForgeKind::TestComponent,
            "{kind:?}"
        );
    }
    for &language in Language::ALL {
        for kind in [
            "test-template",
            "test-port-type",
            "test-configuration",
            "test-case",
            "test-suite",
        ] {
            let parsed = ForgeKind::from_attr(kind).expect("a kind");
            // C++ generates every kind of the profile
            // (`a_test_template_is_generated_in_cpp.rs`,
            // `a_test_suite_is_generated_in_cpp.rs`); a port type writes no file.
            if language == Language::Cpp {
                assert_eq!(codegen_matrix::lookup(parsed, language), EmitOutcome::Emit);
                let output =
                    sce_build::compile_forge_file(&example(kind), language, &[], &options())
                        .unwrap_or_else(|e| panic!("{kind} does not generate on C++: {e:?}"));
                assert_eq!(
                    output.files.is_empty(),
                    parsed == ForgeKind::TestPortType,
                    "{kind}: {:?}",
                    output
                        .files
                        .iter()
                        .map(|(file, _)| file)
                        .collect::<Vec<_>>()
                );
                continue;
            }
            assert_eq!(
                codegen_matrix::lookup(parsed, language),
                EmitOutcome::RuntimeMissing,
                "{kind} on {language:?}"
            );
            match sce_build::compile_forge_file(&example(kind), language, &[], &options()) {
                Ok(_) => panic!("{kind} generated on {language:?}"),
                Err(err) => not_generated(kind, language, err),
            }
        }
        let template_dir = sce_build::find_template_dir_for(language);
        let component = example("test-component");
        let generated = sce_build::compile_scxml_lang_typed(
            component.to_str().expect("a UTF-8 path"),
            &template_dir,
            language,
        );
        // C++ generates a test component (`a_test_component_is_generated_in_cpp.rs`),
        // the example's module parameter read from the suite's value
        // (docs/SCE_TEST_PROFILE.md §12.6, slice 5).
        if language == Language::Cpp {
            assert_eq!(
                codegen_matrix::lookup(ForgeKind::TestComponent, language),
                EmitOutcome::Emit
            );
            if let Err(err) = generated {
                panic!("the example component does not generate on C++: {err:?}");
            }
            continue;
        }
        match generated {
            Ok(_) => panic!("test-component generated on {language:?}"),
            Err(err) => not_generated("test-component", language, err),
        }
    }
}
