// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A Rust machine is built from the values its deployment supplies (docs/adr/0109, D12;
// docs/SCE_ACCEPTED_SUBSET.md §2.15, "A value the deployment supplies"), run with no
// script engine. The committed machine under `src/integration/static_datamodel/` is
// generated from `sce-build/tests/fixtures/static_datamodel/static_deployment.scxml`
// (regen: `scripts/regen_static_datamodel_rust.sh`).
//
// The machine reads a record and a list, from the configuration its deployment writes
// under it (`deployment/deploy.yaml`, which the build checks and writes as
// `deployment/static_deployment.config.json`, held to that text by
// `sce-build/tests/a_machine_reads_the_values_its_deployment_supplies.rs`) and from a
// file a host names (`deployment/from_file.json`). A configuration that is no value of
// the types is refused by the reader with the lift's code, pointer and bound, and no
// machine is built. The C++ machine is held to the same texts by
// `tests/integration/AMachineReadsItsDeploymentsValuesTest.cpp`.

use sce_rust_runtime::event_payload::RefusalCode;
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_deployment_sm::{
    StaticDeploymentDeployment, StaticDeploymentPersist, StaticDeploymentPolicy,
};

/// The configuration the build wrote from the deployment.
const FROM_DEPLOYMENT: &str = include_str!(
    "../../../../sce-build/tests/fixtures/static_datamodel/deployment/static_deployment.config.json"
);
/// A configuration a host reads from the file the deployment names.
const FROM_FILE: &str =
    include_str!("../../../../sce-build/tests/fixtures/static_datamodel/deployment/from_file.json");
/// A configuration whose list holds one more element than its type does.
const TOO_MANY_HOSTS: &str = include_str!(
    "../../../../sce-build/tests/fixtures/static_datamodel/deployment/too_many_hosts.json"
);
/// A configuration that leaves out a value the machine declares.
const MISSING_HOSTS: &str = include_str!(
    "../../../../sce-build/tests/fixtures/static_datamodel/deployment/missing_hosts.json"
);

/// The machine built from `text` and started: what a host does at start.
fn started(text: &str) -> Engine<StaticDeploymentPolicy> {
    let deployment = StaticDeploymentDeployment::read(text).expect("the configuration reads");
    let mut engine = Engine::new(StaticDeploymentPolicy::new(deployment));
    let Ok(()) = engine.initialize();
    engine
}

fn take_row(engine: &mut Engine<StaticDeploymentPolicy>) {
    engine.raise_external_by_name("row.taken", "");
    engine.step();
}

/// The values the deployment writes are what the machine starts on: an initial value
/// reads each, the entry reads the record's text and the list's first element, and a
/// guard reads the count it started from.
#[test]
fn a_machine_starts_on_the_values_its_deployment_writes() {
    let mut engine = started(FROM_DEPLOYMENT);
    {
        let policy = engine.policy();
        assert_eq!(policy.remaining(), 2, "the record's `max_rows`");
        assert_eq!(policy.host_count(), 2, "the list's length");
        assert_eq!(policy.label(), "main", "the record's `label`");
        assert_eq!(policy.first_host(), "a.example", "the list's first element");
        assert_eq!(policy.refused(), 0);
    }
    for _ in 0..3 {
        take_row(&mut engine);
    }
    let policy = engine.policy();
    assert_eq!(policy.remaining(), 0, "two rows were taken");
    assert_eq!(policy.refused(), 1, "the third found none left");
}

/// A file a host reads is read by the same reader: its members in an order of their
/// own, and one the machine declares no value of stepped over.
#[test]
fn a_machine_starts_on_the_values_a_file_holds() {
    let engine = started(FROM_FILE);
    let policy = engine.policy();
    assert_eq!(policy.remaining(), 5);
    assert_eq!(policy.host_count(), 1);
    assert_eq!(policy.label(), "file");
    assert_eq!(policy.first_host(), "c.example");
}

/// The values are read once, into a value of the machine's own types, which the host
/// can hold and hand to the machine it builds.
#[test]
fn the_values_are_read_into_the_machines_own_types() {
    let deployment = StaticDeploymentDeployment::read(FROM_DEPLOYMENT).expect("reads");
    assert_eq!(deployment.limits.max_rows, 2);
    assert_eq!(&*deployment.limits.label, "main");
    assert_eq!(deployment.hosts.len(), 2);
    assert_eq!(
        StaticDeploymentDeployment::read(FROM_DEPLOYMENT).expect("reads again"),
        deployment,
        "the same text reads as the same values"
    );
}

/// A configuration that does not read refuses the start: the host is told the code,
/// the place and the bound, and has no deployment to build a machine from.
#[test]
fn a_configuration_that_does_not_read_refuses_the_start() {
    let refusal = StaticDeploymentDeployment::read(TOO_MANY_HOSTS).expect_err("five hosts");
    assert_eq!(refusal.code(), RefusalCode::TooMany);
    assert_eq!(refusal.pointer(), "/hosts");
    assert_eq!(refusal.bound(), Some(4));

    let refusal = StaticDeploymentDeployment::read(MISSING_HOSTS).expect_err("no hosts");
    assert_eq!(refusal.code(), RefusalCode::Missing);
    assert_eq!(refusal.pointer(), "/hosts");

    let refusal = StaticDeploymentDeployment::read("").expect_err("no text");
    assert_eq!(refusal.code(), RefusalCode::NotJson);
    let refusal = StaticDeploymentDeployment::read("[]").expect_err("no object");
    assert_eq!(refusal.code(), RefusalCode::NotObject);
}

/// The values are part of the machine's interface and not of its state
/// (docs/adr/0109, Open question 3): a saved state holds none of them, and a restore
/// into a machine built from other values keeps the saved state's variables and the
/// values the machine it restores into was built from.
#[test]
fn a_saved_state_holds_no_deployment_value() {
    let mut engine = started(FROM_DEPLOYMENT);
    take_row(&mut engine);
    let text = engine.save_at(0).expect("the machine saves").to_json();
    assert!(
        !text.contains("\"limits\"") && !text.contains("\"hosts\""),
        "no deployment value is in the saved state: {text}"
    );
    assert!(text.contains("\"remaining\":1"), "{text}");
    let restored = Engine::<StaticDeploymentPolicy>::restore_with(
        StaticDeploymentPolicy::new(StaticDeploymentDeployment::read(FROM_FILE).expect("reads")),
        &SavedState::from_json(&text).expect("reads back"),
        SceClock::Manual(0),
        0,
    )
    .expect("the state restores into a machine built from other values");
    let policy = restored.policy();
    assert_eq!(policy.remaining(), 1, "the variable the state saved");
    assert_eq!(policy.label(), "main", "the variable the state saved");
}
