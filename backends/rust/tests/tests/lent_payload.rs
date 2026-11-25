// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0124: the data of an event is read the same way and held to one
// contract whether a queue owns it or a host lends it for one call.
//
// The machine under `src/integration/static_datamodel/` is generated from
// `sce-build/tests/fixtures/lent_payload/statechart_lent_payload.scxml` by the
// crate's build script (tree `rust-static-datamodel`), with every document it
// imports, so it is really type-checked:
// a guard reads a text of at most eight bytes, content reads a byte string of at
// most eight, and content hands a payload whole to an imported algorithm.

use sce_rust_runtime::{Engine, StatePolicy};
use sce_rust_tests::integration::static_datamodel::statechart_lent_payload_sm::{
    StatechartLentPayloadEvent, StatechartLentPayloadEventData,
    StatechartLentPayloadFrameReceivedPayload, StatechartLentPayloadFrameReceivedRef,
    StatechartLentPayloadInject, StatechartLentPayloadNoteReceivedRef, StatechartLentPayloadPolicy,
    StatechartLentPayloadState, StatechartLentPayloadTallyReceivedPayload,
    StatechartLentPayloadTallyReceivedRef,
};

fn machine() -> Engine<StatechartLentPayloadPolicy> {
    let mut engine = Engine::new(StatechartLentPayloadPolicy::new());
    let Ok(()) = engine.initialize();
    engine
}

/// The data of `note.received` as a wire carries it.
fn note_wire(note: &str) -> String {
    format!(r#"{{"note":"{note}"}}"#)
}

/// D2: a text past its bound is refused at the door whether a wire carries it
/// or a host lends it, in the same words, and the document answers the
/// `error.execution` taken in its place the same way on both routes.
#[test]
fn a_text_past_its_bound_is_refused_alike_queued_or_lent() {
    let long = "sentinel!";
    assert_eq!(long.len(), 9, "one byte past the note's bound of eight");

    let mut queued = machine();
    queued.raise_external_by_name("note.received", &note_wire(long));
    queued.step();

    let mut lent = machine();
    let received = long.to_string();
    lent.process_note_received_with(StatechartLentPayloadNoteReceivedRef { note: &received });
    drop(received);

    for (route, engine) in [("queued", &queued), ("lent", &lent)] {
        assert_eq!(
            engine.get_current_state(),
            StatechartLentPayloadState::Waiting,
            "{route}: a refused note takes no transition"
        );
        assert_eq!(
            engine.policy().refusals(),
            1,
            "{route}: the document answers the error.execution taken in its place"
        );
    }

    // The two doors refuse in the same words, which the error.execution carries.
    let lifted = StatechartLentPayloadPolicy::lift_event_payload(
        StatechartLentPayloadEvent::NoteReceived,
        &note_wire(long),
        &mut Default::default(),
    )
    .expect_err("a note past its bound, from the wire");
    let admitted = StatechartLentPayloadPolicy::admit_event_data(
        StatechartLentPayloadEventData::NoteReceived(StatechartLentPayloadNoteReceivedRef {
            note: long,
        }),
    )
    .expect_err("a note past its bound, lent");
    assert_eq!(lifted, admitted);
    assert_eq!(
        lifted.reason(),
        "'note' is 9 bytes, past the 8 its type declares"
    );
}

/// At its bound the note is taken on both routes, and the guard reads it.
#[test]
fn a_text_at_its_bound_is_taken_alike_queued_or_lent() {
    let mut queued = machine();
    queued.raise_external_by_name("note.received", &note_wire("sentinel"));
    queued.step();

    let mut lent = machine();
    lent.process_note_received_with(StatechartLentPayloadNoteReceivedRef { note: "sentinel" });

    for (route, engine) in [("queued", &queued), ("lent", &lent)] {
        assert_eq!(
            engine.get_current_state(),
            StatechartLentPayloadState::Noted,
            "{route}: the guard reads the note"
        );
        assert_eq!(engine.policy().refusals(), 0, "{route}: nothing refused");
    }
}

/// A byte string past its bound is refused whole at the door whether a wire
/// carries it or a host lends it, and the document answers the
/// `error.execution` taken in its place the same way on both routes.
#[test]
fn a_byte_string_past_its_bound_is_refused_alike_queued_or_lent() {
    let mut queued = machine();
    queued.raise_external_by_name("frame.received", r#"{"tag":5,"raw":"abcdefghi"}"#);
    queued.step();

    let mut lent = machine();
    lent.process_frame_received_with(StatechartLentPayloadFrameReceivedRef {
        tag: 5,
        raw: b"abcdefghi",
    });

    for (route, engine) in [("queued", &queued), ("lent", &lent)] {
        assert_eq!(
            engine.policy().seen(),
            0,
            "{route}: the content did not run"
        );
        assert_eq!(
            engine.policy().refusals(),
            1,
            "{route}: error.execution answered"
        );
    }
}

/// A payload handed whole to an imported algorithm crosses the call as the
/// record it is, read through its parts, from the queue and lent alike.
#[test]
fn a_payload_handed_whole_to_an_algorithm_crosses_alike_queued_or_lent() {
    let mut queued = machine();
    queued.raise_tally_received(StatechartLentPayloadTallyReceivedPayload { amount: 42 });
    queued.step();
    assert_eq!(queued.policy().total(), 42);

    let mut lent = machine();
    lent.process_tally_received_with(StatechartLentPayloadTallyReceivedRef { amount: 43 });
    assert_eq!(lent.policy().total(), 43);
}

/// A byte string within its bound is read alike from the queue and lent, from
/// a buffer the host drops right after the call.
#[test]
fn a_byte_string_is_read_alike_queued_or_lent() {
    let mut queued = machine();
    queued.raise_frame_received(StatechartLentPayloadFrameReceivedPayload {
        tag: 7,
        raw: ::sce_rust_runtime::SceBytes::from_slice(b"abc").unwrap(),
    });
    queued.step();
    assert_eq!(queued.policy().seen(), 7);

    let mut lent = machine();
    let received = b"abcdefgh".to_vec();
    lent.process_frame_received_with(StatechartLentPayloadFrameReceivedRef {
        tag: 9,
        raw: &received,
    });
    drop(received);
    assert_eq!(lent.policy().seen(), 9);
    assert_eq!(lent.policy().refusals(), 0);
}
