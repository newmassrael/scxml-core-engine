// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": a child session an
// `<invoke type="scxml">` started is part of what a machine is doing, and the
// process that ran it is gone. A saved state lists the invocations whose child
// is running (`invokes`) and a restore starts each again from the beginning of
// its child, under the same id (W3C SCXML 6.4).
//
// `static_invoke.scxml` invokes `worker`, a child that takes `a` and then `b`
// and ends; the parent forwards what it is sent (`autoforward`), so the host
// drives the child through the parent and every case is read from the parent:
// `done.invoke.worker` counts the run in `completed` and leaves the parent in
// `working`. A restored machine's child starts over, so `b` alone does not end
// it where `a` then `b` does, and a child that had ended is not started again.
//
// The shared instance `saved/static_invoke_working.json` is the text the
// Kotlin suite writes and restores too.

use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_invoke_sm::{
    StaticInvokePersist, StaticInvokePolicy, StaticInvokeState,
};

const SHARED_WORKING: &str = include_str!(
    "../../../../sce-build/tests/fixtures/static_datamodel/saved/static_invoke_working.json"
);

/// The wall-clock moment the shared instance was saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

fn started() -> Engine<StaticInvokePolicy> {
    let mut engine = Engine::new(StaticInvokePolicy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    engine
}

fn restored(text: &str) -> Engine<StaticInvokePolicy> {
    try_restore(text).expect("restores")
}

fn try_restore(
    text: &str,
) -> Result<Engine<StaticInvokePolicy>, sce_rust_runtime::saved_state::StateRefusal> {
    Engine::<StaticInvokePolicy>::restore_with(
        StaticInvokePolicy::new(),
        &SavedState::from_json(text)?,
        SceClock::Manual(0),
        SAVED_AT_MS,
    )
}

/// Send `event` to the parent, which forwards it to its child, and let the
/// child run.
fn send(engine: &mut Engine<StaticInvokePolicy>, event: &str) {
    engine.raise_external_by_name(event, "");
    for _ in 0..3 {
        engine.tick();
    }
}

fn working(engine: &Engine<StaticInvokePolicy>) -> bool {
    engine
        .get_active_states()
        .contains(&StaticInvokeState::Working)
}

#[test]
fn a_machine_is_saved_with_the_child_it_is_running() {
    let engine = started();
    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    assert_eq!(saved.invokes, ["worker"]);
    // The text the Kotlin suite writes for the same machine, byte for byte.
    assert_eq!(saved.to_json(), SHARED_WORKING.trim());
}

#[test]
fn a_restored_machine_starts_its_child_again_from_the_beginning() {
    let mut engine = restored(SHARED_WORKING);
    assert!(working(&engine));

    // The child is at `first`: `b` is not what it waits for.
    send(&mut engine, "b");
    assert!(
        working(&engine),
        "`b` alone does not end a child that starts over"
    );
    assert_eq!(engine.policy().completed(), 0);

    send(&mut engine, "a");
    send(&mut engine, "b");
    assert_eq!(
        engine.policy().completed(),
        1,
        "`a` then `b` ends it, and `done.invoke.worker` arrived once"
    );
}

#[test]
fn a_child_that_had_taken_an_event_is_started_over_not_resumed() {
    // Its progress is not saved: the child had left `first` for `second`, and a
    // restored one is at `first` again.
    let mut before = started();
    send(&mut before, "a");
    let saved = before.save_at(SAVED_AT_MS).expect("saves");
    assert_eq!(saved.invokes, ["worker"], "it was running");

    let mut after = restored(&saved.to_json());
    send(&mut after, "b");
    assert!(
        working(&after),
        "a child resumed at `second` would have ended on `b`"
    );
    send(&mut after, "a");
    send(&mut after, "b");
    assert_eq!(after.policy().completed(), 1);
}

#[test]
fn a_child_that_has_ended_is_not_started_again() {
    // `done.invoke.worker` leaves the parent in `working`, so a machine saved
    // after its child ended stands in the state that invokes it with nothing
    // running. The invocation is complete: it is not listed, and a restore does
    // not run it a second time.
    let mut before = started();
    send(&mut before, "a");
    send(&mut before, "b");
    assert_eq!(before.policy().completed(), 1);
    assert!(working(&before));
    let saved = before.save_at(SAVED_AT_MS).expect("saves");
    assert!(
        saved.invokes.is_empty(),
        "the child ended: {:?}",
        saved.invokes
    );

    let mut after = restored(&saved.to_json());
    assert_eq!(after.policy().completed(), 1, "the run is kept");
    send(&mut after, "a");
    send(&mut after, "b");
    assert_eq!(
        after.policy().completed(),
        1,
        "a child started again would have ended again"
    );
}

#[test]
fn a_machine_that_left_the_invoking_state_saves_no_invocation() {
    let mut engine = started();
    send(&mut engine, "abort");
    assert!(!working(&engine));
    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    assert!(
        saved.invokes.is_empty(),
        "cancelled with its state: {:?}",
        saved.invokes
    );

    // And nothing is started when it comes back: no child is there to take `a`
    // and `b`, and `idle` invokes nothing.
    let mut again = restored(&saved.to_json());
    send(&mut again, "a");
    send(&mut again, "b");
    assert!(!again.is_in_final_state());
}

#[test]
fn an_invocation_the_saved_state_does_not_list_is_not_started() {
    // `working` with no running child: a state that says so is a state whose
    // child ended, and starting it again would run the invocation twice.
    let text = SHARED_WORKING.replace(r#""invokes":["worker"]"#, r#""invokes":[]"#);
    let mut engine = restored(&text);
    assert!(working(&engine));
    send(&mut engine, "a");
    send(&mut engine, "b");
    assert!(working(&engine), "nobody is there to take them");
    assert_eq!(engine.policy().completed(), 0);
}

#[test]
fn an_event_saved_in_the_queue_reaches_the_child_the_restore_started() {
    // The saved external queue and the restarted child are restored together:
    // `a` was raised to the machine before it was saved, and it is the
    // restarted child that takes it when the parent forwards it.
    let text = SHARED_WORKING.replace(
        r#""external":[]"#,
        r#""external":[{"name":"a","data":"","type":"external","sendid":"","origin":"","origintype":"","invokeid":""}]"#,
    );
    let mut engine = restored(&text);
    // `a` was queued before the machine was saved and is forwarded to the child
    // the restart made.
    for _ in 0..3 {
        engine.tick();
    }
    send(&mut engine, "b");
    assert_eq!(
        engine.policy().completed(),
        1,
        "the queued `a` reached the restarted child"
    );
}

#[test]
fn a_saved_state_naming_an_invocation_the_document_cannot_have_run_is_refused() {
    for (what, text, expected) in [
        (
            "an id the document does not invoke",
            SHARED_WORKING.replace(r#""invokes":["worker"]"#, r#""invokes":["stranger"]"#),
            "which the document does not invoke",
        ),
        (
            "an id named twice",
            SHARED_WORKING.replace(
                r#""invokes":["worker"]"#,
                r#""invokes":["worker","worker"]"#,
            ),
            "which an earlier entry already names",
        ),
        (
            "an invocation whose state the configuration is not in",
            SHARED_WORKING
                .replace(
                    r#""configuration":["working"]"#,
                    r#""configuration":["idle"]"#,
                )
                .replace(r#""current":"working""#, r#""current":"idle""#),
            "whose state the saved configuration does not stand in",
        ),
    ] {
        let refusal = try_restore(&text)
            .err()
            .unwrap_or_else(|| panic!("{what} restored"));
        assert!(refusal.reason().contains(expected), "{what}: {refusal}");
    }
}

#[test]
fn the_invokes_field_is_required_and_must_be_a_list_of_ids() {
    for (what, text, expected) in [
        (
            "absent",
            SHARED_WORKING.replace(r#""invokes":["worker"],"#, ""),
            "has no 'invokes'",
        ),
        (
            "not an array",
            SHARED_WORKING.replace(r#""invokes":["worker"]"#, r#""invokes":"worker""#),
            "'invokes' is not an array",
        ),
        (
            "an element that is not a text",
            SHARED_WORKING.replace(r#""invokes":["worker"]"#, r#""invokes":[5]"#),
            "'invokes' is not a text",
        ),
    ] {
        let refusal = SavedState::from_json(&text).expect_err(what);
        assert!(refusal.reason().contains(expected), "{what}: {refusal}");
    }
}
