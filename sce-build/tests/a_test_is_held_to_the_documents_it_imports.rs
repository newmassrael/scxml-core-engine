// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The rules of a test's port types, configurations, test cases and suites
//! that need the documents they import (docs/adr/0091;
//! docs/SCE_TEST_PROFILE.md §11.3, §11.6, §11.7, §11.8): a set of documents
//! that holds them all and is accepted (port types, two components, a
//! configuration with a connect and a map, a test case, a suite with a module
//! parameter and a selection); one refusal per rule, with the code it gets and
//! the document and row it is placed on; the check where a document of the
//! profile is compiled; and the reading of a test component an import names.
//!
//! The documents are under `sce-build/tests/fixtures/test_profile/assembly/`:
//! the accepted set, the components and port types the refusals combine, and
//! one `refused_*` document per refusal, named for what it refuses.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::forge::import_source::ImportSource;
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::forge::test_profile_assembly;
use sce_build::generator::Language;
use sce_build::import_closure::{self, Role, UnitKind, WalkOptions};
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/assembly")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The fixture `name`, parsed as the forge document it is, and held to the
/// rules its imports decide. Its refusals carry its path.
fn check(name: &str) -> Result<(), Located<ForgeError>> {
    let path = dir().join(name);
    let text = read(&path);
    let label = path.to_str().expect("a UTF-8 path");
    let parsed = parse_forge_with_imports(
        &text,
        DocumentLabel {
            identifier: name.strip_suffix(".scxml").unwrap_or(name),
            diagnostic_label: label,
        },
    )
    .unwrap_or_else(|e| panic!("{name} parses: {e:?}"))
    .unwrap_or_else(|| panic!("{name} is a forge document"));
    test_profile_assembly::check(&parsed, &dir(), label)
}

/// The first record of a refusal: its wire code and its `actual`.
fn record(err: &Located<ForgeError>) -> (&'static str, Option<String>) {
    let diagnostic = &err.error.to_diagnostics()[0];
    (diagnostic.code.as_str(), diagnostic.actual.clone())
}

fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some("example.com/generated".to_string()),
        ..Default::default()
    }
}

/// Every document of the accepted set passes: the port types, each held to
/// distinct event names; the configuration, whose map joins the tester's port
/// to a system port that imports the same two schemas under other aliases,
/// whose connect joins two control ports, and whose PTC the tester creates;
/// the test case; and the suite, whose selection names its model's features
/// and which declares the tester's module parameter with its type.
#[test]
fn a_test_whose_documents_agree_is_accepted() {
    for name in [
        "ping_sut_port.scxml",
        "ping_control_port.scxml",
        "ping_system_port.scxml",
        "ping_listen_port.scxml",
        "ping_speak_port.scxml",
        "ping_configuration.scxml",
        "ping_case.scxml",
        "ping_suite.scxml",
    ] {
        if let Err(err) = check(name) {
            panic!("{name} is refused: {err:?}");
        }
    }
}

