// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0124: the data of an event is read the same way and held to one
// contract whether a queue owns it or a host lends it for one call.
//
// The machine under `src/integration/lent_payload/` is generated from
// `sce-build/tests/fixtures/lent_payload/statechart_lent_payload.scxml` by the
// crate's build script (tree `rust-integration`), so it is really type-checked:
// a guard reads a text of at most eight bytes, content reads a byte string of at
// most eight, and content hands a payload whole to an imported algorithm.

use sce_rust_runtime::Engine;
use sce_rust_tests::integration::lent_payload::statechart_lent_payload_sm::{
    StatechartLentPayloadFrameReceivedPayload, StatechartLentPayloadFrameReceivedRef,
    StatechartLentPayloadInject, StatechartLentPayloadPolicy,
    StatechartLentPayloadTallyReceivedPayload, StatechartLentPayloadTallyReceivedRef,
};

fn machine() -> Engine<StatechartLentPayloadPolicy> {
    let mut engine = Engine::new(StatechartLentPayloadPolicy::new());
    let Ok(()) = engine.initialize();
    engine
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
