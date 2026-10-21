// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring", for a hybrid `<invoke>`
// (§2.13). A format-2 saved state records the candidate that was running, with
// its whole state, and a restore resumes it whatever `srcexpr` reads now: the
// arguments were evaluated when the invocation started (§scxml-6.4), and a
// resumed child did not start (docs/adr/0045). A format-1 state named the
// invocation alone, and its restore starts it again from its beginning,
// reading `srcexpr` and the arguments from the restored variables as entering
// the state would: the one case in which a child restarts.
//
// `static_invoke_hybrid_saved.scxml` invokes `watch`, whose `srcexpr` is `pick`
// among `static_hybrid_watcher` (ends on the 8 it is handed) and
// `static_hybrid_holder` (ends on the 7). `base` is 7 when the child starts, and
// `bump` makes it 8 without telling the child; `swap` makes `pick` name the other
// candidate.
//
// Kotlin's half is `AStaticHybridInvokeIsStartedAgainByARestoreTest.kt`.

use sce_rust_runtime::saved_state::{SavedInvokes, SavedState};
use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_invoke_hybrid_saved_sm::{
    StaticInvokeHybridSavedPersist, StaticInvokeHybridSavedPolicy, StaticInvokeHybridSavedState,
};

/// The wall-clock moment the machine is saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

fn started() -> Engine<StaticInvokeHybridSavedPolicy> {
    let mut engine = Engine::new(StaticInvokeHybridSavedPolicy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    engine
}

/// Let the child run and report to its parent.
fn settle(engine: &mut Engine<StaticInvokeHybridSavedPolicy>) {
    for _ in 0..5 {
        engine.tick();
    }
}

fn saved_of(engine: &Engine<StaticInvokeHybridSavedPolicy>) -> SavedState {
    engine.save_at(SAVED_AT_MS).expect("saves")
}

fn restored_from(saved: &SavedState) -> Engine<StaticInvokeHybridSavedPolicy> {
    Engine::<StaticInvokeHybridSavedPolicy>::restore_with(
        StaticInvokeHybridSavedPolicy::new(),
        &SavedState::from_json(&saved.to_json()).expect("reads"),
        SceClock::Manual(0),
        SAVED_AT_MS,
    )
    .expect("restores")
}

/// The machine saved and restored, as a host carries it through its own storage.
fn restored(
    engine: &Engine<StaticInvokeHybridSavedPolicy>,
) -> Engine<StaticInvokeHybridSavedPolicy> {
    restored_from(&saved_of(engine))
}

/// The same moment as a format-1 state, which names the running invocation by
/// its id alone. The document holds no enum, so its format-1 shape is the
/// format-2 one the save wrote.
fn as_format_1(saved: &SavedState) -> SavedState {
    SavedState {
        invokes: SavedInvokes::Restarted(
            saved
                .invokes
                .ids()
                .into_iter()
                .map(str::to_string)
                .collect(),
        ),
        session_id: String::new(),
        invoke_starts: Vec::new(),
        ..saved.clone()
    }
}

/// The document each running child of `saved` runs.
fn documents(saved: &SavedState) -> Vec<String> {
    match &saved.invokes {
        SavedInvokes::Resumed(children) => children.iter().map(|c| c.document.clone()).collect(),
        SavedInvokes::Restarted(_) => panic!("a format-2 state names each child's document"),
    }
}

fn working(engine: &Engine<StaticInvokeHybridSavedPolicy>) -> bool {
    engine
        .get_active_states()
        .contains(&StaticInvokeHybridSavedState::Working)
}

#[test]
fn a_restored_child_keeps_the_values_it_was_handed() {
    let mut engine = started();
    settle(&mut engine);
    // The watcher was handed 7 and waits for 8; `bump` makes `base` 8 and it is
    // not told.
    engine.raise_external_by_name("bump", "");
    settle(&mut engine);
    assert_eq!(engine.policy().completed(), 0);

    // Resumed, the watcher holds the 7 it was handed: nothing is evaluated again,
    // so it still waits.
    let mut restored = restored(&engine);
    settle(&mut restored);
    assert_eq!(restored.policy().completed(), 0);
    assert!(working(&restored));
}

#[test]
fn a_restore_resumes_the_candidate_that_was_running_whatever_srcexpr_reads_now() {
    let mut engine = started();
    settle(&mut engine);
    // The watcher is the child that is running when the machine is saved, though
    // `pick` now names the holder.
    engine.raise_external_by_name("swap", "");
    engine.raise_external_by_name("bump", "");
    settle(&mut engine);
    assert_eq!(engine.policy().completed(), 0);
    let saved = saved_of(&engine);
    assert_eq!(documents(&saved), ["static_hybrid_watcher"]);

    let mut restored = restored_from(&saved);
    settle(&mut restored);
    assert_eq!(
        documents(&saved_of(&restored)),
        ["static_hybrid_watcher"],
        "the watcher was resumed, not the holder `pick` names now"
    );
    assert!(
        working(&restored),
        "the watcher waits for an 8 it was not handed"
    );
}

#[test]
fn a_child_nothing_has_changed_for_waits_as_it_did() {
    let mut engine = started();
    settle(&mut engine);
    let mut restored = restored(&engine);
    settle(&mut restored);
    // `base` is still 7: the watcher waits for an 8, as it did.
    assert_eq!(restored.policy().completed(), 0);
    assert!(working(&restored));
}

#[test]
fn a_child_that_ended_is_not_started_again() {
    // In format 1 the watcher started again is handed the 8 `base` holds, and ends.
    let mut engine = started();
    settle(&mut engine);
    engine.raise_external_by_name("bump", "");
    settle(&mut engine);
    let mut restarted = restored_from(&as_format_1(&saved_of(&engine)));
    settle(&mut restarted);
    assert_eq!(restarted.policy().completed(), 1);

    // Saved once its `done.invoke` was taken, the machine is past `working`, and
    // a restore names no invocation to resume or start.
    assert!(!working(&restarted));
    let mut again = restored(&restarted);
    settle(&mut again);
    assert_eq!(again.policy().completed(), 1, "the child ran twice");
    assert!(!working(&again));
}

// ── format 1: the invocation named by its id is started again ───────────────

#[test]
fn a_format_1_restore_hands_the_child_the_values_the_restored_machine_holds() {
    let mut engine = started();
    settle(&mut engine);
    engine.raise_external_by_name("bump", "");
    settle(&mut engine);
    assert_eq!(engine.policy().completed(), 0);

    // Started again from its beginning, the watcher is handed 8, and ends.
    let mut restarted = restored_from(&as_format_1(&saved_of(&engine)));
    settle(&mut restarted);
    assert_eq!(restarted.policy().completed(), 1);
}

#[test]
fn a_format_1_restore_reads_the_srcexpr_again() {
    let mut engine = started();
    settle(&mut engine);
    engine.raise_external_by_name("swap", "");
    engine.raise_external_by_name("bump", "");
    settle(&mut engine);
    assert_eq!(engine.policy().completed(), 0);

    // The restored fields name the holder, which is handed 8 and waits for 7. The
    // watcher, which would have ended on that 8, is not what the restore starts.
    let mut restarted = restored_from(&as_format_1(&saved_of(&engine)));
    settle(&mut restarted);
    assert_eq!(restarted.policy().completed(), 0, "the watcher was started");
    assert!(
        working(&restarted),
        "the holder waits for a 7 it is not handed"
    );
    // The holder keeps its child in a slot of its own: the invocation is still
    // running, and a save names it.
    assert_eq!(documents(&saved_of(&restarted)), ["static_hybrid_holder"]);
}
