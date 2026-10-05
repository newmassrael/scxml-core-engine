// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": a machine hands the
// document an id for a `<send idlocation>`, the document keeps it, and may name
// it in a `<cancel>` long after. Two ids a machine generates must never be the
// same, and a restore must not start counting again: the document that was
// saved may still hold `_auto_send_1`, and a send waiting in the saved state
// carries it. A saved state says how many ids the machine has generated
// (`sendseq`), and a restore carries on from it.
//
// Driven on `SceClock::Manual`, no case sleeps. The shared instance
// `saved/statechart_static_host_invoke_running.json` is the text the Kotlin
// suite restores too; it has generated no id, so each case that needs a count
// writes one into it.

use sce_rust_runtime::saved_state::{SavedState, StateRefusal};
use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::host_processor::{
    StatechartStaticHostInvokePersist as Persist, StatechartStaticHostInvokePolicy as Policy,
};

const SHARED_RUNNING: &str = include_str!(
    "../../../../sce-build/tests/fixtures/host_processor/saved/statechart_static_host_invoke_running.json"
);

/// The wall-clock moment the shared instance was saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

fn started() -> Engine<Policy> {
    let mut engine = Engine::new(Policy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    engine
}

fn try_restore(text: &str) -> Result<Engine<Policy>, StateRefusal> {
    Engine::<Policy>::restore_with(
        Policy::new(),
        &SavedState::from_json(text)?,
        SceClock::Manual(0),
        SAVED_AT_MS,
    )
}

/// The shared instance with `sendseq` written as `count`.
fn shared_with_count(count: &str) -> String {
    SHARED_RUNNING.replace(r#""sendseq":"0""#, &format!(r#""sendseq":"{count}""#))
}

#[test]
fn a_machine_counts_the_ids_it_generates_from_one() {
    let mut engine = started();
    assert_eq!(engine.next_auto_send_id(), "_auto_send_1");
    assert_eq!(engine.next_auto_send_id(), "_auto_send_2");
    assert_eq!(engine.next_auto_send_id(), "_auto_send_3");
}

#[test]
fn two_machines_do_not_share_a_count() {
    let mut first = started();
    let mut second = started();
    assert_eq!(first.next_auto_send_id(), "_auto_send_1");
    assert_eq!(first.next_auto_send_id(), "_auto_send_2");
    assert_eq!(
        second.next_auto_send_id(),
        "_auto_send_1",
        "an id does not depend on what else ran beside the machine"
    );
}

#[test]
fn the_count_is_saved_with_the_machine() {
    let mut engine = started();
    assert_eq!(
        engine.save_at(SAVED_AT_MS).expect("saves").auto_send_seq,
        0,
        "no id generated yet"
    );
    engine.next_auto_send_id();
    engine.next_auto_send_id();

    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    assert_eq!(saved.auto_send_seq, 2);
    assert!(
        saved.to_json().contains(r#""sendseq":"2""#),
        "{}",
        saved.to_json()
    );
}

#[test]
fn a_restore_carries_on_from_the_count_it_was_saved_with() {
    let mut engine = try_restore(&shared_with_count("5")).expect("restores");
    assert_eq!(
        engine.next_auto_send_id(),
        "_auto_send_6",
        "the id a saved machine already handed out is not handed out again"
    );
    assert_eq!(engine.next_auto_send_id(), "_auto_send_7");
}

#[test]
fn a_machine_restored_and_saved_again_before_it_is_driven_keeps_the_count() {
    let engine = try_restore(&shared_with_count("5")).expect("restores");
    assert_eq!(
        engine.save_at(SAVED_AT_MS).expect("saves").to_json(),
        shared_with_count("5").trim(),
        "repeating the round trip never loses the count"
    );
}

#[test]
fn the_send_count_is_required_and_must_be_a_count() {
    let swap = |from: &str, to: &str| SHARED_RUNNING.replace(from, to);
    for (what, text, expected) in [
        (
            "no sendseq",
            swap(r#""sendseq":"0","#, ""),
            "has no 'sendseq'",
        ),
        (
            "a count that is a number",
            swap(r#""sendseq":"0""#, r#""sendseq":0"#),
            "'sendseq' is not a text",
        ),
        (
            "a count that is not digits",
            swap(r#""sendseq":"0""#, r#""sendseq":"-1""#),
            "'sendseq' (-1) is not a whole number",
        ),
        (
            "a count past a signed 64-bit count",
            swap(r#""sendseq":"0""#, r#""sendseq":"9223372036854775808""#),
            "'sendseq' (9223372036854775808) is not a whole number",
        ),
    ] {
        let refusal = SavedState::from_json(&text).expect_err(what);
        assert!(refusal.reason().contains(expected), "{what}: {refusal}");
    }
}
