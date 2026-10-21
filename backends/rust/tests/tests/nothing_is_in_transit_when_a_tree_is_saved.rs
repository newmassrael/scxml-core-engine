// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0045, section 3):
// nothing is in transit between two sessions when a tree is saved. What a child
// sends `#_parent` while a drive of it runs reaches its parent within that drive,
// whichever drive it is: the parent's start or tick of it, or an event the parent
// delivers to it, which the child takes then and there.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml, a
// parent whose in-place child `worker` is forwarded what the parent takes, answers
// `question` at once, and runs a grandchild under the hybrid `helper`.

use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_invoke_tree_sm::{
    StaticInvokeTreePersist, StaticInvokeTreePolicy,
};

/// The session id the shared format-2 fixtures give their root.
const ROOT: &str = "fixture-root";

/// The wall-clock moment the tree is saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

fn started() -> Engine<StaticInvokeTreePolicy> {
    let mut engine =
        Engine::new(StaticInvokeTreePolicy::new_with_session_id(ROOT).expect("an Nmtoken"));
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    engine
}

/// The host raises `event` and drives one `step`: the macrostep alone, no tick.
fn step(engine: &mut Engine<StaticInvokeTreePolicy>, event: &str) {
    engine.raise_external_by_name(event, "");
    engine.step();
}

/// The host raises `event` and drives one `tick`: due timers, the children, then
/// the macrostep.
fn tick(engine: &mut Engine<StaticInvokeTreePolicy>, event: &str) {
    engine.raise_external_by_name(event, "");
    engine.tick();
}

fn saved(engine: &Engine<StaticInvokeTreePolicy>) -> SavedState {
    engine
        .save_at(SAVED_AT_MS)
        .unwrap_or_else(|refusal| panic!("the tree saves at every boundary: {refusal}"))
}

#[test]
fn a_childs_answer_to_a_send_made_at_once_reaches_its_parent_within_the_send() {
    // `ask` sends `question` to the child at once, and the child answers at
    // once. A host that drives with `step` alone finds the answer taken in the
    // step that asked, not waiting in the child's buffer for a tick that has not
    // come, and the tree saves.
    let mut engine = started();
    step(&mut engine, "ask");
    assert_eq!(
        engine.policy().answers(),
        1,
        "the parent took the answer in the step that asked"
    );
    let text = saved(&engine).to_json();
    // The root's own queue is the last member of the text, and it is empty: the
    // answer was taken, not left behind.
    assert!(
        text.ends_with(r#""external":[]}"#),
        "the parent's queue is empty: {text}"
    );
}

#[test]
fn a_save_after_any_host_call_finds_nothing_in_transit() {
    // The transit refusal is never reached by a host: after each call of a run
    // that drives the child every way there is (a send made at once, a delayed
    // one armed, a forwarded event, a tick, a start again), the tree saves, and
    // the text it saves restores.
    let mut engine = started();
    let run: [(&str, bool); 12] = [
        ("ask", false),
        ("go", false),
        ("ask", true),
        ("swap", false),
        ("ask", false),
        ("arm", true),
        ("ask", true),
        ("rest", false),
        ("back", false),
        ("ask", false),
        ("go", true),
        ("ask", false),
    ];
    for (event, ticked) in run {
        if ticked {
            tick(&mut engine, event);
        } else {
            step(&mut engine, event);
        }
        let text = saved(&engine).to_json();
        Engine::<StaticInvokeTreePolicy>::restore_with(
            StaticInvokeTreePolicy::new(),
            &SavedState::from_json(&text).expect("reads"),
            SceClock::Manual(0),
            SAVED_AT_MS,
        )
        .unwrap_or_else(|refusal| panic!("after `{event}`, the saved tree restores: {refusal}"));
    }
    assert_eq!(engine.policy().answers(), 6, "every question was answered");
}
