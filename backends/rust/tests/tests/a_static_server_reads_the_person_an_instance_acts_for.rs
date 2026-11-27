// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0116, D3, D4 and D6 under `datamodel="sce-static"`: a server with no script engine
// reads the person an instance acts for, `_x.principal`, as the edge that took the event in
// stamped it on the event's metadata, and never a quoted one, `_x.quoted`, while no deployment
// lists a machine a quote is taken from.
//
// The machine under `src/integration/host_processor/` is generated from
// `sce-build/tests/fixtures/host_processor/statechart_static_reads_a_principal.scxml` by the
// crate's build script (build.rs, tree `rust-integration`). Its record of a person declares
// `issuer`, `subject` and `proved_ms`, and its transitions count the events said for `alice`,
// for another person and for nobody.

use sce_rust_runtime::saved_state::{self, SavedShape};
use sce_rust_runtime::{Engine, EventMetadata, EventType, Principal};
use sce_rust_tests::integration::host_processor::StatechartStaticForwardsNoPrincipalPolicy as Forwarding;
use sce_rust_tests::integration::host_processor::StatechartStaticReadsAPrincipalPolicy as Policy;

/// Any text, no record or enum lineage, and no product line: the engine-level `save` records
/// the shape it is told.
const SHAPE: SavedShape = SavedShape {
    first: "shape",
    revised: "shape",
    format2_first: "shape",
    format2_revised: "shape",
    legacy: &[],
    records: &[],
    enums: &[],
    features: None,
    migrations: &[],
};

/// When the people of these cases proved who they are, and when a machine is saved.
const PROVED_MS: u64 = 1_700_000_000_000;

/// `subject` of the issuer the document's `enter` transition requires, as the instance `a1`
/// acts for them.
fn person(subject: &str) -> Principal {
    Principal {
        issuer: "https://id.example".to_string(),
        subject: subject.to_string(),
        proved_ms: PROVED_MS,
        method: "test".to_string(),
        instance: "a1".to_string(),
    }
}

/// The metadata of an external event an instance sent, said for `person`.
fn said_for(person: Option<Principal>) -> EventMetadata {
    EventMetadata {
        event_type: EventType::External,
        principal: person.map(Box::new),
        ..EventMetadata::default()
    }
}

/// A machine standing at `serving`.
fn engine() -> Engine<Policy> {
    let mut e = Engine::new(Policy::new());
    let Ok(()) = e.initialize();
    e.step();
    e
}

/// Raise `event` said for `person` and run it.
fn run(e: &mut Engine<Policy>, event: &str, person: Option<Principal>) {
    e.raise_external_by_name_with_meta(event, &said_for(person));
    e.step();
}

/// A guard reads the person an event was said for, a field of the record by name, and the
/// person is absent on an event said for nobody and on an event the machine raises itself.
/// `_x.quoted` is absent on every event. Red if `populate_event_metadata` does not take the
/// person from the metadata, if the lowering reads another place, or if an internal event
/// keeps the person of the event before it.
#[test]
fn a_document_reads_the_person_its_event_was_said_for() {
    let mut e = engine();
    run(&mut e, "ping", Some(person("alice")));
    run(&mut e, "ping", Some(person("bob")));
    run(&mut e, "ping", None);
    e.raise_external_by_name("relay", "");
    e.step();
    let policy = e.policy();
    assert_eq!(
        (policy.alices(), policy.others(), policy.nobodys()),
        (1, 1, 2),
        "alice, bob, and nobody twice: an instance's event and the machine's own"
    );
    assert_eq!(policy.recent(), PROVED_MS);
    assert_eq!(policy.quoted(), 0, "no event carries a quote");
}

/// docs/adr/0116, D3 and D6: a person a field of the record does not hold (a subject past its
/// 64 bytes) refuses the delivery whole, as `error.execution`, and the event is not run as said
/// for nobody. Red if `admit_principal` cuts the text or drops the person.
#[test]
fn a_person_the_record_does_not_hold_refuses_the_delivery() {
    let mut e = engine();
    let long = "s".repeat(65);
    run(&mut e, "ping", Some(person(&long)));
    let policy = e.policy();
    assert_eq!(policy.refused(), 1, "error.execution in its place");
    assert_eq!(
        (policy.alices(), policy.others(), policy.nobodys()),
        (0, 0, 0),
        "the ping is not run"
    );
}

/// `sce:principal="required"` is the guard `_x.principal !== null`: an event said for nobody
/// does not select the transition, and its own condition reads the person where it is proven
/// present. Red if the parser does not join the guard before the condition.
#[test]
fn a_transition_that_requires_a_person_is_not_selected_without_one() {
    let mut e = engine();
    run(&mut e, "enter", None);
    assert_eq!(e.policy().entered(), 0, "said for nobody");
    run(&mut e, "enter", Some(person("alice")));
    assert_eq!(e.policy().entered(), 1);
    run(
        &mut e,
        "enter",
        Some(Principal {
            issuer: "https://elsewhere.example".to_string(),
            ..person("alice")
        }),
    );
    assert_eq!(e.policy().entered(), 1, "a person of another issuer");
}

/// docs/adr/0116, D4: nothing passes on by itself. A server that forwards what it is sent to its
/// child (autoforward) reads the person its event was said for, and the child, told the event
/// as said for nobody, answers that it read none. Red if the engine forwards the person with
/// the copy.
#[test]
fn a_child_is_told_a_forwarded_event_as_said_for_nobody() {
    // A machine that invokes a child is driven by `tick`, which runs the child's turn
    // as well as its own; `step` drives no child (`Engine::step`).
    let mut e = Engine::new(Forwarding::new());
    let Ok(()) = e.initialize();
    e.tick();
    e.raise_external_by_name_with_meta("ping", &said_for(Some(person("alice"))));
    // The parent runs `ping` and forwards it, the child runs it on the next tick, and its
    // answer is the parent's event after that.
    for _ in 0..4 {
        e.tick();
    }
    let policy = e.policy();
    assert_eq!(
        (
            policy.parent_saw(),
            policy.child_saw(),
            policy.child_saw_none()
        ),
        (1, 0, 1)
    );
}

/// docs/adr/0116, D6: a saved state holds no person, so a machine whose queue holds an event
/// said for one and not yet run refuses the save, and saves once it has run it. Red if
/// `saved_state::save` writes the event without its person, which a restore would run as
/// nobody's.
#[test]
fn a_machine_holding_an_unrun_event_of_a_person_does_not_save() {
    let mut e = engine();
    e.raise_external_by_name_with_meta("ping", &said_for(Some(person("alice"))));
    let refused = saved_state::save(&e, &SHAPE, Vec::new(), Vec::new(), Vec::new(), PROVED_MS)
        .expect_err("an unrun event of a person is not saved");
    assert!(
        refused.to_string().contains("said for a person"),
        "{refused}"
    );
    e.step();
    assert!(
        saved_state::save(&e, &SHAPE, Vec::new(), Vec::new(), Vec::new(), PROVED_MS).is_ok(),
        "saved once the event is run"
    );
}
