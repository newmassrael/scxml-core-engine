// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": the id a machine generates
// for a `<send idlocation>` is held by the document in a variable, and by the
// send that waits under it, so a saved state carries both and the count the next
// id comes from. A machine restored from it cancels the send by the id the
// document holds, and numbers the ids it generates next after the count: an id
// the saved state already holds is never generated again, which would let one
// `<cancel>` remove two sends.
//
// `static_send_idlocation.scxml`: `arm` sends two delayed events, `first.due` and
// `second.due`, each under an id written to `first` and `second`, and
// `cancel.first` and `cancel.second` remove the send whose id the variable holds.
// Everything runs on a host-owned clock, so no case sleeps. The shared fixture
// `saved/static_send_idlocation_armed.json` is the text the Kotlin suite writes
// and restores too.

use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_send_idlocation_sm::{
    StaticSendIdlocationPersist as Persist, StaticSendIdlocationPolicy as Policy,
};

const SHARED_ARMED: &str = include_str!(
    "../../../../sce-build/tests/fixtures/static_datamodel/saved/static_send_idlocation_armed.json"
);

/// The wall-clock moment the shared fixture was saved at: 2023-11-14T22:13:20Z.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

/// A machine that has run `arm` once, on a clock the test owns.
fn armed() -> Engine<Policy> {
    let mut engine = Engine::new(Policy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    send(&mut engine, "arm");
    engine
}

fn restored() -> Engine<Policy> {
    Engine::<Policy>::restore_with(
        Policy::new(),
        &SavedState::from_json(SHARED_ARMED).expect("reads"),
        SceClock::Manual(0),
        SAVED_AT_MS,
    )
    .expect("restores")
}

fn send(engine: &mut Engine<Policy>, name: &str) {
    engine.raise_external_by_name(name, "");
    engine.tick();
}

#[test]
fn a_machine_that_armed_two_sends_saves_the_ids_it_generated() {
    let engine = armed();
    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    assert_eq!(saved.auto_send_seq, 2, "two ids were generated");
    assert_eq!(saved.pending.len(), 2, "two sends wait");
    let text = saved.to_json();
    for id in ["_auto_send_1", "_auto_send_2"] {
        assert!(
            text.contains(&format!(r#""sendid":"{id}""#)),
            "a waiting send carries {id}: {text}"
        );
    }
    // The text the Kotlin suite writes for the same run, byte for byte: the
    // variables hold the ids the waiting sends are known by.
    assert_eq!(text, SHARED_ARMED.trim());
}

#[test]
fn a_restored_send_is_cancelled_by_the_id_the_document_holds() {
    let mut engine = restored();
    send(&mut engine, "cancel.first");
    engine.advance_time_ms(250);
    assert_eq!(engine.policy().first_fired(), 0, "its send was removed");
    assert_eq!(engine.policy().second_fired(), 1, "the other came due");
}

#[test]
fn a_restored_machine_numbers_its_next_ids_after_the_saved_count() {
    let mut engine = restored();
    // The ids the saved state holds are 1 and 2: `arm` again writes 3 and 4 to
    // the variables, and a `<cancel>` by `first` removes the send of id 3 and not
    // the one of id 1, which is still waiting.
    send(&mut engine, "arm");
    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    assert_eq!(saved.auto_send_seq, 4);
    assert!(
        saved
            .to_json()
            .contains(r#""first":"_auto_send_3","second":"_auto_send_4""#),
        "{}",
        saved.to_json()
    );

    send(&mut engine, "cancel.first");
    engine.advance_time_ms(250);
    assert_eq!(
        engine.policy().first_fired(),
        1,
        "the send of id 1 came due, the one of id 3 was removed"
    );
    assert_eq!(engine.policy().second_fired(), 2, "ids 2 and 4 came due");
}
