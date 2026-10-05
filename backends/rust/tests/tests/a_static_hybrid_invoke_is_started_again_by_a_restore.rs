// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring", for a hybrid `<invoke>`
// (§2.13): a saved state names the invocation whose child is running, and a
// restore starts it again from its beginning (W3C SCXML 6.4), as it does a
// static child — and what the start reads is read again, `srcexpr` among it, so
// the restored fields name the candidate as they would had the state been entered.
//
// `static_invoke_hybrid_saved.scxml` invokes `watch`, whose `srcexpr` is `pick`
// among `static_hybrid_watcher` (ends on the 8 it is handed) and
// `static_hybrid_holder` (ends on the 7). `base` is 7 when the child starts, and
// `bump` makes it 8 without telling the child; `swap` makes `pick` name the other
// candidate.
//
// Kotlin's half is `AStaticHybridInvokeIsStartedAgainByARestoreTest.kt`.

use sce_rust_runtime::saved_state::SavedState;
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

/// The machine saved and restored, as a host carries it through its own storage.
fn restored(
    engine: &Engine<StaticInvokeHybridSavedPolicy>,
) -> Engine<StaticInvokeHybridSavedPolicy> {
    let saved = engine.save_at(SAVED_AT_MS).expect("saves");
    Engine::<StaticInvokeHybridSavedPolicy>::restore_with(
        StaticInvokeHybridSavedPolicy::new(),
        &SavedState::from_json(&saved.to_json()).expect("reads"),
        SceClock::Manual(0),
        SAVED_AT_MS,
    )
    .expect("restores")
}

fn working(engine: &Engine<StaticInvokeHybridSavedPolicy>) -> bool {
    engine
        .get_active_states()
        .contains(&StaticInvokeHybridSavedState::Working)
}

#[test]
fn a_restored_child_is_handed_the_values_the_restored_machine_holds() {
    let mut engine = started();
    settle(&mut engine);
    // The watcher was handed 7 and waits for 8; `bump` makes `base` 8 and it is
    // not told.
    engine.raise_external_by_name("bump", "");
    settle(&mut engine);
    assert_eq!(engine.policy().completed(), 0);

    // A restored child is started again from its beginning, and a start
    // evaluates its arguments: the new watcher is handed 8, and ends.
    let mut restored = restored(&engine);
    settle(&mut restored);
    assert_eq!(restored.policy().completed(), 1);
}

#[test]
fn a_restore_reads_the_srcexpr_again() {
    let mut engine = started();
    settle(&mut engine);
    // The watcher is the child that is running when the machine is saved.
    engine.raise_external_by_name("swap", "");
    engine.raise_external_by_name("bump", "");
    settle(&mut engine);
    assert_eq!(engine.policy().completed(), 0);

    // The restored fields name the holder, which is handed 8 and waits for 7. The
    // watcher, which would have ended on that 8, is not what a restore starts.
    let mut restored = restored(&engine);
    settle(&mut restored);
    assert_eq!(restored.policy().completed(), 0, "the watcher was started");
    assert!(
        working(&restored),
        "the holder waits for a 7 it is not handed"
    );
}

#[test]
fn a_restored_child_of_the_other_candidate_is_saved_again() {
    let mut engine = started();
    settle(&mut engine);
    engine.raise_external_by_name("swap", "");
    engine.raise_external_by_name("bump", "");
    settle(&mut engine);
    // The watcher is running, in the slot of the candidate the value named when
    // the state was entered.
    assert_eq!(
        engine.save_at(SAVED_AT_MS).expect("saves").invokes,
        ["watch"]
    );

    // The restore started the holder, which keeps its child in a slot of its own
    // and waits for a 7 it is not handed: the invocation is still running, and a
    // second save names it.
    let mut restored = restored(&engine);
    settle(&mut restored);
    assert!(working(&restored));
    assert_eq!(
        restored.save_at(SAVED_AT_MS).expect("saves").invokes,
        ["watch"]
    );
}

#[test]
fn a_child_nothing_has_changed_for_is_handed_what_it_was_handed() {
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
    let mut engine = started();
    settle(&mut engine);
    engine.raise_external_by_name("bump", "");
    settle(&mut engine);
    let mut restored = restored(&engine);
    settle(&mut restored);
    assert_eq!(restored.policy().completed(), 1);

    // Saved once its `done.invoke` was taken, the machine is past `working`, and
    // a restore names no invocation to start.
    assert!(!working(&restored));
    let mut again = self::restored(&restored);
    settle(&mut again);
    assert_eq!(again.policy().completed(), 1, "the child ran twice");
    assert!(!working(&again));
}
