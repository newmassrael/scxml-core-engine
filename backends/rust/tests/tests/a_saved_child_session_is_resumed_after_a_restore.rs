// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0045): a child
// session an `<invoke type="scxml">` started is part of what a machine is doing,
// so a saved state holds each running child inside its parent, with its whole
// state, and a restore resumes it where it stood. Nothing a child did at its
// start happens again, and it raises its `done.invoke` once, when it ends.
//
// `static_invoke.scxml` invokes `worker`, a child that takes `a` and then `b`
// and ends; the parent forwards what it is sent (`autoforward`), so the host
// drives the child through the parent and every case is read from the parent:
// `done.invoke.worker` counts the run in `completed` and leaves the parent in
// `working`. A child that had taken `a` is resumed at `second`, so `b` alone
// ends it.
//
// The shared instance `saved/static_invoke_working.json` is the format-2 text of
// the machine just started, its root given the id the fixtures name it by. A
// format-1 text holds the child's id alone, and the build that wrote it promised
// to start the child again from its beginning: `saved/format1/static_invoke_working.json`
// is the one case in which a restore does (docs/adr/0045, section 3).

use sce_rust_runtime::saved_state::{SavedInvokes, SavedState};
use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_invoke_sm::{
    StaticInvokePersist, StaticInvokePolicy, StaticInvokeState,
};

const SHARED_WORKING: &str = include_str!(
    "../../../../sce-build/tests/fixtures/static_datamodel/saved/static_invoke_working.json"
);
const FORMAT1_WORKING: &str = include_str!(
    "../../../../sce-build/tests/fixtures/static_datamodel/saved/format1/static_invoke_working.json"
);

/// The session id the shared format-2 fixtures give their root.
const ROOT: &str = "fixture-root";

/// The wall-clock moment the shared instance was saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