/// One refusal per rule its imports decide: the fixture, the code, the value
/// the record names as `actual` (`None` where the rule is about an element as
/// a whole), and the document the refusal is placed in.
const REFUSALS: &[(&str, DiagnosticCode, Option<&str>, &str)] = &[
    // §11.3: the port type.
    (
        "refused_port_type_event_name_twice.scxml",
        DiagnosticCode::TestProfileMessageEventName,
        Some("test.ping"),
        "refused_port_type_event_name_twice.scxml",
    ),
    // §11.6: the configuration's documents.
    (
        "refused_configuration_mtc_not_a_component.scxml",
        DiagnosticCode::ImportKindMismatch,
        Some("test-component"),
        "refused_configuration_mtc_not_a_component.scxml",
    ),
    (
        "refused_configuration_system_port_not_a_port_type.scxml",
        DiagnosticCode::ImportKindMismatch,
        Some("test-port-type"),
        "refused_configuration_system_port_not_a_port_type.scxml",
    ),
    // §11.6: the ports a connect or a map names.
    (
        "refused_configuration_map_port_undeclared.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("sut"),
        "refused_configuration_map_port_undeclared.scxml",
    ),
    (
        "refused_configuration_endpoint_port_undeclared.scxml",
        DiagnosticCode::ValidationInvalidAttribute,
        Some("ctl"),
        "refused_configuration_endpoint_port_undeclared.scxml",
    ),
    // §11.6, -1 §21.1.1 b and c: the lists of the joined ports.
    (
        "refused_configuration_connect_lists.scxml",
        DiagnosticCode::TestProfilePortLists,
        Some("control"),
        "refused_configuration_connect_lists.scxml",
    ),
    (
        "refused_configuration_connect_sends_nothing.scxml",
        DiagnosticCode::TestProfilePortLists,
        None,
        "refused_configuration_connect_sends_nothing.scxml",
    ),
    (
        "refused_configuration_map_sends_off_list.scxml",
        DiagnosticCode::TestProfilePortLists,
        Some("control"),
        "refused_configuration_map_sends_off_list.scxml",
    ),
    (
        "refused_configuration_map_receives_off_list.scxml",
        DiagnosticCode::TestProfilePortLists,
        Some("link"),
        "refused_configuration_map_receives_off_list.scxml",
    ),
    (
        "refused_configuration_map_crosses_nothing.scxml",
        DiagnosticCode::TestProfilePortLists,
        None,
        "refused_configuration_map_crosses_nothing.scxml",
    ),
    // §2.2, §11.6: every port in exactly one join.
    (
        "refused_configuration_port_unjoined.scxml",
        DiagnosticCode::TestProfilePortConnection,
        Some("control"),
        "refused_configuration_port_unjoined.scxml",
    ),
    // §11.6, P9: the PTCs and the invocations that create them.
    (
        "refused_configuration_ptc_not_invoked.scxml",
        DiagnosticCode::TestProfilePtcInvocation,
        Some("extra"),
        "refused_configuration_ptc_not_invoked.scxml",
    ),
    (
        "refused_configuration_invoke_names_no_ptc.scxml",
        DiagnosticCode::TestProfilePtcInvocation,
        Some("helper"),
        "refused_configuration_invoke_names_no_ptc.scxml",
    ),
    (
        "refused_configuration_ptc_invoked_twice.scxml",
        DiagnosticCode::TestProfilePtcInvocation,
        Some("helper"),
        "refused_configuration_ptc_invoked_twice.scxml",
    ),
    (
        "refused_configuration_ptc_of_another_type.scxml",
        DiagnosticCode::TestProfilePtcInvocation,
        Some("ping_helper.scxml"),
        "refused_configuration_ptc_of_another_type.scxml",
    ),
    (
        "refused_configuration_ptc_cycle.scxml",
        DiagnosticCode::TestProfilePtcInvocation,
        Some("a"),
        "refused_configuration_ptc_cycle.scxml",
    ),
    // §11.7: the test case.
    (
        "refused_case_configuration_not_a_configuration.scxml",
        DiagnosticCode::ImportKindMismatch,
        Some("test-configuration"),
        "refused_case_configuration_not_a_configuration.scxml",
    ),
    // §11.8: the suite.
    (
        "refused_suite_test_case_not_a_case.scxml",
        DiagnosticCode::ImportKindMismatch,
        Some("test-case"),
        "refused_suite_test_case_not_a_case.scxml",
    ),
    (
        "refused_suite_feature_undeclared.scxml",
        DiagnosticCode::ValidationFeatureUndeclared,
        Some("ping-burst"),
        "refused_suite_feature_undeclared.scxml",
    ),
    (
        "refused_suite_module_parameter_undeclared.scxml",
        DiagnosticCode::TestProfileModuleParameter,
        Some("limit"),
        "refused_suite_module_parameter_undeclared.scxml",
    ),
    (
        "refused_suite_module_parameter_type.scxml",
        DiagnosticCode::TestProfileModuleParameter,
        Some("uint16"),
        "refused_suite_module_parameter_type.scxml",
    ),
    // A suite reaches its test cases' configurations: the refusal of one is
    // the configuration's, placed in it.
    (
        "refused_suite_reaches_an_unjoined_port.scxml",
        DiagnosticCode::TestProfilePortConnection,
        Some("control"),
        "refused_configuration_port_unjoined.scxml",
    ),
];