fn started() -> Engine<StaticInvokePolicy> {
    let mut engine =
        Engine::new(StaticInvokePolicy::new_with_session_id(ROOT).expect("an Nmtoken"));
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
fn a_machine_is_saved_with_the_whole_state_of_the_child_it_is_running() {
    let engine = started();
    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    let SavedInvokes::Resumed(children) = &saved.invokes else {
        panic!("a format-2 state: {:?}", saved.invokes)
    };
    let [child] = children.as_slice() else {
        panic!("one running child: {children:?}")
    };
    assert_eq!(child.id, "worker");
    assert_eq!(child.start, 1, "its first start");
    assert_eq!(saved.invoke_starts, [("worker".to_string(), 1)]);
    // Its id is this session's and the id it was started under: the tree's alone.
    assert_eq!(child.invoke_id, "working.1.worker");
    assert_eq!(child.session.session_id, "fixture-root.working.1.worker");
    assert_eq!(child.session.current, "first");
    // The text every backend writes for the same machine, byte for byte.
    assert_eq!(saved.to_json(), SHARED_WORKING.trim());
}

#[test]
fn a_child_that_had_taken_an_event_is_resumed_with_it_taken() {
    let mut before = started();
    send(&mut before, "a");
    let saved = before.save_at(SAVED_AT_MS).expect("saves");

    // Resumed at `second`: `b` alone ends it, and its `done.invoke` arrives once.
    let mut after = restored(&saved.to_json());
    assert!(working(&after));
    send(&mut after, "b");
    assert_eq!(
        after.policy().completed(),
        1,
        "the child resumed where it stood, and ended on `b`"
    );
    send(&mut after, "b");
    assert_eq!(after.policy().completed(), 1, "it ended once");
}

#[test]
fn a_restored_tree_saves_the_text_it_was_restored_from() {
    let engine = restored(SHARED_WORKING);
    assert_eq!(
        engine.save_at(SAVED_AT_MS).expect("saves").to_json(),
        SHARED_WORKING.trim(),
        "the child kept its id, its start and where it stood"
    );
}

#[test]
fn the_count_of_starts_is_carried_through_a_restore() {
    // The resumed child was not started again, and when it ends the count the
    // restore carried on is still its one start.
    let mut before = started();
    send(&mut before, "a");
    let mut after = restored(&before.save_at(SAVED_AT_MS).expect("saves").to_json());
    send(&mut after, "b");
    assert_eq!(after.policy().completed(), 1);
    let again = after.save_at(SAVED_AT_MS).expect("saves");
    assert_eq!(again.invoke_starts, [("worker".to_string(), 1)]);
    assert!(
        again.invokes.ids().is_empty(),
        "the child ended: {:?}",
        again.invokes
    );
}

#[test]
fn a_child_that_has_ended_is_not_resumed_or_started_again() {
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
        saved.invokes.ids().is_empty(),
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
        saved.invokes.ids().is_empty(),
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
fn an_event_saved_in_the_childs_queue_is_taken_by_the_resumed_child() {
    // `a` reached the child's own queue and the child had not taken it when the
    // tree was saved: it is in the child's `external`, and the resumed child takes
    // it, then ends on `b`.
    let text = SHARED_WORKING.trim().replacen(
        r#""configuration":["first"],"current":"first","variables":{},"history":{},"pending":[],"invokes":[],"invokestarts":{},"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]"#,
        r#""configuration":["first"],"current":"first","variables":{},"history":{},"pending":[],"invokes":[],"invokestarts":{},"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[{"name":"a","data":"","type":"external","sendid":"","origin":"","origintype":"","invokeid":"","hostinvoketoken":null}]"#,
        1,
    );
    assert_ne!(text, SHARED_WORKING.trim(), "the child's queue was changed");
    let mut engine = restored(&text);
    for _ in 0..3 {
        engine.tick();
    }
    send(&mut engine, "b");
    assert_eq!(
        engine.policy().completed(),
        1,
        "the queued `a` reached the resumed child"
    );
}

#[test]
fn a_tree_whose_child_is_not_one_the_document_could_run_is_refused_whole() {
    for (what, from, to, expected) in [
        (
            "a child the document does not invoke",
            r#""id":"worker""#,
            r#""id":"stranger""#,
            "which the document does not invoke",
        ),
        (
            "a start that is not the count its invoke holds",
            r#""start":"1""#,
            r#""start":"2""#,
            "the session counts 1 starts of it",
        ),
        (
            "a child of a document its invoke does not start",
            r#""document":"static_invoke__sce_synth_invoke__worker""#,
            r#""document":"static_hybrid_holder""#,
            "does not start",
        ),
        (
            "a child whose id is not its parent's and the id it was started under",
            r#""sessionid":"fixture-root.working.1.worker""#,
            r#""sessionid":"elsewhere""#,
            "a child of 'fixture-root' started under 'working.1.worker'",
        ),
        (
            "a child of another shape",
            "688ad00d954ad9f35bdaa5eae06768a788173e0f8a8236e670efa68038781a6d",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "the session at invokes[0].session: the saved state is of a document of shape",
        ),
        (
            "a child that stands in no state of its document",
            r#""configuration":["first"],"current":"first""#,
            r#""configuration":["nowhere"],"current":"nowhere""#,
            "the session at invokes[0].session: the document has no state 'nowhere'",
        ),
    ] {
        let text = SHARED_WORKING.trim().replacen(from, to, 1);
        assert_ne!(
            text,
            SHARED_WORKING.trim(),
            "{what}: the change did not apply"
        );
        let refusal = try_restore(&text)
            .err()
            .unwrap_or_else(|| panic!("{what} restored"));
        assert!(refusal.reason().contains(expected), "{what}: {refusal}");
    }
}

// ── format 1: a child named by its id is started again ─────────────────────

#[test]
fn a_format_1_state_starts_its_child_again_from_the_beginning() {
    // The only case in which a child restarts: the text holds nothing of it.
    let mut engine = restored(FORMAT1_WORKING);
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
fn a_format_1_state_naming_an_invocation_the_document_cannot_have_run_is_refused() {
    for (what, text, expected) in [
        (
            "an id the document does not invoke",
            FORMAT1_WORKING.replace(r#""invokes":["worker"]"#, r#""invokes":["stranger"]"#),
            "which the document does not invoke",
        ),
        (
            "an id named twice",
            FORMAT1_WORKING.replace(
                r#""invokes":["worker"]"#,
                r#""invokes":["worker","worker"]"#,
            ),
            "which an earlier entry already names",
        ),
        (
            "an invocation whose state the configuration is not in",
            FORMAT1_WORKING
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
fn the_invokes_field_of_a_format_1_state_is_required_and_a_list_of_ids() {
    for (what, text, expected) in [
        (
            "absent",
            FORMAT1_WORKING.replace(r#""invokes":["worker"],"#, ""),
            "has no 'invokes'",
        ),
        (
            "not an array",
            FORMAT1_WORKING.replace(r#""invokes":["worker"]"#, r#""invokes":"worker""#),
            "'invokes' is not an array",
        ),
        (
            "an element that is not a text",
            FORMAT1_WORKING.replace(r#""invokes":["worker"]"#, r#""invokes":[5]"#),
            "'invokes' is not a text",
        ),
    ] {
        let refusal = SavedState::from_json(&text).expect_err(what);
        assert!(refusal.reason().contains(expected), "{what}: {refusal}");
    }
}