#[test]
fn each_rule_the_imports_decide_is_refused_with_its_own_code() {
    let mut wrong: Vec<String> = Vec::new();
    for (name, code, actual, placed_in) in REFUSALS {
        let err = match check(name) {
            Ok(()) => {
                wrong.push(format!("{name}: accepted"));
                continue;
            }
            Err(err) => err,
        };
        let (got_code, got_actual) = record(&err);
        if got_code != code.as_str() {
            wrong.push(format!(
                "{name}: {got_code}, expected {}: {err:?}",
                code.as_str()
            ));
            continue;
        }
        if got_actual.as_deref() != *actual {
            wrong.push(format!(
                "{name}: actual {got_actual:?}, expected {actual:?}: {err:?}"
            ));
        }
        if !err.location.file.ends_with(placed_in) {
            wrong.push(format!(
                "{name}: placed in {}, expected {placed_in}",
                err.location.file
            ));
        }
        if err.location.line.is_none() {
            wrong.push(format!("{name}: placed on no line"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Every refusal fixture in the directory is in the table, so a fixture whose
/// row is lost cannot pass by not being read.
#[test]
fn every_refusal_fixture_is_judged() {
    let mut on_disk: Vec<String> = std::fs::read_dir(dir())
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
        .map(|(name, _, _, _)| (*name).to_string())
        .collect();
    judged.sort();
    assert_eq!(on_disk, judged);
}

/// A refusal stands on the row of what it names: the second message type of
/// one event name, the PTC nothing creates, the module parameter declared of
/// another type.
#[test]
fn each_refusal_is_placed_on_the_row_that_holds_it() {
    for (name, row) in [
        ("refused_port_type_event_name_twice.scxml", 11),
        ("refused_configuration_ptc_not_invoked.scxml", 13),
        ("refused_suite_module_parameter_type.scxml", 8),
    ] {
        let err = check(name).expect_err("refused");
        assert_eq!(err.location.line, Some(row), "{name}: {err:?}");
    }
}

/// The rules run where a document of the profile is compiled, before the
/// refusal a backend gives one until the profile's runtime lands there: the
/// accepted suite reaches that refusal on Swift, the last backend without any
/// kind of the profile, which generates no suite, and a suite that breaks a rule
/// is refused by the rule on Rust, which generates one.
#[test]
fn the_rules_run_where_a_document_of_the_profile_is_compiled() {
    assert!(!sce_build::forge::codegen_matrix::TEST_SUITE_BACKENDS.contains(&Language::Swift));
    match sce_build::compile_forge_file(
        &dir().join("ping_suite.scxml"),
        Language::Swift,
        &[],
        &options(),
    ) {
        Ok(_) => panic!("a test suite generated on Swift"),
        Err(err) => match err.error {
            ForgeError::Generate(boxed) => {
                assert!(
                    matches!(*boxed, GenerateError::UnsupportedFeature { .. }),
                    "{boxed:?}"
                );
            }
            other => panic!("the accepted suite is refused by a rule: {other:?}"),
        },
    }
    let Err(err) = sce_build::compile_forge_file(
        &dir().join("refused_suite_module_parameter_type.scxml"),
        Language::Rust,
        &[],
        &options(),
    ) else {
        panic!("a module parameter of another type is refused at the compile");
    };
    assert_eq!(
        record(&err).0,
        DiagnosticCode::TestProfileModuleParameter.as_str(),
        "{err:?}"
    );
}

/// A test component is a statechart: the import that names one reads it with
/// the statechart parser, and a document of another kind is no component.
#[test]
fn a_test_component_an_import_names_is_read_as_the_statechart_it_is() {
    let tester = ImportSource::read_src(&dir(), "ping_tester.scxml").expect("the tester reads");
    assert!(tester.is_test_component());
    let model = tester
        .parse_test_component()
        .unwrap_or_else(|e| panic!("the tester parses: {e:?}"))
        .expect("a test component");
    let profile = model.test_component.as_ref().expect("its profile");
    let ports: Vec<&str> = profile.ports.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ports, ["sut", "control"]);
    assert_eq!(profile.module_parameters, ["limit"]);
    assert_eq!(profile.invocations.len(), 1);
    assert_eq!(profile.invocations[0].id.as_deref(), Some("helper"));

    let port = ImportSource::read_src(&dir(), "ping_sut_port.scxml").expect("the port type reads");
    assert!(!port.is_test_component());
    assert!(port
        .parse_test_component()
        .unwrap_or_else(|e| panic!("{e:?}"))
        .is_none());
}

/// A run that generates a configuration with what it imports reaches its
/// components as statecharts' units, rather than refusing a statechart an
/// import names.
#[test]
fn a_walk_from_a_configuration_reaches_its_components() {
    let configuration = dir().join("ping_configuration.scxml").display().to_string();
    let units = import_closure::walk(std::slice::from_ref(&configuration), WalkOptions::default())
        .unwrap_or_else(|refusal| panic!("the walk was refused: {}", refusal.error));
    let components: Vec<(&str, &Role)> = units
        .iter()
        .filter(|unit| unit.kind == UnitKind::Statechart)
        .map(|unit| (unit.name.as_str(), &unit.role))
        .collect();
    assert_eq!(
        components,
        [
            ("ping_tester", &Role::Imported),
            ("ping_helper", &Role::Imported)
        ]
    );
}
